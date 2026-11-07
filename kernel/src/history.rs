//! Version history for the desk: "stamps".
//!
//! A stamp is a saved copy of the desk at one moment, with a number, a date and a
//! short note. Every name and text is kept only once for the whole history (a pool
//! of shared strings), so a stamp costs the text that is new plus a little room per
//! drawer and page. The drawers looked after by an app (mail, contacts, calendar,
//! system) are not part of the history: only their apps change them, and a rewind
//! never un-sends a mail.
//!
//! The desk is always "on" one stamp (the base). `changes` compares the desk with
//! it, `stamp` saves a new one on top of it, `rewind` puts the whole desk back as
//! it was in an older stamp (later stamps are kept, so nothing is ever lost), and
//! `bring` brings back one page from a stamp. A command that fails changes nothing.
//!
//! Memory (the heap is 16 MB): the history keeps at most MAX_HISTORY bytes, counting
//! new text, new names and ENTRY_COST per drawer or page per stamp. A listing of
//! changes is cut off at MAX_LINES lines / MAX_OUT_BYTES bytes while it is made, and
//! pages longer than MAX_PAGE_LINES lines are not compared line by line.

use crate::fs::{self, Entry, Fs};
use alloc::collections::{BTreeMap, BTreeSet};
use core::iter::Peekable;
use alloc::format;
use alloc::rc::Rc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// At most this many stamps.
pub const MAX_STAMPS: usize = 99;
/// Room for the whole history: new text and names, plus ENTRY_COST per entry per stamp.
pub const MAX_HISTORY: usize = 2 * 1024 * 1024;
/// What one drawer or page costs in one stamp (its place in the stamp's list).
pub const ENTRY_COST: usize = 64;
/// What a newly kept string costs besides its text (its own block and its place in the pool).
const STRING_COST: usize = 96;
/// A desk with more drawers and pages than this (outside the app drawers) can't be stamped.
pub const MAX_ENTRIES: usize = fs::MAX_ITEMS;
/// At most this many start-file problems are kept, each at most this long.
const MAX_START_ERRORS: usize = 20;
const MAX_ERROR_LEN: usize = 160;
/// Notes are short: one line, at most this many characters.
pub const MAX_NOTE: usize = 60;
/// A changes/compare listing holds at most this many lines and bytes.
pub const MAX_LINES: usize = 6000;
pub const MAX_OUT_BYTES: usize = 256_000;
/// Longer page versions are not compared line by line.
pub const MAX_PAGE_LINES: usize = 3000;
/// Line diffs use at most this many table cells; bigger changes are shown as
/// "all old lines out, all new lines in".
const MAX_CELLS: usize = 250_000;

pub type Result<T> = core::result::Result<T, String>;

/// One saved desk: path -> page text, or None for a drawer.
pub type Snap = BTreeMap<Rc<str>, Option<Rc<str>>>;

/// One drawer or page of a desk (saved or current): path, and page text (None for a drawer).
type Ent<'a> = (&'a str, Option<&'a str>);

pub struct Stamp {
    pub number: usize,
    /// The stamp the desk was on when this one was made.
    pub from: Option<usize>,
    pub date: String,
    pub note: String,
    pub desk: Snap,
}

pub struct History {
    stamps: Vec<Stamp>,
    base: Option<usize>,
    bytes: usize,
    /// Names and texts kept, each exactly once.
    pool: BTreeSet<Rc<str>>,
    /// Problems with "#stamp" lines in the start files (shown by the inspector).
    pub start_errors: Vec<String>,
}

fn tracked(path: &str) -> bool {
    fs::owner(path).is_none()
}

/// The desk's drawers and pages outside the app drawers, sorted by path. Allocates nothing.
fn desk_items(desk: &Fs) -> impl Iterator<Item = Ent<'_>> {
    desk.all()
        .filter(|(p, _)| tracked(p))
        .map(|(p, e)| (p.as_str(), match e { Entry::Page(t) => Some(t.as_str()), Entry::Drawer => None }))
}

/// A stamp's drawers and pages, sorted by path (the same order as desk_items).
fn snap_items(snap: &Snap) -> impl Iterator<Item = Ent<'_>> {
    snap.iter().map(|(p, t)| (&**p, t.as_deref()))
}

/// Walk two sorted lists of entries side by side: (path, old, new), where old/new is
/// None if the path is missing on that side. Allocates nothing.
struct Merge<'a, A: Iterator<Item = Ent<'a>>, B: Iterator<Item = Ent<'a>>> {
    a: Peekable<A>,
    b: Peekable<B>,
}

