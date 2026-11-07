//! The OS side of chores: what a chore may see (`OsWorld`), how its changes
//! are made (`apply`), the `do` command's Output screen, and the complete
//! language reference shown by `guide do`.

use crate::calendar;
use crate::chore::{self, Action, World};
use crate::fs::{self, Fs};
use crate::keyboard::Key;
use crate::mail;
use crate::screen::{Screen, ACCENT, NORMAL, WARN};
use crate::ui;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// Pages a chore makes may not grow beyond this (Notes stays usable).
const MAX_PAGE: usize = 16 * 1024;

pub struct OsWorld<'a> {
    pub fs: &'a Fs,
    pub today: String,
}

impl World for OsWorld<'_> {
    fn page(&self, path: &str) -> Option<String> {
        self.fs.read_page(path).ok().map(|t| t.to_string())
    }

    fn is_drawer(&self, path: &str) -> bool {
        self.fs.is_drawer(path)
    }

    fn pages_in(&self, drawer: &str) -> Vec<String> {
        self.fs.children(drawer).into_iter().filter(|(_, d)| !d).map(|(n, _)| fs::join(drawer, &n)).collect()
    }

    fn check_write(&self, path: &str) -> Result<(), String> {
        if let Some((app, _)) = fs::owner(path) {
            return Err(format!(
                "{} is looked after by the {app} app; chores change app pages only with send(...) or event(...)",
                fs::display(path)
            ));
        }
        if self.fs.is_drawer(path) {
            return Err(format!("{} is a drawer, not a page", fs::display(path)));
        }
        if !self.fs.is_drawer(fs::parent(path)) {
            return Err(format!("there is no drawer called {}", fs::display(fs::parent(path))));
        }
        Ok(())
    }

    fn check_toss(&self, path: &str) -> Result<(), String> {
        if let Some((app, _)) = fs::owner(path) {
            return Err(format!("{} is looked after by the {app} app; chores can't toss it", fs::display(path)));
        }
        match self.fs.read_page(path) {
            Ok(_) => Ok(()),
            Err(_) if self.fs.is_drawer(path) => Err(format!("{} is a drawer; chores only toss pages", fs::display(path))),
            Err(_) => Err(format!("there is no page called {}", fs::display(path))),
        }
    }

    fn check_send(&self, to: &str, subject: &str, text: &str) -> Result<(), String> {
        mail::check_send(self.fs, to, subject, text)
    }

    fn check_event(&self, date: &str, time: &str, title: &str, with: &str) -> Result<(), String> {
        calendar::check_event(date, time, title, with)
    }

    fn today(&self) -> String {
        self.today.clone()
    }
}

/// Make all of a chore's changes, or none: they are made on a copy of the
/// desk, which replaces the real one only if every change worked.
/// Returns a line describing each change.
pub fn apply(desk: &mut Fs, actions: &[Action]) -> Result<Vec<String>, String> {
    // Check the room first, so the copy below can never run out of memory.
    let adds: usize = actions
        .iter()
        .map(|a| match a {
            Action::Write { text, .. } => text.len(),
            Action::Add { line, .. } => line.len() + 1,
            Action::Toss { .. } => 0,
            Action::Send { to, subject, text } => to.len() + subject.len() + text.len() + 32,
            Action::Event { date, time, title, with } => date.len() + time.len() + title.len() + with.len() + 32,
        })
        .sum();
    if desk.total_bytes() + adds > fs::MAX_DESK {
        return Err("the desk would be too full (all pages together can hold at most 1 MB of text)".into());
    }
    make(desk, actions)
}

