use core::ptr::NonNull;
use volatile::VolatilePtr;

const WHITE_ON_BLUE: u8 = 0x1f;
const VGA_WIDTH: usize = 80;
const VGA_HEIGHT: usize = 25;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct ScreenChar {
    pub ascii_character: u8,
    pub color_code: u8,
}

struct Writer {
    row: usize,
    column: usize,
    color_code: u8,
    buffer: *mut ScreenChar,
}

impl Writer {
    const fn new() -> Self {
        Self {
            row: 0,
            column: 0,
            color_code: WHITE_ON_BLUE,
            buffer: 0xb8000 as *mut ScreenChar,
        }
    }

    fn clear_screen(&mut self) {
        for offset in 0..(VGA_WIDTH * VGA_HEIGHT) {
            self.write_cell(offset, b' ', self.color_code);
        }
        self.row = 0;
        self.column = 0;
    }

    fn write_centered(&mut self, row: usize, s: &str) {
        let width = s.bytes().count().min(VGA_WIDTH);
        let column = (VGA_WIDTH - width) / 2;

        self.write_string_at(row, column, s);
    }

    fn write_string_at(&mut self, row: usize, column: usize, s: &str) {
        if row >= VGA_HEIGHT || column >= VGA_WIDTH {
            return;
        }

        let start = row * VGA_WIDTH + column;
        let max_len = VGA_WIDTH - column;

        for (offset, byte) in s.bytes().take(max_len).enumerate() {
            self.write_cell(start + offset, vga_byte(byte), self.color_code);
        }
    }

    fn set_cursor(&mut self, row: usize, column: usize) {
        self.row = row.min(VGA_HEIGHT - 1);
        self.column = column.min(VGA_WIDTH - 1);
    }

    fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
    }

    fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            8 => self.backspace(),
            byte => {
                if self.column >= VGA_WIDTH {
                    self.new_line();
                }

                let offset = self.row * VGA_WIDTH + self.column;
                self.write_cell(offset, vga_byte(byte), self.color_code);
                self.column += 1;
            }
        }
    }

    fn new_line(&mut self) {
        self.column = 0;
        if self.row + 1 >= VGA_HEIGHT {
            self.scroll_up();
        } else {
            self.row += 1;
        }
    }

    fn backspace(&mut self) {
        if self.column > 0 {
            self.column -= 1;
        } else if self.row > 0 {
            self.row -= 1;
            self.column = VGA_WIDTH - 1;
        } else {
            return;
        }

        let offset = self.row * VGA_WIDTH + self.column;
        self.write_cell(offset, b' ', self.color_code);
    }

    fn scroll_up(&mut self) {
        for row in 1..VGA_HEIGHT {
            for column in 0..VGA_WIDTH {
                let from = row * VGA_WIDTH + column;
                let to = (row - 1) * VGA_WIDTH + column;
                let character = self.read_cell(from);
                self.write_cell(to, character.ascii_character, character.color_code);
            }
        }

        self.clear_row(VGA_HEIGHT - 1);
        self.row = VGA_HEIGHT - 1;
        self.column = 0;
    }

    fn clear_row(&mut self, row: usize) {
        let start = row * VGA_WIDTH;
        for column in 0..VGA_WIDTH {
            self.write_cell(start + column, b' ', self.color_code);
        }
    }

    fn read_cell(&self, offset: usize) -> ScreenChar {
        let cell = unsafe { VolatilePtr::new(NonNull::new_unchecked(self.buffer.add(offset))) };
        cell.read()
    }

    fn write_cell(&self, offset: usize, byte: u8, color_code: u8) {
        let cell = unsafe { VolatilePtr::new(NonNull::new_unchecked(self.buffer.add(offset))) };

        cell.write(ScreenChar {
            ascii_character: byte,
            color_code,
        });
    }
}

static mut WRITER: Writer = Writer::new();

pub fn init() {
    clear_screen();
}

pub fn show_splash() {
    with_writer(|writer| {
        writer.clear_screen();
        writer.write_centered(10, "CloudOS");
        writer.write_centered(12, "Kernel v0.0.1 - Booted");
        writer.write_string_at(16, 4, "Keyboard input ready:");
        writer.set_cursor(18, 4);
        writer.write_string("> ");
    });
}

pub fn write_byte(byte: u8) {
    with_writer(|writer| writer.write_byte(byte));
}

pub fn write_string(s: &str) {
    with_writer(|writer| writer.write_string(s));
}

pub fn clear_screen() {
    with_writer(|writer| writer.clear_screen());
}

fn with_writer<R>(f: impl FnOnce(&mut Writer) -> R) -> R {
    unsafe {
        let writer = core::ptr::addr_of_mut!(WRITER);
        f(&mut *writer)
    }
}

fn vga_byte(byte: u8) -> u8 {
    match byte {
        0x20..=0x7e => byte,
        b'\n' => byte,
        8 => byte,
        _ => b'?',
    }
}
