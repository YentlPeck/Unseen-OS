//! Mail: an inbox and a sent box. Each message is a page:
//!   mail:inbox:m<N> or mail:sent:s<N>, text = "from:", "to:", "subject:" lines,
//!   a blank line, then the message.

use crate::fs::{self, Fs};
use crate::keyboard::Key;
use crate::screen::{Screen, ACCENT, NORMAL, SELECT, WARN};
use crate::ui::{self, Edit, Form, List, TextArea};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

const LIST_TOP: usize = 4;
const LIST_ROWS: usize = 17;
const BODY_TOP: usize = 6;
const BODY_ROWS: usize = 17;

#[derive(Clone, Copy, PartialEq)]
enum Box_ {
    Inbox,
    Sent,
}

impl Box_ {
    fn drawer(self) -> &'static str {
        match self {
            Box_::Inbox => "mail:inbox",
            Box_::Sent => "mail:sent",
        }
    }
}

struct Message {
    path: String,
    from: String,
    to: String,
    subject: String,
    body: String,
}

fn number(name: &str) -> u32 {
    name.trim_start_matches(|c: char| c.is_ascii_alphabetic()).parse().unwrap_or(0)
}

fn messages(fs: &Fs, b: Box_) -> Vec<Message> {
    let mut names: Vec<String> = fs.children(b.drawer()).into_iter().filter(|(_, d)| !d).map(|(n, _)| n).collect();
    names.sort_by_key(|n| (number(n), n.clone()));
    names
        .into_iter()
        .map(|n| {
            let path = fs::join(b.drawer(), &n);
            let t = fs.read_page(&path).unwrap_or("");
            let body = t.split_once("\n\n").map_or("", |(_, b)| b).to_string();
            Message {
                from: ui::field(t, "from").to_string(),
                to: ui::field(t, "to").to_string(),
                subject: ui::field(t, "subject").to_string(),
                body,
                path,
            }
        })
        .collect()
}

struct Compose {
    form: Form, // To, Subject
    body: TextArea,
    in_body: bool,
    warned: bool,
    /// Set after warning that the To address is not in Contacts; a second Ctrl+K sends.
    unknown_ok: Option<String>,
}

enum Mode {
    List,
    Read { path: String, top: usize },
    Compose(Compose),
    Toss(String),
}

pub struct Mail {
    view: Box_,
    list: List,
    mode: Mode,
    message: String,
}

impl Mail {
    pub fn new() -> Self {
        Self { view: Box_::Inbox, list: List::default(), mode: Mode::List, message: String::new() }
    }

    fn compose(to: &str, subject: &str, body: &str) -> Compose {
        Compose {
            form: Form::new(&[("To", to, 24), ("Subject", subject, 60)]),
            body: TextArea::new(body),
            in_body: false,
            warned: false,
            unknown_ok: None,
        }
    }

    fn find(fs: &Fs, path: &str) -> Option<Message> {
        [Box_::Inbox, Box_::Sent].into_iter().flat_map(|b| messages(fs, b)).find(|m| m.path == path)
    }

    fn send(fs: &mut Fs, c: &mut Compose) -> Result<String, String> {
        if c.form.overflow() || c.body.overflow {
            return Err(ui::OVERFLOW_MSG.into());
        }
        let to = c.form.value(0);
        if !fs::valid_name(to) {
            return Err("To must be one mail address: one word like mira (see Contacts).".into());
        }
        if !fs.exists(&fs::join("contacts", to)) && c.unknown_ok.as_deref() != Some(to) {
            c.unknown_ok = Some(to.to_string());
            return Err(format!(
                "No contact has the address '{to}' (addresses must match exactly). Ctrl+K again sends anyway."
            ));
        }
        let mut n = 1;
        while fs.exists(&format!("mail:sent:s{n}")) {
            n += 1;
        }
        let text = format!("from: me\nto: {to}\nsubject: {}\n\n{}", c.form.value(1), c.body.text().trim_end());
        fs.app_write(&format!("mail:sent:s{n}"), &text);
        Ok(format!("Sent to {to}."))
    }