/// Make the changes on a copy of the desk, which replaces the real one only if every change
/// worked. Each change checks the room it needs (the net size: replacing a page with a text
/// of the same length needs no more room). Unseen Script programs use this directly: the
/// engine has already counted how much their changes make the desk grow.
pub fn make(desk: &mut Fs, actions: &[Action]) -> Result<Vec<String>, String> {
    let mut copy = desk.clone();
    let mut done = Vec::new();
    for a in actions {
        let line = match a {
            Action::Write { page, text } => {
                if text.len() > MAX_PAGE {
                    return Err(format!("{} would be too long", fs::display(page)));
                }
                copy.write_page(page, text)?;
                format!("wrote {}", fs::display(page))
            }
            Action::Add { page, line } => {
                let old = copy.read_page(page).map(|t| t.to_string()).unwrap_or_default();
                let new = if old.is_empty() { line.clone() } else { format!("{old}\n{line}") };
                if new.len() > MAX_PAGE {
                    return Err(format!("{} would be too long", fs::display(page)));
                }
                copy.write_page(page, &new)?;
                format!("added a line to {}", fs::display(page))
            }
            Action::Toss { page } => {
                copy.toss(page)?;
                format!("tossed {}", fs::display(page))
            }
            Action::Send { to, subject, text } => {
                let path = mail::deliver(&mut copy, to, subject, text)?;
                format!("sent a mail to {to} ({})", fs::display(&path))
            }
            Action::Event { date, time, title, with } => {
                let path = calendar::add_event(&mut copy, date, time, title, with)?;
                format!("added the event {date} {time} {title} ({})", fs::display(&path))
            }
        };
        done.push(line);
    }
    *desk = copy;
    Ok(done)
}

/// Run the chore on `page` and build its Output screen.
pub fn run(desk: &mut Fs, page: &str, today: String) -> Viewer {
    let title = format!("Chore - {}", fs::display(page));
    let source = match desk.read_page(page) {
        Ok(t) => t.to_string(),
        Err(e) => return Viewer::new(&title, "failed", &[format!("Could not do that: {e}.")], WARN),
    };
    let outcome = chore::run(&source, &OsWorld { fs: desk, today });
    let mut text = Vec::new();
    let (state, color) = match outcome.error {
        Some(e) => {
            text.push(format!("Stopped: {e}"));
            text.push("Nothing was changed.".to_string());
            ("failed", WARN)
        }
        None => match apply(desk, &outcome.actions) {
            Ok(changes) => {
                let s = if changes.len() == 1 { "" } else { "s" };
                text.push(format!("Finished: {} steps, {} change{s} made.", outcome.steps, changes.len()));
                if !outcome.output.is_empty() || !changes.is_empty() {
                    text.push(String::new());
                }
                text.extend(outcome.output.iter().cloned());
                if !changes.is_empty() {
                    if !outcome.output.is_empty() {
                        text.push(String::new());
                    }
                    text.push("Changes:".to_string());
                    text.extend(changes.into_iter().map(|c| format!("  {c}")));
                }
                return Viewer::new(&title, "finished", &text, ACCENT);
            }
            Err(e) => {
                text.push(format!("Stopped while making the changes: {e}"));
                text.push("Nothing was changed.".to_string());
                ("failed", WARN)
            }
        },
    };
    if !outcome.output.is_empty() {
        text.push(String::new());
        text.push("It said this before it stopped:".to_string());
        text.extend(outcome.output);
    }
    Viewer::new(&title, state, &text, color)
}

/// An Output screen keeps at most this many screen lines; the rest are only counted, so
/// the screen never needs much more memory than the text it shows.
const MAX_VIEW_LINES: usize = 6400;
/// The last line of a text is always shown if it fits in this many screen lines (a cut
/// listing, like the history's, ends with a line that says how to see less).
const TAIL_ROWS: usize = 8;

/// The screen lines being made, and how many did not fit.
struct Rows {
    lines: Vec<String>,
    hidden: usize,
    room: usize,
}

impl Rows {
    /// Add one line of text (no new lines in it), cut into pieces of at most WIDTH bytes at
    /// character boundaries; pieces that don't fit are only counted.
    fn line(&mut self, line: &str) {
        let mut rest = line;
        loop {
            let mut cut = rest.len().min(WIDTH);
            while !rest.is_char_boundary(cut) {
                cut += 1;
            }
            if self.lines.len() < self.room {
                self.lines.push(rest[..cut].to_string());
            } else {
                self.hidden += 1;
            }
            rest = &rest[cut..];
            if rest.is_empty() {
                break;
            }
        }
    }

    fn text(&mut self, text: &str) {
        for line in text.split('\n') {
            self.line(line);
        }
    }
}

