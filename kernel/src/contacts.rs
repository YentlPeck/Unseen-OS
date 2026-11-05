//! Contacts: an address book. Each person is a page contacts:<mail> with
//! lines "name:", "mail:", "phone:", "note:".

use crate::fs::{self, Fs};
use crate::keyboard::Key;
use crate::screen::{Screen, ACCENT, NORMAL, SELECT, WARN};
use crate::ui::{self, Edit, Form, List};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

const LIST_TOP: usize = 3;
const LIST_ROWS: usize = 17;

struct Person {
    path: String,
    name: String,
    mail: String,
    phone: String,
    note: String,
}

fn people(fs: &Fs) -> Vec<Person> {
    let mut out: Vec<Person> = fs
        .children("contacts")
        .into_iter()
        .filter(|(_, drawer)| !drawer)
        .map(|(n, _)| {
            let path = fs::join("contacts", &n);
            let t = fs.read_page(&path).unwrap_or("");
            Person {
                name: ui::field(t, "name").to_string(),
                mail: ui::field(t, "mail").to_string(),
                phone: ui::field(t, "phone").to_string(),
                note: ui::field(t, "note").to_string(),
                path,
            }
        })
        .collect();
    out.sort_by(|a, b| a.name.to_ascii_lowercase().cmp(&b.name.to_ascii_lowercase()));
    out
}

enum Mode {
    List,
    Edit { form: Form, old: Option<String>, warned: bool },
    Toss,
}

pub struct Contacts {
    list: List,
    mode: Mode,
    message: String,
}

impl Contacts {
    pub fn new() -> Self {
        Self { list: List::default(), mode: Mode::List, message: String::new() }
    }

    fn form(p: Option<&Person>) -> Form {
        let (n, m, ph, no) = p.map_or(("", "", "", ""), |p| (&*p.name, &*p.mail, &*p.phone, &*p.note));
        Form::new(&[("Name", n, 30), ("Mail", m, 24), ("Phone", ph, 20), ("Note", no, 60)])
    }

    fn save(fs: &mut Fs, form: &Form, old: &Option<String>) -> Result<String, String> {
        if form.overflow() {
            return Err(ui::OVERFLOW_MSG.into());
        }
        let (name, mail) = (form.value(0), form.value(1));
        if name.is_empty() {
            return Err("Name can't be empty.".into());
        }
        if !fs::valid_name(mail) {
            return Err("Mail must be one word: letters, digits, - _ . (at most 24).".into());
        }
        let path = fs::join("contacts", mail);
        if old.as_deref() != Some(path.as_str()) && fs.exists(&path) {
            return Err(format!("There is already a contact with mail {mail}."));
        }
        if let Some(o) = old {
            fs.app_toss(o);
        }
        let text = format!("name: {name}\nmail: {mail}\nphone: {}\nnote: {}", form.value(2), form.value(3));
        fs.app_write(&path, &text);
        Ok(format!("Kept {name}."))
    }

    /// Returns true when the app should close.
    pub fn key(&mut self, fs: &mut Fs, key: Key) -> bool {
        self.message.clear();
        let ppl = people(fs);
        match &mut self.mode {
            Mode::List => match key {
                Key::Escape => return true,
                Key::Enter => match ppl.get(self.list.sel) {
                    Some(p) => self.mode = Mode::Edit { form: Self::form(Some(p)), old: Some(p.path.clone()), warned: false },
                    None => self.message = "There is nobody to open. Press A to add someone.".into(),
                },
                Key::Char(b'a') | Key::Char(b'A') => {
                    self.mode = Mode::Edit { form: Self::form(None), old: None, warned: false }
                }
                Key::Char(b't') | Key::Char(b'T') => {
                    if ppl.is_empty() {
                        self.message = "There is nobody to toss.".into();
                    } else {
                        self.mode = Mode::Toss;
                    }
                }
                k => {
                    self.list.key(k, ppl.len());
                }
            },
            Mode::Toss => {
                if key != Key::Ctrl(b'k') && key != Key::Escape {
                    self.message = "Press Ctrl+K to toss it, or Esc to keep it.".into();
                    return false;
                }
                if key == Key::Ctrl(b'k') {
                    if let Some(p) = ppl.get(self.list.sel) {
                        fs.app_toss(&p.path);
                        self.message = format!("Tossed {}.", p.name);
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
        let ppl = people(fs);
        ui::clear_body(screen);
        if let Mode::Edit { form, old, .. } = &self.mode {
            let title = if old.is_some() { "Contacts - change person" } else { "Contacts - new person" };
            ui::bars(screen, title, "", "Up/Down/Tab move   Ctrl+K keep   Esc back", &self.message);
            let (c, r) = form.draw(screen, 3);
            screen.put_str(2, 9, "Mail is the person's address on this computer (one word, e.g. mira).", NORMAL);
            screen.set_cursor(c, r);
            return;
        }
        self.list.fit(ppl.len(), LIST_ROWS);
        ui::bars(screen, "Contacts", &format!("{} people", ppl.len()),
                 "Up/Down choose  Enter open/change  A add  T toss  Esc leave", "");
        screen.put_str(2, 2, &format!("{:<28}{:<20}{}", "NAME", "MAIL", "PHONE"), ACCENT);
        for (i, p) in ppl.iter().enumerate().skip(self.list.top).take(LIST_ROWS) {
            let row = LIST_TOP + i - self.list.top;
            let color = if i == self.list.sel { SELECT } else { NORMAL };
            let text = format!(" {:<27}{:<20}{}", p.name, p.mail, p.phone);
            screen.put_str(1, row, &format!("{text:<78}"), color);
        }
        if ppl.is_empty() {
            screen.put_str(2, LIST_TOP, "(no contacts yet - press A to add someone)", NORMAL);
        } else if let Some(p) = ppl.get(self.list.sel) {
            screen.put_str(2, 21, &format!("Note: {}", p.note), NORMAL);
        }
        if let Mode::Toss = self.mode {
            screen.put_str(2, 23, "Toss this person? Press Ctrl+K to toss, Esc to keep.", WARN);
        } else if !self.message.is_empty() {
            screen.put_str(2, 23, &self.message, WARN);
        }
        screen.set_cursor(1, LIST_TOP + self.list.sel.saturating_sub(self.list.top));
    }
}
