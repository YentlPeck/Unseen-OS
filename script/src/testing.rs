//! Test pages (design §6): `test 'TITLE'` ... `done` blocks with `check` and `pretend`.
//!
//! Each test starts fresh: its own names, its own budgets and its own test world. The code
//! comes from the real desk (the test page and the pages it uses); everything the program reads
//! while it runs comes from the test world: the empty app drawers, the default settings and the
//! pages the test pretends. Nothing a test does is ever made on the real desk.

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::format;
use alloc::rc::Rc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::cell::{Cell, RefCell};

use crate::app::{deep_copy, is_key, App, Step, APP_COLS, APP_ROWS, COLOURS};
use crate::limits::*;
use crate::parse::{builtin_name, Expr, Op, Pretend, Program, Stmt, Test, B};
use crate::run::{at, mistake, prepare, text_len, write_text, Action, Mistake, Runner, Running, World};
use crate::util::*;
use crate::value::*;

/// The date every test runs on, unless it says `pretend today is ...`.
pub const TEST_TODAY: &str = "2026-11-14";
const DEFAULT_SETTINGS: &str = "name: friend\ntheme: blue\nstart: shell\nclock: 24h";
const APP_DRAWERS: [(&str, &str); 4] = [("mail", "Mail"), ("contacts", "Contacts"), ("calendar", "Calendar"), ("system", "Settings")];
const MAX_DESK: usize = 1024 * 1024;
/// Drawers and pages in all, as the kernel's desk (fs::MAX_ITEMS).
const MAX_ITEMS: usize = 5000;
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
    let results = program.tests.iter().map(|t| one_test(&program, t, desk)).collect();
    TestReport { error: None, results }
}

fn one_test<D: World>(program: &Program, t: &Test, desk: &D) -> TestResult {
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
    // 2. the test itself; launch and press make their steps' changes in the test world
    let world = RefCell::new(world);
    let live = Live(&world);
    let tester = Tester { desk, world: &world, app: RefCell::new(None), launched: Cell::new(false), open: Cell::new(false), mistake: RefCell::new(String::new()), made: RefCell::new(Vec::new()), steps: Cell::new(0) };
    let m = Meter::new(STEP_MEMORY);
    let mut r = Runner::new(&live, program, &m);
    r.harness = Some(&tester);
    r.part = Running::Test;
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
                // made by the app's steps first, then planned by the test's own lines
                let made = self.harness.map(|h| h.made()).unwrap_or_default();
                for a in made.iter().chain(self.actions.iter()) {
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
            B::AppState | B::ScreenLine | B::ScreenColour | B::AppOpen | B::AppMistake => match self.harness {
                Some(h) => {
                    let (v, work) = h.look(b, &v, &self.m)?;
                    self.tick(work)?;
                    v
                }
                None => return Err("internal: no test".into()),
            },
            _ => return Err("internal: not a test function".into()),
        })
    }
}

/// What a test's own lines use for launch, press and the app checks.
pub(crate) trait Harness {
    /// `own`: the steps the test's own lines have taken so far (they share one budget).
    fn launch(&self, page: &str, own: u64) -> Res<()>;
    fn press(&self, key: &str, own: u64) -> Res<()>;
    /// app_state(), screen_line(row), screen_colour(row, col), app_open(), app_mistake(),
    /// and the work it took (counted as steps).
    fn look(&self, b: B, v: &[Value], m: &Rc<Meter>) -> Res<(Value, usize)>;
    /// The mails and events the app's steps made.
    fn made(&self) -> Vec<Action>;
    /// The steps all the app's steps took so far.
    fn used(&self) -> u64;
}

/// The test world, seen by a program while the test's steps change it.
struct Live<'w>(&'w RefCell<TestWorld>);

impl World for Live<'_> {
    fn page(&self, p: &str) -> Option<String> {
        self.0.borrow().page(p)
    }
    fn is_drawer(&self, p: &str) -> bool {
        self.0.borrow().is_drawer(p)
    }
    fn pages_in(&self, d: &str) -> Vec<String> {
        self.0.borrow().pages_in(d)
    }
    fn check_write(&self, p: &str) -> Result<(), String> {
        self.0.borrow().check_write(p)
    }
    fn check_toss(&self, p: &str) -> Result<(), String> {
        self.0.borrow().check_toss(p)
    }
    fn check_send(&self, a: &str, b: &str, c: &str) -> Result<(), String> {
        self.0.borrow().check_send(a, b, c)
    }
    fn check_event(&self, a: &str, b: &str, c: &str, d: &str) -> Result<(), String> {
        self.0.borrow().check_event(a, b, c, d)
    }
    fn today(&self) -> String {
        self.0.borrow().today()
    }
    fn max_page(&self) -> usize {
        self.0.borrow().max_page()
    }
    fn page_len(&self, p: &str) -> Option<usize> {
        self.0.borrow().pages.get(p).map(|t| t.len())
    }
    fn free_space(&self) -> usize {
        self.0.borrow().free_space()
    }
    fn free_items(&self) -> usize {
        self.0.borrow().free_items()
    }
}