/// The screen lines for a text given as lines (short lines are moved, not copied).
fn screen_lines(mut text: Vec<String>) -> Vec<String> {
    let tail_rows = |t: &str| t.split('\n').map(|l| l.len().div_ceil(WIDTH).max(1)).sum::<usize>();
    let tail = match text.last() {
        Some(t) if tail_rows(t) <= TAIL_ROWS => text.pop(),
        _ => None,
    };
    let reserve = tail.as_deref().map_or(0, tail_rows) + 1; // the tail, and the "not shown" line
    let mut rows = Rows { lines: Vec::with_capacity(text.len().min(MAX_VIEW_LINES)), hidden: 0, room: MAX_VIEW_LINES - reserve };
    for t in text {
        if t.len() <= WIDTH && !t.contains('\n') && rows.lines.len() < rows.room {
            rows.lines.push(t);
        } else {
            rows.text(&t);
        }
    }
    if rows.hidden > 0 {
        let n = rows.hidden;
        rows.lines.push(format!("({n} more lines are not shown: at most {MAX_VIEW_LINES} fit on this screen)"));
    }
    if let Some(t) = tail {
        rows.room = usize::MAX;
        rows.text(&t);
    }
    rows.lines
}

const BODY_TOP: usize = 2;
const BODY_ROWS: usize = 22;
const WIDTH: usize = 76;

/// A read-only, scrollable text screen (chore output and the chore guide).
pub struct Viewer {
    title: String,
    right: String,
    lines: Vec<String>,
    /// The first line is drawn in this color (the result line).
    first_color: u8,
    top: usize,
}

impl Viewer {
    pub fn new(title: &str, right: &str, text: &[String], first_color: u8) -> Self {
        Self::from_lines(title, right, text.to_vec(), first_color)
    }

    /// Like new, but takes the lines: short lines are moved, not copied, so a long
    /// listing is not held twice in memory.
    pub fn from_lines(title: &str, right: &str, text: Vec<String>, first_color: u8) -> Self {
        Self { title: title.into(), right: right.into(), lines: screen_lines(text), first_color, top: 0 }
    }

    pub fn guide() -> Self {
        let text: Vec<String> = GUIDE.lines().map(|l| l.to_string()).collect();
        Self::new("Guide - chores", "", &text, ACCENT)
    }

    /// `guide script`: the complete Unseen Script reference (the manual has it word for word).
    pub fn script_guide() -> Self {
        let text: Vec<String> = crate::programs::SCRIPT_GUIDE.lines().map(|l| l.to_string()).collect();
        Self::new("Guide - Unseen Script", "", &text, ACCENT)
    }

    fn max_top(&self) -> usize {
        self.lines.len().saturating_sub(BODY_ROWS)
    }

    /// Returns true when the screen should close.
    pub fn key(&mut self, key: Key) -> bool {
        match key {
            Key::Escape => return true,
            Key::Up => self.top = self.top.saturating_sub(1),
            Key::Down => self.top = (self.top + 1).min(self.max_top()),
            Key::Home => self.top = 0,
            Key::End => self.top = self.max_top(),
            _ => {}
        }
        false
    }

    pub fn draw(&self, screen: &mut Screen) {
        ui::clear_body(screen);
        let more = self.top < self.max_top();
        let pos = if self.lines.len() > BODY_ROWS {
            format!("lines {}-{} of {}", self.top + 1, (self.top + BODY_ROWS).min(self.lines.len()), self.lines.len())
        } else {
            String::new()
        };
        let help = if more { "Down: more below   Up/Down scroll   Esc back" } else { "Up/Down scroll   Esc back" };
        let right = if pos.is_empty() { self.right.clone() } else { format!("{}  {pos}", self.right) };
        ui::bars(screen, &self.title, right.trim(), help, "");
        for (i, line) in self.lines.iter().skip(self.top).take(BODY_ROWS).enumerate() {
            let color = if self.top + i == 0 { self.first_color } else { NORMAL };
            screen.put_str(2, BODY_TOP + i, line, color);
        }
        screen.set_cursor(2, BODY_TOP);
    }
}

/// The complete chore reference (`guide do`). Same text in every condition of
/// the experiment; the manual adds explanations and worked examples.
pub const GUIDE: &str = "\
CHORES: the complete reference
A chore is a page of instructions. Write it (for example in Notes), then
run it from the prompt with:   do <page>      e.g.  do desk:count
An Output screen shows what the chore said and whether it worked.

EXAMPLE
  -- count Omar's mails and save the answer
  set total to 0
  for each m in pages('mail:inbox')
    when field(m, 'from') is 'omar'
      set total to total + 1
    done
  done
  say 'Omar sent ' & total & ' mails'
  write('desk:answer', total)

