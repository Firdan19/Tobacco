use core::ptr::NonNull;
use volatile::VolatilePtr;

pub const VGA_BUFFER_WIDTH: usize = 80;
pub const VGA_BUFFER_HEIGHT: usize = 25;
pub const VGA_BUFFER_SIZE: usize = VGA_BUFFER_WIDTH * VGA_BUFFER_HEIGHT;

pub const COLOR_BLACK: u8 = 0x0;
pub const COLOR_LIGHT_GREEN: u8 = 0xa;
pub const COLOR_CODE: u8 = (COLOR_BLACK << 4) | COLOR_LIGHT_GREEN;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ScreenChar {
    pub ascii_character: u8,
    pub color_code: u8,
}

pub fn write_string(vga: VolatilePtr<'static, ScreenChar>, s: &str) {
    clear_row(vga, 0);

    for (column, character) in s.chars().take(VGA_BUFFER_WIDTH).enumerate() {
        write_byte(vga, column, vga_byte(character));
    }
}

fn clear_row(vga: VolatilePtr<'static, ScreenChar>, row: usize) {
    let start = row * VGA_BUFFER_WIDTH;
    let end = start + VGA_BUFFER_WIDTH;

    for index in start..end.min(VGA_BUFFER_SIZE) {
        write_at(vga, index, b' ');
    }
}

fn write_byte(vga: VolatilePtr<'static, ScreenChar>, column: usize, byte: u8) {
    if column < VGA_BUFFER_WIDTH {
        write_at(vga, column, byte);
    }
}

fn write_at(vga: VolatilePtr<'static, ScreenChar>, index: usize, byte: u8) {
    let cell = unsafe {
        vga.map(|ptr| {
            let next = ptr.as_ptr().wrapping_add(index);
            NonNull::new_unchecked(next)
        })
    };

    cell.write(ScreenChar {
        ascii_character: byte,
        color_code: COLOR_CODE,
    });
}

fn vga_byte(character: char) -> u8 {
    match character {
        '\n' => b' ',
        '—' => 0xc4,
        character if character.is_ascii() => character as u8,
        _ => b'?',
    }
}
