//! Notes: a full-screen editor for one page, with line numbers.
//! Ctrl+K keeps (saves) the page, Ctrl+G goes to a line, Ctrl+F finds a word, Esc leaves.

use crate::fs::{self, Fs};
use crate::keyboard::Key;
use crate::screen::{Screen, BAR};
use crate::ui::{self, Edit, LineEdit, TextArea};
use alloc::format;
use alloc::string::{String, ToString};

const VIEW_TOP: usize = 1;
const VIEW_ROWS: usize = 23; // rows 1-23; row 24 is the key help bar

/// A question asked in the bottom bar.
enum Ask {
    None,
    Line(LineEdit),
    Find(LineEdit),
}

pub struct Notes {
    path: String,
    area: TextArea,
    warned: bool,
    message: String,
    ask: Ask,
    /// The last word looked for (Ctrl+F offers it again).
    last_find: String,
}

impl Notes {
    pub fn open(fs: &Fs, path: String) -> fs::Result<Self> {
        if fs.is_drawer(&path) {
            return Err(format!("{} is a drawer, not a page", fs::display(&path)));
        }
        if !fs.is_drawer(fs::parent(&path)) {
            return Err(format!("there is no drawer called {}", fs::display(fs::parent(&path))));
        }
        if let Ok(t) = fs.read_page(&path) {
            if t.bytes().filter(|&b| b == b'\n').count() >= crate::ui::MAX_AREA_LINES {
                return Err(format!(
                    "{} has more than {} lines, too many for Notes (read shows it)",
                    fs::display(&path),
                    crate::ui::MAX_AREA_LINES
                ));
            }
        }
        let (text, message) = match fs.read_page(&path) {
            Ok(t) => (t.to_string(), String::new()),
            Err(_) => (String::new(), "New page. It is not on the desk until you keep it.".to_string()),
        };
        Ok(Self { path, area: TextArea::new(&text), warned: false, message, ask: Ask::None, last_find: String::new() })
    }

    /// A key while a question is asked in the bottom bar.
    fn answer(&mut self, key: Key) {
        let field = match &mut self.ask {
            Ask::Line(f) | Ask::Find(f) => f,
            Ask::None => return,
        };
        match key {
            Key::Escape => self.ask = Ask::None,
            Key::Enter => {
                let text = field.text.trim().to_string();
                match core::mem::replace(&mut self.ask, Ask::None) {
                    Ask::Line(_) => match text.parse::<usize>() {
                        Ok(n) if self.area.go_to(n, VIEW_ROWS) => {}
                        _ => self.message = format!("There is no line '{text}': lines 1-{}.", self.area.lines.len()),
                    },
                    Ask::Find(_) => {
                        match self.area.find(&text, VIEW_ROWS) {
                            Some(n) => self.message = format!("Found '{text}' on line {n}. Ctrl+F Enter finds the next."),
                            None => self.message = format!("'{text}' is not on this page."),
                        }
                        self.last_find = text;
                    }
                    Ask::None => {}
                }
            }
            k => {
                if field.key(k) == Edit::Full {
                    self.message = "That is as long as it can be.".into();
                }
            }
        }
    }

    /// Handle a key. Returns true when the app should close.
    pub fn key(&mut self, fs: &mut Fs, key: Key) -> bool {
        if !matches!(self.ask, Ask::None) {
            self.message.clear();
            self.answer(key);
            return false;
        }
        if key != Key::Escape {
            self.warned = false;
        }
        match key {
            Key::Escape => {
                if self.area.changed && !self.warned {
                    self.warned = true;
                    self.message = "Not kept! Esc again leaves without keeping. Ctrl+K keeps.".into();
                    return false;
                }
                return true;
            }
            Key::Ctrl(b'k') => {
                if self.area.overflow {
                    self.message = ui::OVERFLOW_MSG.into();
                    return false;
                }
                match fs.write_page(&self.path, &self.area.text()) {
                    Ok(()) => {
                        self.area.changed = false;
                        self.message = "Kept.".into();
                    }
                    Err(e) => self.message = format!("Could not keep: {e}."),
                }
                return false;
            }
            Key::Ctrl(b'g') => {
                self.message.clear();
                self.ask = Ask::Line(LineEdit::new("", 5));
                return false;
            }
            Key::Ctrl(b'f') => {
                self.message.clear();
                self.ask = Ask::Find(LineEdit::new(&self.last_find, 60));
                return false;
            }
            _ => {}
        }
        self.message = self.area.key(key, VIEW_ROWS).unwrap_or("").into();
        false
    }

    pub fn draw(&self, screen: &mut Screen) {
        let title = format!("Notes - {}{}", fs::display(&self.path), if self.area.changed { "  (changed)" } else { "" });
        let pos = format!("line {}/{}", self.area.row + 1, self.area.lines.len());
        ui::bars(
            screen,
            &title,
            &pos,
            "Ctrl+K keep   Ctrl+G go to line   Ctrl+F find   Esc leave   arrows move",
            &self.message,
        );
        let (col, row) = self.area.draw_numbered(screen, VIEW_TOP, VIEW_ROWS);
        match &self.ask {
            Ask::None => screen.set_cursor(col, row),
            Ask::Line(f) | Ask::Find(f) => {
                let label = if matches!(self.ask, Ask::Line(_)) { "Go to line: " } else { "Find: " };
                screen.fill_row(24, BAR);
                screen.put_str(1, 24, label, BAR);
                let hint = "Enter go   Esc back";
                screen.put_str(crate::screen::COLS - hint.len() - 1, 24, hint, BAR);
                let c = f.draw(screen, 1 + label.len(), 24, 60, BAR);
                screen.set_cursor(c, 24);
            }
        }
    }
}
