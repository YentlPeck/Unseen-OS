//! Calendar: events, each a page calendar:<id> with lines
//! "date: YYYY-MM-DD", "time: HH:MM", "title:", "with:".

use crate::fs::{self, Fs};
use crate::keyboard::Key;
use crate::screen::{Screen, ACCENT, NORMAL, SELECT, WARN};
use crate::settings;
use crate::ui::{self, Edit, Form, List};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

const LIST_TOP: usize = 3;
const LIST_ROWS: usize = 18;

struct Event {
    path: String,
    date: String,
    time: String,
    title: String,
    with: String,
}

fn events(fs: &Fs) -> Vec<Event> {
    let mut out: Vec<Event> = fs
        .children("calendar")
        .into_iter()
        .filter(|(_, drawer)| !drawer)
        .map(|(n, _)| {
            let path = fs::join("calendar", &n);
            let t = fs.read_page(&path).unwrap_or("");
            Event {
                date: ui::field(t, "date").to_string(),
                time: ui::field(t, "time").to_string(),
                title: ui::field(t, "title").to_string(),
                with: ui::field(t, "with").to_string(),
                path,
            }
        })
        .collect();
    out.sort_by(|a, b| (&a.date, &a.time, &a.title).cmp(&(&b.date, &b.time, &b.title)));
    out
}

fn digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

/// (year, month, day) if `s` is a real date written YYYY-MM-DD.
pub fn parse_date(s: &str) -> Option<(i32, u32, u32)> {
    let p: Vec<&str> = s.split('-').collect();
    if p.len() != 3 || p[0].len() != 4 || p[1].len() != 2 || p[2].len() != 2 || !p.iter().all(|x| digits(x)) {
        return None;
    }
    let (y, m, d) = (p[0].parse().ok()?, p[1].parse().ok()?, p[2].parse().ok()?);
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let days = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    (m >= 1 && m <= 12 && d >= 1 && d <= days[m as usize - 1]).then_some((y, m, d))
}

fn valid_time(s: &str) -> bool {
    let Some((h, m)) = s.split_once(':') else { return false };
    h.len() == 2 && m.len() == 2 && digits(h) && digits(m) && h.parse::<u32>().unwrap() < 24 && m.parse::<u32>().unwrap() < 60
}

/// Day of the week (Sakamoto's method).
fn weekday(y: i32, m: u32, d: u32) -> &'static str {
    let t = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y = if m < 3 { y - 1 } else { y };
    let w = (y + y / 4 - y / 100 + y / 400 + t[m as usize - 1] + d as i32) % 7;
    ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"][w as usize]
}

/// A time for display, following the clock setting.
pub fn show_time(time: &str, twelve: bool) -> String {
    if !twelve || !valid_time(time) {
        return time.to_string();
    }
    let (h, m) = time.split_once(':').unwrap();
    let h: u32 = h.parse().unwrap();
    let (h12, ampm) = match h {
        0 => (12, "am"),
        1..=11 => (h, "am"),
        12 => (12, "pm"),
        _ => (h - 12, "pm"),
    };
    format!("{h12}:{m} {ampm}")
}

enum Mode {
    List,
    Edit { form: Form, old: Option<String>, warned: bool },
    Toss,
}

pub struct Calendar {
    list: List,
    mode: Mode,
    message: String,
}

impl Calendar {
    pub fn new() -> Self {
        Self { list: List::default(), mode: Mode::List, message: String::new() }
    }

    fn form(e: Option<&Event>) -> Form {
        let (d, t, ti, w) = e.map_or(("", "", "", ""), |e| (&*e.date, &*e.time, &*e.title, &*e.with));
        Form::new(&[("Date", d, 10), ("Time", t, 5), ("Title", ti, 50), ("With", w, 40)])
    }

    fn save(fs: &mut Fs, form: &Form, old: &Option<String>) -> Result<String, String> {
        if form.overflow() {
            return Err(ui::OVERFLOW_MSG.into());
        }
        let (date, time, title) = (form.value(0), form.value(1), form.value(2));
        if parse_date(date).is_none() {
            return Err("Date must be a real date written YYYY-MM-DD, e.g. 2026-11-12.".into());
        }
        if !valid_time(time) {
            return Err("Time must be written HH:MM in 24-hour form, e.g. 19:30.".into());
        }
        if title.is_empty() {
            return Err("Title can't be empty.".into());
        }
        let path = match old {
            Some(p) => p.clone(),
            None => {
                let mut n = 1;
                while fs.exists(&format!("calendar:e{n}")) {
                    n += 1;
                }
                format!("calendar:e{n}")
            }
        };
        let text = format!("date: {date}\ntime: {time}\ntitle: {title}\nwith: {}", form.value(3));
        fs.app_write(&path, &text);
        Ok(format!("Kept: {title}."))
    }

