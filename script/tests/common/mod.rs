//! A test world on top of the kernel's real file system code, the same as the chore tests use
//! (chore-tests/src/tests.rs). It works for both engines, so their results can be compared.
#![allow(dead_code)]

use crate::fs::{self, Entry, Fs};

pub const START: &str = "\
#page contacts:omar
name: Omar Haddad
mail: omar
#end
#page contacts:lina
name: Lina Berg
mail: lina
note: piano teacher
#end
#page mail:inbox:m1
from: omar
to: me
subject: Bread

Fresh bread today.
#end
#page mail:inbox:m2
from: lina
to: me
subject: Lesson

Can we move it?
#end
#page mail:inbox:m10
from: omar
to: me
subject: Closed

Closed on Monday.
#end
#page notes:shopping
eggs
bread
tea
#end
#drawer empty
";

pub struct TestWorld {
    pub fs: Fs,
    /// How much more the desk can hold (the kernel's real limit comes in M4).
    pub free: usize,
}

impl TestWorld {
    pub fn new() -> Self {
        Self::from(START)
    }

    pub fn from(start: &str) -> Self {
        let mut fs = Fs::from_start_files(start);
        fs.ensure_system();
        Self { fs, free: usize::MAX }
    }

    fn page(&self, path: &str) -> Option<String> {
        self.fs.read_page(path).ok().map(|s| s.to_string())
    }
    fn pages_in(&self, drawer: &str) -> Vec<String> {
        self.fs
            .all()
            .filter(|(p, e)| fs::parent(p) == drawer && !p.is_empty() && matches!(e, Entry::Page(_)))
            .map(|(p, _)| p.clone())
            .collect()
    }
    fn check_write(&self, path: &str) -> Result<(), String> {
        if let Some((app, _)) = fs::owner(path) {
            return Err(format!("{} is looked after by the {app} app", fs::display(path)));
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
        if fs::owner(path).is_some() {
            return Err("app page".into());
        }
        if self.fs.read_page(path).is_err() {
            return Err(format!("there is no page called {}", fs::display(path)));
        }
        Ok(())
    }
    fn check_send(&self, to: &str, subject: &str, _text: &str) -> Result<(), String> {
        if !fs::valid_name(to) || !self.fs.exists(&fs::join("contacts", to)) {
            return Err(format!("no contact has the address '{to}'"));
        }
        if subject.contains('\n') {
            return Err("a subject must be one line".into());
        }
        Ok(())
    }
    fn check_event(&self, date: &str, time: &str, title: &str, _with: &str) -> Result<(), String> {
        if date.len() != 10 || time.len() != 5 || title.is_empty() {
            return Err("bad event".into());
        }
        Ok(())
    }
}

impl script::World for TestWorld {
    fn page(&self, path: &str) -> Option<String> {
        TestWorld::page(self, path)
    }
    fn is_drawer(&self, path: &str) -> bool {
        self.fs.is_drawer(path)
    }
    fn pages_in(&self, drawer: &str) -> Vec<String> {
        TestWorld::pages_in(self, drawer)
    }
    fn check_write(&self, path: &str) -> Result<(), String> {
        TestWorld::check_write(self, path)
    }
    fn check_toss(&self, path: &str) -> Result<(), String> {
        TestWorld::check_toss(self, path)
    }
    fn check_send(&self, to: &str, subject: &str, text: &str) -> Result<(), String> {
        TestWorld::check_send(self, to, subject, text)
    }
    fn check_event(&self, date: &str, time: &str, title: &str, with: &str) -> Result<(), String> {
        TestWorld::check_event(self, date, time, title, with)
    }
    fn today(&self) -> String {
        "2026-11-12".into()
    }
    fn max_page(&self) -> usize {
        16 * 1024 // as kernel/src/chores.rs
    }
    fn free_space(&self) -> usize {
        self.free
    }
}

pub fn run(src: &str) -> script::Outcome {
    script::run("main", src, &TestWorld::new())
}

/// Output lines of a program that must succeed.
pub fn out(src: &str) -> Vec<String> {
    let o = run(src);
    assert!(o.error.is_none(), "unexpected mistake {:?} for:\n{src}", o.error.map(|e| e.to_string()));
    o.output
}

pub fn say(expr: &str) -> String {
    out(&format!("say {expr}")).remove(0)
}

/// The mistake of a program that must fail; also checks nothing would change.
pub fn err(src: &str) -> String {
    let o = run(src);
    assert!(o.actions.is_empty());
    o.error.unwrap_or_else(|| panic!("expected a mistake for:\n{src}")).to_string()
}

pub fn has(e: &str, part: &str) {
    assert!(e.contains(part), "mistake {e:?} should contain {part:?}");
}
