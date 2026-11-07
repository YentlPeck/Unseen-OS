//! Test pages (design §6): `test 'TITLE'` ... `done` blocks with `check` and `pretend`.
//!
//! Each test starts fresh: its own names, its own budgets and its own test world. The code
//! comes from the real desk (the test page and the pages it uses); everything the program reads
//! while it runs comes from the test world: the empty app drawers, the default settings and the
//! pages the test pretends. Nothing a test does is ever made on the real desk.

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::limits::*;
use crate::parse::{Expr, Op, Pretend, Program, Stmt, Test, B};
use crate::run::{at, mistake, prepare, text_len, write_text, Action, Mistake, Runner, World};
use crate::util::*;
use crate::value::*;

/// The date every test runs on, unless it says `pretend today is ...`.
pub const TEST_TODAY: &str = "2026-11-14";
const DEFAULT_SETTINGS: &str = "name: friend\ntheme: blue\nstart: shell\nclock: 24h";
const APP_DRAWERS: [(&str, &str); 4] = [("mail", "Mail"), ("contacts", "Contacts"), ("calendar", "Calendar"), ("system", "Settings")];
const MAX_DESK: usize = 1024 * 1024;
/// Counted for every page and drawer of a test world, on top of its path and text.
const ENTRY_COST: usize = 64;

#[derive(Debug)]
pub struct TestResult {
    pub title: String,
    /// The line of its `test`.
    pub line: usize,
    pub passed: bool,
    /// Why it failed ("" when it passed).
    pub why: String,
}

#[derive(Debug)]
pub struct TestReport {
    /// A mistake found before any test ran (then no test ran).
    pub error: Option<Mistake>,
    pub results: Vec<TestResult>,
}

impl TestReport {
    /// The report as the `test` command shows it: one line per test.
    pub fn lines(&self) -> Vec<String> {
        if let Some(e) = &self.error {
            return alloc::vec![format!("Stopped: {e}")];
        }
        let mut out: Vec<String> = self
            .results
            .iter()
            .map(|r| if r.passed { format!("PASS {}", r.title) } else { format!("FAIL {}: {}", r.title, r.why) })
            .collect();
        let passed = self.results.iter().filter(|r| r.passed).count();
        out.push(format!("{passed} of {} tests passed", self.results.len()));
        out
    }

    pub fn all_passed(&self) -> bool {
        self.error.is_none() && !self.results.is_empty() && self.results.iter().all(|r| r.passed)
    }
}

/// Run every test on a test page. The page and the pages it uses are read from `desk`.
pub fn run_tests<W: World>(page: &str, source: &str, desk: &W) -> TestReport {
    let program = match prepare(page, source, desk) {
        Ok(p) => p,
        Err(e) => return TestReport { error: Some(e), results: Vec::new() },
    };
    if let Some(first) = program.main.first() {
        let e = Mistake {
            page: page.into(),
            line: first.n,
            message: "a test page may only hold use, fixed, task and test lines".into(),
            limit: false,
            before_running: true,
            trail: Vec::new(),
        };
        return TestReport { error: Some(e), results: Vec::new() };
    }
    if program.tests.is_empty() {
        let e = Mistake {
            page: page.into(),
            line: 0,
            message: "this page has no tests (test 'what it checks' ... done)".into(),
            limit: false,
            before_running: true,
            trail: Vec::new(),
        };
        return TestReport { error: Some(e), results: Vec::new() };
    }
    let results = program.tests.iter().map(|t| one_test(&program, t)).collect();
    TestReport { error: None, results }
}

