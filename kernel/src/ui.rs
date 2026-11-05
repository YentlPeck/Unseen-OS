//! Building blocks shared by the apps: bars, a one-line text field, a
//! multi-line text area, a form of labelled fields, and list scrolling.

use crate::keyboard::Key;
use crate::screen::{Screen, ACCENT, BAR, COLS, NORMAL};
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// Top bar (title + right text) and bottom bar (help, or a message if set).
pub fn bars(screen: &mut Screen, title: &str, right: &str, help: &str, message: &str) {
    screen.fill_row(0, BAR);
    screen.put_str(1, 0, title, BAR);
    if !right.is_empty() {
        screen.put_str(COLS.saturating_sub(right.len() + 1), 0, right, BAR);
    }
    screen.fill_row(24, BAR);
    screen.put_str(1, 24, if message.is_empty() { help } else { message }, BAR);
}

/// Blank rows 1-23.
pub fn clear_body(screen: &mut Screen) {
    for r in 1..24 {
        screen.fill_row(r, NORMAL);
    }
}

/// What happened to a key given to a text field.
#[derive(PartialEq)]
pub enum Edit {
    Used,
    /// A character was refused because the field is full.
    Full,
    /// Not a text-editing key.
    Ignored,
}

/// A one-line text field.
#[derive(Clone)]
pub struct LineEdit {
    pub text: String,
    pub cursor: usize,
    pub max: usize,
    /// Set when typing didn't fit; cleared by Backspace/Delete. While set,
    /// apps refuse to save, so cut-off text is never kept silently.
    pub overflow: bool,
}

impl LineEdit {
    pub fn new(text: &str, max: usize) -> Self {
        Self { text: text.to_string(), cursor: text.len(), max, overflow: false }
    }

    pub fn key(&mut self, key: Key) -> Edit {
        match key {
            Key::Char(_) if self.text.len() >= self.max => {
                self.overflow = true;
                return Edit::Full;
            }
            Key::Char(c) => {
                self.text.insert(self.cursor, c as char);
                self.cursor += 1;
            }
            Key::Backspace if self.cursor > 0 => {
                self.cursor -= 1;
                self.text.remove(self.cursor);
                self.overflow = false;
            }
            Key::Delete if self.cursor < self.text.len() => {
                self.text.remove(self.cursor);
                self.overflow = false;
            }
            Key::Left => self.cursor = self.cursor.saturating_sub(1),
            Key::Right => self.cursor = (self.cursor + 1).min(self.text.len()),
            Key::Home => self.cursor = 0,
            Key::End => self.cursor = self.text.len(),
            Key::Backspace | Key::Delete => {}
            _ => return Edit::Ignored,
        }
        Edit::Used
    }

    /// Draw the field; returns the screen column of the cursor.
    pub fn draw(&self, screen: &mut Screen, col: usize, row: usize, width: usize, color: u8) -> usize {
        let start = if self.cursor >= width { self.cursor + 1 - width } else { 0 };
        let end = (start + width).min(self.text.len());
        screen.put_str(col, row, &self.text[start..end], color);
        col + self.cursor - start
    }
}

/// A multi-line text area (used by Notes and by Mail for message text).
pub struct TextArea {
    pub lines: Vec<String>,
    pub row: usize,
    pub col: usize,
    pub top: usize,
    pub changed: bool,
    /// Set when typing didn't fit on a line; cleared by Backspace/Delete.
    pub overflow: bool,
}

pub const MAX_LINE: usize = COLS - 2;

impl TextArea {
    pub fn new(text: &str) -> Self {
        Self { lines: text.split('\n').map(|l| l.to_string()).collect(), row: 0, col: 0, top: 0, changed: false, overflow: false }
    }

    pub fn text(&self) -> String {
        self.lines.join("\n")
    }

    pub fn is_empty(&self) -> bool {
        self.lines.iter().all(|l| l.is_empty())
    }