    /// Returns true when the app should close.
    pub fn key(&mut self, fs: &mut Fs, key: Key) -> bool {
        self.message.clear();
        let msgs = messages(fs, self.view);
        match &mut self.mode {
            Mode::List => match key {
                Key::Escape => return true,
                Key::Tab => {
                    self.view = if self.view == Box_::Inbox { Box_::Sent } else { Box_::Inbox };
                    self.list = List::default();
                }
                Key::Enter => match msgs.get(self.list.sel) {
                    Some(m) => self.mode = Mode::Read { path: m.path.clone(), top: 0 },
                    None => self.message = "There is no message to open.".into(),
                },
                Key::Char(b'w') | Key::Char(b'W') => self.mode = Mode::Compose(Self::compose("", "", "")),
                Key::Char(b't') | Key::Char(b'T') => match msgs.get(self.list.sel) {
                    Some(m) => self.mode = Mode::Toss(m.path.clone()),
                    None => self.message = "There is no message to toss.".into(),
                },
                k => {
                    self.list.key(k, msgs.len());
                }
            },
            Mode::Read { path, top } => {
                let Some(m) = Self::find(fs, path) else {
                    self.mode = Mode::List;
                    return false;
                };
                match key {
                    Key::Escape => self.mode = Mode::List,
                    Key::Up => *top = top.saturating_sub(1),
                    Key::Down => *top = (*top + 1).min(ui::wrap(&m.body, 76).len().saturating_sub(1)),
                    Key::Char(b'r') | Key::Char(b'R') => {
                        let who = if m.path.starts_with("mail:sent") { &m.to } else { &m.from };
                        let subj = if m.subject.to_ascii_lowercase().starts_with("re:") {
                            m.subject.clone()
                        } else {
                            format!("Re: {}", m.subject)
                        };
                        let mut c = Self::compose(who, &subj, "");
                        c.in_body = true;
                        self.mode = Mode::Compose(c);
                    }
                    Key::Char(b'f') | Key::Char(b'F') => {
                        self.mode = Mode::Compose(Self::compose("", &format!("Fwd: {}", m.subject), &m.body));
                    }
                    Key::Char(b't') | Key::Char(b'T') => self.mode = Mode::Toss(m.path.clone()),
                    _ => {}
                }
            }
            Mode::Toss(path) => {
                if key != Key::Ctrl(b'k') && key != Key::Escape {
                    self.message = "Press Ctrl+K to toss it, or Esc to keep it.".into();
                    return false;
                }
                if key == Key::Ctrl(b'k') {
                    fs.app_toss(path);
                    self.message = "Tossed the message.".into();
                } else {
                    self.message = "Kept it.".into();
                }
                self.mode = Mode::List;
            }
            Mode::Compose(c) => {
                if key != Key::Escape {
                    c.warned = false;
                }
                match key {
                    Key::Ctrl(b'k') => match Self::send(fs, c) {
                        Ok(m) => {
                            self.message = m;
                            self.mode = Mode::List;
                        }
                        Err(e) => self.message = e,
                    },
                    Key::Escape => {
                        if !c.warned {
                            c.warned = true;
                            self.message = "Not sent! Esc again throws this message away. Ctrl+K sends.".into();
                        } else {
                            self.message = "Message thrown away.".into();
                            self.mode = Mode::List;
                        }
                    }
                    Key::Tab => {
                        if c.in_body {
                            c.in_body = false;
                            c.form.focus = 0;
                        } else if c.form.focus + 1 == c.form.fields.len() {
                            c.in_body = true;
                        } else {
                            c.form.focus += 1;
                        }
                    }
                    k if c.in_body => self.message = c.body.key(k, BODY_ROWS).unwrap_or("").into(),
                    Key::Down if c.form.focus + 1 == c.form.fields.len() => c.in_body = true,
                    k => {
                        if c.form.key(k) == Edit::Full {
                            self.message = "That field is full.".into();
                        }
                    }
                }
            }
        }
        false
    }

