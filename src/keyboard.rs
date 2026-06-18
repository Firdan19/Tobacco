use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use x86_64::instructions::port::Port;

const KEYBOARD_DATA_PORT: u16 = 0x60;
const KEYBOARD_STATUS_PORT: u16 = 0x64;
const KEY_BUFFER_SIZE: usize = 256;

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

static KEY_BUFFER: KeyBuffer = KeyBuffer::new();
static SHIFT_PRESSED: AtomicBool = AtomicBool::new(false);
static CAPS_LOCK: AtomicBool = AtomicBool::new(false);
static EXTENDED_SCANCODE: AtomicBool = AtomicBool::new(false);

pub fn init() {
    drain_output_buffer();
}

pub fn handle_interrupt() {
    let scancode = read_scancode();
    handle_scancode(scancode);
}

pub fn poll() {
    for _ in 0..16 {
        if !has_pending_scancode() {
            break;
        }

        handle_scancode(read_scancode());
    }
}

pub fn pop_key() -> Option<u8> {
    KEY_BUFFER.pop()
}

fn drain_output_buffer() {
    for _ in 0..32 {
        if !has_pending_scancode() {
            break;
        }

        let _ = read_scancode();
    }
}

fn has_pending_scancode() -> bool {
    let mut status_port = Port::<u8>::new(KEYBOARD_STATUS_PORT);
    let status = unsafe { status_port.read() };

    status & 0x01 != 0
}

fn read_scancode() -> u8 {
    let mut data_port = Port::<u8>::new(KEYBOARD_DATA_PORT);
    unsafe { data_port.read() }
}

fn handle_scancode(scancode: u8) {
    if scancode == 0xe0 {
        EXTENDED_SCANCODE.store(true, Ordering::Release);
        return;
    }

    if EXTENDED_SCANCODE.swap(false, Ordering::AcqRel) {
        return;
    }

    match scancode {
        0x2a | 0x36 => {
            SHIFT_PRESSED.store(true, Ordering::Release);
            return;
        }
        0xaa | 0xb6 => {
            SHIFT_PRESSED.store(false, Ordering::Release);
            return;
        }
        0x3a => {
            let enabled = CAPS_LOCK.load(Ordering::Acquire);
            CAPS_LOCK.store(!enabled, Ordering::Release);
            return;
        }
        _ => {}
    }

    if scancode & 0x80 != 0 {
        return;
    }

    if let Some(byte) = scancode_to_ascii(scancode) {
        KEY_BUFFER.push(byte);
    }
}

fn scancode_to_ascii(scancode: u8) -> Option<u8> {
    let shifted = SHIFT_PRESSED.load(Ordering::Acquire);
    let caps = CAPS_LOCK.load(Ordering::Acquire);

    match scancode {
        0x02 => Some(if shifted { b'!' } else { b'1' }),
        0x03 => Some(if shifted { b'@' } else { b'2' }),
        0x04 => Some(if shifted { b'#' } else { b'3' }),
        0x05 => Some(if shifted { b'$' } else { b'4' }),
        0x06 => Some(if shifted { b'%' } else { b'5' }),
        0x07 => Some(if shifted { b'^' } else { b'6' }),
        0x08 => Some(if shifted { b'&' } else { b'7' }),
        0x09 => Some(if shifted { b'*' } else { b'8' }),
        0x0a => Some(if shifted { b'(' } else { b'9' }),
        0x0b => Some(if shifted { b')' } else { b'0' }),
        0x0c => Some(if shifted { b'_' } else { b'-' }),
        0x0d => Some(if shifted { b'+' } else { b'=' }),
        0x0e => Some(8),
        0x0f => Some(b'\t'),
        0x10 => Some(letter(b'q', shifted, caps)),
        0x11 => Some(letter(b'w', shifted, caps)),
        0x12 => Some(letter(b'e', shifted, caps)),
        0x13 => Some(letter(b'r', shifted, caps)),
        0x14 => Some(letter(b't', shifted, caps)),
        0x15 => Some(letter(b'y', shifted, caps)),
        0x16 => Some(letter(b'u', shifted, caps)),
        0x17 => Some(letter(b'i', shifted, caps)),
        0x18 => Some(letter(b'o', shifted, caps)),
        0x19 => Some(letter(b'p', shifted, caps)),
        0x1a => Some(if shifted { b'{' } else { b'[' }),
        0x1b => Some(if shifted { b'}' } else { b']' }),
        0x1c => Some(b'\n'),
        0x1e => Some(letter(b'a', shifted, caps)),
        0x1f => Some(letter(b's', shifted, caps)),
        0x20 => Some(letter(b'd', shifted, caps)),
        0x21 => Some(letter(b'f', shifted, caps)),
        0x22 => Some(letter(b'g', shifted, caps)),
        0x23 => Some(letter(b'h', shifted, caps)),
        0x24 => Some(letter(b'j', shifted, caps)),
        0x25 => Some(letter(b'k', shifted, caps)),
        0x26 => Some(letter(b'l', shifted, caps)),
        0x27 => Some(if shifted { b':' } else { b';' }),
        0x28 => Some(if shifted { b'"' } else { b'\'' }),
        0x29 => Some(if shifted { b'~' } else { b'`' }),
        0x2b => Some(if shifted { b'|' } else { b'\\' }),
        0x2c => Some(letter(b'z', shifted, caps)),
        0x2d => Some(letter(b'x', shifted, caps)),
        0x2e => Some(letter(b'c', shifted, caps)),
        0x2f => Some(letter(b'v', shifted, caps)),
        0x30 => Some(letter(b'b', shifted, caps)),
        0x31 => Some(letter(b'n', shifted, caps)),
        0x32 => Some(letter(b'm', shifted, caps)),
        0x33 => Some(if shifted { b'<' } else { b',' }),
        0x34 => Some(if shifted { b'>' } else { b'.' }),
        0x35 => Some(if shifted { b'?' } else { b'/' }),
        0x39 => Some(b' '),
        _ => None,
    }
}

fn letter(lowercase: u8, shifted: bool, caps: bool) -> u8 {
    if shifted ^ caps {
        lowercase - 32
    } else {
        lowercase
    }
}
