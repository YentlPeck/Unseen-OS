//! The OS side of Unseen Script (docs/script-design.md §7): what a program may see
//! (`ScriptWorld`), the `run` and `test` commands' Output screens, the Lab screen, and the
//! line commands `view`, `edit`, `insert` and `cut`.
//!
//! Chores are untouched: `do` still uses chore.rs, so experiments 1-4 stay as they were.

use crate::calendar;
use crate::chore;
use crate::chores::{self, Viewer};
use crate::fs::{self, Fs};
use crate::keyboard::Key;
use crate::mail;
use crate::screen::{Screen, ACCENT, BRIGHT, NORMAL, WARN};
use crate::ui::{self, LineEdit};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use script::{Action, Lab, World};

/// Pages a program makes may not grow beyond this (as chores; Notes stays usable).
const MAX_PAGE: usize = 16 * 1024;
/// A line made with edit or insert fits on one Notes line.
pub const MAX_LINE: usize = ui::MAX_LINE;
/// `view` shows at most this many lines at a time.
const VIEW_ROWS: usize = 20;

pub struct ScriptWorld<'a> {
    pub fs: &'a Fs,
    pub today: String,
}

impl World for ScriptWorld<'_> {
    fn page(&self, path: &str) -> Option<String> {
        self.fs.read_page(path).ok().map(|t| t.to_string())
    }

    fn page_len(&self, path: &str) -> Option<usize> {
        self.fs.read_page(path).ok().map(|t| t.len())
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
                "{} is looked after by the {app} app; programs change app pages only with send(...) or event(...)",
                fs::display(path)
            ));
        }
        if path.len() > fs::MAX_PATH {
            return Err(format!("that path is too long (at most {} characters)", fs::MAX_PATH));
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
            return Err(format!("{} is looked after by the {app} app; programs can't toss it", fs::display(path)));
        }
        match self.fs.read_page(path) {
            Ok(_) => Ok(()),
            Err(_) if self.fs.is_drawer(path) => Err(format!("{} is a drawer; programs only toss pages", fs::display(path))),
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

    fn max_page(&self) -> usize {
        MAX_PAGE
    }

    fn free_space(&self) -> usize {
        fs::MAX_DESK.saturating_sub(self.fs.total_bytes())
    }

    fn free_items(&self) -> usize {
        fs::MAX_ITEMS.saturating_sub(self.fs.count())
    }
}

/// A program's changes are made like a chore's: all together on a copy of the desk, or none.
/// (Without the chores' first check of the total they add: the engine counts the net growth.)
fn apply(desk: &mut Fs, actions: Vec<Action>) -> Result<Vec<String>, String> {
    let actions: Vec<chore::Action> = actions
        .into_iter()
        .map(|a| match a {
            Action::Write { page, text } => chore::Action::Write { page, text },
            Action::Add { page, line } => chore::Action::Add { page, line },
            Action::Toss { page } => chore::Action::Toss { page },
            Action::Send { to, subject, text } => chore::Action::Send { to, subject, text },
            Action::Event { date, time, title, with } => chore::Action::Event { date, time, title, with },
        })
        .collect();
    chores::make(desk, &actions)
}

