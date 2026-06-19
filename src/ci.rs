use crate::{
    gdt, heap, interrupts, keyboard, klog, multiboot, paging, physmem, serial, shell, stats,
};

const CI_BOOT_FLAG: &[u8] = b"tobacco.ci=smoke";
const VGA_BUFFER_ADDRESS: u64 = 0x000b_8000;

pub fn run_if_requested() {
    let boot_info = multiboot::summary();
    let command_line = boot_info.command_line.as_str().as_bytes();

    if !contains_bytes(command_line, CI_BOOT_FLAG) {
        return;
    }

    serial::log("ci", "command smoke requested");
    run_command_table_checks();

    if run_selftest_checks() {
        serial::log("ci", "selftest status: PASS");
    } else {
        serial::log("ci", "selftest status: FAIL");
    }

    if run_stability_stress() {
        serial::log("ci", "stress status: PASS");
    } else {
        serial::log("ci", "stress status: FAIL");
    }

    serial::log("ci", "command smoke complete");
}

fn run_command_table_checks() {
    serial::log_u64("ci", "command table count", shell::command_count() as u64);
    check("command help", shell::command_exists(b"help"));
    check("command selftest", shell::command_exists(b"selftest"));
    check("command stress", shell::command_exists(b"stress"));
    check("command paging", shell::command_exists(b"paging"));
    check("command heap", shell::command_exists(b"heap"));
    check("command vmtest", shell::command_exists(b"vmtest"));
    check("command mem", shell::command_exists(b"mem"));
    check("command log", shell::command_exists(b"log"));
}

fn run_selftest_checks() -> bool {
    let boot_info = multiboot::summary();
    let memory = boot_info.memory;
    let frames = physmem::snapshot();
    let paging_state = paging::snapshot();
    let vga_translation = paging::translate(VGA_BUFFER_ADDRESS);
    let high_translation = paging::translate(0xffff_8000_0000_0000);
    let gdt = gdt::snapshot();
    let heap_snapshot = heap::snapshot();
    let log = klog::snapshot();
    let counters = stats::snapshot();
    let ticks = interrupts::ticks();

    let mut ok = true;

    ok &= check("selftest multiboot magic", boot_info.valid_magic);
    ok &= check(
        "selftest multiboot parsed",
        boot_info.parsed && boot_info.tag_count > 0,
    );
    ok &= check(
        "selftest memory map usable",
        memory.has_memory_map && multiboot::stored_region_count() > 0 && memory.usable_bytes > 0,
    );
    ok &= check(
        "selftest frame allocator ready",
        frames.initialized && frames.allocatable_frames > 0 && frames.free_frames > 0,
    );
    ok &= check(
        "selftest kernel memory protected",
        frames.kernel_end > frames.kernel_start && frames.protected_until >= frames.kernel_end,
    );
    ok &= check(
        "selftest paging initialized",
        paging_state.initialized && paging_state.mapper_initialized && paging_state.cr3 != 0,
    );
    ok &= check(
        "selftest boot identity map",
        paging_state.p4_present_entries >= 1
            && paging_state.p3_present_entries >= 1
            && paging_state.p2_present_entries >= 512
            && paging_state.huge_pages >= 512
            && paging_state.identity_mapped_bytes >= paging::BOOT_IDENTITY_MAP_BYTES,
    );
    ok &= check(
        "selftest vga translation",
        vga_translation.mapped
            && vga_translation.phys == VGA_BUFFER_ADDRESS
            && vga_translation.huge_page,
    );
    ok &= check("selftest high address unmapped", !high_translation.mapped);
    ok &= check(
        "selftest virtual map unmap",
        paging::probe_map_unmap(paging::KERNEL_VM_TEST_PAGE),
    );
    ok &= check(
        "selftest heap ready",
        heap_snapshot.initialized
            && heap_snapshot.mapped_pages == heap::HEAP_PAGES
            && heap_snapshot.remaining <= heap_snapshot.size
            && !paging::translate(heap_snapshot.guard_low).mapped
            && !paging::translate(heap_snapshot.guard_high).mapped,
    );
    ok &= check("selftest heap probe", heap::probe());
    ok &= check(
        "selftest gdt tss ist ready",
        gdt.loaded
            && gdt.code_selector != 0
            && gdt.data_selector != 0
            && gdt.tss_selector != 0
            && gdt.double_fault_stack_bytes >= 16 * 1024,
    );
    ok &= check(
        "selftest kernel log ready",
        log.initialized && log.capacity == klog::ENTRY_COUNT as u64 && log.count > 0,
    );
    ok &= check("selftest serial active", counters.serial_bytes > 0);
    ok &= check(
        "selftest vga active",
        counters.vga_clears > 0 && counters.vga_cell_writes > 0,
    );
    ok &= check("selftest timer sane", ticks >= counters.shell_ready_tick);
    ok &= check(
        "selftest keyboard queue sane",
        keyboard::pending_events() < 256,
    );
    ok &= check("selftest command table sane", shell::command_count() >= 26);

    ok
}

