//! The shell: reads a line, finds the command word, runs it.
//! Command names are our own on purpose (not Linux or Windows words), so
//! AI models can't rely on memory and must figure the system out.
//!
//! The shell also owns the open apps as a stack: keys go to the top app, and
//! when the last app closes we are back at the prompt.

use crate::calc::Calc;
use crate::chores::{self, Viewer};
use crate::calendar::Calendar;
use crate::contacts::Contacts;
use crate::files::{Files, Outcome};
use crate::mail::Mail;
use crate::settings::{self, Settings};
use crate::fs::{self, Entry, Fs};
use crate::keyboard::Key;
use crate::notes::Notes;
use crate::screen::{Screen, ACCENT, BRIGHT, NORMAL, WARN};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::Write;

const MAX_LINE: usize = 200;

struct Command {
    name: &'static str,
    usage: &'static str,
    about: &'static str,
    run: fn(&mut Shell, &mut Screen, &str),
}

const COMMANDS: &[Command] = &[
    Command { name: "guide", usage: "guide [word]", about: "list the words this computer understands, or explain one", run: cmd_guide },
    Command { name: "here", usage: "here", about: "say which drawer you are in", run: cmd_here },
    Command { name: "peek", usage: "peek [drawer]", about: "list what is in a drawer", run: cmd_peek },
    Command { name: "enter", usage: "enter <drawer>", about: "go into a drawer", run: cmd_enter },
    Command { name: "leave", usage: "leave", about: "go back out to the drawer above", run: cmd_leave },
    Command { name: "read", usage: "read <page>", about: "show what a page says", run: cmd_read },
    Command { name: "write", usage: "write <page> <text>", about: "make a page with this text (replaces it)", run: cmd_write },
    Command { name: "add", usage: "add <page> <text>", about: "add a line to the end of a page", run: cmd_add },
    Command { name: "build", usage: "build <drawer>", about: "make a new drawer", run: cmd_build },
    Command { name: "rename", usage: "rename <name> <new>", about: "give a page or drawer a new name", run: cmd_rename },
    Command { name: "move", usage: "move <name> <drawer>", about: "put a page or drawer into another drawer", run: cmd_move },
    Command { name: "copy", usage: "copy <page> <new>", about: "make a copy of a page", run: cmd_copy },
    Command { name: "toss", usage: "toss <name>", about: "throw away a page or an empty drawer", run: cmd_toss },
    Command { name: "seek", usage: "seek <word>", about: "find pages whose name or text has a word", run: cmd_seek },
    Command { name: "notes", usage: "notes <page>", about: "open a page in the Notes app", run: cmd_notes },
    Command { name: "files", usage: "files", about: "open the Files app", run: cmd_files },
    Command { name: "mail", usage: "mail", about: "open the Mail app (inbox, sent, write)", run: cmd_mail },
    Command { name: "contacts", usage: "contacts", about: "open the Contacts app (address book)", run: cmd_contacts },
    Command { name: "calendar", usage: "calendar", about: "open the Calendar app (events)", run: cmd_calendar },
    Command { name: "settings", usage: "settings", about: "open the Settings app", run: cmd_settings },
    Command { name: "calc", usage: "calc", about: "open the Calculator app", run: cmd_calc },
    Command { name: "do", usage: "do <page>", about: "carry out the chore written on a page (guide do explains chores)", run: cmd_do },
    Command { name: "show", usage: "show <text>", about: "write text on the screen", run: cmd_show },
    Command { name: "wipe", usage: "wipe", about: "clean the screen", run: cmd_wipe },
    Command { name: "clock", usage: "clock", about: "today's date and time", run: cmd_clock },
    Command { name: "about", usage: "about", about: "what this computer is", run: cmd_about },
];

enum App {
    Notes(Notes),
    Files(Files),
    Mail(Mail),
    Contacts(Contacts),
    Calendar(Calendar),
    Settings(Settings),
    Calc(Calc),
    Viewer(Viewer),
}

pub struct Shell {
    line: String,
    too_long: bool,
    last: String,
    here: String,
    fs: Fs,
    apps: Vec<App>,
    memory_mb: u32,
    /// The date from the start files, if the experiment froze it.
    today: Option<String>,
}

impl Shell {
    pub fn new(fs: Fs, memory_mb: u32, today: Option<String>) -> Self {
        Self { line: String::new(), too_long: false, last: String::new(), here: String::new(), fs, apps: Vec::new(), memory_mb, today }
    }