fn one_test(program: &Program, t: &Test) -> TestResult {
    let fail = |why: String| TestResult { title: t.title.clone(), line: t.line, passed: false, why };
    let show = |m: Mistake| -> String {
        // inside the test page, the page name is clear: show "line N: ..."
        let s = m.to_string();
        match s.strip_prefix(&format!("{} ", display(&program.pages[0]))) {
            Some(rest) => rest.to_string(),
            None => s,
        }
    };
    // 1. the test world, from the pretend lines (in order)
    let mut world = TestWorld::new();
    let mut steps = 0; // the pretend lines and the test share one step budget
    for l in &t.body {
        let Stmt::Pretend(p) = &l.stmt else { break };
        let m = Meter::new(STEP_MEMORY);
        let mut r = Runner::new(&world, program, &m);
        let res = r.fixed_values().map_err(|(n, f)| mistake(program, n, f)).and_then(|_| {
            r.steps = steps; // (working out the fixed values doesn't count)
            let mut eval = |e: &Expr| r.value_text(e).map_err(|f| mistake(program, l.n, at(t.page, l.n, f).1));
            match p {
                Pretend::Page(path, text) => Ok((Some(eval(path)?), eval(text)?)),
                Pretend::Today(d) => Ok((None, eval(d)?)),
            }
        });
        steps = r.steps;
        let (path, text) = match res {
            Ok(x) => x,
            Err(e) => return fail(show(e)),
        };
        let done = match path {
            Some(p) => world.pretend_page(&p, &text),
            None => world.pretend_today(&text),
        };
        if let Err(e) = done {
            return fail(format!("line {}: {e}", l.n));
        }
    }
    // 2. the test itself
    let m = Meter::new(STEP_MEMORY);
    let mut r = Runner::new(&world, program, &m);
    let result = r.fixed_values().and_then(|_| {
        r.steps = steps;
        r.set_page(t.page);
        r.block(&t.body)
    });
    match result {
        Err((_, f)) if f.stop => fail("stop ended the test before its done".into()),
        Ok(crate::run::Flow::Stop) => fail("stop ended the test before its done".into()),
        Err((n, f)) => fail(show(mistake(program, n, f))),
        Ok(_) if r.checks == 0 => fail("this test has no check".into()),
        Ok(_) => TestResult { title: t.title.clone(), line: t.line, passed: true, why: String::new() },
    }
}

impl<'a, W: World> Runner<'a, W> {
    pub(crate) fn set_page(&mut self, page: usize) {
        self.page = page;
    }

    /// The value of an expression as text (for pretend lines).
    pub(crate) fn value_text(&mut self, e: &Expr) -> Res<String> {
        let v = self.eval(e)?;
        let len = text_len(&v, MAX_TEXT)?;
        let mut s = String::new();
        if s.try_reserve_exact(len).is_err() {
            return Err(limit_fault("the computer has no more memory for this program".into()));
        }
        write_text(&v, &mut s);
        Ok(s)
    }

    /// `check CONDITION`. For `a is b` and `a is not b`, a failed check shows both values.
    pub(crate) fn check(&mut self, e: &Expr) -> Res<()> {
        match self.check_here(e) {
            Ok(()) => {
                self.checks += 1;
                Ok(())
            }
            // a mistake while working out a check can't be caught by try either
            Err(mut f) => {
                f.check = true;
                Err(f)
            }
        }
    }

    fn check_here(&mut self, e: &Expr) -> Res<()> {
        if let Expr::Bin(op @ (Op::Is | Op::IsNot), l, r) = e {
            let a = self.eval(l)?;
            let b = self.eval(r)?;
            let same = self.binary(Op::Is, a.clone(), b.clone())?;
            let ok = matches!(same, Value::Bool(s) if s == (*op == Op::Is));
            if !ok {
                let (x, y) = (shown(&a), shown(&b));
                return Err(Fault::check_failed(if *op == Op::Is {
                    format!("check failed: got {x}, expected {y}")
                } else {
                    format!("check failed: both are {x}")
                }));
            }
            return Ok(());
        }
        match self.eval(e)? {
            Value::Bool(true) => Ok(()),
            Value::Bool(false) => Err(Fault::check_failed("check failed".into())),
            v => Err(format!("check needs yes or no, got {}", v.describe()).into()),
        }
    }