LINES
  One instruction per line. Spaces at the start of a line don't matter.
  -- starts a comment: the rest of the line is ignored.
  Capitals don't matter in words and names (SET = set, Total = total).

INSTRUCTIONS
  set NAME to VALUE       give NAME a value (names: letters, digits, _)
  say VALUE               show a line on the Output screen
  when CONDITION          the lines up to done run only if CONDITION is yes
  otherwise               (optional, inside when) lines for when it is no
  for each NAME in LIST   the lines up to done run once for every item
  repeat NUMBER times     the lines up to done run NUMBER times (0-10000)
  done                    ends a when, for each or repeat block
  stop                    ends the chore here; it still counts as finished
  ACTION(...)             one of the actions below, on a line of its own

VALUES
  numbers    12   3.5   -2
  text       'hello' or \"hello\" (no line breaks; a text can't contain
             its own quote mark, so use \"it's\" or 'say \"hi\"')
  yes, no    the answers of comparisons
  lists      made by pages(...), lines(...) and words(...)
  A name must get a value with set before it is used.

OPERATORS, strongest first
  ( )            brackets
  -              minus in front: -x
  * /            multiply, divide (numbers only)
  + -            add, subtract (numbers only)
  &              join into text: 'total: ' & 5 gives 'total: 5'
  is, is not     the same value? Both sides must be the same kind, and
                 capitals count ('A' is not 'a')
  < > <= >=      compare two numbers, or two texts in alphabetical order
                 (dates written 2026-11-14 compare correctly as text)
  has            text has text: 'has a cat' has 'cat' is yes;
                 list has text: yes if the text is one of the items
  not            not yes is no; not x is y means x is not y
  and, or        combine yes/no answers (and is stronger than or)
  One comparison at a time: a < b and b < c, not a < b < c.
  Mixing kinds is a mistake: 1 + '2' fails. Use number('2').

NUMBERS
  Shown and written like 163, 12.5 or 0.333333 (at most 6 decimals).
  They must stay below 1000000000000. Dividing by 0 is a mistake.

READING
  pages(DRAWER)      list of the pages directly in DRAWER, as paths like
                     mail:inbox:m1, in name order (m2 before m10)
  read(PAGE)         all the text of PAGE
  field(PAGE, KEY)   the value of a 'key: value' line at the top of PAGE
                     (up to the first empty line); '' if there is none
  body(PAGE)         the text after the first empty line ('' if none)
  exists(PATH)       yes if there is a page or drawer at PATH
  lines(TEXT)        list of the lines of TEXT
  words(TEXT)        list of the words of TEXT
  count(X)           number of items in a list, or characters in a text
  item(LIST, N)      item number N of LIST (the first is 1)
  number(TEXT)       text to number: number('12.5') is 12.5
  lower(TEXT)        TEXT in small letters
  today()            today's date, like 2026-11-12
  weekday(DATE)      the day of DATE, like monday
  add_days(DATE, N)  the date N days after DATE (before it if N is below 0)
  Paths always start at the desk: 'notes:list' and 'desk:notes:list' are
  the same page, wherever you are.

ACTIONS (each on a line of its own)
  write(PAGE, TEXT)              make PAGE with this text, or replace it
  add(PAGE, TEXT)                add a line at the end of PAGE (makes it
                                 if needed)
  toss(PAGE)                     throw PAGE away
  send(TO, SUBJECT, TEXT)        send a mail like the Mail app does; TO
                                 must be the mail address of a contact
  event(DATE, TIME, TITLE, WITH) add an event like the Calendar app does:
                                 DATE like 2026-11-14, TIME like 18:00,
                                 WITH can be ''
  write, add and toss can't change the pages of the apps (the drawers
  mail, contacts, calendar and system). Use send and event for those.

ALL OR NOTHING
  The whole page is checked first: a mistake on any line stops the chore
  before anything runs. While it runs, it sees the pages as they were
  when it started. Its changes are made together at the end, and only if
  it finished without a mistake. If it stops, nothing is changed, so you
  can fix the page and run it again.

LIMITS (going over one is a mistake that stops the chore)
  120 lines and 4096 characters per chore; blocks 16 deep; brackets 12
  deep; 4096 characters per text; 1000 items per list; 100 names;
  200 say lines; 200 actions; 50000 steps (every part of a calculation
  and every turn of a loop is a step).";