fn run_stability_stress() -> bool {
    let ticks_before = interrupts::ticks();
    let log_before = klog::snapshot();
    let serial_before = stats::snapshot().serial_bytes;
    let mut checksum = 0x544f_4241_4343_4f43u64;
    let mut paging_ok = true;

    for cycle in 0..4u64 {
        checksum = stress_cpu_round(checksum, cycle);

        if !stress_paging_round(cycle, &mut checksum) {
            paging_ok = false;
        }

        klog::append_u64("ci", "stress cycle", cycle);
    }

    for index in 0..(klog::ENTRY_COUNT + 8) {
        klog::append_u64("ci", "ring", index as u64);
    }

    let ticks_after = interrupts::ticks();
    let log_after = klog::snapshot();
    let serial_after = stats::snapshot().serial_bytes;

    let mut ok = true;
    ok &= check("stress paging translation", paging_ok);
    ok &= check(
        "stress log bounded",
        log_after.initialized
            && log_after.capacity == klog::ENTRY_COUNT as u64
            && log_after.count <= log_after.capacity,
    );
    ok &= check(
        "stress log wrap monotonic",
        log_after.next_sequence > log_before.next_sequence
            && log_after.dropped >= log_before.dropped,
    );
    ok &= check("stress timer monotonic", ticks_after >= ticks_before);
    ok &= check("stress serial monotonic", serial_after >= serial_before);
    ok &= check("stress checksum stable", checksum != 0);
    serial::log_hex_u64("ci", "stress checksum", checksum);

    ok
}

fn stress_cpu_round(mut checksum: u64, cycle: u64) -> u64 {
    for value in 0..1024u64 {
        checksum = checksum
            .rotate_left(7)
            .wrapping_add(value ^ cycle.rotate_left(5))
            .wrapping_mul(0x1000_0000_01b3);
        checksum = core::hint::black_box(checksum);
    }

    checksum
}

fn stress_paging_round(cycle: u64, checksum: &mut u64) -> bool {
    let rolling_address = (cycle % 512) * paging::HUGE_PAGE_SIZE;
    let mapped_addresses = [
        0,
        0x1000,
        VGA_BUFFER_ADDRESS,
        rolling_address,
        paging::BOOT_IDENTITY_MAP_BYTES - 1,
    ];
    let unmapped_addresses = [paging::BOOT_IDENTITY_MAP_BYTES, 0xffff_8000_0000_0000];

    for address in mapped_addresses.iter().copied() {
        let result = paging::translate(address);

        if !result.mapped || result.phys != address {
            return false;
        }

        *checksum ^= result.phys.rotate_left(((address >> 12) as u32 & 31) + 1);
    }

    for address in unmapped_addresses.iter().copied() {
        if paging::translate(address).mapped {
            return false;
        }
    }

    true
}

fn check(label: &str, ok: bool) -> bool {
    if ok {
        serial::log("ci", label);
    } else {
        serial::log("ci", "FAIL");
        serial::log("ci", label);
    }

    ok
}

fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }

    if needle.len() > haystack.len() {
        return false;
    }

    for start in 0..=(haystack.len() - needle.len()) {
        if &haystack[start..(start + needle.len())] == needle {
            return true;
        }
    }

    false
}
