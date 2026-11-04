//! Files: browse drawers with the arrow keys, open pages in Notes,
//! rename, move, throw away, and build drawers.

use crate::fs::{self, Fs};
use crate::keyboard::Key;
use crate::screen::{Screen, ACCENT, BAR, COLS, NORMAL, SELECT, WARN};
use alloc::format;
use alloc::string::{String, ToString};

const LIST_TOP: usize = 2;
const LIST_ROWS: usize = 20; // rows 2-21
const PROMPT_ROW: usize = 23;
const MAX_INPUT: usize = 200;

#[derive(Clone, Copy, PartialEq)]
enum Ask {
    Rename,
    Move,
    Toss,
    Build,
}

pub enum Outcome {
    Stay,
    Leave,
    OpenPage(String),
}

pub struct Files {
    dir: String,
    sel: usize,
    top: usize,
    ask: Option<(Ask, String)>,
    too_long: bool,
    pub message: String,
}

impl Files {
    pub fn new(dir: String) -> Self {
        Self { dir, sel: 0, top: 0, ask: None, too_long: false, message: String::new() }
    }

    fn selected(&self, fs: &Fs) -> Option<(String, bool)> {
        fs.children(&self.dir).get(self.sel).map(|(n, d)| (fs::join(&self.dir, n), *d))
    }

    pub fn key(&mut self, fs: &mut Fs, key: Key) -> Outcome {
        if let Some((ask, mut input)) = self.ask.take() {
            match key {
                Key::Escape => self.message = "Cancelled.".into(),
                Key::Enter if core::mem::take(&mut self.too_long) => {
                    self.message = format!("That was too long (at most {MAX_INPUT} characters). Nothing changed.");
                }
                Key::Enter => self.answer(fs, ask, &input),
                Key::Backspace => {
                    input.pop();
                    self.ask = Some((ask, input));
                }
                Key::Char(c) if input.len() < MAX_INPUT => {
                    input.push(c as char);
                    self.ask = Some((ask, input));
                }
                Key::Char(_) => {
                    self.too_long = true;
                    self.ask = Some((ask, input));
                }
                _ => self.ask = Some((ask, input)),
            }
            return Outcome::Stay;
        }

        self.message.clear();
        let count = fs.children(&self.dir).len();
        match key {
            Key::Escape => return Outcome::Leave,
            Key::Up if self.sel > 0 => self.sel -= 1,
            Key::Down if self.sel + 1 < count => self.sel += 1,
            Key::Enter => match self.selected(fs) {
                Some((path, true)) => {
                    self.dir = path;
                    self.sel = 0;
                    self.top = 0;
                }
                Some((path, false)) => return Outcome::OpenPage(path),
                None => {}
            },
            Key::Backspace if !self.dir.is_empty() => {
                let came_from = fs::name(&self.dir).to_string();
                self.dir = fs::parent(&self.dir).to_string();
                self.sel = fs.children(&self.dir).iter().position(|(n, _)| *n == came_from).unwrap_or(0);
            }
            Key::Char(c) => {
                let ask = match c.to_ascii_lowercase() {
                    b'n' => Ask::Rename,
                    b'm' => Ask::Move,
                    b't' => Ask::Toss,
                    b'b' => Ask::Build,
                    _ => return Outcome::Stay,
                };
                if ask != Ask::Build && self.selected(fs).is_none() {
                    self.message = "Nothing is selected.".into();
                } else {
                    self.ask = Some((ask, String::new()));
                    self.too_long = false;
                }
            }
            _ => {}
        }
        Outcome::Stay
    }

    fn answer(&mut self, fs: &mut Fs, ask: Ask, input: &str) {
        let input = input.trim();
        let result = match (ask, self.selected(fs)) {
            (Ask::Build, _) => fs.resolve(&self.dir, input).and_then(|p| fs.build_drawer(&p)).map(|_| "Drawer built."),
            (Ask::Rename, Some((path, _))) => fs.rename(&path, input).map(|_| "Renamed."),
            (Ask::Move, Some((path, _))) => fs.resolve(&self.dir, input).and_then(|d| fs.move_into(&path, &d)).map(|_| "Moved."),
            (Ask::Toss, Some((path, _))) if input.eq_ignore_ascii_case("y") => fs.toss(&path).map(|_| "Thrown away."),
            (Ask::Toss, Some(_)) => Ok("Kept it."),
            _ => Err("Nothing is selected.".to_string()),
        };
        self.message = match result {
            Ok(m) => m.to_string(),
            Err(e) => e,
        };
    }

    pub fn draw(&mut self, fs: &Fs, screen: &mut Screen) {
        let items = fs.children(&self.dir);
        self.sel = self.sel.min(items.len().saturating_sub(1));
        if self.sel < self.top {
            self.top = self.sel;
        } else if self.sel >= self.top + LIST_ROWS {
            self.top = self.sel + 1 - LIST_ROWS;
        }

        screen.fill_row(0, BAR);
        screen.put_str(1, 0, &format!("Files - {}", fs::display(&self.dir)), BAR);
        let count = format!("{} item(s)", items.len());
        screen.put_str(COLS - 1 - count.len(), 0, &count, BAR);
        screen.fill_row(1, NORMAL);

        for i in 0..LIST_ROWS {
            let row = LIST_TOP + i;
            screen.fill_row(row, NORMAL);
            let Some((name, drawer)) = items.get(self.top + i) else { continue };
            let color = if self.top + i == self.sel { SELECT } else { NORMAL };
            let kind = if *drawer { "drawer" } else { "page  " };
            let preview = if *drawer {
                format!("{} item(s)", fs.children(&fs::join(&self.dir, name)).len())
            } else {
                fs.read_page(&fs::join(&self.dir, name)).unwrap_or("").lines().next().unwrap_or("").to_string()
            };
            let text = format!(" {kind}  {name:<24}  {preview}");
            for col in 1..COLS - 1 {
                crate::screen::put(col, row, b' ', color);
            }
            screen.put_str(1, row, &text[..text.len().min(COLS - 2)], color);
        }
        if items.is_empty() {
            screen.put_str(2, LIST_TOP, "(this drawer is empty)", NORMAL);
        }

        screen.fill_row(22, NORMAL);
        screen.fill_row(PROMPT_ROW, NORMAL);
        let mut cursor = None;
        if let Some((ask, input)) = &self.ask {
            let q = match ask {
                Ask::Rename => "New name: ",
                Ask::Move => "Move into which drawer? ",
                Ask::Toss => "Toss it (throw away)? Type y and press Enter: ",
                Ask::Build => "Name of the new drawer: ",
            };
            screen.put_str(1, PROMPT_ROW, q, ACCENT);
            // Long input: show only the end that fits, marked with '<'.
            let room = COLS - 3 - q.len();
            let shown = if input.len() > room { format!("<{}", &input[input.len() - (room - 1)..]) } else { input.clone() };
            screen.put_str(1 + q.len(), PROMPT_ROW, &shown, NORMAL);
            cursor = Some(1 + q.len() + shown.len());
        } else if !self.message.is_empty() {
            screen.put_str(1, PROMPT_ROW, &self.message, WARN);
        }

        screen.fill_row(24, BAR);
        screen.put_str(1, 24, "Enter open  Bksp up  N rename  M move  T toss  B new drawer  Esc leave", BAR);
        match cursor {
            Some(col) => screen.set_cursor(col, PROMPT_ROW),
            None => screen.set_cursor(1, LIST_TOP + self.sel - self.top),
        }
    }
}
