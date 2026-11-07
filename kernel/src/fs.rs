//! The file system, kept in memory.
//!
//! Vocabulary: folders are "drawers", files are "pages", the top drawer is
//! the "desk". Paths use ':' between names, e.g. `desk:letters:mira`.
//! Internally a path leaves out "desk": the desk itself is "".

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

#[derive(Clone)]
pub enum Entry {
    Drawer,
    Page(String),
}

#[derive(Clone)]
pub struct Fs {
    entries: BTreeMap<String, Entry>,
    /// (old path, new path) of every rename/move, so the shell can follow
    /// along if the drawer it is in gets moved.
    moves: Vec<(String, String)>,
    /// Drawers and pages in the start files that were left out (too many, or a path too long).
    pub skipped: usize,
}

pub type Result<T> = core::result::Result<T, String>;

/// How a path is shown to the user: "desk" or "desk:letters:mira".
pub fn display(path: &str) -> String {
    if path.is_empty() { "desk".to_string() } else { format!("desk:{path}") }
}

pub fn parent(path: &str) -> &str {
    path.rfind(':').map_or("", |i| &path[..i])
}

pub fn name(path: &str) -> &str {
    path.rfind(':').map_or(path, |i| &path[i + 1..])
}

pub fn join(dir: &str, name: &str) -> String {
    if dir.is_empty() { name.to_string() } else { format!("{dir}:{name}") }
}

pub fn valid_name(n: &str) -> bool {
    !n.is_empty()
        && n.len() <= 24
        && n != "desk"
        && n.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
}

/// Drawers looked after by an app: (top drawer, app name, command that opens it).
/// Outside their app they can be read but not changed.
pub const APP_DRAWERS: [(&str, &str, &str); 4] = [
    ("mail", "Mail", "mail"),
    ("contacts", "Contacts", "contacts"),
    ("calendar", "Calendar", "calendar"),
    ("system", "Settings", "settings"),
];

/// All pages together may hold at most this much text (the heap is 16 MB; a chore's
/// all-or-nothing copy of the desk needs room for a second copy).
pub const MAX_DESK: usize = 1024 * 1024;

/// At most this many drawers and pages in all (with the app drawers), and paths of at most
/// MAX_PATH characters: with MAX_DESK this bounds the memory the desk can take.
pub const MAX_ITEMS: usize = 5000;
pub const MAX_PATH: usize = 120;

pub const DEFAULT_SETTINGS: &str = "name: friend\ntheme: blue\nstart: shell\nclock: 24h";

/// The app that looks after this path, if any: (app name, command).
pub fn owner(path: &str) -> Option<(&'static str, &'static str)> {
    let top = path.split(':').next().unwrap_or("");
    APP_DRAWERS.iter().find(|(d, _, _)| *d == top).map(|(_, app, cmd)| (*app, *cmd))
}

fn check_path(path: &str) -> Result<()> {
    if path.len() > MAX_PATH {
        Err(format!("that path is too long (at most {MAX_PATH} characters)"))
    } else {
        Ok(())
    }
}

fn guard(path: &str) -> Result<()> {
    match owner(path) {
        Some((app, cmd)) => Err(format!("{} is looked after by the {app} app (open it with: {cmd})", display(path))),
        None => Ok(()),
    }
}

impl Fs {
    pub fn new() -> Self {
        Self { entries: BTreeMap::new(), moves: Vec::new(), skipped: 0 }
    }

    /// Make sure the app drawers and the settings page exist.
    pub fn ensure_system(&mut self) {
        for d in ["mail", "mail:inbox", "mail:sent", "contacts", "calendar", "system"] {
            self.ensure_drawers(d);
        }
        self.entries.entry("system:settings".into()).or_insert_with(|| Entry::Page(DEFAULT_SETTINGS.into()));
    }

    /// For apps only: write a page inside an app drawer (no protection check).
    pub fn app_write(&mut self, path: &str, text: &str) -> Result<()> {
        self.check_room(self.needed(path))?;
        self.check_bytes(path, text)?;
        self.ensure_drawers(parent(path));
        self.entries.insert(path.to_string(), Entry::Page(text.to_string()));
        Ok(())
    }

