use core::arch::global_asm;
use core::cell::UnsafeCell;
use core::mem::size_of;
use core::sync::atomic::{AtomicUsize, Ordering};
use x86_64::instructions::interrupts as cpu_interrupts;
use x86_64::instructions::port::Port;
use x86_64::instructions::tables::lidt;
use x86_64::structures::DescriptorTablePointer;
use x86_64::VirtAddr;

const IDT_ENTRIES: usize = 256;
const CODE_SELECTOR: u16 = 0x08;
const INTERRUPT_GATE: u16 = 0x8e00;

const PIC_1_COMMAND: u16 = 0x20;
const PIC_1_DATA: u16 = 0x21;
const PIC_2_COMMAND: u16 = 0xa0;
const PIC_2_DATA: u16 = 0xa1;
const PIC_EOI: u8 = 0x20;

const PIC_1_OFFSET: u8 = 32;
const PIC_2_OFFSET: u8 = 40;
const KEYBOARD_IRQ: u8 = 1;
const KEYBOARD_VECTOR: usize = (PIC_1_OFFSET + KEYBOARD_IRQ) as usize;
const KEYBOARD_DATA_PORT: u16 = 0x60;

const KEY_BUFFER_SIZE: usize = 256;

global_asm!(
    r#"
    .att_syntax prefix
    .section .text.interrupts, "ax"
    .code64

    .macro PUSH_REGS
        pushq %rax
        pushq %rbx
        pushq %rcx
        pushq %rdx
        pushq %rsi
        pushq %rdi
        pushq %rbp
        pushq %r8
        pushq %r9
        pushq %r10
        pushq %r11
        pushq %r12
        pushq %r13
        pushq %r14
        pushq %r15
    .endm

    .macro POP_REGS
        popq %r15
        popq %r14
        popq %r13
        popq %r12
        popq %r11
        popq %r10
        popq %r9
        popq %r8
        popq %rbp
        popq %rdi
        popq %rsi
        popq %rdx
        popq %rcx
        popq %rbx
        popq %rax
    .endm

    .global keyboard_interrupt_stub
keyboard_interrupt_stub:
    PUSH_REGS
    movq %rsp, %rax
    andq $-16, %rsp
    subq $16, %rsp
    movq %rax, (%rsp)
    cld
    call keyboard_interrupt_handler
    movq (%rsp), %rsp
    POP_REGS
    iretq

    .global default_irq_stub
default_irq_stub:
    PUSH_REGS
    movq %rsp, %rax
    andq $-16, %rsp
    subq $16, %rsp
    movq %rax, (%rsp)
    cld
    call default_irq_handler
    movq (%rsp), %rsp
    POP_REGS
    iretq

    .global default_interrupt_stub
default_interrupt_stub:
    iretq

    .global default_exception_with_error_stub
default_exception_with_error_stub:
    addq $8, %rsp
    iretq
"#
);

unsafe extern "C" {
    fn keyboard_interrupt_stub();
    fn default_irq_stub();
    fn default_interrupt_stub();
    fn default_exception_with_error_stub();
}

#[derive(Clone, Copy)]
#[repr(C, packed)]
struct IdtEntry {
    offset_low: u16,
    selector: u16,
    ist: u16,
    options: u16,
    offset_mid: u16,
    offset_high: u32,
    reserved: u32,
}

impl IdtEntry {
    const fn missing() -> Self {
        Self {
            offset_low: 0,
            selector: 0,
            ist: 0,
            options: 0,
            offset_mid: 0,
            offset_high: 0,
            reserved: 0,
        }
    }

    fn set_handler(&mut self, handler: unsafe extern "C" fn()) {
        let address = handler as usize as u64;

        self.offset_low = address as u16;
        self.selector = CODE_SELECTOR;
        self.ist = 0;
        self.options = INTERRUPT_GATE;
        self.offset_mid = (address >> 16) as u16;
        self.offset_high = (address >> 32) as u32;
        self.reserved = 0;
    }
}

struct KeyBuffer {
    buffer: UnsafeCell<[u8; KEY_BUFFER_SIZE]>,
    read_index: AtomicUsize,
    write_index: AtomicUsize,
}

unsafe impl Sync for KeyBuffer {}

impl KeyBuffer {
    const fn new() -> Self {
        Self {
            buffer: UnsafeCell::new([0; KEY_BUFFER_SIZE]),
            read_index: AtomicUsize::new(0),
            write_index: AtomicUsize::new(0),
        }
    }

    fn push(&self, byte: u8) {
        let write = self.write_index.load(Ordering::Relaxed);
        let next_write = (write + 1) % KEY_BUFFER_SIZE;

        if next_write == self.read_index.load(Ordering::Acquire) {
            return;
        }

        unsafe {
            let ptr = self.buffer.get().cast::<u8>().add(write);
            ptr.write(byte);
        }

        self.write_index.store(next_write, Ordering::Release);
    }

    fn pop(&self) -> Option<u8> {
        let read = self.read_index.load(Ordering::Relaxed);

        if read == self.write_index.load(Ordering::Acquire) {
            return None;
        }

        let byte = unsafe {
            let ptr = self.buffer.get().cast::<u8>().add(read);
            ptr.read()
        };

        let next_read = (read + 1) % KEY_BUFFER_SIZE;
        self.read_index.store(next_read, Ordering::Release);

        Some(byte)
    }
}

static mut IDT: [IdtEntry; IDT_ENTRIES] = [IdtEntry::missing(); IDT_ENTRIES];
static KEY_BUFFER: KeyBuffer = KeyBuffer::new();

