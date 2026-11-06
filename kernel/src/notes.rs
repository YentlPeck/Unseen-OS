//! Notes: a full-screen editor for one page.
//! Ctrl+K keeps (saves) the page, Esc leaves.

use crate::fs::{self, Fs};
use crate::keyboard::Key;
use crate::screen::Screen;
use crate::ui::{self, TextArea};
use alloc::format;
use alloc::string::{String, ToString};

const VIEW_TOP: usize = 1;
const VIEW_ROWS: usize = 23; // rows 1-23; row 24 is the key help bar

pub struct Notes {
    path: String,
    area: TextArea,
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
        Ok(Self { path, area: TextArea::new(&text), warned: false, message })
    }

    /// Handle a key. Returns true when the app should close.
    pub fn key(&mut self, fs: &mut Fs, key: Key) -> bool {
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
            _ => {}
        }
        self.message = self.area.key(key, VIEW_ROWS).unwrap_or("").into();
        false
    }

    pub fn draw(&self, screen: &mut Screen) {
        let title = format!("Notes - {}{}", fs::display(&self.path), if self.area.changed { "  (changed)" } else { "" });
        let pos = format!("line {}/{}", self.area.row + 1, self.area.lines.len());
        ui::bars(screen, &title, &pos, "Ctrl+K keep   Esc leave   arrows move", &self.message);
        let (col, row) = self.area.draw(screen, VIEW_TOP, VIEW_ROWS);
        screen.set_cursor(col, row);
    }
}
