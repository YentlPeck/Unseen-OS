//! The lab (design §7): type one line, see its value. Names, tasks, fixed values and `use`
//! lines are kept for the next lines; planned changes are only shown ("would write desk:x"),
//! never made.
//!
//! How it works: the lab keeps the lines that worked, and runs all of them again, with the
//! new line, every time. The desk does not change while the lab is open and a program always
//! does the same on the same desk, so the earlier lines do exactly what they did before, and
//! only what the new line adds is shown. This way a line in the lab means exactly what it
//! means on a page, with the same checks, limits and mistakes.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::lex::{lex, Tok};
use crate::limits::*;
use crate::parse::{builtin, is_instruction_word, parse, Stmt};
use crate::run::{mistake, prepare, Action, Flow, Runner, World};
use crate::util::display;
use crate::value::{Meter, Value};

/// The lab's own page name: never the name of a real page (pages can't have brackets).
pub const LAB_PAGE: &str = "(lab)";
/// A value is shown with at most this many characters.
const MAX_SHOWN: usize = 600;
/// Room kept for one value line in a full lab (a command line is at most 200 characters).
const ROOM_FOR_VALUE: usize = 256;

pub struct Lab {
    /// `use` lines that worked (they must come first on a page).
    uses: Vec<String>,
    /// The other lines that worked, in order.
    kept: Vec<String>,
    /// The lines of a block being typed (a `when`, `task`, ... until its `done`).
    pending: Vec<String>,
    depth: usize,
    /// How many say lines and planned changes the kept lines make (shown already).
    said: usize,
    planned: usize,
    /// A fingerprint of those say lines and changes, to check that running the kept lines
    /// again does exactly the same.
    print: u64,
}

/// FNV-1a over the say lines and planned changes (written out with their kind and fields).
fn fingerprint(said: &[String], planned: &[Action]) -> u64 {
    struct H(u64);
    impl core::fmt::Write for H {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            for b in s.bytes().chain(core::iter::once(0xff)) {
                self.0 = (self.0 ^ b as u64).wrapping_mul(0x100000001b3);
            }
            Ok(())
        }
    }
    use core::fmt::Write;
    let mut h = H(0xcbf29ce484222325);
    for s in said {
        let _ = h.write_str(s);
    }
    let _ = h.write_str("--changes--");
    for a in planned {
        let _ = write!(h, "{a:?}");
    }
    h.0
}

impl Default for Lab {
    fn default() -> Self {
        Self::new()
    }
}

impl Lab {
    pub fn new() -> Self {
        Self {
            uses: Vec::new(),
            kept: Vec::new(),
            pending: Vec::new(),
            depth: 0,
            said: 0,
            planned: 0,
            print: fingerprint(&[], &[]),
        }
    }

    /// True while a block is being typed: the next line belongs to it.
    pub fn in_block(&self) -> bool {
        self.depth > 0
    }

    /// Type one line. Returns what to show (nothing while a block is still being typed).
    pub fn line<W: World>(&mut self, input: &str, world: &W) -> Vec<String> {
        let first = first_word(input);
        match first.as_deref() {
            // only a real block header starts a block (`while.done` is a value, not a while)
            Some("when" | "for" | "repeat" | "while" | "task" | "try" | "test") if is_header(input) => self.depth += 1,
            Some("done") if self.depth > 0 => self.depth -= 1,
            _ => {}
        }
        self.pending.push(input.to_string());
        if self.depth > 0 {
            if self.pending.len() > MAX_PAGE_LINES {
                self.forget_block();
                return alloc::vec![String::from("mistake: that block is too long; it was forgotten")];
            }
            return Vec::new();
        }
        let entry = core::mem::take(&mut self.pending);
        self.enter(entry, world)
    }

    /// Forget the block being typed.
    pub fn forget_block(&mut self) {
        self.pending.clear();
        self.depth = 0;
    }

