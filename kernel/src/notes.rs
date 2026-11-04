//! Notes: a full-screen editor for one page.
//! Ctrl+K keeps (saves) the page, Esc leaves.

use crate::fs::{self, Fs};
use crate::keyboard::Key;
use crate::screen::{Screen, BAR, COLS, NORMAL};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

const VIEW_TOP: usize = 1;
const VIEW_ROWS: usize = 23; // rows 1-23; row 24 is the key help bar
const MAX_COLS: usize = COLS - 2;

pub struct Notes {
    path: String,
    lines: Vec<String>,
    row: usize,
    col: usize,
    top: usize,
    changed: bool,
    warned: bool,
    message: String,
}

impl Notes {
    pub fn open(fs: &Fs, path: String) -> fs::Result<Self> {
        if fs.is_drawer(&path) {
            return Err(format!("{} is a drawer, not a page", fs::display(&path)));
        }
        if !fs.is_drawer(fs::parent(&path)) {
            return Err(format!("there is no drawer called {}", fs::display(fs::parent(&path))));
        }
        let (text, message) = match fs.read_page(&path) {
            Ok(t) => (t.to_string(), String::new()),
            Err(_) => (String::new(), "New page. It is not on the desk until you keep it.".to_string()),
        };
        let lines = text.split('\n').map(|l| l.to_string()).collect();
        Ok(Self { path, lines, row: 0, col: 0, top: 0, changed: false, warned: false, message })
    }

    /// Handle a key. Returns true when the app should close.
    pub fn key(&mut self, fs: &mut Fs, key: Key) -> bool {
        if key != Key::Escape {
            self.warned = false;
        }
        match key {
            Key::Escape => {
                if self.changed && !self.warned {
                    self.warned = true;
                    self.message = "Not kept! Esc again leaves without keeping. Ctrl+K keeps.".into();
                    return false;
                }
                return true;
            }
            Key::Ctrl(b'k') => {
                let text = self.lines.join("\n");
                match fs.write_page(&self.path, &text) {
                    Ok(()) => {
                        self.changed = false;
                        self.message = "Kept.".into();
                    }
                    Err(e) => self.message = e,
                }
                return false;
            }
            _ => {}
        }
        self.message.clear();
        let line_len = self.lines[self.row].len();
        match key {
            Key::Char(c) if line_len < MAX_COLS => {
                self.lines[self.row].insert(self.col, c as char);
                self.col += 1;
                self.changed = true;
            }
            Key::Char(_) => {
                self.message = format!("This line is full ({MAX_COLS} characters). Press Enter for a new line.");
            }
            Key::Enter => {
                let rest = self.lines[self.row].split_off(self.col);
                self.lines.insert(self.row + 1, rest);
                self.row += 1;
                self.col = 0;
                self.changed = true;
            }
            Key::Backspace => {
                if self.col > 0 {
                    self.col -= 1;
                    self.lines[self.row].remove(self.col);
                    self.changed = true;
                } else if self.row > 0 && self.lines[self.row - 1].len() + line_len <= MAX_COLS {
                    let line = self.lines.remove(self.row);
                    self.row -= 1;
                    self.col = self.lines[self.row].len();
                    self.lines[self.row].push_str(&line);
                    self.changed = true;
                } else if self.row > 0 {
                    self.message = format!("Can't join: together these lines are longer than {MAX_COLS} characters.");
                }
            }
            Key::Delete => {
                if self.col < line_len {
                    self.lines[self.row].remove(self.col);
                    self.changed = true;
                } else if self.row + 1 < self.lines.len() && line_len + self.lines[self.row + 1].len() <= MAX_COLS {
                    let next = self.lines.remove(self.row + 1);
                    self.lines[self.row].push_str(&next);
                    self.changed = true;
                } else if self.row + 1 < self.lines.len() {
                    self.message = format!("Can't join: together these lines are longer than {MAX_COLS} characters.");
                }
            }
            Key::Left if self.col > 0 => self.col -= 1,
            Key::Left if self.row > 0 => {
                self.row -= 1;
                self.col = self.lines[self.row].len();
            }
            Key::Right if self.col < line_len => self.col += 1,
            Key::Right if self.row + 1 < self.lines.len() => {
                self.row += 1;
                self.col = 0;
            }
            Key::Up if self.row > 0 => self.row -= 1,
            Key::Down if self.row + 1 < self.lines.len() => self.row += 1,
            Key::Home => self.col = 0,
            Key::End => self.col = line_len,
            _ => {}
        }
        self.col = self.col.min(self.lines[self.row].len());
        if self.row < self.top {
            self.top = self.row;
        } else if self.row >= self.top + VIEW_ROWS {
            self.top = self.row + 1 - VIEW_ROWS;
        }
        false
    }

    pub fn draw(&self, screen: &mut Screen) {
        screen.fill_row(0, BAR);
        let title = format!("Notes - {}{}", fs::display(&self.path), if self.changed { "  (changed)" } else { "" });
        screen.put_str(1, 0, &title, BAR);
        let pos = format!("line {}/{}", self.row + 1, self.lines.len());
        screen.put_str(COLS - 1 - pos.len(), 0, &pos, BAR);

        for i in 0..VIEW_ROWS {
            let row = VIEW_TOP + i;
            screen.fill_row(row, NORMAL);
            if let Some(line) = self.lines.get(self.top + i) {
                screen.put_str(1, row, line, NORMAL);
            }
        }

        screen.fill_row(24, BAR);
        let help = if self.message.is_empty() { "Ctrl+K keep   Esc leave   arrows move" } else { self.message.as_str() };
        screen.put_str(1, 24, help, BAR);
        screen.set_cursor(1 + self.col, VIEW_TOP + self.row - self.top);
    }
}
