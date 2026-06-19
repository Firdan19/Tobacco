use crate::interrupts;
use core::cell::UnsafeCell;
use x86_64::instructions::interrupts as cpu_interrupts;

pub const ENTRY_COUNT: usize = 128;
pub const TAG_CAPACITY: usize = 12;
pub const MESSAGE_CAPACITY: usize = 80;

#[derive(Clone, Copy)]
pub struct Snapshot {
    pub initialized: bool,
    pub capacity: u64,
    pub count: u64,
    pub dropped: u64,
    pub next_sequence: u64,
}

#[derive(Clone, Copy)]
pub struct Entry {
    pub sequence: u64,
    pub tick: u64,
    tag: [u8; TAG_CAPACITY],
    tag_len: u8,
    message: [u8; MESSAGE_CAPACITY],
    message_len: u8,
}

impl Entry {
    const fn empty() -> Self {
        Self {
            sequence: 0,
            tick: 0,
            tag: [0; TAG_CAPACITY],
            tag_len: 0,
            message: [0; MESSAGE_CAPACITY],
            message_len: 0,
        }
    }

    pub fn tag(&self) -> &[u8] {
        &self.tag[..self.tag_len as usize]
    }

    pub fn message(&self) -> &[u8] {
        &self.message[..self.message_len as usize]
    }
}

struct LogRing {
    initialized: bool,
    entries: [Entry; ENTRY_COUNT],
    write_index: usize,
    count: usize,
    dropped: u64,
    next_sequence: u64,
}

impl LogRing {
    const fn new() -> Self {
        Self {
            initialized: false,
            entries: [Entry::empty(); ENTRY_COUNT],
            write_index: 0,
            count: 0,
            dropped: 0,
            next_sequence: 0,
        }
    }

    fn init(&mut self) {
        self.write_index = 0;
        self.count = 0;
        self.dropped = 0;
        self.next_sequence = 0;
        self.initialized = true;
    }

    fn append(&mut self, tag: &[u8], message: &[u8]) {
        if !self.initialized {
            self.initialized = true;
        }

        let mut entry = Entry::empty();
        entry.sequence = self.next_sequence;
        entry.tick = interrupts::ticks();
        entry.tag_len = copy_sanitized(tag, &mut entry.tag) as u8;
        entry.message_len = copy_sanitized(message, &mut entry.message) as u8;

        self.entries[self.write_index] = entry;
        self.write_index = (self.write_index + 1) % ENTRY_COUNT;
        self.next_sequence = self.next_sequence.saturating_add(1);

        if self.count < ENTRY_COUNT {
            self.count += 1;
        } else {
            self.dropped = self.dropped.saturating_add(1);
        }
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            initialized: self.initialized,
            capacity: ENTRY_COUNT as u64,
            count: self.count as u64,
            dropped: self.dropped,
            next_sequence: self.next_sequence,
        }
    }

    fn entry(&self, index_from_oldest: usize) -> Option<Entry> {
        if index_from_oldest >= self.count {
            return None;
        }

        let oldest = if self.count < ENTRY_COUNT {
            0
        } else {
            self.write_index
        };
        let index = (oldest + index_from_oldest) % ENTRY_COUNT;

        Some(self.entries[index])
    }
}

struct LogStore {
    value: UnsafeCell<LogRing>,
}

unsafe impl Sync for LogStore {}

static KERNEL_LOG: LogStore = LogStore {
    value: UnsafeCell::new(LogRing::new()),
};

pub fn init() {
    cpu_interrupts::without_interrupts(|| ring_mut().init());
}

pub fn append(tag: &str, message: &str) {
    append_bytes(tag.as_bytes(), message.as_bytes());
}

pub fn append_labeled_bytes(tag: &str, label: &str, bytes: &[u8]) {
    let mut builder = MessageBuilder::new();
    builder.push_str(label);
    builder.push_str(": ");
    builder.push_bytes(bytes);
    append_bytes(tag.as_bytes(), builder.as_bytes());
}

