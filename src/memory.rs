use core::ptr::NonNull;
use volatile::VolatilePtr;

const GREEN_ON_BLACK: u8 = 0x0a;
const VGA_WIDTH: usize = 80;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct ScreenChar {
    pub ascii_character: u8,
    pub color_code: u8,
}

pub fn write_string(vga_buffer: VolatilePtr<'static, ScreenChar>, s: &str) {
    for (offset, character) in s.chars().take(VGA_WIDTH).enumerate() {
        let cell = unsafe {
            vga_buffer.map(|ptr| {
                let next = ptr.as_ptr().wrapping_add(offset);
                NonNull::new_unchecked(next)
            })
        };

        cell.write(ScreenChar {
            ascii_character: vga_byte(character),
            color_code: GREEN_ON_BLACK,
        });
    }
}

fn vga_byte(character: char) -> u8 {
    match character {
        '—' => 0xc4,
        character if character.is_ascii() => character as u8,
        _ => b'?',
    }
}
