//! Settings: change how the computer works. Stored in the page system:settings.

use crate::fs::{Fs, DEFAULT_SETTINGS};
use crate::keyboard::Key;
use crate::screen::{self, Screen, ACCENT, NORMAL, SELECT};
use crate::ui::{self, Edit, LineEdit};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

pub const PAGE: &str = "system:settings";

/// (key, explanation, choices). Empty choices = free text.
pub const SETTINGS: [(&str, &str, &[&str]); 4] = [
    ("name", "your name, shown in the top bar", &[]),
    ("theme", "screen colours", &["blue", "black", "brown", "purple"]),
    ("start", "what opens when the computer starts", &["shell", "files", "mail", "calendar", "contacts"]),
    ("clock", "how times are shown", &["24h", "12h"]),
];

/// The current value of a setting (falls back to the default).
pub fn get(fs: &Fs, key: &str) -> String {
    let text = fs.read_page(PAGE).unwrap_or(DEFAULT_SETTINGS);
    let v = ui::field(text, key);
    if v.is_empty() { ui::field(DEFAULT_SETTINGS, key).to_string() } else { v.to_string() }
}

pub struct Settings {
    values: Vec<String>,
    sel: usize,
    editing: Option<LineEdit>,
    changed: bool,
    warned: bool,
    message: String,
}

impl Settings {
    pub fn new(fs: &Fs) -> Self {
        let values = SETTINGS.iter().map(|(k, _, _)| get(fs, k)).collect();
        Self { values, sel: 0, editing: None, changed: false, warned: false, message: String::new() }
    }

    fn cycle(&mut self, step: isize) {
        let choices = SETTINGS[self.sel].2;
        let pos = choices.iter().position(|c| *c == self.values[self.sel]).unwrap_or(0) as isize;
        let next = (pos + step).rem_euclid(choices.len() as isize) as usize;
        self.values[self.sel] = choices[next].to_string();
        self.changed = true;
    }

    /// Returns true when the app should close.
    pub fn key(&mut self, fs: &mut Fs, key: Key) -> bool {
        if key != Key::Escape {
            self.warned = false;
        }
        self.message.clear();
        if let Some(edit) = &mut self.editing {
            match key {
                Key::Enter => {
                    let v = edit.text.trim().to_string();
                    if edit.overflow {
                        self.message = ui::OVERFLOW_MSG.into();
                    } else if v.is_empty() {
                        self.message = "The name can't be empty.".into();
                    } else {
                        self.values[self.sel] = v;
                        self.changed = true;
                        self.editing = None;
                    }
                }
                Key::Escape => self.editing = None,
                _ => {
                    if edit.key(key) == Edit::Full {
                        self.message = "The name can be at most 20 characters.".into();
                    }
                }
            }
            return false;
        }
        let is_text = SETTINGS[self.sel].2.is_empty();
        match key {
            Key::Escape => {
                if self.changed && !self.warned {
                    self.warned = true;
                    self.message = "Not kept! Esc again leaves without keeping. Ctrl+K keeps.".into();
                    return false;
                }
                return true;
            }
            Key::Up if self.sel > 0 => self.sel -= 1,
            Key::Down if self.sel + 1 < SETTINGS.len() => self.sel += 1,
            Key::Enter if is_text => self.editing = Some(LineEdit::new(&self.values[self.sel], 20)),
            Key::Enter | Key::Right if !is_text => self.cycle(1),
            Key::Left if !is_text => self.cycle(-1),
            Key::Ctrl(b'k') => {
                let text: Vec<String> =
                    SETTINGS.iter().zip(&self.values).map(|((k, _, _), v)| format!("{k}: {v}")).collect();
                let _ = fs.app_write(PAGE, &text.join("\n")); // the settings page always exists
                screen::set_theme(&self.values[1]);
                self.changed = false;
                self.message = "Kept.".into();
            }
            _ => {}
        }
        false
    }

    pub fn draw(&self, screen: &mut Screen) {
        let title = if self.changed { "Settings  (changed)" } else { "Settings" };
        let help = if self.editing.is_some() {
            "type the name, Enter done, Esc cancel"
        } else {
            "Up/Down choose  Left/Right change  Enter change/edit  Ctrl+K keep  Esc leave"
        };
        ui::bars(screen, title, "", help, &self.message);
        ui::clear_body(screen);
        let mut cursor = (1, 2 + self.sel);
        for (i, ((key, about, choices), value)) in SETTINGS.iter().zip(&self.values).enumerate() {
            let row = 2 + i * 2;
            let color = if i == self.sel { SELECT } else { NORMAL };
            screen.put_str(2, row, &format!(" {key:<8}"), color);
            match (&self.editing, i == self.sel) {
                (Some(edit), true) => cursor = (edit.draw(screen, 13, row, 30, ACCENT), row),
                _ => {
                    let shown = if choices.is_empty() { value.clone() } else { format!("< {value} >") };
                    screen.put_str(13, row, &shown, if i == self.sel { ACCENT } else { NORMAL });
                    if i == self.sel {
                        cursor = (13, row);
                    }
                }
            }
            screen.put_str(36, row, about, NORMAL);
            if !choices.is_empty() {
                screen.put_str(13, row + 1, &format!("choices: {}", choices.join(", ")), NORMAL);
            }
        }
        screen.set_cursor(cursor.0, cursor.1);
    }
}