    fn enter<W: World>(&mut self, entry: Vec<String>, world: &W) -> Vec<String> {
        let single = entry.len() == 1;
        // a use line only if it reads as one (`use.done` is a value: a name called use)
        let is_use = single && first_word(&entry[0]).as_deref() == Some("use") && parse(&entry[0]).is_ok();
        let is_value = single && !is_use && is_value_line(&entry[0]);
        // the program: the use lines, the kept lines, then the new entry
        let mut lines: Vec<&str> = self.uses.iter().map(|s| s.as_str()).collect();
        let entry_at; // the line number of the entry's first line
        if is_use {
            entry_at = lines.len() + 1;
            lines.push(&entry[0]);
            lines.extend(self.kept.iter().map(|s| s.as_str()));
        } else {
            lines.extend(self.kept.iter().map(|s| s.as_str()));
            entry_at = lines.len() + 1;
            lines.extend(entry.iter().map(|s| s.as_str()));
        }
        let say_line;
        if is_value {
            // checked like `say VALUE`, so the value line gets every check a page line gets
            say_line = format!("say {}", entry[0]);
            lines.pop();
            lines.push(&say_line);
        }
        let source = lines.join("\n");
        let lab_lines = lines.len();
        drop(lines);
        // a full lab keeps room for one more value line, so names can still be looked at
        if !is_value && (lab_lines >= MAX_PAGE_LINES || source.len() > MAX_PAGE_CHARS - ROOM_FOR_VALUE) {
            return alloc::vec![full(lab_lines)];
        }
        let mut program = match prepare(LAB_PAGE, &source, world) {
            Ok(p) => p,
            Err(e) => return alloc::vec![self.problem(&e, entry_at, entry.len(), lab_lines)],
        };
        let value = if is_value {
            match program.main.pop() {
                Some(l) => match l.stmt {
                    Stmt::Say(e) => Some((l.n, e)),
                    _ => return alloc::vec![String::from("mistake: the lab could not read that line")],
                },
                None => None,
            }
        } else {
            None
        };
        let m = Meter::new(STEP_MEMORY);
        let mut r = Runner::new(world, &program, &m);
        let mut result = r.fixed_values().and_then(|_| r.block(&program.main));
        let mut shown = None;
        if let (Ok(Flow::Next), Some((n, e))) = (&result, &value) {
            match r.eval(e) {
                Ok(v) => shown = Some(show(&v)),
                Err(f) => result = Err((*n, f)),
            }
        }
        // Did the earlier lines do exactly what they did before? A new fixed value or use line
        // can change them (they are seen from every line): then the new line is not kept.
        let same_before = r.out.len() >= self.said
            && r.actions.len() >= self.planned
            && fingerprint(&r.out[..self.said], &r.actions[..self.planned]) == self.print;
        let mut out: Vec<String> = Vec::new();
        if same_before {
            out.extend(r.out.iter().skip(self.said).cloned());
        }
        let stopped = matches!(result, Ok(Flow::Stop)) || matches!(&result, Err((_, f)) if f.stop);
        match result {
            Err((line, f)) if !stopped => {
                let e = mistake(&program, line, f);
                out.push(self.problem(&e, entry_at, entry.len(), lab_lines));
                return out;
            }
            _ if !same_before => {
                return alloc::vec![String::from(
                    "mistake: this line changes what the lines before it did (they would now say or \
                     plan something else), so it was not kept; press Esc and start again with it first"
                )];
            }
            _ if stopped => {
                out.extend(r.actions.iter().skip(self.planned).map(would));
                out.push(String::from("(stopped: lines after a stop never run, so this line was not kept)"));
                return out;
            }
            _ => {}
        }
        out.extend(r.actions.iter().skip(self.planned).map(would));
        if let Some(s) = shown {
            out.push(s);
        }
        if !is_value {
            self.said = r.out.len();
            self.planned = r.actions.len();
            self.print = fingerprint(&r.out, &r.actions);
            if is_use {
                self.uses.extend(entry);
            } else {
                self.kept.extend(entry);
            }
        }
        out
    }

    /// A mistake as the lab shows it: on the lab's own lines without the page name.
    fn problem(&self, e: &crate::run::Mistake, entry_at: usize, entry_len: usize, lab_lines: usize) -> String {
        if e.page != LAB_PAGE {
            return format!("mistake: {e}");
        }
        if e.line == 0 && e.before_running {
            return full(lab_lines); // the lab's page itself is too big
        }
        let mut s = String::from("mistake: ");
        if entry_len > 1 && e.line >= entry_at && e.line < entry_at + entry_len {
            s.push_str(&format!("line {} of the block: ", e.line + 1 - entry_at));
        }
        s.push_str(&e.message);
        for (p, l) in &e.trail {
            if p != LAB_PAGE {
                s.push_str(&format!("\n  called from {} line {l}", display(p)));
            }
        }
        s
    }
}