    /// How many new entries writing `path` would make (missing drawers above it, and itself).
    pub fn needed(&self, path: &str) -> usize {
        let mut n = 0;
        let mut p = path;
        while !p.is_empty() && !self.entries.contains_key(p) {
            n += 1;
            p = parent(p);
        }
        n
    }

    /// Is there room for this many new drawers and pages?
    pub fn check_room(&self, new_entries: usize) -> Result<()> {
        if self.entries.len() + new_entries > MAX_ITEMS {
            Err(format!("the desk is full (at most {MAX_ITEMS} drawers and pages in all)"))
        } else {
            Ok(())
        }
    }

    /// Would writing `text` to `path` keep all pages together within MAX_DESK?
    pub fn check_bytes(&self, path: &str, text: &str) -> Result<()> {
        let old = match self.entries.get(path) { Some(Entry::Page(t)) => t.len(), _ => 0 };
        if self.total_bytes() - old + text.len() > MAX_DESK {
            Err("the desk is full (all pages together can hold at most 1 MB of text)".into())
        } else {
            Ok(())
        }
    }

    /// How many drawers and pages there are (with the app drawers).
    pub fn count(&self) -> usize {
        self.entries.len()
    }

    /// For the start files: leave out what doesn't fit (room is kept for the app drawers).
    fn loadable(&mut self, path: &str) -> bool {
        if path.len() > MAX_PATH || self.entries.len() + self.needed(path) > MAX_ITEMS - 16 {
            self.skipped += 1;
            false
        } else {
            true
        }
    }

    /// For apps only: remove a page inside an app drawer.
    pub fn app_toss(&mut self, path: &str) {
        self.entries.remove(path);
    }

    pub fn take_moves(&mut self) -> Vec<(String, String)> {
        core::mem::take(&mut self.moves)
    }

    /// A small starting desk, used when no task files are given.
    pub fn sample() -> Self {
        let mut fs = Self::new();
        let page = |fs: &mut Fs, p: &str, t: &str| fs.entries.insert(p.to_string(), Entry::Page(t.to_string()));
        page(&mut fs, "welcome", "Welcome to your desk.\nDrawers hold pages and other drawers.\nType guide to see what you can do.");
        fs.entries.insert("letters".into(), Entry::Drawer);
        page(&mut fs, "letters:mira", "Hi! Could you send me the shopping list?\n- Mira");
        fs.entries.insert("lists".into(), Entry::Drawer);
        page(&mut fs, "lists:shopping", "eggs\nbread\ntea");
        fs
    }

    /// Build a desk from a task's start-files text:
    ///   #drawer letters
    ///   #page letters:mira
    ///   text lines...
    ///   #end
    /// Paths leave out "desk:". Missing parent drawers are created.
    #[allow(dead_code)] // the kernel uses load_start_files; host tests use this
    pub fn from_start_files(text: &str) -> Self {
        Self::load_start_files(text, |_, _| {})
    }