type Pair<'a> = (&'a str, Option<Option<&'a str>>, Option<Option<&'a str>>);

impl<'a, A: Iterator<Item = Ent<'a>>, B: Iterator<Item = Ent<'a>>> Iterator for Merge<'a, A, B> {
    type Item = Pair<'a>;

    fn next(&mut self) -> Option<Pair<'a>> {
        let (pa, pb) = (self.a.peek().map(|e| e.0), self.b.peek().map(|e| e.0));
        match (pa, pb) {
            (None, None) => None,
            (Some(x), Some(y)) if x == y => {
                let (p, t) = self.a.next()?;
                let (_, u) = self.b.next()?;
                Some((p, Some(t), Some(u)))
            }
            (Some(x), Some(y)) if x < y => self.a.next().map(|(p, t)| (p, Some(t), None)),
            (Some(_), None) => self.a.next().map(|(p, t)| (p, Some(t), None)),
            _ => self.b.next().map(|(p, u)| (p, None, Some(u))),
        }
    }
}

fn merge<'a, A: Iterator<Item = Ent<'a>>, B: Iterator<Item = Ent<'a>>>(a: A, b: B) -> Merge<'a, A, B> {
    Merge { a: a.peekable(), b: b.peekable() }
}

/// How many paths differ between two sorted lists of entries.
fn count_differing<'a>(a: impl Iterator<Item = Ent<'a>>, b: impl Iterator<Item = Ent<'a>>) -> usize {
    merge(a, b).filter(|(_, x, y)| x != y).count()
}

/// "#3", "3" -> 3. Anything made only of digits is a stamp number (too big ones become
/// a number no stamp has).
pub fn parse_number(s: &str) -> Option<usize> {
    let s = s.trim();
    let s = s.strip_prefix('#').unwrap_or(s);
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some(s.parse().unwrap_or(usize::MAX))
}

