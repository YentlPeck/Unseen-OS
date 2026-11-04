//! PS/2 keyboard, read by polling (asking "any key?" in a loop).
//! Turns raw scancodes (set 1) into key presses.

use crate::port::inb;

#[derive(Clone, Copy, PartialEq)]
pub enum Key {
    Char(u8),
    Enter,
    Backspace,
    Tab,
    Escape,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    Delete,
    /// A letter pressed while Ctrl is held, e.g. Ctrl('k').
    Ctrl(u8),
}

const NORMAL: &[u8; 58] = b"\0\x1b1234567890-=\x08\tqwertyuiop[]\n\0asdfghjkl;'`\0\\zxcvbnm,./\0*\0 ";
const SHIFTED: &[u8; 58] = b"\0\x1b!@#$%^&*()_+\x08\tQWERTYUIOP{}\n\0ASDFGHJKL:\"~\0|ZXCVBNM<>?\0*\0 ";

pub struct Keyboard {
    shift: bool,
    ctrl: bool,
    extended: bool,
}

impl Keyboard {
    pub fn new() -> Self {
        Self { shift: false, ctrl: false, extended: false }
    }

    /// Returns a key press if one is waiting.
    pub fn poll(&mut self) -> Option<Key> {
        let status = inb(0x64);
        if status & 0x01 == 0 || status & 0x20 != 0 {
            return None; // nothing waiting, or it's mouse data
        }
        let code = inb(0x60);
        if code == 0xE0 {
            self.extended = true;
            return None;
        }
        let extended = core::mem::take(&mut self.extended);
        let released = code & 0x80 != 0;
        let code = code & 0x7F;

        if extended && (code == 0x2A || code == 0x36) {
            return None; // "fake shift" some keyboards send around arrow keys
        }
        if code == 0x2A || code == 0x36 {
            self.shift = !released;
            return None;
        }
        if code == 0x1D {
            self.ctrl = !released; // left or right Ctrl
            return None;
        }
        if released {
            return None;
        }
        if extended {
            return match code {
                0x48 => Some(Key::Up),
                0x50 => Some(Key::Down),
                0x4B => Some(Key::Left),
                0x4D => Some(Key::Right),
                0x47 => Some(Key::Home),
                0x4F => Some(Key::End),
                0x53 => Some(Key::Delete),
                0x1C => Some(Key::Enter), // keypad Enter
                _ => None,
            };
        }
        let table = if self.shift { SHIFTED } else { NORMAL };
        match table.get(code as usize).copied().unwrap_or(0) {
            0 => None,
            b'\n' => Some(Key::Enter),
            0x08 => Some(Key::Backspace),
            b'\t' => Some(Key::Tab),
            0x1B => Some(Key::Escape),
            c if self.ctrl && c.is_ascii_alphabetic() => Some(Key::Ctrl(c.to_ascii_lowercase())),
            c => Some(Key::Char(c)),
        }
    }
}
