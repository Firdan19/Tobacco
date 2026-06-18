#![no_std]
#![no_main]

use core::panic::PanicInfo;
use core::ptr::NonNull;
use volatile::VolatilePtr;
use x86_64::instructions::hlt;

mod memory;

core::arch::global_asm!(
    r#"
    .section .text
    .global _start
_start:
    cli
    lea rsp, [rip + stack_top]
    xor rbp, rbp
    call kernel_main

.halt:
    hlt
    jmp .halt

    .section .bss
    .align 16
stack_bottom:
    .skip 16384
stack_top:
"#
);

#[no_mangle]
pub extern "C" fn kernel_main() -> ! {
    let vga_ptr = unsafe { NonNull::new_unchecked(0xb8000 as *mut memory::ScreenChar) };
    let vga = unsafe { VolatilePtr::new(vga_ptr) };

    memory::write_string(vga, "CloudOS Kernel v0.0.1 — Booted");

    halt_loop();
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    halt_loop();
}

fn halt_loop() -> ! {
    loop {
        hlt();
    }
}