struct Tester<'w, D: World> {
    /// Where the app's code comes from.
    desk: &'w D,
    world: &'w RefCell<TestWorld>,
    app: RefCell<Option<App>>,
    /// launch was used (even if the app didn't open).
    launched: Cell<bool>,
    open: Cell<bool>,
    /// The mistake that closed the app ('' if none).
    mistake: RefCell<String>,
    made: RefCell<Vec<Action>>,
    /// All the app's steps in this test.
    steps: Cell<u64>,
}

impl<D: World> Tester<'_, D> {
    /// How many steps the next app step may take: the test's own lines and all its app
    /// steps (also the failed ones) share MAX_TEST_STEPS.
    fn budget(&self, own: u64) -> Res<u64> {
        let used = own.saturating_add(self.steps.get());
        if used >= MAX_TEST_STEPS {
            return Err(too_many_steps());
        }
        Ok((MAX_TEST_STEPS - used).min(MAX_STEPS))
    }

    /// After a step: count its work; if it worked, make its changes in the test world, then
    /// keep its state and screen.
    fn finish(&self, app: &mut App, (step, used): (Result<Step, Mistake>, u64), own: u64) -> Res<()> {
        let total = self.steps.get().saturating_add(used);
        self.steps.set(total);
        if total.saturating_add(own) > MAX_TEST_STEPS {
            self.fail("the test's steps ran out".into());
            return Err(too_many_steps());
        }
        let step = match step {
            Ok(s) => s,
            Err(e) => return Ok(self.fail(e.to_string())),
        };
        if let Err(e) = self.world.borrow_mut().make(&step.actions) {
            return Ok(self.fail(format!("Stopped while making the changes: {e}")));
        }
        self.made.borrow_mut().extend(step.actions.iter().filter(|a| matches!(a, Action::Send { .. } | Action::Event { .. })).cloned());
        if app.keep(step) {
            self.open.set(false);
        }
        Ok(())
    }

    fn fail(&self, why: String) {
        self.open.set(false);
        *self.mistake.borrow_mut() = why;
    }

    fn need_app(&self, b: B) -> Res<()> {
        if !self.launched.get() {
            return Err(format!("{}() needs an app: launch 'PAGE' first", builtin_name(b)).into());
        }
        if self.app.borrow().is_none() {
            return Err(format!("{}() needs an app, but it never started: {}", builtin_name(b), self.mistake.borrow()).into());
        }
        Ok(())
    }
}

fn too_many_steps() -> Fault {
    limit_fault(format!("this test took too many steps in all, with its app's steps (at most {MAX_TEST_STEPS})"))
}

fn row_arg(v: &Value, what: &str, min: usize, max: usize) -> Res<usize> {
    match v {
        Value::Num(n) if *n >= min as f64 && *n <= max as f64 && *n == (*n as i64) as f64 => Ok(*n as usize),
        _ => Err(format!("{what} must be a whole number from {min} to {max}, got {}", v.describe()).into()),
    }
}