    /// Today's date as YYYY-MM-DD (frozen by the start files, or the clock's).
    fn today(&self) -> String {
        match &self.today {
            Some(d) => d.clone(),
            None => {
                let t = crate::rtc::now();
                format!("{:04}-{:02}-{:02}", t.year, t.month, t.day)
            }
        }
    }

    pub fn fs(&self) -> &Fs {
        &self.fs
    }

    fn status(&self, screen: &mut Screen) {
        let name = settings::get(&self.fs, "name");
        screen.status_bar(&format!("Unseen OS 0.3 - {name}"), "type guide for help");
    }

    pub fn start(&mut self, screen: &mut Screen) {
        crate::screen::set_theme(&settings::get(&self.fs, "theme"));
        self.status(screen);
        screen.clear();
        screen.write_colored("Welcome to Unseen OS.\n", BRIGHT);
        screen.write_colored("Type ", NORMAL);
        screen.write_colored("guide", ACCENT);
        screen.write_colored(" and press Enter to see what this computer understands.\n\n", NORMAL);
        self.prompt(screen);
        let app = match settings::get(&self.fs, "start").as_str() {
            "files" => Some(App::Files(Files::new(String::new()))),
            "mail" => Some(App::Mail(Mail::new())),
            "calendar" => Some(App::Calendar(Calendar::new())),
            "contacts" => Some(App::Contacts(Contacts::new())),
            _ => None,
        };
        if let Some(app) = app {
            self.apps.push(app);
            self.redraw(screen);
        }
    }

    fn prompt(&self, screen: &mut Screen) {
        screen.write_colored(&fs::display(&self.here), ACCENT);
        screen.write_colored(" >> ", ACCENT);
    }

    pub fn key(&mut self, screen: &mut Screen, key: Key) {
        if !self.apps.is_empty() {
            self.app_key(screen, key);
            return;
        }
        match key {
            Key::Char(c) if self.line.len() < MAX_LINE => {
                self.line.push(c as char);
                screen.write_byte(c);
            }
            Key::Char(_) => self.too_long = true,
            Key::Backspace if !self.line.is_empty() => {
                self.line.pop();
                screen.backspace();
            }
            Key::Up => {
                for _ in 0..self.line.len() {
                    screen.backspace();
                }
                self.too_long = false;
                self.line = self.last.clone();
                screen.write_colored(&self.line.clone(), NORMAL);
            }
            Key::Enter => {
                screen.write_byte(b'\n');
                let line = core::mem::take(&mut self.line);
                if core::mem::take(&mut self.too_long) {
                    warn(screen, &format!("That line was too long (at most {MAX_LINE} characters). Nothing was done."));
                } else {
                    if !line.trim().is_empty() {
                        self.last = line.clone();
                    }
                    self.run(screen, line.trim());
                    self.follow_moves();
                }
                if self.apps.is_empty() {
                    self.prompt(screen);
                }
            }
            _ => {}
        }
    }

    fn app_key(&mut self, screen: &mut Screen, key: Key) {
        enum Next {
            Redraw,
            Close,
            Open(String),
        }
        let next = match self.apps.last_mut() {
            Some(App::Notes(n)) => {
                if n.key(&mut self.fs, key) { Next::Close } else { Next::Redraw }
            }
            Some(App::Files(f)) => match f.key(&mut self.fs, key) {
                Outcome::Stay => Next::Redraw,
                Outcome::Leave => Next::Close,
                Outcome::OpenPage(p) => Next::Open(p),
            },
            Some(App::Mail(a)) => if a.key(&mut self.fs, key) { Next::Close } else { Next::Redraw },
            Some(App::Contacts(a)) => if a.key(&mut self.fs, key) { Next::Close } else { Next::Redraw },
            Some(App::Calendar(a)) => if a.key(&mut self.fs, key) { Next::Close } else { Next::Redraw },
            Some(App::Settings(a)) => if a.key(&mut self.fs, key) { Next::Close } else { Next::Redraw },
            Some(App::Calc(a)) => if a.key(key) { Next::Close } else { Next::Redraw },
            Some(App::Viewer(v)) => if v.key(key) { Next::Close } else { Next::Redraw },
            None => return,
        };
        match next {
            Next::Redraw => {}
            Next::Close => {
                self.apps.pop();
            }
            Next::Open(path) => match Notes::open(&self.fs, path) {
                Ok(n) => self.apps.push(App::Notes(n)),
                Err(e) => {
                    if let Some(App::Files(f)) = self.apps.last_mut() {
                        f.message = e;
                    }
                }
            },
        }
        self.follow_moves();
        self.redraw(screen);
    }