    /// The functions that only work in tests.
    #[inline(never)]
    pub(crate) fn test_function(&mut self, b: B, v: Vec<Value>) -> Res<Value> {
        let page_arg = |x: &Value| -> Res<String> {
            match x {
                Value::Text(t) => Ok(path_of(&t.s)?),
                _ => Err(format!("a page must be given as text, got {}", x.describe()).into()),
            }
        };
        Ok(match b {
            B::PageNow => {
                let p = page_arg(&v[0])?;
                match self.w.page(&p) {
                    Some(t) => text(&self.m, &t)?,
                    None => Value::Nothing,
                }
            }
            B::After => {
                let p = page_arg(&v[0])?;
                match self.planned(&p) {
                    Some(Some(t)) => {
                        let t = t.clone();
                        text(&self.m, &t)?
                    }
                    Some(None) => Value::Nothing,
                    None => match self.w.page(&p) {
                        Some(t) => text(&self.m, &t)?,
                        None => Value::Nothing,
                    },
                }
            }
            B::Sent | B::Events => {
                let mut rows: Vec<Vec<(&str, String)>> = Vec::new();
                for a in &self.actions {
                    match (b, a) {
                        (B::Sent, Action::Send { to, subject, text }) => {
                            rows.push(alloc::vec![("to", to.clone()), ("subject", subject.clone()), ("text", text.clone())])
                        }
                        (B::Events, Action::Event { date, time, title, with }) => rows.push(alloc::vec![
                            ("date", date.clone()),
                            ("time", time.clone()),
                            ("title", title.clone()),
                            ("with", with.clone()),
                        ]),
                        _ => {}
                    }
                }
                self.tick(rows.len())?;
                let mut lm = ListMaker::new(&self.m, rows.len())?;
                for row in rows {
                    let charge = Charge::new(&self.m, record_cost(row.iter().map(|(k, _)| *k)))?;
                    let mut fields = Vec::new();
                    for (k, t) in row {
                        fields.push((String::from(k), text(&self.m, &t)?));
                    }
                    lm.push(record_charged(charge, fields)?)?;
                }
                lm.finish()
            }
            B::Said => {
                let lines: Vec<String> = self.out.clone();
                self.tick(lines.len())?;
                let mut lm = ListMaker::new(&self.m, lines.len())?;
                for l in &lines {
                    lm.push(text(&self.m, l)?)?;
                }
                lm.finish()
            }
            _ => return Err("internal: not a test function".into()),
        })
    }
}

/// A value as a failed check shows it: texts in quotes.
fn shown(v: &Value) -> String {
    match v {
        Value::Text(t) => format!("'{}'", short_long(&t.s)),
        _ => {
            let len = text_len(v, 200).unwrap_or(200);
            let mut s = String::new();
            if len < 200 {
                write_text(v, &mut s);
                s
            } else {
                v.describe()
            }
        }
    }
}

fn short_long(s: &str) -> String {
    match s.char_indices().nth(60) {
        Some((i, _)) => format!("{}...", &s[..i]),
        None => s.into(),
    }
}

/// The world a test runs in: the empty app drawers, the default settings, and what the test
/// pretends. Its checks are the kernel's (kernel/src/chores.rs, mail.rs, calendar.rs).
pub struct TestWorld {
    pages: BTreeMap<String, String>,
    drawers: BTreeSet<String>,
    today: String,
}

impl TestWorld {
    pub fn new() -> Self {
        let mut w = TestWorld { pages: BTreeMap::new(), drawers: BTreeSet::new(), today: TEST_TODAY.into() };
        for d in ["", "mail", "mail:inbox", "mail:sent", "contacts", "calendar", "system"] {
            w.drawers.insert(d.into());
        }
        w.pages.insert("system:settings".into(), DEFAULT_SETTINGS.into());
        w
    }

    /// What the test world holds: texts, paths and drawers, with their bookkeeping.
    fn total(&self) -> usize {
        self.pages.iter().map(|(p, t)| p.len() + t.len() + ENTRY_COST).sum::<usize>()
            + self.drawers.iter().map(|d| d.len() + ENTRY_COST).sum::<usize>()
    }

    pub fn pretend_page(&mut self, path: &str, text: &str) -> Result<(), String> {
        let p = path_of(path)?;
        if p.is_empty() || self.drawers.contains(&p) {
            return Err(format!("{} is a drawer, so a test can't pretend it is a page", display(&p)));
        }
        if text.len() > 16 * 1024 {
            return Err(format!("{} would be too long", display(&p)));
        }
        // what the page and the new drawers above it would add, counted before they are made
        let mut adds = p.len() + text.len() + ENTRY_COST;
        let mut d = parent(&p);
        while !d.is_empty() && !self.drawers.contains(d) {
            adds += d.len() + ENTRY_COST;
            d = parent(d);
        }
        if let Some(old) = self.pages.get(&p) {
            adds = adds.saturating_sub(p.len() + old.len() + ENTRY_COST);
        }
        if self.total() + adds > MAX_DESK {
            return Err("the test world would be too full".into());
        }
        // the drawers above it
        let mut d = parent(&p);
        loop {
            if self.pages.contains_key(d) {
                return Err(format!("{} is a page, so it can't hold other pages", display(d)));
            }
            self.drawers.insert(d.into());
            if d.is_empty() {
                break;
            }
            d = parent(d);
        }
        self.pages.insert(p, text.into());
        Ok(())
    }