    /// Like from_start_files, with two more lines for building a history:
    ///   #toss letters:mira   remove a page or drawer (and what is inside it)
    ///   #stamp <note>        calls `on_stamp` with the desk loaded so far
    pub fn load_start_files(text: &str, mut on_stamp: impl FnMut(&Fs, &str)) -> Self {
        let mut fs = Self::new();
        // (path, text so far, has a line yet, too big): the text is built directly, without
        // a list of lines, and never grows past what the desk can still hold
        let mut page: Option<(String, String, bool, bool)> = None;
        let mut room = MAX_DESK - DEFAULT_SETTINGS.len();
        for line in text.lines() {
            let line = line.trim_end_matches('\r');
            if !line.bytes().all(|b| (0x20..0x7f).contains(&b)) {
                continue; // only plain ASCII text is allowed in start files
            }
            if let Some((path, body, started, too_big)) = &mut page {
                if line == "#end" {
                    let path = core::mem::take(path);
                    let mut body = core::mem::take(body);
                    body.shrink_to_fit();
                    let old = match fs.entries.get(&path) { Some(Entry::Page(t)) => t.len(), _ => 0 };
                    if *too_big || body.len() > room + old {
                        fs.skipped += 1;
                    } else if fs.loadable(&path) {
                        room = room + old - body.len();
                        fs.ensure_drawers(parent(&path));
                        fs.entries.insert(path, Entry::Page(body));
                    }
                    page = None;
                } else if !*too_big {
                    let add = line.len() + usize::from(*started);
                    if body.len() + add > room + match fs.entries.get(path.as_str()) {
                        Some(Entry::Page(t)) => t.len(),
                        _ => 0,
                    } {
                        *too_big = true;
                        *body = String::new();
                    } else {
                        if *started {
                            body.push('\n');
                        }
                        body.push_str(line);
                        *started = true;
                    }
                }
            } else if let Some(p) = line.strip_prefix("#drawer ") {
                if fs.loadable(p.trim()) {
                    fs.ensure_drawers(p.trim());
                }
            } else if let Some(p) = line.strip_prefix("#page ") {
                page = Some((p.trim().to_string(), String::new(), false, false));
            } else if let Some(p) = line.strip_prefix("#toss ") {
                let p = p.trim();
                let inside = format!("{p}:");
                fs.entries.retain(|k, _| k != p && !k.starts_with(&inside));
            } else if let Some(note) = line.strip_prefix("#stamp ") {
                on_stamp(&fs, note);
            }
        }
        fs.moves.clear();
        fs
    }

    /// Replace every drawer and page outside the app drawers (for a rewind).
    pub fn replace_tracked(&mut self, entries: Vec<(String, Entry)>) {
        self.entries.retain(|k, _| owner(k).is_some());
        self.entries.extend(entries.into_iter().filter(|(k, _)| owner(k).is_none()));
    }

    pub fn ensure_drawers(&mut self, path: &str) {
        if path.is_empty() || self.entries.contains_key(path) {
            return;
        }
        self.ensure_drawers(parent(path));
        self.entries.insert(path.to_string(), Entry::Drawer);
    }

    pub fn is_drawer(&self, path: &str) -> bool {
        path.is_empty() || matches!(self.entries.get(path), Some(Entry::Drawer))
    }

    pub fn exists(&self, path: &str) -> bool {
        path.is_empty() || self.entries.contains_key(path)
    }

    /// Turn what the user typed into a path. Relative to `here`, unless it
    /// starts with "desk".
    pub fn resolve(&self, here: &str, input: &str) -> Result<String> {
        let input = input.trim();
        let (mut path, rest) = if input == "desk" {
            (String::new(), "")
        } else if let Some(r) = input.strip_prefix("desk:") {
            (String::new(), r)
        } else {
            (here.to_string(), input)
        };
        for part in rest.split(':').filter(|s| !s.is_empty()) {
            if !valid_name(part) {
                return Err(format!("'{part}' is not a valid name (use letters, digits, - _ . and at most 24 characters)"));
            }
            path = join(&path, part);
        }
        if path.len() > MAX_PATH {
            return Err(format!("that path is too long (at most {MAX_PATH} characters)"));
        }
        Ok(path)
    }

    /// Names in a drawer: drawers first, then pages, each alphabetical.
    pub fn children(&self, dir: &str) -> Vec<(String, bool)> {
        let mut drawers = Vec::new();
        let mut pages = Vec::new();
        for (path, entry) in &self.entries {
            if parent(path) == dir && path != dir {
                match entry {
                    Entry::Drawer => drawers.push((name(path).to_string(), true)),
                    Entry::Page(_) => pages.push((name(path).to_string(), false)),
                }
            }
        }
        drawers.extend(pages);
        drawers
    }

    fn check_parent(&self, path: &str) -> Result<()> {
        let p = parent(path);
        if self.is_drawer(p) {
            Ok(())
        } else {
            Err(format!("there is no drawer called {}", display(p)))
        }
    }

    pub fn read_page(&self, path: &str) -> Result<&str> {
        match self.entries.get(path) {
            Some(Entry::Page(t)) => Ok(t),
            Some(Entry::Drawer) => Err(format!("{} is a drawer, not a page", display(path))),
            None if path.is_empty() => Err("the desk is a drawer, not a page".into()),
            None => Err(format!("there is no page called {}", display(path))),
        }
    }