fn full(lines: usize) -> String {
    format!(
        "mistake: the lab is full ({lines} lines; it keeps at most {} lines and {} characters); \
         press Esc and open it again",
        MAX_PAGE_LINES - 1,
        MAX_PAGE_CHARS - ROOM_FOR_VALUE
    )
}

/// True if the line can start a block: it reads as the header of one (with an end after it).
fn is_header(line: &str) -> bool {
    let end = if first_word(line).as_deref() == Some("try") { "instead e\ndone" } else { "done" };
    match parse(&format!("{line}\n{end}")) {
        Ok(_) => true,
        Err((n, _)) => n != 1,
    }
}

/// The first word of a line, in small letters (None if it starts with something else).
fn first_word(line: &str) -> Option<String> {
    match lex(line).ok()?.first()? {
        Tok::Word(w) => Some(w.clone()),
        _ => None,
    }
}

/// True if the line is a value to show (not an instruction): `1 + 2`, `count(items)`,
/// `tax(5)` (a task: its value is shown), `items`. A line that can't be read is not a value
/// (it is run as an instruction, so it gets the page's mistake message).
fn is_value_line(line: &str) -> bool {
    let Ok(toks) = lex(line) else { return false };
    match toks.first() {
        None => false,
        Some(Tok::Word(w)) => {
            if toks.get(1) == Some(&Tok::Sym("(")) {
                match builtin(w) {
                    Some((_, _, action)) => !action, // write(...) is an instruction
                    None => !is_instruction_word(w), // a task gives a value
                }
            } else if !is_instruction_word(w) {
                true
            } else {
                // a name that is also an instruction word (`while.done`): a value if the line
                // can't be read as an instruction but can be read as a value
                !matches!(w.as_str(), "done" | "otherwise" | "or" | "instead")
                    && matches!(parse(line), Err((1, _)))
                    && parse(&format!("say {line}")).is_ok()
            }
        }
        Some(_) => true,
    }
}

fn would(a: &Action) -> String {
    match a {
        Action::Write { page, .. } => format!("would write {}", display(page)),
        Action::Add { page, .. } => format!("would add a line to {}", display(page)),
        Action::Toss { page } => format!("would toss {}", display(page)),
        Action::Send { to, subject, .. } => format!("would send a mail to {to}: {subject}"),
        Action::Event { date, time, title, .. } => format!("would add the event {date} {time} {title}"),
    }
}

/// A value as you would type it: texts in quotes, lists in [ ], records in { }.
pub fn show(v: &Value) -> String {
    let mut s = String::new();
    write_shown(v, &mut s);
    if s.len() > MAX_SHOWN {
        let mut cut = MAX_SHOWN;
        while !s.is_char_boundary(cut) {
            cut -= 1;
        }
        s.truncate(cut);
        s.push_str(" ...");
    }
    s
}

/// Every value written adds at least one character, so this stops after at most MAX_SHOWN
/// values, however big the value is.
fn write_shown(v: &Value, s: &mut String) {
    if s.len() > MAX_SHOWN {
        return;
    }
    match v {
        Value::Text(t) => {
            s.push('\'');
            for (i, part) in t.s.split('\'').enumerate() {
                if s.len() > MAX_SHOWN {
                    return;
                }
                if i > 0 {
                    s.push_str("''");
                }
                let room = MAX_SHOWN + 1 - s.len().min(MAX_SHOWN + 1);
                match part.char_indices().nth(room) {
                    Some((cut, _)) => s.push_str(&part[..cut]),
                    None => s.push_str(part),
                }
            }
            s.push('\'');
        }
        Value::Nothing => s.push_str("nothing()"),
        Value::List(l) => {
            s.push('[');
            for (i, x) in l.v.iter().enumerate() {
                if s.len() > MAX_SHOWN {
                    return;
                }
                if i > 0 {
                    s.push_str(", ");
                }
                write_shown(x, s);
            }
            s.push(']');
        }
        Value::Rec(r) => {
            s.push('{');
            for (i, (k, x)) in r.f.iter().enumerate() {
                if s.len() > MAX_SHOWN {
                    return;
                }
                if i > 0 {
                    s.push_str(", ");
                }
                s.push_str(k);
                s.push_str(": ");
                write_shown(x, s);
            }
            s.push('}');
        }
        other => crate::run::write_text(other, s),
    }
}