    pub fn pretend_today(&mut self, date: &str) -> Result<(), String> {
        if parse_date(date).is_none() {
            return Err(format!("'{}' is not a date like 2026-11-14", short(date)));
        }
        self.today = date.into();
        Ok(())
    }
}

impl Default for TestWorld {
    fn default() -> Self {
        Self::new()
    }
}

fn owner(path: &str) -> Option<&'static str> {
    let top = path.split(':').next().unwrap_or("");
    APP_DRAWERS.iter().find(|(d, _)| *d == top).map(|(_, app)| *app)
}

fn valid_time(t: &str) -> bool {
    let b = t.as_bytes();
    b.len() == 5
        && b[2] == b':'
        && b[..2].iter().chain(&b[3..]).all(|c| c.is_ascii_digit())
        && t[..2].parse::<u32>().map_or(false, |h| h < 24)
        && t[3..].parse::<u32>().map_or(false, |m| m < 60)
}

impl World for TestWorld {
    fn page(&self, path: &str) -> Option<String> {
        self.pages.get(path).cloned()
    }
    fn is_drawer(&self, path: &str) -> bool {
        self.drawers.contains(path)
    }
    fn pages_in(&self, drawer: &str) -> Vec<String> {
        self.pages.keys().filter(|p| parent(p) == drawer && !p.is_empty()).cloned().collect()
    }
    fn check_write(&self, path: &str) -> Result<(), String> {
        if let Some(app) = owner(path) {
            return Err(format!("{} is looked after by the {app} app; programs change app pages only with send(...) or event(...)", display(path)));
        }
        if self.drawers.contains(path) {
            return Err(format!("{} is a drawer, not a page", display(path)));
        }
        if !self.drawers.contains(parent(path)) {
            return Err(format!("there is no drawer called {}", display(parent(path))));
        }
        Ok(())
    }
    fn check_toss(&self, path: &str) -> Result<(), String> {
        if let Some(app) = owner(path) {
            return Err(format!("{} is looked after by the {app} app; programs can't toss it", display(path)));
        }
        if self.pages.contains_key(path) {
            Ok(())
        } else if self.drawers.contains(path) {
            Err(format!("{} is a drawer; programs only toss pages", display(path)))
        } else {
            Err(format!("there is no page called {}", display(path)))
        }
    }
    fn check_send(&self, to: &str, subject: &str, text: &str) -> Result<(), String> {
        if !valid_name(to) {
            return Err(format!("'{to}' is not a mail address (one word like mira, see Contacts)"));
        }
        if !self.pages.contains_key(&join("contacts", to)) {
            return Err(format!("no contact has the address '{to}' (addresses must match exactly)"));
        }
        if subject.contains('\n') || subject.len() > 60 {
            return Err("a subject must be one line of at most 60 characters".into());
        }
        if text.len() > 4096 {
            return Err("a message can be at most 4096 characters".into());
        }
        Ok(())
    }
    fn check_event(&self, date: &str, time: &str, title: &str, with: &str) -> Result<(), String> {
        if parse_date(date).is_none() {
            return Err("Date must be a real date written YYYY-MM-DD, e.g. 2026-11-12".into());
        }
        if !valid_time(time) {
            return Err("Time must be written HH:MM in 24-hour form, e.g. 19:30".into());
        }
        if title.trim().is_empty() {
            return Err("Title can't be empty".into());
        }
        if title.contains('\n') || title.len() > 50 {
            return Err("Title must be one line of at most 50 characters".into());
        }
        if with.contains('\n') || with.len() > 40 {
            return Err("With must be one line of at most 40 characters".into());
        }
        Ok(())
    }
    fn today(&self) -> String {
        self.today.clone()
    }
    fn max_page(&self) -> usize {
        16 * 1024
    }
    fn free_space(&self) -> usize {
        MAX_DESK.saturating_sub(self.total())
    }
}