    pub fn draw(&mut self, fs: &Fs, screen: &mut Screen) {
        ui::clear_body(screen);
        match &self.mode {
            Mode::Compose(c) => {
                ui::bars(screen, "Mail - write a message", "", "Tab next field   Ctrl+K send   Esc cancel", &self.message);
                let (fc, fr) = c.form.draw(screen, 2);
                screen.put_str(2, 4, "Message (Tab moves between To, Subject and here):", ACCENT);
                screen.put_str(2, 5, &"-".repeat(76), NORMAL);
                let (bc, br) = c.body.draw(screen, BODY_TOP, BODY_ROWS);
                if c.in_body {
                    screen.set_cursor(bc, br);
                } else {
                    screen.set_cursor(fc, fr);
                }
            }
            Mode::Read { path, top } => {
                let Some(m) = Self::find(fs, path) else { return };
                ui::bars(screen, "Mail - message", "", "R reply  F forward  T toss  Up/Down scroll  Esc back", &self.message);
                screen.put_str(2, 2, &format!("From:    {}", m.from), NORMAL);
                screen.put_str(2, 3, &format!("To:      {}", m.to), NORMAL);
                screen.put_str(2, 4, &format!("Subject: {}", m.subject), ACCENT);
                screen.put_str(2, 5, &"-".repeat(76), NORMAL);
                let lines = ui::wrap(&m.body, 76);
                for (i, line) in lines.iter().skip(*top).take(BODY_ROWS).enumerate() {
                    screen.put_str(2, BODY_TOP + i, line, NORMAL);
                }
                if lines.len() > *top + BODY_ROWS {
                    screen.put_str(2, BODY_TOP + BODY_ROWS - 1, "(more below - press Down)", ACCENT);
                }
                screen.set_cursor(2, BODY_TOP);
            }
            Mode::List | Mode::Toss(_) => {
                let msgs = messages(fs, self.view);
                self.list.fit(msgs.len(), LIST_ROWS);
                let (title, who) = match self.view {
                    Box_::Inbox => ("Mail - Inbox", "FROM"),
                    Box_::Sent => ("Mail - Sent", "TO"),
                };
                ui::bars(screen, title, &format!("{} messages", msgs.len()),
                         "Up/Down choose  Enter read  W write  T toss  Tab inbox/sent  Esc leave", "");
                let tabs = if self.view == Box_::Inbox { "[Inbox]  Sent " } else { " Inbox  [Sent]" };
                screen.put_str(2, 2, tabs, ACCENT);
                screen.put_str(2, 3, &format!("{:<20}{}", who, "SUBJECT"), ACCENT);
                for (i, m) in msgs.iter().enumerate().skip(self.list.top).take(LIST_ROWS) {
                    let row = LIST_TOP + i - self.list.top;
                    let name = if self.view == Box_::Inbox { &m.from } else { &m.to };
                    let color = if i == self.list.sel { SELECT } else { NORMAL };
                    let text = format!(" {:<19}{}", name, m.subject);
                    screen.put_str(1, row, &format!("{:<78}", &text[..text.len().min(78)]), color);
                }
                if msgs.is_empty() {
                    screen.put_str(2, LIST_TOP, "(no messages)", NORMAL);
                }
                if let Mode::Toss(_) = self.mode {
                    screen.put_str(2, 23, "Toss this message? Press Ctrl+K to toss, Esc to keep.", WARN);
                } else if !self.message.is_empty() {
                    screen.put_str(2, 23, &self.message, WARN);
                }
                screen.set_cursor(1, LIST_TOP + self.list.sel.saturating_sub(self.list.top));
            }
        }
    }
}