fn looks_like_date(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10 && b[4] == b'-' && b[7] == b'-' && b.iter().enumerate().all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// How many lines a page has (an empty page has none). Allocates nothing.
fn line_count(text: &str) -> usize {
    if text.is_empty() { 0 } else { text.bytes().filter(|&b| b == b'\n').count() + 1 }
}

/// The lines of a page; an empty page has none.
fn lines(text: &str) -> Vec<&str> {
    if text.is_empty() { Vec::new() } else { text.split('\n').collect() }
}

/// Is `path` the filter itself or inside it? ("" = everything)
fn in_filter(path: &str, filter: &str) -> bool {
    filter.is_empty()
        || path == filter
        || (path.len() > filter.len() && path.starts_with(filter) && path.as_bytes()[filter.len()] == b':')
}

impl History {
    pub fn new() -> Self {
        Self { stamps: Vec::new(), base: None, bytes: 0, pool: BTreeSet::new(), start_errors: Vec::new() }
    }

    pub fn stamps(&self) -> &[Stamp] {
        &self.stamps
    }

    pub fn base(&self) -> Option<usize> {
        self.base
    }

    /// Bytes the history uses (as counted against MAX_HISTORY).
    #[allow(dead_code)]
    pub fn used(&self) -> usize {
        self.bytes
    }

    fn get(&self, n: usize) -> Result<&Stamp> {
        self.stamps.get(n.wrapping_sub(1)).ok_or_else(|| match self.stamps.len() {
            0 => "there are no stamps yet".to_string(),
            k if n > 99_999 => format!("there is no such stamp (the stamps are #1 to #{k})"),
            k => format!("there is no stamp #{n} (the stamps are #1 to #{k})"),
        })
    }

    fn base_stamp(&self) -> Result<&Stamp> {
        match self.base {
            Some(n) => self.get(n),
            None => Err("there are no stamps yet (stamp <note> makes the first one)".into()),
        }
    }

    fn pooled(&self, s: &str) -> Option<&Rc<str>> {
        self.pool.get(s)
    }

    fn intern(&mut self, s: &str) -> Rc<str> {
        if let Some(r) = self.pooled(s) {
            return r.clone();
        }
        let r: Rc<str> = Rc::from(s);
        self.pool.insert(r.clone());
        r
    }

    /// Keep a start-file problem (short, and only the first few).
    pub fn start_error(&mut self, msg: &str) {
        if self.start_errors.len() > MAX_START_ERRORS {
            return;
        }
        if self.start_errors.len() == MAX_START_ERRORS {
            self.start_errors.push("(more problems not shown)".to_string());
            return;
        }
        let mut msg = msg.to_string();
        if msg.len() > MAX_ERROR_LEN {
            let mut cut = MAX_ERROR_LEN;
            while !msg.is_char_boundary(cut) {
                cut -= 1;
            }
            msg.truncate(cut);
            msg.push_str("...");
        }
        self.start_errors.push(msg);
    }

    /// How many drawers and pages changed since the stamp the desk is on (all of them
    /// if there is none). Allocates nothing.
    pub fn unstamped_count(&self, desk: &Fs) -> usize {
        match self.base_stamp() {
            Ok(s) => count_differing(snap_items(&s.desk), desk_items(desk)),
            Err(_) => desk_items(desk).count(),
        }
    }

    /// Save the desk as a new stamp on top of the one it is on. Returns its number.
    pub fn stamp(&mut self, desk: &Fs, date: &str, note: &str) -> Result<usize> {
        let note = note.trim();
        if note.is_empty() {
            return Err("a stamp needs a note, e.g. stamp fixed the totals".into());
        }
        if note.len() > MAX_NOTE {
            return Err(format!("the note is too long (at most {MAX_NOTE} characters)"));
        }
        if self.stamps.len() >= MAX_STAMPS {
            return Err(format!("the history is full (at most {MAX_STAMPS} stamps)"));
        }
        if let Ok(base) = self.base_stamp() {
            if count_differing(snap_items(&base.desk), desk_items(desk)) == 0 {
                return Err(format!("nothing changed since stamp #{}", base.number));
            }
        }
        let entries = desk_items(desk).count();
        if entries > MAX_ENTRIES {
            return Err(format!(
                "the desk has too many drawers and pages to stamp ({entries}; at most {MAX_ENTRIES})"
            ));
        }
        // First count what it would cost (nothing is kept yet), then keep it. A string
        // that is new is counted once, however often it appears in this stamp.
        let mut cost = note.len() + date.len() + 2 * STRING_COST + entries * ENTRY_COST;
        let mut fresh: BTreeSet<&str> = BTreeSet::new();
        for (path, text) in desk_items(desk) {
            for s in core::iter::once(path).chain(text) {
                if self.pooled(s).is_none() && fresh.insert(s) {
                    cost += s.len() + STRING_COST;
                }
            }
        }
        drop(fresh);
        if self.bytes + cost > MAX_HISTORY {
            return Err("the history is full (it can keep at most 2 MB of text and lists of pages)".into());
        }
        // (collecting a sorted list builds a tightly packed map)
        let snap: Snap = desk
            .all()
            .filter(|(p, _)| tracked(p))
            .map(|(path, entry)| {
                let text = match entry {
                    Entry::Drawer => None,
                    Entry::Page(t) => Some(self.intern(t)),
                };
                (self.intern(path), text)
            })
            .collect();
        self.bytes += cost;
        let number = self.stamps.len() + 1;
        self.stamps.push(Stamp { number, from: self.base, date: date.to_string(), note: note.to_string(), desk: snap });
        self.base = Some(number);
        Ok(number)
    }

    /// A "#stamp [date] note" line in the start files: stamp the desk loaded so far.
    /// A stamp that can't be made is a mistake in the start files: it is recorded in
    /// start_errors (the harness checks them) and in the serial log.
    pub fn start_stamp(&mut self, desk: &Fs, line: &str, today: &str) -> Result<usize> {
        let line = line.trim();
        let (date, note) = match line.split_once(' ') {
            Some((d, n)) if looks_like_date(d) => (d, n),
            _ => (today, line),
        };
        self.stamp(desk, date, note).map_err(|e| {
            let shown: String = line.chars().take(40).collect();
            let more = if line.len() > shown.len() { "..." } else { "" };
            let msg = format!("#stamp {shown}{more}: {e}");
            self.start_error(&msg);
            msg
        })
    }

    /// Put the whole desk back as it was in stamp `n`. Refused while the desk has
    /// changes that are not stamped, so nothing can be lost.
    pub fn rewind(&mut self, desk: &mut Fs, n: usize) -> Result<String> {
        let target = self.get(n)?;
        let dirty = self.unstamped_count(desk);
        if dirty > 0 {
            return Err(format!(
                "the desk has changes that are not stamped yet ({}; see changes). Stamp them first",
                plural(dirty, "page or drawer", "pages or drawers")
            ));
        }
        if self.base == Some(n) {
            return Err(format!("the desk is already on stamp #{n}"));
        }
        let app_bytes: usize = desk
            .all()
            .filter(|(p, _)| !tracked(p))
            .map(|(_, e)| match e { Entry::Page(t) => t.len(), Entry::Drawer => 0 })
            .sum();
        let target_bytes: usize = target.desk.values().flatten().map(|t| t.len()).sum();
        if app_bytes + target_bytes > fs::MAX_DESK {
            return Err("that stamp no longer fits on the desk next to the apps' pages (at most 1 MB of text)".into());
        }
        let app_items = desk.count() - desk_items(desk).count();
        if app_items + target.desk.len() > fs::MAX_ITEMS {
            return Err(format!(
                "that stamp no longer fits on the desk next to the apps' pages (at most {} drawers and pages)",
                fs::MAX_ITEMS
            ));
        }
        let entries = target
            .desk
            .iter()
            .map(|(p, t)| (p.to_string(), match t { Some(t) => Entry::Page(t.to_string()), None => Entry::Drawer }))
            .collect();
        let msg = format!("Rewound the desk to stamp #{n} ({}). Later stamps are kept: stamps lists them all.", target.note);
        desk.replace_tracked(entries);
        self.base = Some(n);
        Ok(msg)
    }

    /// Bring back one page as it was in stamp `n`. Everything is checked before
    /// anything changes.
    pub fn bring(&self, desk: &mut Fs, path: &str, n: usize) -> Result<String> {
        let shown = fs::display(path);
        if !tracked(path) {
            let (app, _) = fs::owner(path).unwrap_or(("an", ""));
            return Err(format!("{shown} is looked after by the {app} app and is not kept in stamps"));
        }
        let stamp = self.get(n)?;
        let text = match stamp.desk.get(path) {
            Some(Some(t)) => t.clone(),
            Some(None) => return Err(format!("{shown} was a drawer in stamp #{n}; bring works on pages")),
            None => return Err(format!("there was no page {shown} in stamp #{n}")),
        };
        let now = match desk.read_page(path) {
            Ok(t) => Some(t),
            Err(_) if desk.exists(path) => return Err(format!("{shown} is a drawer now")),
            Err(_) => None,
        };
        if now == Some(&*text) {
            return Err(format!("{shown} is already as it was in stamp #{n}"));
        }
        let base_text = self.base_stamp().ok().and_then(|b| b.desk.get(path).cloned()).flatten();
        if now.is_some() && now != base_text.as_deref() {
            return Err(format!("{shown} has changes that are not stamped yet; stamp them first (or toss the page)"));
        }
        // Every drawer above it must be a drawer (or missing, then it is made again).
        let mut above = fs::parent(path);
        while !above.is_empty() {
            if desk.exists(above) && !desk.is_drawer(above) {
                return Err(format!("{} is a page now, so {shown} can't go back inside it", fs::display(above)));
            }
            above = fs::parent(above);
        }
        let old = now.map_or(0, |t| t.len());
        if desk.total_bytes() - old + text.len() > fs::MAX_DESK {
            return Err("the desk is full (all pages together can hold at most 1 MB of text)".into());
        }
        desk.check_room(desk.needed(path))?;
        desk.ensure_drawers(fs::parent(path));
        desk.write_page(path, &text)?;
        Ok(format!("Brought back {shown} as it was in stamp #{n}."))
    }

    /// What changed since the stamp the desk is on (only inside `filter`, if given).
    pub fn changes(&self, desk: &Fs, filter: &str) -> Result<Vec<String>> {
        let base = self.base_stamp()?;
        let mut out = Out::new();
        let n = diff(snap_items(&base.desk), desk_items(desk), filter, &mut out);
        let place = if filter.is_empty() { String::new() } else { format!(" in {}", fs::display(filter)) };
        let head = if n == 0 {
            format!("Nothing changed{place} since stamp #{} ({}).", base.number, base.note)
        } else {
            format!("{} changed{place} since stamp #{} ({}):", plural(n, "item", "items"), base.number, base.note)
        };
        Ok(out.finish(head, "changes <page or drawer>"))
    }

    /// What changed from stamp `a` to stamp `b` (or to the desk as it is now),
    /// only inside `filter` if given.
    pub fn compare(&self, desk: &Fs, a: usize, b: Option<usize>, filter: &str) -> Result<Vec<String>> {
        let old = &self.get(a)?.desk;
        let mut out = Out::new();
        let (n, to, again) = match b {
            Some(b) => (
                diff(snap_items(old), snap_items(&self.get(b)?.desk), filter, &mut out),
                format!("stamp #{b}"),
                format!("compare {a} {b} <page or drawer>"),
            ),
            None => (
                diff(snap_items(old), desk_items(desk), filter, &mut out),
                "the desk now".to_string(),
                format!("compare {a} <page or drawer>"),
            ),
        };
        let place = if filter.is_empty() { String::new() } else { format!(" in {}", fs::display(filter)) };
        let head = if n == 0 {
            format!("No differences{place} from stamp #{a} to {to}.")
        } else {
            format!("{} changed{place} from stamp #{a} to {to}:", plural(n, "item", "items"))
        };
        Ok(out.finish(head, &again))
    }

    /// The list of stamps, newest first.
    pub fn list(&self, desk: &Fs) -> Vec<String> {
        let mut out = Vec::new();
        let dirty = self.unstamped_count(desk);
        out.push(match (self.base, dirty) {
            (None, _) => "There are no stamps yet. stamp <note> saves the desk as it is now.".to_string(),
            (Some(b), 0) => format!("The desk is on stamp #{b}, with no changes since."),
            (Some(b), d) => format!(
                "The desk is on stamp #{b}, with {} not stamped yet (changes shows them).",
                plural(d, "change", "changes")
            ),
        });
        if !self.stamps.is_empty() {
            out.push(String::new());
        }
        for s in self.stamps.iter().rev() {
            let mark = if self.base == Some(s.number) { '*' } else { ' ' };
            let from = match s.from {
                Some(f) if f + 1 != s.number => format!("  (made on #{f})"),
                _ => String::new(),
            };
            out.push(format!("{mark} #{:<3} {}  {}{from}", s.number, s.date, s.note));
        }
        if !self.stamps.is_empty() {
            out.push(String::new());
            out.push(format!("* = the stamp the desk is on. History used: {} KB of 2048 KB.", self.bytes.div_ceil(1024)));
        }
        out
    }
}

/// A listing that stops growing at MAX_LINES lines or MAX_OUT_BYTES bytes.
struct Out {
    lines: Vec<String>,
    bytes: usize,
    full: bool,
}

impl Out {
    fn new() -> Self {
        Self { lines: Vec::new(), bytes: 0, full: false }
    }

    fn push(&mut self, line: String) {
        if self.full || self.lines.len() >= MAX_LINES || self.bytes + line.len() > MAX_OUT_BYTES {
            self.full = true;
            return;
        }
        self.bytes += line.len();
        self.lines.push(line);
    }

    fn finish(mut self, head: String, narrower: &str) -> Vec<String> {
        self.lines.insert(0, head);
        if self.full {
            self.lines.push(String::new());
            self.lines.push("... the rest is not shown (too long for one screen list).".to_string());
            self.lines.push(format!("To see less at once, name one page or drawer: {narrower}"));
        }
        self.lines
    }
}

/// Write the differences between two desks into `out`. Returns how many paths differ
/// (all of them are counted, even when the listing is cut off).
fn diff<'a>(old: impl Iterator<Item = Ent<'a>>, new: impl Iterator<Item = Ent<'a>>, filter: &str, out: &mut Out) -> usize {
    let mut n = 0;
    for (path, before, after) in merge(old, new) {
        if before == after || !in_filter(path, filter) {
            continue;
        }
        n += 1;
        if out.full {
            continue;
        }
        let shown = fs::display(path);
        out.push(String::new());
        match (before, after) {
            (Some(Some(a)), Some(Some(b))) => {
                let (na, nb) = (line_count(a), line_count(b));
                if na > MAX_PAGE_LINES || nb > MAX_PAGE_LINES {
                    out.push(format!(
                        "changed  {shown}  (too long to compare line by line: {} before, {} now)",
                        plural(na, "line", "lines"),
                        plural(nb, "line", "lines")
                    ));
                    continue;
                }
                let (la, lb) = (lines(a), lines(b));
                let ops = line_diff(&la, &lb);
                let outs = ops.iter().filter(|o| matches!(o, Op::Out(_))).count();
                let ins = ops.iter().filter(|o| matches!(o, Op::In(_))).count();
                out.push(format!(
                    "changed  {shown}  ({} out, {} in)",
                    plural(outs, "line", "lines"),
                    plural(ins, "line", "lines")
                ));
                hunks(&ops, &la, &lb, out);
            }
            (old_e, new_e) => {
                if let Some(e) = old_e {
                    gone_or_new(&shown, e, "gone", '-', out);
                }
                if let Some(e) = new_e {
                    gone_or_new(&shown, e, "new", '+', out);
                }
            }
        }
    }
    n
}