/// `run <page>`: run the program on `page` and build its Output screen.
pub fn run(desk: &mut Fs, page: &str, today: String) -> Viewer {
    let title = format!("Program - {}", fs::display(page));
    let source = match desk.read_page(page) {
        Ok(t) => t.to_string(),
        Err(e) => return Viewer::new(&title, "failed", &[format!("Could not do that: {e}.")], WARN),
    };
    let outcome = script::run(page, &source, &ScriptWorld { fs: desk, today });
    drop(source);
    let mut text = Vec::new();
    let (state, color) = match outcome.error {
        Some(e) => {
            text.push(format!("Stopped: {e}"));
            text.push("Nothing was changed.".to_string());
            ("failed", WARN)
        }
        None => match apply(desk, outcome.actions) {
            Ok(changes) => {
                let s = if changes.len() == 1 { "" } else { "s" };
                text.push(format!("Finished: {} steps, {} change{s} made.", outcome.steps, changes.len()));
                if !outcome.output.is_empty() || !changes.is_empty() {
                    text.push(String::new());
                }
                let said = !outcome.output.is_empty();
                text.extend(outcome.output);
                if !changes.is_empty() {
                    if said {
                        text.push(String::new());
                    }
                    text.push("Changes:".to_string());
                    text.extend(changes.into_iter().map(|c| format!("  {c}")));
                }
                return Viewer::from_lines(&title, "finished", text, ACCENT);
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
    Viewer::from_lines(&title, state, text, color)
}

/// `test <page>`: run the tests on a test page and show the report.
pub fn test(desk: &Fs, page: &str, today: String) -> Viewer {
    let title = format!("Tests - {}", fs::display(page));
    let source = match desk.read_page(page) {
        Ok(t) => t.to_string(),
        Err(e) => return Viewer::new(&title, "failed", &[format!("Could not do that: {e}.")], WARN),
    };
    let report = script::run_tests(page, &source, &ScriptWorld { fs: desk, today });
    drop(source);
    let passed = report.results.iter().filter(|r| r.passed).count();
    let all = report.error.is_none() && !report.results.is_empty() && passed == report.results.len();
    let right = if report.error.is_some() { String::from("failed") } else { format!("{passed} of {} passed", report.results.len()) };
    // the summary line first, then one line per test
    let mut lines = report.lines();
    if let Some(last) = lines.pop() {
        lines.insert(0, last);
        if lines.len() > 1 {
            lines.insert(1, String::new());
        }
    }
    Viewer::from_lines(&title, &right, lines, if all { ACCENT } else { WARN })
}

// ---- the Lab screen ----------------------------------------------------------------------

const LAB_TOP: usize = 1;
/// Rows 1-22 show what was typed and shown; row 23 is the line being typed.
const LAB_ROWS: usize = 22;
const LAB_WIDTH: usize = 78;
/// The lab remembers at most this many screen lines of what was typed and shown.
const LAB_KEEP: usize = 400;

pub struct LabApp {
    lab: Lab,
    /// Today's date, fixed when the lab opens (the kept lines are run again with each line,
    /// so they must see the same date every time).
    today: String,
    input: LineEdit,
    /// What was typed and shown, as screen lines (with the color of each).
    seen: Vec<(String, u8)>,
    /// The last line typed (Up brings it back).
    last: String,
    /// Lines scrolled back from the bottom.
    back: usize,
    message: String,
}

impl LabApp {
    pub fn new(today: String) -> Self {
        let mut a = Self {
            lab: Lab::new(),
            today,
            input: LineEdit::new("", LAB_WIDTH - 4),
            seen: Vec::new(),
            last: String::new(),
            back: 0,
            message: String::new(),
        };
        a.note("Type a line of Unseen Script and press Enter: a value is shown, an", NORMAL);
        a.note("instruction is carried out. Names, tasks and use lines are kept while the", NORMAL);
        a.note("lab is open. Changes to pages are only shown (would write ...), never made.", NORMAL);
        a.note("", NORMAL);
        a
    }

    fn note(&mut self, text: &str, color: u8) {
        // one line of the text at a time, trimmed now and then (not after every line), so
        // even a text of thousands of lines never needs more than about 2 * LAB_KEEP rows
        for line in text.split('\n') {
            for l in ui::wrap(line, LAB_WIDTH) {
                self.seen.push((l, color));
            }
            if self.seen.len() > 2 * LAB_KEEP {
                let extra = self.seen.len() - LAB_KEEP;
                self.seen.drain(..extra);
            }
        }
    }

    fn prompt(&self) -> &'static str {
        if self.lab.in_block() { "... " } else { ">   " }
    }

    /// Returns true when the screen should close (Esc).
    pub fn key(&mut self, fs: &Fs, key: Key) -> bool {
        self.message.clear();
        match key {
            Key::Escape => return true,
            Key::Ctrl(b'x') if self.lab.in_block() => {
                self.lab.forget_block();
                self.note("(the block was forgotten)", WARN);
            }
            Key::Up => {
                self.input = LineEdit::new(&self.last, LAB_WIDTH - 4);
            }
            Key::Ctrl(b'u') => self.back = (self.back + LAB_ROWS / 2).min(self.seen.len().saturating_sub(LAB_ROWS)),
            Key::Ctrl(b'd') => self.back = self.back.saturating_sub(LAB_ROWS / 2),
            Key::Enter => {
                if self.input.overflow {
                    self.message = ui::OVERFLOW_MSG.into();
                    return false;
                }
                let line = core::mem::replace(&mut self.input, LineEdit::new("", LAB_WIDTH - 4)).text;
                let typed = format!("{}{line}", self.prompt());
                self.note(&typed, BRIGHT);
                if !line.trim().is_empty() {
                    self.last = line.clone();
                }
                crate::log!("lab: {}", line);
                let shown = self.lab.line(&line, &ScriptWorld { fs, today: self.today.clone() });
                for s in shown {
                    let color = if s.starts_with("mistake: ") { WARN } else if s.starts_with("would ") { ACCENT } else { NORMAL };
                    self.note(&s, color);
                }
                self.back = 0;
            }
            k => {
                if self.input.key(k) == ui::Edit::Full {
                    self.message = format!("A lab line is at most {} characters.", LAB_WIDTH - 4);
                }
            }
        }
        false
    }

    pub fn draw(&self, screen: &mut Screen) {
        ui::clear_body(screen);
        let help = if self.lab.in_block() {
            "Enter next line   Ctrl+X forget the block   Esc leave (nothing is kept)"
        } else {
            "Enter run   Up last line   Ctrl+U/Ctrl+D scroll   Esc leave (nothing is kept)"
        };
        let right = if self.back > 0 { "scrolled back" } else { "" };
        ui::bars(screen, "Lab - try Unseen Script", right, help, &self.message);
        let end = self.seen.len() - self.back.min(self.seen.len());
        let start = end.saturating_sub(LAB_ROWS);
        for (i, (l, c)) in self.seen[start..end].iter().enumerate() {
            screen.put_str(1, LAB_TOP + i, l, *c);
        }
        let row = LAB_TOP + LAB_ROWS;
        screen.put_str(1, row, self.prompt(), ACCENT);
        let col = self.input.draw(screen, 5, row, LAB_WIDTH - 4, BRIGHT);
        screen.set_cursor(col, row);
    }
}

// ---- view, edit, insert, cut ------------------------------------------------------------
//
// A page's lines are its text split at each new line, as Notes shows them: a page always
// has at least one line (an empty page is one empty line), and "a\n" is two lines. The
// commands work on byte ranges of the text, never on a list of all its lines, so a page with
// very many lines costs no more memory than its own text.

/// How many lines a page has.
fn line_count(text: &str) -> usize {
    text.bytes().filter(|&b| b == b'\n').count() + 1
}

/// The byte range of line `n` (from 1; it must be there), without its new line.
fn line_span(text: &str, n: usize) -> (usize, usize) {
    let mut start = 0;
    let mut k = 1;
    for (i, b) in text.bytes().enumerate() {
        if b == b'\n' {
            if k == n {
                return (start, i);
            }
            k += 1;
            start = i + 1;
        }
    }
    (start, text.len())
}

/// A line with its number, as view shows it.
fn numbered(n: usize, line: &str) -> String {
    format!("{n:>4}  {line}")
}

fn lines_word(n: usize) -> &'static str {
    if n == 1 { "line" } else { "lines" }
}

/// `view <page> [from] [to]`: the lines from..to (at most VIEW_ROWS of them), with numbers.
pub fn view(desk: &Fs, path: &str, from: Option<usize>, to: Option<usize>) -> Result<Vec<String>, String> {
    let text = desk.read_page(path)?;
    let total = line_count(text);
    let from = from.unwrap_or(1).max(1);
    if from > total {
        return Err(format!("{} has only {total} {}", fs::display(path), lines_word(total)));
    }
    let to = to.unwrap_or(total).min(total).min(from + VIEW_ROWS - 1);
    if to < from {
        return Err("the second line number must not be smaller than the first".into());
    }
    let mut out = Vec::new();
    out.push(format!("{} lines {from}-{to} of {total}:", fs::display(path)));
    for (i, l) in text.split('\n').skip(from - 1).take(to + 1 - from).enumerate() {
        // a very long line is cut (read shows all of it)
        let end = (0..=l.len().min(MAX_LINE + 2)).rev().find(|&e| l.is_char_boundary(e)).unwrap_or(0);
        out.push(numbered(from + i, &l[..end]));
    }
    if to < total {
        out.push(format!("(more below: view {} {})", fs::display(path), to + 1));
    }
    Ok(out)
}

fn check_line(text: &str) -> Result<(), String> {
    if text.len() > MAX_LINE {
        return Err(format!("that line is {} characters long; a line is at most {MAX_LINE}", text.len()));
    }
    if text.contains('\n') {
        return Err("a line can't hold a new line".into());
    }
    Ok(())
}

fn line_there(total: usize, n: usize) -> Result<(), String> {
    if n == 0 || n > total {
        Err(format!("there is no line {n}; the page has {} 1-{total}", lines_word(total)))
    } else {
        Ok(())
    }
}

/// `edit <page> <line> <text>`: replace one line.
pub fn edit(desk: &mut Fs, path: &str, n: usize, line: &str) -> Result<String, String> {
    check_line(line)?;
    let text = desk.read_page(path)?;
    line_there(line_count(text), n)?;
    let (a, b) = line_span(text, n);
    let new = [&text[..a], line, &text[b..]].concat();
    desk.write_page(path, &new)?;
    Ok(format!("Line {n} of {} is now:\n{}", fs::display(path), numbered(n, line)))
}

/// `insert <page> <line> <text>`: put a new line before line N (N = last + 1 adds it at the
/// end). A page that isn't there yet is made by inserting its line 1.
pub fn insert(desk: &mut Fs, path: &str, n: usize, line: &str) -> Result<String, String> {
    check_line(line)?;
    let new = match desk.read_page(path) {
        Ok(text) => {
            let total = line_count(text);
            if n == 0 || n > total + 1 {
                return Err(format!("a line can go before lines 1-{total} (or {} to add it at the end)", total + 1));
            }
            if n == total + 1 {
                [text, "\n", line].concat()
            } else {
                let (a, _) = line_span(text, n);
                [&text[..a], line, "\n", &text[a..]].concat()
            }
        }
        Err(_) if !desk.exists(path) && n == 1 => line.to_string(),
        Err(_) if !desk.exists(path) => {
            return Err(format!("there is no page called {}; insert its line 1 to make it", fs::display(path)))
        }
        Err(e) => return Err(e),
    };
    let total = line_count(&new);
    desk.write_page(path, &new)?;
    Ok(format!("Put in line {n} of {} ({total} {} now):\n{}", fs::display(path), lines_word(total), numbered(n, line)))
}

/// `cut <page> <line> [to]`: remove one line, or the lines from..to. Cutting every line leaves
/// the page empty (one empty line).
pub fn cut(desk: &mut Fs, path: &str, from: usize, to: Option<usize>) -> Result<String, String> {
    let text = desk.read_page(path)?;
    let total = line_count(text);
    let to = to.unwrap_or(from);
    line_there(total, from)?;
    line_there(total, to)?;
    if to < from {
        return Err("the second line number must not be smaller than the first".into());
    }
    let new = if to < total {
        // from the start of line `from` to the start of the line after `to`
        let (a, _) = line_span(text, from);
        let (_, b) = line_span(text, to);
        [&text[..a], &text[b + 1..]].concat()
    } else if from > 1 {
        // the last lines: from the end of the line before `from`
        let (_, a) = line_span(text, from - 1);
        text[..a].to_string()
    } else {
        String::new()
    };
    let left = line_count(&new);
    desk.write_page(path, &new)?;
    let what = if to == from { format!("line {from}") } else { format!("lines {from}-{to}") };
    if new.is_empty() {
        return Ok(format!("Cut {what} of {}; it is empty now.", fs::display(path)));
    }
    Ok(format!("Cut {what} of {} ({left} {} now).", fs::display(path), lines_word(left)))
}