    /// Keep `here` valid: follow the drawer if it was renamed or moved, and
    /// step out to the nearest drawer that still exists if it was tossed.
    fn follow_moves(&mut self) {
        for (from, to) in self.fs.take_moves() {
            if self.here == from {
                self.here = to;
            } else if let Some(rest) = self.here.strip_prefix(&format!("{from}:")) {
                self.here = format!("{to}:{rest}");
            }
        }
        while !self.fs.is_drawer(&self.here) {
            self.here = fs::parent(&self.here).to_string();
        }
    }

    /// Draw the top app, or go back to the prompt if no app is open.
    fn redraw(&mut self, screen: &mut Screen) {
        match self.apps.last_mut() {
            Some(App::Notes(n)) => n.draw(screen),
            Some(App::Files(f)) => f.draw(&self.fs, screen),
            Some(App::Mail(a)) => a.draw(&self.fs, screen),
            Some(App::Contacts(a)) => a.draw(&self.fs, screen),
            Some(App::Calendar(a)) => a.draw(&self.fs, screen),
            Some(App::Settings(a)) => a.draw(screen),
            Some(App::Calc(a)) => a.draw(screen),
            Some(App::Viewer(v)) => v.draw(screen),
            None => {
                self.status(screen);
                screen.clear();
                self.prompt(screen);
            }
        }
    }

    fn run(&mut self, screen: &mut Screen, line: &str) {
        if line.is_empty() {
            return;
        }
        let (word, rest) = match line.find(' ') {
            Some(i) => (&line[..i], line[i + 1..].trim()),
            None => (line, ""),
        };
        crate::log!("command: {}", line);
        match COMMANDS.iter().find(|c| c.name == word) {
            Some(cmd) => (cmd.run)(self, screen, rest),
            None => warn(
                screen,
                &format!("Unknown word '{word}'. Type guide to see the words this computer understands."),
            ),
        }
    }

    fn path(&self, input: &str) -> fs::Result<String> {
        self.fs.resolve(&self.here, input)
    }
}

// ---- helpers --------------------------------------------------------------

fn warn(screen: &mut Screen, msg: &str) {
    screen.write_colored(msg, WARN);
    screen.write_byte(b'\n');
}

fn done(screen: &mut Screen, result: fs::Result<String>) {
    match result {
        Ok(msg) => {
            let _ = writeln!(screen, "{msg}");
        }
        Err(e) => warn(screen, &format!("Could not do that: {e}.")),
    }
}

/// Split "a b c" into ("a", "b c"). None if either part is missing.
fn two_args(rest: &str) -> Option<(&str, &str)> {
    let i = rest.find(' ')?;
    let (a, b) = (&rest[..i], rest[i + 1..].trim());
    (!b.is_empty()).then_some((a, b))
}

fn usage(screen: &mut Screen, word: &str) {
    if let Some(c) = COMMANDS.iter().find(|c| c.name == word) {
        warn(screen, &format!("Use it like this: {}", c.usage));
    }
}

// ---- commands -------------------------------------------------------------

const GROUPS: [(&str, &[&str]); 5] = [
    ("Drawers", &["here", "peek", "enter", "leave", "build"]),
    ("Pages", &["read", "write", "add", "copy", "rename", "move", "toss", "seek"]),
    ("Apps", &["notes", "files", "mail", "contacts", "calendar", "settings", "calc"]),
    ("Chores", &["do"]),
    ("Other", &["show", "wipe", "clock", "about", "guide"]),
];

fn cmd_guide(sh: &mut Shell, screen: &mut Screen, rest: &str) {
    if rest == "do" {
        // chores have a full reference of their own
        sh.apps.push(App::Viewer(Viewer::guide()));
        sh.redraw(screen);
        return;
    }
    if !rest.is_empty() {
        match COMMANDS.iter().find(|c| c.name == rest) {
            Some(c) => {
                screen.write_colored(c.usage, ACCENT);
                let _ = writeln!(screen, "   {}", c.about);
            }
            None => warn(screen, &format!("There is no word '{rest}'. Type guide to see them all.")),
        }
        return;
    }
    screen.write_colored("Words this computer understands (guide <word> explains one, e.g. guide write):\n", BRIGHT);
    for (group, words) in GROUPS {
        screen.write_colored(&format!("  {group:<9}"), NORMAL);
        screen.write_colored(&words.join("  "), ACCENT);
        screen.write_byte(b'\n');
    }
}