    /// Returns true when the app should close.
    pub fn key(&mut self, fs: &mut Fs, key: Key) -> bool {
        self.message.clear();
        let evs = events(fs);
        match &mut self.mode {
            Mode::List => match key {
                Key::Escape => return true,
                Key::Enter => match evs.get(self.list.sel) {
                    Some(e) => self.mode = Mode::Edit { form: Self::form(Some(e)), old: Some(e.path.clone()), warned: false },
                    None => self.message = "There is no event to open. Press A to add one.".into(),
                },
                Key::Char(b'a') | Key::Char(b'A') => self.mode = Mode::Edit { form: Self::form(None), old: None, warned: false },
                Key::Char(b't') | Key::Char(b'T') => {
                    if evs.is_empty() {
                        self.message = "There is no event to toss.".into();
                    } else {
                        self.mode = Mode::Toss;
                    }
                }
                k => {
                    self.list.key(k, evs.len());
                }
            },
            Mode::Toss => {
                if key != Key::Ctrl(b'k') && key != Key::Escape {
                    self.message = "Press Ctrl+K to toss it, or Esc to keep it.".into();
                    return false;
                }
                if key == Key::Ctrl(b'k') {
                    if let Some(e) = evs.get(self.list.sel) {
                        fs.app_toss(&e.path);
                        self.message = format!("Tossed {}.", e.title);
                    }
                } else {
                    self.message = "Kept it.".into();
                }
                self.mode = Mode::List;
            }
            Mode::Edit { form, old, warned } => match key {
                Key::Ctrl(b'k') => match Self::save(fs, form, old) {
                    Ok(m) => {
                        self.message = m;
                        self.mode = Mode::List;
                    }
                    Err(e) => self.message = e,
                },
                Key::Escape => {
                    if !*warned {
                        *warned = true;
                        self.message = "Not kept! Esc again goes back without keeping. Ctrl+K keeps.".into();
                    } else {
                        self.mode = Mode::List;
                    }
                }
                k => {
                    *warned = false;
                    if form.key(k) == Edit::Full {
                        self.message = "That field is full.".into();
                    }
                }
            },
        }
        false
    }

    pub fn draw(&mut self, fs: &Fs, screen: &mut Screen) {
        let evs = events(fs);
        let twelve = settings::get(fs, "clock") == "12h";
        ui::clear_body(screen);
        if let Mode::Edit { form, old, .. } = &self.mode {
            let title = if old.is_some() { "Calendar - change event" } else { "Calendar - new event" };
            ui::bars(screen, title, "", "Up/Down/Tab move   Ctrl+K keep   Esc back", &self.message);
            let (c, r) = form.draw(screen, 3);
            screen.put_str(2, 8, "Date: YYYY-MM-DD.  Time: HH:MM, 24-hour (e.g. 19:30).  With: who it is with.", NORMAL);
            screen.set_cursor(c, r);
            return;
        }
        self.list.fit(evs.len(), LIST_ROWS);
        ui::bars(screen, "Calendar", &format!("{} events", evs.len()),
                 "Up/Down choose  Enter open/change  A add  T toss  Esc leave", "");
        screen.put_str(2, 2, &format!("{:<16}{:<10}{:<34}{}", "DATE", "TIME", "TITLE", "WITH"), ACCENT);
        for (i, e) in evs.iter().enumerate().skip(self.list.top).take(LIST_ROWS) {
            let row = LIST_TOP + i - self.list.top;
            let day = parse_date(&e.date).map_or("", |(y, m, d)| weekday(y, m, d));
            let text = format!(" {:<11}{:<4}{:<10}{:<34}{}", e.date, day, show_time(&e.time, twelve), e.title, e.with);
            let color = if i == self.list.sel { SELECT } else { NORMAL };
            screen.put_str(1, row, &format!("{:<78}", &text[..text.len().min(78)]), color);
        }
        if evs.is_empty() {
            screen.put_str(2, LIST_TOP, "(no events yet - press A to add one)", NORMAL);
        }
        if let Mode::Toss = self.mode {
            screen.put_str(2, 23, "Toss this event? Press Ctrl+K to toss, Esc to keep.", WARN);
        } else if !self.message.is_empty() {
            screen.put_str(2, 23, &self.message, WARN);
        }
        screen.set_cursor(1, LIST_TOP + self.list.sel.saturating_sub(self.list.top));
    }
}
