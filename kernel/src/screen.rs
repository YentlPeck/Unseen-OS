//! The text screen: 80 columns x 25 rows in VGA text mode.
//! Row 0 is a status bar; rows 1-24 are the work area.

use crate::port::outb;
use core::fmt;

pub const COLS: usize = 80;
pub const ROWS: usize = 25;
const TOP: usize = 1; // first row of the work area

const VGA: *mut u16 = 0xB8000 as *mut u16;

// Colours: high nibble = background, low nibble = text.
pub const NORMAL: u8 = 0x17; // light grey on dark blue
pub const BRIGHT: u8 = 0x1F; // white on dark blue
pub const ACCENT: u8 = 0x1B; // light cyan on dark blue
pub const WARN: u8 = 0x1E; // yellow on dark blue
pub const BAR: u8 = 0x30; // black on cyan
pub const SELECT: u8 = 0x71; // blue on light grey

pub struct Screen {
    col: usize,
    row: usize,
    pub color: u8,
}

/// Background colour of the theme (VGA colour 0-7). Everything drawn on the
/// normal dark-blue background gets this background instead.
static THEME_BG: core::sync::atomic::AtomicU8 = core::sync::atomic::AtomicU8::new(1);

/// Theme names and their background colours.
pub const THEMES: [(&str, u8); 4] = [("blue", 1), ("black", 0), ("brown", 6), ("purple", 5)];

pub fn set_theme(name: &str) {
    if let Some((_, bg)) = THEMES.iter().find(|(n, _)| *n == name) {
        THEME_BG.store(*bg, core::sync::atomic::Ordering::Relaxed);
    }
}

pub fn put(col: usize, row: usize, ch: u8, color: u8) {
    let color = if color >> 4 == 1 {
        (THEME_BG.load(core::sync::atomic::Ordering::Relaxed) << 4) | (color & 0x0F)
    } else {
        color
    };
    unsafe { VGA.add(row * COLS + col).write_volatile(((color as u16) << 8) | ch as u16) }
}

fn get(col: usize, row: usize) -> u16 {
    unsafe { VGA.add(row * COLS + col).read_volatile() }
}

impl Screen {
    pub fn new() -> Self {
        let mut s = Self { col: 0, row: TOP, color: NORMAL };
        s.status_bar("Unseen OS 0.1", "type guide for help");
        s.clear();
        s
    }

    pub fn status_bar(&mut self, left: &str, right: &str) {
        for col in 0..COLS {
            put(col, 0, b' ', BAR);
        }
        for (i, b) in left.bytes().enumerate().take(COLS) {
            put(1 + i, 0, b, BAR);
        }
        let start = COLS.saturating_sub(right.len() + 1);
        for (i, b) in right.bytes().enumerate() {
            put(start + i, 0, b, BAR);
        }
    }

    pub fn clear(&mut self) {
        for row in TOP..ROWS {
            for col in 0..COLS {
                put(col, row, b' ', NORMAL);
            }
        }
        self.col = 0;
        self.row = TOP;
        self.update_cursor();
    }

    fn scroll(&mut self) {
        for row in TOP + 1..ROWS {
            for col in 0..COLS {
                unsafe { VGA.add((row - 1) * COLS + col).write_volatile(get(col, row)) }
            }
        }
        for col in 0..COLS {
            put(col, ROWS - 1, b' ', NORMAL);
        }
    }

    fn newline(&mut self) {
        self.col = 0;
        if self.row == ROWS - 1 {
            self.scroll();
        } else {
            self.row += 1;
        }
    }

    pub fn write_byte(&mut self, b: u8) {
        match b {
            b'\n' => self.newline(),
            _ => {
                if self.col == COLS {
                    self.newline();
                }
                put(self.col, self.row, b, self.color);
                self.col += 1;
            }
        }
        self.update_cursor();
    }

    /// Erase the character left of the cursor, wrapping back to the end of
    /// the previous row when a long line was wrapped.
    pub fn backspace(&mut self) {
        if self.col == 0 && self.row > TOP {
            self.row -= 1;
            self.col = COLS;
        }
        if self.col > 0 {
            self.col -= 1;
            put(self.col, self.row, b' ', NORMAL);
            self.update_cursor();
        }
    }

    pub fn write_colored(&mut self, s: &str, color: u8) {
        let old = self.color;
        self.color = color;
        for b in s.bytes() {
            self.write_byte(b);
        }
        self.color = old;
    }

    fn update_cursor(&self) {
        let pos = (self.row * COLS + self.col.min(COLS - 1)) as u16;
        outb(0x3D4, 0x0F);
        outb(0x3D5, (pos & 0xFF) as u8);
        outb(0x3D4, 0x0E);
        outb(0x3D5, (pos >> 8) as u8);
    }

    /// Write text at a fixed spot (cut off at the right edge).
    pub fn put_str(&mut self, col: usize, row: usize, s: &str, color: u8) {
        for (i, b) in s.bytes().enumerate() {
            if col + i >= COLS {
                break;
            }
            put(col + i, row, b, color);
        }
    }

    pub fn fill_row(&mut self, row: usize, color: u8) {
        for col in 0..COLS {
            put(col, row, b' ', color);
        }
    }

    pub fn set_cursor(&mut self, col: usize, row: usize) {
        self.col = col.min(COLS - 1);
        self.row = row.min(ROWS - 1);
        self.update_cursor();
    }

    /// The whole screen as plain text, one line per row (for the test harness later).
    pub fn text_row(&self, row: usize, out: &mut [u8; COLS]) {
        for (col, slot) in out.iter_mut().enumerate() {
            *slot = (get(col, row) & 0xFF) as u8;
        }
    }
}

impl fmt::Write for Screen {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for b in s.bytes() {
            self.write_byte(b);
        }
        Ok(())
    }
}