fn cmd_here(sh: &mut Shell, screen: &mut Screen, _: &str) {
    let _ = writeln!(screen, "You are in {}", fs::display(&sh.here));
}

fn cmd_peek(sh: &mut Shell, screen: &mut Screen, rest: &str) {
    let dir = if rest.is_empty() { Ok(sh.here.clone()) } else { sh.path(rest) };
    let dir = match dir {
        Ok(d) if sh.fs.is_drawer(&d) => d,
        Ok(d) => return warn(screen, &format!("There is no drawer called {}.", fs::display(&d))),
        Err(e) => return warn(screen, &e),
    };
    let items = sh.fs.children(&dir);
    if items.is_empty() {
        let _ = writeln!(screen, "{} is empty.", fs::display(&dir));
    }
    for (name, drawer) in items {
        if drawer {
            screen.write_colored("  drawer  ", NORMAL);
            screen.write_colored(&name, ACCENT);
            screen.write_byte(b'\n');
        } else {
            let _ = writeln!(screen, "  page    {name}");
        }
    }
}

fn cmd_enter(sh: &mut Shell, screen: &mut Screen, rest: &str) {
    if rest.is_empty() {
        return usage(screen, "enter");
    }
    match sh.path(rest) {
        Ok(d) if sh.fs.is_drawer(&d) => sh.here = d,
        Ok(d) => warn(screen, &format!("There is no drawer called {}.", fs::display(&d))),
        Err(e) => warn(screen, &e),
    }
}

fn cmd_leave(sh: &mut Shell, screen: &mut Screen, _: &str) {
    if sh.here.is_empty() {
        return warn(screen, "You are already on the desk, the top drawer.");
    }
    sh.here = fs::parent(&sh.here).to_string();
}

fn cmd_read(sh: &mut Shell, screen: &mut Screen, rest: &str) {
    if rest.is_empty() {
        return usage(screen, "read");
    }
    let text = sh.path(rest).and_then(|p| sh.fs.read_page(&p).map(|t| t.to_string()));
    match text {
        Ok(t) => {
            for line in t.split('\n') {
                let _ = writeln!(screen, "{line}");
            }
        }
        Err(e) => warn(screen, &format!("Could not do that: {e}.")),
    }
}

fn cmd_write(sh: &mut Shell, screen: &mut Screen, rest: &str) {
    let Some((page, text)) = two_args(rest) else { return usage(screen, "write") };
    let r = sh.path(page).and_then(|p| sh.fs.write_page(&p, text).map(|_| format!("Wrote {}.", fs::display(&p))));
    done(screen, r);
}

fn cmd_add(sh: &mut Shell, screen: &mut Screen, rest: &str) {
    let Some((page, text)) = two_args(rest) else { return usage(screen, "add") };
    let r = sh.path(page).and_then(|p| {
        let old = sh.fs.read_page(&p)?.to_string();
        let new = if old.is_empty() { text.to_string() } else { format!("{old}\n{text}") };
        sh.fs.write_page(&p, &new).map(|_| format!("Added a line to {}.", fs::display(&p)))
    });
    done(screen, r);
}

fn cmd_build(sh: &mut Shell, screen: &mut Screen, rest: &str) {
    if rest.is_empty() {
        return usage(screen, "build");
    }
    let r = sh.path(rest).and_then(|p| sh.fs.build_drawer(&p).map(|_| format!("Built drawer {}.", fs::display(&p))));
    done(screen, r);
}

fn cmd_rename(sh: &mut Shell, screen: &mut Screen, rest: &str) {
    let Some((name, new)) = two_args(rest) else { return usage(screen, "rename") };
    let r = sh.path(name).and_then(|p| sh.fs.rename(&p, new).map(|_| format!("Renamed to {new}.")));
    done(screen, r);
}

fn cmd_move(sh: &mut Shell, screen: &mut Screen, rest: &str) {
    let Some((name, drawer)) = two_args(rest) else { return usage(screen, "move") };
    let r = sh.path(name).and_then(|p| {
        let d = sh.path(drawer)?;
        sh.fs.move_into(&p, &d).map(|_| format!("Moved into {}.", fs::display(&d)))
    });
    done(screen, r);
}

fn cmd_copy(sh: &mut Shell, screen: &mut Screen, rest: &str) {
    let Some((page, new)) = two_args(rest) else { return usage(screen, "copy") };
    let r = sh.path(page).and_then(|p| {
        let n = sh.path(new)?;
        sh.fs.copy_page(&p, &n).map(|_| format!("Copied to {}.", fs::display(&n)))
    });
    done(screen, r);
}