    /// Handle an editing key. Returns a message to show, if any.
    pub fn key(&mut self, key: Key, view_rows: usize) -> Option<&'static str> {
        let mut message = None;
        let line_len = self.lines[self.row].len();
        match key {
            Key::Char(c) if line_len < MAX_LINE => {
                self.lines[self.row].insert(self.col, c as char);
                self.col += 1;
                self.changed = true;
            }
            Key::Char(_) => {
                self.overflow = true;
                message = Some("This line is full (78 characters): that letter was not added. Press Enter for a new line.");
            }
            Key::Enter => {
                let rest = self.lines[self.row].split_off(self.col);
                self.lines.insert(self.row + 1, rest);
                self.row += 1;
                self.col = 0;
                self.changed = true;
            }
            Key::Backspace => {
                self.overflow = false;
                if self.col > 0 {
                    self.col -= 1;
                    self.lines[self.row].remove(self.col);
                    self.changed = true;
                } else if self.row > 0 && self.lines[self.row - 1].len() + line_len <= MAX_LINE {
                    let line = self.lines.remove(self.row);
                    self.row -= 1;
                    self.col = self.lines[self.row].len();
                    self.lines[self.row].push_str(&line);
                    self.changed = true;
                } else if self.row > 0 {
                    message = Some("Can't join: together these lines are longer than 78 characters.");
                }
            }
            Key::Delete => {
                self.overflow = false;
                if self.col < line_len {
                    self.lines[self.row].remove(self.col);
                    self.changed = true;
                } else if self.row + 1 < self.lines.len() && line_len + self.lines[self.row + 1].len() <= MAX_LINE {
                    let next = self.lines.remove(self.row + 1);
                    self.lines[self.row].push_str(&next);
                    self.changed = true;
                } else if self.row + 1 < self.lines.len() {
                    message = Some("Can't join: together these lines are longer than 78 characters.");
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
        } else if self.row >= self.top + view_rows {
            self.top = self.row + 1 - view_rows;
        }
        message
    }

    /// Draw rows `first..first+rows`; returns the cursor position (col, row).
    pub fn draw(&self, screen: &mut Screen, first: usize, rows: usize) -> (usize, usize) {
        for i in 0..rows {
            screen.fill_row(first + i, NORMAL);
            if let Some(line) = self.lines.get(self.top + i) {
                screen.put_str(1, first + i, line, NORMAL);
                if line.len() > MAX_LINE {
                    screen.put_str(COLS - 1, first + i, ">", ACCENT); // more text to the right
                }
            }
        }
        (1 + self.col, first + self.row - self.top)
    }
}

/// A form: labelled one-line fields. Up/Down/Tab move between fields.
pub struct Form {
    pub fields: Vec<(&'static str, LineEdit)>,
    pub focus: usize,
}

impl Form {
    pub fn new(fields: &[(&'static str, &str, usize)]) -> Self {
        Self { fields: fields.iter().map(|(l, v, max)| (*l, LineEdit::new(v, *max))).collect(), focus: 0 }
    }

    pub fn value(&self, i: usize) -> &str {
        self.fields[i].1.text.trim()
    }

    /// True if typing didn't fit in some field (see LineEdit::overflow).
    pub fn overflow(&self) -> bool {
        self.fields.iter().any(|(_, f)| f.overflow)
    }

    pub fn key(&mut self, key: Key) -> Edit {
        let n = self.fields.len();
        match key {
            Key::Tab | Key::Down => self.focus = (self.focus + 1) % n,
            Key::Up => self.focus = (self.focus + n - 1) % n,
            _ => return self.fields[self.focus].1.key(key),
        }
        Edit::Used
    }

    /// Draw from `first_row`, one field per row; returns the cursor (col, row).
    pub fn draw(&self, screen: &mut Screen, first_row: usize) -> (usize, usize) {
        let label_w = self.fields.iter().map(|(l, _)| l.len()).max().unwrap_or(0) + 2;
        let mut cursor = (0, first_row);
        for (i, (label, field)) in self.fields.iter().enumerate() {
            let row = first_row + i;
            screen.fill_row(row, NORMAL);
            screen.put_str(2, row, label, if i == self.focus { ACCENT } else { NORMAL });
            screen.put_str(2 + label.len(), row, ":", NORMAL);
            let col = field.draw(screen, 3 + label_w, row, COLS - 4 - label_w, NORMAL);
            if i == self.focus {
                cursor = (col, row);
            }
        }
        cursor
    }
}

/// Selection and scrolling for a list.
#[derive(Default)]
pub struct List {
    pub sel: usize,
    pub top: usize,
}

impl List {
    /// Up/Down. Returns true if the key was used.
    pub fn key(&mut self, key: Key, len: usize) -> bool {
        match key {
            Key::Up if self.sel > 0 => self.sel -= 1,
            Key::Down if self.sel + 1 < len => self.sel += 1,
            Key::Up | Key::Down => {}
            _ => return false,
        }
        true
    }

    /// Keep the selection valid and visible for `rows` visible rows.
    pub fn fit(&mut self, len: usize, rows: usize) {
        self.sel = self.sel.min(len.saturating_sub(1));
        if self.sel < self.top {
            self.top = self.sel;
        } else if self.sel >= self.top + rows {
            self.top = self.sel + 1 - rows;
        }
    }
}

/// Parse "key: value" lines (as used by contacts, events, mail headers, settings).
pub fn field<'a>(text: &'a str, key: &str) -> &'a str {
    for line in text.lines() {
        if let Some((k, v)) = line.split_once(':') {
            if k.trim().eq_ignore_ascii_case(key) {
                return v.trim();
            }
        }
        if line.is_empty() {
            break; // headers end at the first blank line
        }
    }
    ""
}

pub const OVERFLOW_MSG: &str = "Some typing didn't fit (a field or line was full). Fix it with Backspace first.";

/// Split text into screen lines of at most `width` characters (for reading).
pub fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.split('\n') {
        if line.is_empty() {
            out.push(String::new());
        }
        let mut rest = line;
        while !rest.is_empty() {
            let cut = rest.len().min(width);
            out.push(rest[..cut].to_string());
            rest = &rest[cut..];
        }
    }
    out
}