pub fn init() {
    cpu_interrupts::disable();

    unsafe {
        init_idt();
        remap_pic();
    }

    cpu_interrupts::enable();
}

pub fn pop_key() -> Option<u8> {
    KEY_BUFFER.pop()
}

unsafe fn init_idt() {
    let idt = core::ptr::addr_of_mut!(IDT).cast::<IdtEntry>();

    for index in 0..IDT_ENTRIES {
        unsafe {
            (*idt.add(index)).set_handler(default_interrupt_stub);
        }
    }

    for vector in [8usize, 10, 11, 12, 13, 14, 17, 21, 29, 30] {
        unsafe {
            (*idt.add(vector)).set_handler(default_exception_with_error_stub);
        }
    }

    for vector in PIC_1_OFFSET as usize..(PIC_2_OFFSET as usize + 8) {
        unsafe {
            (*idt.add(vector)).set_handler(default_irq_stub);
        }
    }

    unsafe {
        (*idt.add(KEYBOARD_VECTOR)).set_handler(keyboard_interrupt_stub);
    }

    let idt_ptr = DescriptorTablePointer {
        limit: (size_of::<[IdtEntry; IDT_ENTRIES]>() - 1) as u16,
        base: VirtAddr::from_ptr(core::ptr::addr_of!(IDT)),
    };

    unsafe {
        lidt(&idt_ptr);
    }
}

unsafe fn remap_pic() {
    let mut pic1_command = Port::<u8>::new(PIC_1_COMMAND);
    let mut pic1_data = Port::<u8>::new(PIC_1_DATA);
    let mut pic2_command = Port::<u8>::new(PIC_2_COMMAND);
    let mut pic2_data = Port::<u8>::new(PIC_2_DATA);

    unsafe {
        pic1_command.write(0x11);
        io_wait();
        pic2_command.write(0x11);
        io_wait();

        pic1_data.write(PIC_1_OFFSET);
        io_wait();
        pic2_data.write(PIC_2_OFFSET);
        io_wait();

        pic1_data.write(4);
        io_wait();
        pic2_data.write(2);
        io_wait();

        pic1_data.write(0x01);
        io_wait();
        pic2_data.write(0x01);
        io_wait();

        pic1_data.write(0b1111_1101);
        pic2_data.write(0xff);
    }
}

unsafe fn io_wait() {
    let mut wait_port = Port::<u8>::new(0x80);
    unsafe {
        wait_port.write(0);
    }
}

unsafe fn send_eoi(irq: u8) {
    if irq >= 8 {
        let mut slave_command = Port::<u8>::new(PIC_2_COMMAND);
        unsafe {
            slave_command.write(PIC_EOI);
        }
    }

    let mut master_command = Port::<u8>::new(PIC_1_COMMAND);
    unsafe {
        master_command.write(PIC_EOI);
    }
}

#[no_mangle]
pub extern "C" fn keyboard_interrupt_handler() {
    let mut keyboard_port = Port::<u8>::new(KEYBOARD_DATA_PORT);
    let scancode = unsafe { keyboard_port.read() };

    if let Some(byte) = scancode_to_ascii(scancode) {
        KEY_BUFFER.push(byte);
    }

    unsafe {
        send_eoi(KEYBOARD_IRQ);
    }
}

#[no_mangle]
pub extern "C" fn default_irq_handler() {
    unsafe {
        send_eoi(0);
    }
}

fn scancode_to_ascii(scancode: u8) -> Option<u8> {
    if scancode & 0x80 != 0 {
        return None;
    }

    match scancode {
        0x02 => Some(b'1'),
        0x03 => Some(b'2'),
        0x04 => Some(b'3'),
        0x05 => Some(b'4'),
        0x06 => Some(b'5'),
        0x07 => Some(b'6'),
        0x08 => Some(b'7'),
        0x09 => Some(b'8'),
        0x0a => Some(b'9'),
        0x0b => Some(b'0'),
        0x0c => Some(b'-'),
        0x0d => Some(b'='),
        0x0e => Some(8),
        0x0f => Some(b'\t'),
        0x10 => Some(b'q'),
        0x11 => Some(b'w'),
        0x12 => Some(b'e'),
        0x13 => Some(b'r'),
        0x14 => Some(b't'),
        0x15 => Some(b'y'),
        0x16 => Some(b'u'),
        0x17 => Some(b'i'),
        0x18 => Some(b'o'),
        0x19 => Some(b'p'),
        0x1a => Some(b'['),
        0x1b => Some(b']'),
        0x1c => Some(b'\n'),
        0x1e => Some(b'a'),
        0x1f => Some(b's'),
        0x20 => Some(b'd'),
        0x21 => Some(b'f'),
        0x22 => Some(b'g'),
        0x23 => Some(b'h'),
        0x24 => Some(b'j'),
        0x25 => Some(b'k'),
        0x26 => Some(b'l'),
        0x27 => Some(b';'),
        0x28 => Some(b'\''),
        0x29 => Some(b'`'),
        0x2b => Some(b'\\'),
        0x2c => Some(b'z'),
        0x2d => Some(b'x'),
        0x2e => Some(b'c'),
        0x2f => Some(b'v'),
        0x30 => Some(b'b'),
        0x31 => Some(b'n'),
        0x32 => Some(b'm'),
        0x33 => Some(b','),
        0x34 => Some(b'.'),
        0x35 => Some(b'/'),
        0x39 => Some(b' '),
        _ => None,
    }
}
