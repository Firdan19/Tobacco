#![no_std]
#![no_main]

mod memory;

use bootloader_api::{config::Mapping, entry_point, BootInfo, BootloaderConfig};
use core::panic::PanicInfo;
use core::ptr::NonNull;
use memory::{write_string, ScreenChar};
use volatile::VolatilePtr;
use x86_64::instructions::hlt;

pub static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.kernel_stack_size = 64 * 1024;
    config.mappings.physical_memory = Some(Mapping::FixedAddress(0));
    config
};

entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

fn kernel_main(_boot_info: &'static mut BootInfo) -> ! {
    let vga_ptr = NonNull::new(0xb8000 as *mut ScreenChar).expect("VGA buffer pointer is null");
    let vga = unsafe { VolatilePtr::new(vga_ptr) };

    write_string(vga, "CloudOS Kernel v0.0.1 — Booted");

    halt_loop();
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    let vga_ptr = NonNull::new(0xb8000 as *mut ScreenChar).expect("VGA buffer pointer is null");
    let vga = unsafe { VolatilePtr::new(vga_ptr) };

    write_string(vga, "PANIC");

    halt_loop();
}

fn halt_loop() -> ! {
    loop {
        hlt();
    }
}