fn cmd_toss(sh: &mut Shell, screen: &mut Screen, rest: &str) {
    if rest.is_empty() {
        return usage(screen, "toss");
    }
    let r = sh.path(rest).and_then(|p| sh.fs.toss(&p).map(|_| format!("Tossed {}.", fs::display(&p))));
    done(screen, r);
}

fn cmd_seek(sh: &mut Shell, screen: &mut Screen, rest: &str) {
    if rest.is_empty() {
        return usage(screen, "seek");
    }
    let word = rest.to_ascii_lowercase();
    let mut found = 0;
    for (path, entry) in sh.fs.all() {
        let Entry::Page(text) = entry else { continue };
        let in_name = fs::name(path).to_ascii_lowercase().contains(&word);
        if in_name || text.to_ascii_lowercase().contains(&word) {
            found += 1;
            screen.write_colored(&format!("  {}", fs::display(path)), ACCENT);
            screen.write_byte(b'\n');
        }
    }
    if found == 0 {
        let _ = writeln!(screen, "No pages have '{rest}' in them.");
    }
}

fn cmd_notes(sh: &mut Shell, screen: &mut Screen, rest: &str) {
    if rest.is_empty() {
        return usage(screen, "notes");
    }
    match sh.path(rest).and_then(|p| Notes::open(&sh.fs, p)) {
        Ok(n) => {
            sh.apps.push(App::Notes(n));
            sh.redraw(screen);
        }
        Err(e) => warn(screen, &format!("Could not do that: {e}.")),
    }
}

fn cmd_files(sh: &mut Shell, screen: &mut Screen, _: &str) {
    sh.apps.push(App::Files(Files::new(sh.here.clone())));
    sh.redraw(screen);
}

fn cmd_do(sh: &mut Shell, screen: &mut Screen, rest: &str) {
    if rest.is_empty() {
        return usage(screen, "do");
    }
    let page = match sh.path(rest) {
        Ok(p) => p,
        Err(e) => return warn(screen, &format!("Could not do that: {e}.")),
    };
    if let Err(e) = sh.fs.read_page(&page) {
        return warn(screen, &format!("Could not do that: {e}."));
    }
    crate::log!("chore: {}", fs::display(&page));
    let today = sh.today();
    let viewer = chores::run(&mut sh.fs, &page, today);
    sh.apps.push(App::Viewer(viewer));
    sh.redraw(screen);
}

fn cmd_show(_: &mut Shell, screen: &mut Screen, rest: &str) {
    let _ = writeln!(screen, "{rest}");
}

fn cmd_wipe(_: &mut Shell, screen: &mut Screen, _: &str) {
    screen.clear();
}

fn cmd_clock(sh: &mut Shell, screen: &mut Screen, _: &str) {
    let t = crate::rtc::now();
    let time = format!("{:02}:{:02}", t.hour, t.minute);
    let twelve = settings::get(&sh.fs, "clock") == "12h";
    let _ = writeln!(screen, "{}  {}", sh.today(), crate::calendar::show_time(&time, twelve));
}

fn open_app(sh: &mut Shell, screen: &mut Screen, app: App) {
    sh.apps.push(app);
    sh.redraw(screen);
}

fn cmd_mail(sh: &mut Shell, screen: &mut Screen, _: &str) {
    open_app(sh, screen, App::Mail(Mail::new()));
}

fn cmd_contacts(sh: &mut Shell, screen: &mut Screen, _: &str) {
    open_app(sh, screen, App::Contacts(Contacts::new()));
}

fn cmd_calendar(sh: &mut Shell, screen: &mut Screen, _: &str) {
    open_app(sh, screen, App::Calendar(Calendar::new()));
}

fn cmd_settings(sh: &mut Shell, screen: &mut Screen, _: &str) {
    let s = Settings::new(&sh.fs);
    open_app(sh, screen, App::Settings(s));
}

fn cmd_calc(sh: &mut Shell, screen: &mut Screen, _: &str) {
    open_app(sh, screen, App::Calc(Calc::new()));
}

fn cmd_about(sh: &mut Shell, screen: &mut Screen, _: &str) {
    screen.write_colored("Unseen OS 0.3\n", BRIGHT);
    let _ = writeln!(screen, "A small computer that no AI has seen before.");
    let _ = writeln!(screen, "Memory: {} MB", sh.memory_mb);
}