fn gone_or_new(shown: &str, entry: Option<&str>, word: &str, mark: char, out: &mut Out) {
    match entry {
        None => out.push(format!("{word} drawer  {shown}")),
        Some(t) => {
            out.push(format!("{word} page  {shown}  ({})", plural(line_count(t), "line", "lines")));
            if !t.is_empty() {
                for (i, l) in t.split('\n').enumerate() {
                    if out.full {
                        break;
                    }
                    out.push(format!("{:>5} {mark} {l}", i + 1));
                }
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Op {
    /// Line i of the old version and line j of the new one are the same.
    Same(usize, usize),
    /// Line i of the old version is gone.
    Out(usize),
    /// Line j of the new version is new.
    In(usize),
}

/// A shortest edit between two lists of lines (longest common subsequence).
pub fn line_diff(a: &[&str], b: &[&str]) -> Vec<Op> {
    let mut pre = 0;
    while pre < a.len() && pre < b.len() && a[pre] == b[pre] {
        pre += 1;
    }
    let mut suf = 0;
    while suf < a.len() - pre && suf < b.len() - pre && a[a.len() - 1 - suf] == b[b.len() - 1 - suf] {
        suf += 1;
    }
    let (ma, mb) = (&a[pre..a.len() - suf], &b[pre..b.len() - suf]);
    let mut ops: Vec<Op> = (0..pre).map(|i| Op::Same(i, i)).collect();
    let (n, m) = (ma.len(), mb.len());
    if n > 0 && m > 0 && (n + 1) * (m + 1) <= MAX_CELLS {
        // lcs[i][j] = longest common part of ma[i..] and mb[j..] (at most MAX_PAGE_LINES,
        // so it fits in 16 bits)
        let w = m + 1;
        let mut lcs = alloc::vec![0u16; (n + 1) * w];
        for i in (0..n).rev() {
            for j in (0..m).rev() {
                lcs[i * w + j] = if ma[i] == mb[j] {
                    lcs[(i + 1) * w + j + 1] + 1
                } else {
                    lcs[(i + 1) * w + j].max(lcs[i * w + j + 1])
                };
            }
        }
        let (mut i, mut j) = (0, 0);
        while i < n && j < m {
            if ma[i] == mb[j] {
                ops.push(Op::Same(pre + i, pre + j));
                i += 1;
                j += 1;
            } else if lcs[(i + 1) * w + j] >= lcs[i * w + j + 1] {
                ops.push(Op::Out(pre + i));
                i += 1;
            } else {
                ops.push(Op::In(pre + j));
                j += 1;
            }
        }
        ops.extend((i..n).map(|i| Op::Out(pre + i)));
        ops.extend((j..m).map(|j| Op::In(pre + j)));
    } else {
        ops.extend((0..n).map(|i| Op::Out(pre + i)));
        ops.extend((0..m).map(|j| Op::In(pre + j)));
    }
    ops.extend((0..suf).map(|k| Op::Same(a.len() - suf + k, b.len() - suf + k)));
    ops
}

/// Show the changed lines with one unchanged line around them; "..." for skipped lines.
/// Line numbers: old numbers for lines out, new numbers for the others.
fn hunks(ops: &[Op], a: &[&str], b: &[&str], out: &mut Out) {
    if ops.is_empty() {
        return;
    }
    let near = |k: usize| {
        let lo = k.saturating_sub(1);
        let hi = (k + 1).min(ops.len() - 1);
        (lo..=hi).any(|x| !matches!(ops[x], Op::Same(..)))
    };
    let mut last: Option<usize> = None;
    for k in 0..ops.len() {
        if out.full {
            return;
        }
        if !near(k) {
            continue;
        }
        if last.is_some_and(|l| l + 1 != k) {
            out.push("  ...".to_string());
        }
        last = Some(k);
        out.push(match ops[k] {
            Op::Same(_, j) => format!("{:>5}   {}", j + 1, b[j]),
            Op::Out(i) => format!("{:>5} - {}", i + 1, a[i]),
            Op::In(j) => format!("{:>5} + {}", j + 1, b[j]),
        });
    }
}