    /// Bytes of text in all pages together.
    pub fn total_bytes(&self) -> usize {
        self.entries.values().map(|e| match e { Entry::Page(t) => t.len(), Entry::Drawer => 0 }).sum()
    }

    pub fn write_page(&mut self, path: &str, text: &str) -> Result<()> {
        guard(path)?;
        check_path(path)?;
        let old = match self.entries.get(path) { Some(Entry::Page(t)) => t.len(), _ => 0 };
        if self.total_bytes() - old + text.len() > MAX_DESK {
            return Err("the desk is full (all pages together can hold at most 1 MB of text)".into());
        }
        if self.is_drawer(path) {
            return Err(format!("{} is a drawer, not a page", display(path)));
        }
        self.check_parent(path)?;
        if !self.entries.contains_key(path) {
            self.check_room(1)?;
        }
        self.entries.insert(path.to_string(), Entry::Page(text.to_string()));
        Ok(())
    }

    pub fn build_drawer(&mut self, path: &str) -> Result<()> {
        guard(path)?;
        check_path(path)?;
        if self.exists(path) {
            return Err(format!("{} already exists", display(path)));
        }
        self.check_parent(path)?;
        self.check_room(1)?;
        self.entries.insert(path.to_string(), Entry::Drawer);
        Ok(())
    }

    pub fn toss(&mut self, path: &str) -> Result<()> {
        if path.is_empty() {
            return Err("the desk cannot be thrown away".into());
        }
        guard(path)?;
        if !self.exists(path) {
            return Err(format!("there is nothing called {}", display(path)));
        }
        if self.is_drawer(path) && !self.children(path).is_empty() {
            return Err(format!("{} is not empty; empty it first", display(path)));
        }
        self.entries.remove(path);
        Ok(())
    }

    /// Move `from` (and everything inside it) to the new path `to`.
    fn relocate(&mut self, from: &str, to: &str) -> Result<()> {
        if from.is_empty() {
            return Err("the desk cannot be moved".into());
        }
        guard(from)?;
        guard(to)?;
        if !self.exists(from) {
            return Err(format!("there is nothing called {}", display(from)));
        }
        if self.exists(to) {
            return Err(format!("{} already exists", display(to)));
        }
        if to == from || to.starts_with(&format!("{from}:")) {
            return Err("a drawer cannot go inside itself".into());
        }
        self.check_parent(to)?;
        let prefix = format!("{from}:");
        let moving: Vec<String> =
            self.entries.keys().filter(|k| *k == from || k.starts_with(&prefix)).cloned().collect();
        let longest = moving.iter().map(|k| k.len() - from.len()).max().unwrap_or(0);
        if to.len() + longest > MAX_PATH {
            return Err(format!("the paths inside would get too long (at most {MAX_PATH} characters)"));
        }
        for old in moving {
            let entry = self.entries.remove(&old).unwrap();
            let new = format!("{to}{}", &old[from.len()..]);
            self.entries.insert(new, entry);
        }
        self.moves.push((from.to_string(), to.to_string()));
        Ok(())
    }

    pub fn rename(&mut self, path: &str, new_name: &str) -> Result<()> {
        if !valid_name(new_name) {
            return Err(format!("'{new_name}' is not a valid name"));
        }
        self.relocate(path, &join(parent(path), new_name))
    }

    pub fn move_into(&mut self, path: &str, drawer: &str) -> Result<()> {
        if !self.is_drawer(drawer) {
            return Err(format!("there is no drawer called {}", display(drawer)));
        }
        self.relocate(path, &join(drawer, name(path)))
    }

    pub fn copy_page(&mut self, from: &str, to: &str) -> Result<()> {
        let text = self.read_page(from)?.to_string();
        if self.exists(to) {
            return Err(format!("{} already exists", display(to)));
        }
        self.write_page(to, &text)
    }

    pub fn all(&self) -> impl Iterator<Item = (&String, &Entry)> {
        self.entries.iter()
    }
}