impl<D: World> Harness for Tester<'_, D> {
    fn launch(&self, page: &str, own: u64) -> Res<()> {
        let budget = self.budget(own)?;
        let path = path_of(page)?;
        let Some(source) = self.desk.page(&path) else {
            return Err(format!("there is no page called {} (an app's code comes from the desk)", display(&path)).into());
        };
        self.mistake.borrow_mut().clear();
        self.launched.set(true);
        self.open.set(true);
        *self.app.borrow_mut() = None;
        // the code comes from the desk; what it reads, from the test world
        let mut app = match App::load(&path, &source, self.desk) {
            Ok(a) => a,
            Err(e) => {
                self.fail(e.to_string());
                return Ok(());
            }
        };
        let step = app.start_within(&Live(self.world), budget);
        let r = self.finish(&mut app, step, own);
        *self.app.borrow_mut() = Some(app);
        r
    }

    fn press(&self, key: &str, own: u64) -> Res<()> {
        if !is_key(key) {
            return Err(format!("'{}' is not a key (keys are like 'a', 'enter', 'up' or 'ctrl+s')", short(key)).into());
        }
        if !self.launched.get() {
            return Err(Fault::check_failed(format!("press '{key}', but no app was launched (launch 'PAGE' first)")));
        }
        let mut slot = self.app.borrow_mut();
        let (Some(app), true) = (slot.as_mut(), self.open.get()) else {
            let why = self.mistake.borrow();
            let after = if why.is_empty() { String::new() } else { format!(" after a mistake: {why}") };
            return Err(Fault::check_failed(format!("press '{key}', but the app has closed{after}")));
        };
        if key == "ctrl+q" {
            self.open.set(false);
            return Ok(());
        }
        let budget = self.budget(own)?;
        let step = app.key_within(key, &Live(self.world), budget);
        self.finish(app, step, own)
    }

    fn look(&self, b: B, v: &[Value], m: &Rc<Meter>) -> Res<(Value, usize)> {
        let mut work = 1;
        let v = match b {
            B::AppOpen => Value::Bool(self.open.get()),
            B::AppMistake => text(m, &self.mistake.borrow())?,
            B::AppState => {
                self.need_app(b)?;
                // a copy counted by the test (a shared value would keep old states alive uncounted)
                match self.app.borrow().as_ref() {
                    Some(a) => deep_copy(a.state(), m, &mut work)?,
                    None => Value::Nothing,
                }
            }
            B::ScreenLine => {
                let row = row_arg(&v[0], "screen_line(row): the row", 0, APP_ROWS + 1)?;
                self.need_app(b)?;
                let line = self.app.borrow().as_ref().map(|a| a.screen().line(row)).unwrap_or_default();
                text(m, &line)?
            }
            B::ScreenColour => {
                let row = row_arg(&v[0], "screen_colour(row, col): the row", 1, APP_ROWS)?;
                let col = row_arg(&v[1], "screen_colour(row, col): the column", 1, APP_COLS)?;
                self.need_app(b)?;
                let c = self.app.borrow().as_ref().map(|a| a.screen().cell(row, col).1).unwrap_or(0);
                text(m, COLOURS[c as usize])?
            }
            _ => return Err("internal: not an app check".into()),
        };
        Ok((v, work))
    }

    fn made(&self) -> Vec<Action> {
        self.made.borrow().clone()
    }

    fn used(&self) -> u64 {
        self.steps.get()
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
        let mut new = usize::from(!self.pages.contains_key(&p));
        let mut d = parent(&p);
        while !d.is_empty() && !self.drawers.contains(d) {
            new += 1;
            d = parent(d);
        }
        if new > self.free_items() {
            return Err(format!("the test world would hold too many pages (at most {MAX_ITEMS} drawers and pages)"));
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

    /// Make a step's changes, all of them or none, as the kernel makes them (chores::make).
    pub fn make(&mut self, actions: &[Action]) -> Result<(), String> {
        if actions.is_empty() {
            return Ok(());
        }
        let mut copy = TestWorld { pages: self.pages.clone(), drawers: self.drawers.clone(), today: self.today.clone() };
        let too_long = |p: &str| format!("{} would be too long", display(p));
        for a in actions {
            match a {
                Action::Write { page, text } => {
                    copy.check_write(page)?;
                    if text.len() > 16 * 1024 {
                        return Err(too_long(page));
                    }
                    copy.pages.insert(page.clone(), text.clone());
                }
                Action::Add { page, line } => {
                    copy.check_write(page)?;
                    let old = copy.pages.get(page).cloned().unwrap_or_default();
                    let new = if old.is_empty() { line.clone() } else { format!("{old}\n{line}") };
                    if new.len() > 16 * 1024 {
                        return Err(too_long(page));
                    }
                    copy.pages.insert(page.clone(), new);
                }
                Action::Toss { page } => {
                    if copy.pages.remove(page).is_none() {
                        return Err(format!("there is no page called {}", display(page)));
                    }
                }
                Action::Send { to, subject, text } => {
                    let mut n = 1;
                    while copy.taken(&format!("mail:sent:s{n}")) {
                        n += 1;
                    }
                    let body = format!("from: me\nto: {to}\nsubject: {subject}\n\n{}", text.trim_end());
                    copy.pages.insert(format!("mail:sent:s{n}"), body);
                }
                Action::Event { date, time, title, with } => {
                    let mut n = 1;
                    while copy.taken(&format!("calendar:e{n}")) {
                        n += 1;
                    }
                    copy.pages.insert(format!("calendar:e{n}"), format!("date: {date}\ntime: {time}\ntitle: {title}\nwith: {with}"));
                }
            }
        }
        if copy.total() > MAX_DESK {
            return Err("the test world would be too full".into());
        }
        if copy.count() > MAX_ITEMS {
            return Err(format!("the desk is full (at most {MAX_ITEMS} drawers and pages in all)"));
        }
        *self = copy;
        Ok(())
    }

    /// A page or a drawer is there (Mail and Calendar pick the first free number).
    fn taken(&self, p: &str) -> bool {
        self.pages.contains_key(p) || self.drawers.contains(p)
    }

    /// Drawers and pages, as the kernel counts them (the desk itself is not one).
    fn count(&self) -> usize {
        self.pages.len() + self.drawers.len() - usize::from(self.drawers.contains(""))
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
    fn free_items(&self) -> usize {
        MAX_ITEMS.saturating_sub(self.count())
    }
}