pub fn append_u64(tag: &str, label: &str, value: u64) {
    let mut builder = MessageBuilder::new();
    builder.push_str(label);
    builder.push_str(": ");
    builder.push_u64(value);
    append_bytes(tag.as_bytes(), builder.as_bytes());
}

pub fn append_hex_u64(tag: &str, label: &str, value: u64) {
    let mut builder = MessageBuilder::new();
    builder.push_str(label);
    builder.push_str(": ");
    builder.push_hex_u64(value);
    append_bytes(tag.as_bytes(), builder.as_bytes());
}

pub fn append_bool(tag: &str, label: &str, enabled: bool) {
    let mut builder = MessageBuilder::new();
    builder.push_str(label);
    builder.push_str(": ");
    if enabled {
        builder.push_str("on");
    } else {
        builder.push_str("off");
    }
    append_bytes(tag.as_bytes(), builder.as_bytes());
}

pub fn snapshot() -> Snapshot {
    let mut snapshot = Snapshot {
        initialized: false,
        capacity: ENTRY_COUNT as u64,
        count: 0,
        dropped: 0,
        next_sequence: 0,
    };

    cpu_interrupts::without_interrupts(|| {
        snapshot = ring().snapshot();
    });

    snapshot
}

pub fn entry(index_from_oldest: usize) -> Option<Entry> {
    let mut result = None;

    cpu_interrupts::without_interrupts(|| {
        result = ring().entry(index_from_oldest);
    });

    result
}

fn append_bytes(tag: &[u8], message: &[u8]) {
    cpu_interrupts::without_interrupts(|| ring_mut().append(tag, message));
}

fn ring() -> &'static LogRing {
    unsafe { &*KERNEL_LOG.value.get() }
}

fn ring_mut() -> &'static mut LogRing {
    unsafe { &mut *KERNEL_LOG.value.get() }
}

fn copy_sanitized(source: &[u8], destination: &mut [u8]) -> usize {
    let mut len = 0;

    for byte in source.iter().copied() {
        if len >= destination.len() {
            break;
        }

        destination[len] = sanitize_byte(byte);
        len += 1;
    }

    len
}

fn sanitize_byte(byte: u8) -> u8 {
    match byte {
        b'\r' | b'\n' | b'\t' => b' ',
        0x20..=0x7e => byte,
        _ => b'.',
    }
}

struct MessageBuilder {
    bytes: [u8; MESSAGE_CAPACITY],
    len: usize,
}

impl MessageBuilder {
    const fn new() -> Self {
        Self {
            bytes: [0; MESSAGE_CAPACITY],
            len: 0,
        }
    }

    fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }

    fn push_str(&mut self, value: &str) {
        self.push_bytes(value.as_bytes());
    }

    fn push_bytes(&mut self, value: &[u8]) {
        for byte in value.iter().copied() {
            self.push_byte(byte);
        }
    }

    fn push_byte(&mut self, byte: u8) {
        if self.len < self.bytes.len() {
            self.bytes[self.len] = sanitize_byte(byte);
            self.len += 1;
        }
    }

    fn push_u64(&mut self, mut value: u64) {
        let mut digits = [0u8; 20];
        let mut index = digits.len();

        if value == 0 {
            self.push_byte(b'0');
            return;
        }

        while value > 0 {
            index -= 1;
            digits[index] = b'0' + (value % 10) as u8;
            value /= 10;
        }

        self.push_bytes(&digits[index..]);
    }

    fn push_hex_u64(&mut self, value: u64) {
        self.push_str("0x");

        let mut started = false;
        let mut shift = 60u64;

        loop {
            let nibble = ((value >> shift) & 0x0f) as u8;

            if nibble != 0 || started || shift == 0 {
                started = true;
                let byte = if nibble < 10 {
                    b'0' + nibble
                } else {
                    b'a' + (nibble - 10)
                };
                self.push_byte(byte);
            }

            if shift == 0 {
                break;
            }

            shift -= 4;
        }
    }
}
