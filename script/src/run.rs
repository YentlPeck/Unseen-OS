//! The runner: carries out a checked page against a read-only view of the world (design §4).
//!
//! Changes (write, add, toss, send, event) are only planned: the caller makes them together,
//! and only when the run finished without a mistake ("all or nothing"). Each planned change is
//! checked when it is planned, against the world as it would be after the changes before it.

use alloc::format;
use alloc::rc::Rc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::cmp::Ordering;
use core::fmt;

use crate::limits::*;
use crate::check::check;
use crate::parse::{builtin_name, Expr, Line, Op, Part, Program, Stmt, Target, B};
use crate::program::load;
use crate::util::*;
use crate::value::*;

/// What a program may look at, and the checks its changes must pass. Paths are internal
/// paths: no "desk:" in front, the desk itself is "". (The same as the chore engine's World.)
pub trait World {
    /// Text of a page, or None if there is no page there (missing or a drawer).
    fn page(&self, path: &str) -> Option<String>;
    fn is_drawer(&self, path: &str) -> bool;
    /// Paths of the pages directly inside a drawer (any order).
    fn pages_in(&self, drawer: &str) -> Vec<String>;
    fn check_write(&self, path: &str) -> Result<(), String>;
    fn check_toss(&self, path: &str) -> Result<(), String>;
    fn check_send(&self, to: &str, subject: &str, text: &str) -> Result<(), String>;
    fn check_event(&self, date: &str, time: &str, title: &str, with: &str) -> Result<(), String>;
    fn today(&self) -> String;
    /// The longest text a page may have.
    fn max_page(&self) -> usize {
        usize::MAX
    }
    /// The length of a page's text (None if there is no page there).
    fn page_len(&self, path: &str) -> Option<usize> {
        self.page(path).map(|t| t.len())
    }
    /// How much more text the desk can hold (bytes of page text, as the kernel counts them).
    fn free_space(&self) -> usize {
        usize::MAX
    }
    /// How many more pages and drawers the desk can hold.
    fn free_items(&self) -> usize {
        usize::MAX
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Write { page: String, text: String },
    Add { page: String, line: String },
    Toss { page: String },
    Send { to: String, subject: String, text: String },
    Event { date: String, time: String, title: String, with: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Mistake {
    /// The page the mistake is on, as an internal path.
    pub page: String,
    /// 0: about the whole page.
    pub line: usize,
    pub message: String,
    /// A limit ran out.
    pub limit: bool,
    /// True if it was found before anything ran.
    pub before_running: bool,
    /// The (page, line)s the failing task was called from, innermost first.
    pub trail: Vec<(String, usize)>,
}

impl fmt::Display for Mistake {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.line == 0 {
            write!(f, "{}: {}", display(&self.page), self.message)?;
        } else {
            write!(f, "{} line {}: {}", display(&self.page), self.line, self.message)?;
        }
        // a task calling itself many times: one line, with how many times
        let mut i = 0;
        while i < self.trail.len() {
            let mut j = i + 1;
            while j < self.trail.len() && self.trail[j] == self.trail[i] {
                j += 1;
            }
            let (p, t) = &self.trail[i];
            write!(f, "\n  called from {} line {t}", display(p))?;
            if j - i > 1 {
                write!(f, " ({} times)", j - i)?;
            }
            i = j;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct Outcome {
    /// The `say` lines, in order (kept even when the run fails, as chores).
    pub output: Vec<String>,
    /// The changes to make. Always empty when `error` is set.
    pub actions: Vec<Action>,
    pub steps: u64,
    /// The most memory the run used at one moment (bytes, as counted).
    pub peak_memory: usize,
    pub error: Option<Mistake>,
}

/// Load and check a program: the page `page` with this source, and the pages it uses (read
/// from `world`). A mistake here is found before anything runs.
pub(crate) fn prepare<W: World>(page: &str, source: &str, world: &W) -> Result<Program, Mistake> {
    let program = load(page, source, world).map_err(|e| Mistake {
        page: e.page,
        line: e.line,
        message: e.message,
        limit: false,
        before_running: true,
        trail: Vec::new(),
    })?;
    check(&program).map_err(|(p, line, message)| Mistake {
        page: program.pages[p].clone(),
        line,
        message,
        limit: false,
        before_running: true,
        trail: Vec::new(),
    })?;
    Ok(program)
}

/// The mistake a fault becomes, with its pages named.
pub(crate) fn mistake(p: &Program, line: usize, f: Fault) -> Mistake {
    let f = *f.0;
    let ((page, line), trail) = f.place.unwrap_or(((0, line), Vec::new()));
    let name = |i: usize| p.pages.get(i).cloned().unwrap_or_default();
    Mistake {
        page: name(page),
        line,
        message: f.msg,
        limit: f.limit,
        before_running: false,
        trail: trail.into_iter().map(|(pg, l)| (name(pg), l)).collect(),
    }
}

/// Check and run one page (and the pages it uses). Never panics.
pub fn run<W: World>(page: &str, source: &str, world: &W) -> Outcome {
    let program = match prepare(page, source, world) {
        Ok(p) => p,
        Err(error) => {
            return Outcome { output: Vec::new(), actions: Vec::new(), steps: 0, peak_memory: 0, error: Some(error) };
        }
    };
    let m = Meter::new(STEP_MEMORY);
    let mut r = Runner::new(world, &program, &m);
    let result = r.fixed_values().and_then(|_| r.block(&program.main));
    let error = match result {
        Ok(_) => None,
        Err((_, f)) if f.stop => None,
        Err((line, f)) => Some(mistake(&program, line, f)),
    };
    let actions = if error.is_some() { Vec::new() } else { core::mem::take(&mut r.actions) };
    let output = core::mem::take(&mut r.out);
    let steps = r.steps;
    drop(r);
    Outcome { output, actions, steps, peak_memory: m.peak(), error }
}

pub(crate) enum Flow {
    Next,
    Stop,
    Skip,
    Quit,
    Give(Value),
}

/// A fault passing line `n` of page `page`: the first line it passes is where it happened;
/// the next ones (outside a task) are the lines the task was called from.
pub(crate) fn at(page: usize, n: usize, mut e: Fault) -> (usize, Fault) {
    match &mut e.place {
        Some((_, trail)) => trail.push((page, n)),
        None => e.place = Some(((page, n), Vec::new())),
    }
    (n, e)
}

pub(crate) struct Runner<'a, W: World> {
    pub(crate) w: &'a W,
    p: &'a Program,
    pub(crate) m: Rc<Meter>,
    /// The names of the task (or main page) that is running.
    vars: Vec<(String, Value)>,
    fixed: Vec<(String, Value)>,
    pub(crate) out: Vec<String>,
    pub(crate) actions: Vec<Action>,
    /// The planned state of each page changed so far: Some(text), or None if tossed.
    pub(crate) plan: Vec<(String, Option<String>)>,
    calls: usize,
    /// How deep the running code is (values being worked out, inside tasks): bounds the stack.
    level: usize,
    /// The page whose lines are running (an index into Program::pages).
    pub(crate) page: usize,
    /// The checks a test has made.
    pub(crate) checks: usize,
    pub(crate) steps: u64,
    /// How much the planned changes make the desk's text grow (bytes; below 0: shrink).
    growth: i64,
    /// How many pages the planned changes add (below 0: take away).
    items: i64,
}

/// What a `try` puts back if its try part fails.
struct Snapshot {
    names: Vec<(String, Value)>,
    kept: Charge,
    said: usize,
    changes: usize,
    planned: usize,
    growth: i64,
    items: i64,
}

/// What a target change does at its end.
enum Change {
    Assign(Value),
    Push(Value),
}

enum PartV {
    Field(String),
    At(usize),
}

impl<'a, W: World> Runner<'a, W> {
    pub(crate) fn new(w: &'a W, p: &'a Program, m: &Rc<Meter>) -> Self {
        Runner {
            w,
            p,
            m: m.clone(),
            vars: Vec::new(),
            fixed: Vec::new(),
            out: Vec::new(),
            actions: Vec::new(),
            plan: Vec::new(),
            steps: 0,
            growth: 0,
            items: 0,
            calls: 0,
            level: 0,
            page: 0,
            checks: 0,
        }
    }

    pub(crate) fn tick(&mut self, n: usize) -> Res<()> {
        self.steps = self.steps.saturating_add(n as u64);
        if self.steps > MAX_STEPS {
            self.steps = MAX_STEPS + 1;
            Err(limit_fault(format!("the program took too many steps (at most {MAX_STEPS}); is a loop too big?")))
        } else {
            Ok(())
        }
    }

    /// `same`, counting its work as steps (and stopping as soon as the steps run out).
    pub(crate) fn same(&mut self, a: &Value, b: &Value) -> Res<bool> {
        let mut work = 0;
        let budget = (MAX_STEPS - self.steps.min(MAX_STEPS)) as usize;
        match same(a, b, &mut work, budget) {
            Some(r) => {
                self.tick(work)?;
                Ok(r)
            }
            None => {
                self.tick(budget + 1)?;
                Err("internal: over budget".into())
            }
        }
    }

    fn get(&self, name: &str) -> Res<Value> {
        match self.vars.iter().chain(self.fixed.iter()).find(|(n, _)| n == name) {
            Some((_, v)) => Ok(v.clone()),
            None => Err(format!("{name} has no value yet (give it one with set {name} to ...)").into()),
        }
    }

    /// Work out the fixed values, in order, before the program runs.
    /// Fixed values belong to the program, not to a run: they are counted by the program
    /// budget, and the steps spent working them out don't count for the run.
    pub(crate) fn fixed_values(&mut self) -> Result<Flow, (usize, Fault)> {
        if self.p.fixed.is_empty() {
            return Ok(Flow::Next);
        }
        let step_meter = core::mem::replace(&mut self.m, Meter::new(PROGRAM_MEMORY));
        if let Err(e) = self.m.charge(self.p.cost) {
            self.m = step_meter;
            return Err(at(0, 0, e));
        }
        let r = (|| {
            for f in &self.p.fixed {
                self.page = f.page;
                let v = self.eval(&f.value).map_err(|e| at(f.page, f.line, e))?;
                self.m.charge(NAME_COST + f.name.len()).map_err(|e| at(f.page, f.line, e))?;
                self.fixed.push((f.name.clone(), v));
            }
            self.page = 0;
            Ok(Flow::Next)
        })();
        self.m = step_meter;
        self.steps = 0;
        r
    }

    /// Give back the memory of a task's names when it ends.
    fn drop_names(&mut self, names: Vec<(String, Value)>) {
        for (n, _) in &names {
            self.m.release(NAME_COST + n.len());
        }
    }

    #[inline(never)] // keeps the recursive eval() frame small (small kernel stack)
    fn call_task(&mut self, name: &str, args: Vec<Value>) -> Res<Value> {
        if self.calls >= MAX_CALLS {
            return Err(limit_fault(format!("tasks call each other too deep (at most {MAX_CALLS})")));
        }
        let p = self.p;
        let Some(t) = p.tasks.iter().find(|t| t.name == name) else {
            return Err(format!("there is no task called {name}(...)").into()); // the checker prevents this
        };
        self.tick(1)?;
        let saved = core::mem::take(&mut self.vars);
        let mut r = Ok(Flow::Next);
        for (x, v) in t.params.iter().zip(args) {
            if let Err(e) = self.set_var(x, v) {
                r = Err((t.line, e));
                break;
            }
        }
        let caller = self.page;
        if r.is_ok() && self.level + CALL_LEVELS >= MAX_LEVEL {
            r = Err((t.line, limit_fault("the program goes too deep (tasks inside tasks inside long lines)".into())));
        }
        if r.is_ok() {
            // a call puts several frames on the stack: it counts as several levels
            self.calls += 1;
            self.level += CALL_LEVELS;
            self.page = t.page;
            r = self.block(&t.body);
            self.page = caller;
            self.level -= CALL_LEVELS;
            self.calls -= 1;
        }
        let names = core::mem::replace(&mut self.vars, saved);
        self.drop_names(names);
        match r {
            Ok(Flow::Give(v)) => Ok(v),
            Ok(Flow::Stop) => Err(Fault::stopped()),
            Ok(_) => Ok(Value::Nothing),
            Err((line, mut f)) => {
                if f.place.is_none() {
                    f.place = Some(((t.page, line), Vec::new()));
                }
                Err(f)
            }
        }
    }

    fn set_var(&mut self, name: &str, v: Value) -> Res<()> {
        if let Some(slot) = self.vars.iter_mut().find(|(n, _)| n == name) {
            slot.1 = v;
            return Ok(());
        }
        self.m.charge(NAME_COST + name.len())?;
        self.vars.push((name.to_string(), v));
        Ok(())
    }

    /// Run lines. Counts as one level (with values being worked out and task calls), so the
    /// stack used by blocks inside tasks inside blocks stays bounded.
    pub(crate) fn block(&mut self, lines: &[Line]) -> Result<Flow, (usize, Fault)> {
        if self.level >= MAX_LEVEL {
            let n = lines.first().map_or(0, |l| l.n);
            return Err(at(self.page, n, limit_fault("the program goes too deep (blocks and tasks inside each other)".into())));
        }
        self.level += 1;
        let mut r = Ok(Flow::Next);
        for line in lines {
            match self.line(line) {
                Ok(Flow::Next) => {}
                other => {
                    r = other;
                    break;
                }
            }
        }
        self.level -= 1;
        r
    }

    /// Runs a loop body once; Some(flow) when the loop must end with that flow.
    fn turn(&mut self, body: &[Line]) -> Result<Option<Flow>, (usize, Fault)> {
        Ok(match self.block(body)? {
            Flow::Next | Flow::Skip => None,
            Flow::Quit => Some(Flow::Next),
            Flow::Stop => Some(Flow::Stop),
            Flow::Give(v) => Some(Flow::Give(v)),
        })
    }

    /// One line. Kept small: the work is in helpers that are not inlined, so the frames on the
    /// way down through blocks and tasks stay small (the kernel stack is small).
    fn line(&mut self, line: &Line) -> Result<Flow, (usize, Fault)> {
        let (pg, n) = (self.page, line.n);
        self.tick(1).map_err(|e| at(pg, n, e))?;
        match &line.stmt {
            Stmt::When(branches, otherwise) => {
                let body = self.branch(branches, otherwise).map_err(|e| at(pg, n, e))?;
                self.block(body)
            }
            Stmt::ForEach(..) | Stmt::Repeat(..) | Stmt::While(..) => self.looped(line),
            Stmt::Try(..) => self.try_block(line),
            _ => self.simple(&line.stmt).map_err(|e| at(pg, n, e)),
        }
    }

    /// `try` ... `instead NAME` ... `done`: if a mistake happens in the try part, everything
    /// it did is undone (names, say lines, planned changes), then the instead part runs.
    /// Kept small: the snapshot and the undoing are in helpers (the kernel stack is small).
    #[inline(never)]
    fn try_block(&mut self, line: &Line) -> Result<Flow, (usize, Fault)> {
        let (pg, n) = (self.page, line.n);
        let Stmt::Try(body, name, handler) = &line.stmt else {
            return Ok(Flow::Next);
        };
        let snap = self.snapshot().map_err(|e| at(pg, n, e))?;
        // a try level uses about twice the stack of a block level, also while its instead
        // part runs (the try's own frame is still on the stack then)
        self.level += 1;
        let r = match self.block(body) {
            Err((_, f)) if !f.uncatchable() => match self.undo(snap, &f.msg, name) {
                Ok(()) => self.block(handler),
                Err(e) => Err(at(pg, n, e)),
            },
            other => other,
        };
        self.level -= 1;
        r
    }

    #[inline(never)]
    fn snapshot(&mut self) -> Res<Snapshot> {
        let names_cost: usize = self.vars.iter().map(|(k, _)| NAME_COST + k.len()).sum();
        let kept = Charge::new(&self.m, names_cost + OVERHEAD)?;
        let mut names = Vec::new();
        if names.try_reserve_exact(self.vars.len()).is_err() {
            return Err(limit_fault("the computer has no more memory for this program".into()));
        }
        names.extend(self.vars.iter().cloned());
        Ok(Snapshot { names, kept, said: self.out.len(), changes: self.actions.len(), planned: self.plan.len(), growth: self.growth, items: self.items })
    }

    /// Undo what a try part did, and give the instead part its problem.
    #[inline(never)]
    fn undo(&mut self, snap: Snapshot, problem: &str, name: &str) -> Res<()> {
        let now = core::mem::replace(&mut self.vars, snap.names);
        self.drop_names(now);
        snap.kept.keep(); // the names put back are counted again...
        self.m.release(OVERHEAD); // ...but not the snapshot itself
        for s in self.out.drain(snap.said..) {
            self.m.release(s.len() + OVERHEAD);
        }
        for a in self.actions.drain(snap.changes..) {
            self.m.release(action_cost(&a));
        }
        for (p, t) in self.plan.drain(snap.planned..) {
            self.m.release(OVERHEAD + p.len() + t.map_or(0, |t| t.len()));
        }
        self.growth = snap.growth;
        self.items = snap.items;
        let v = text(&self.m, problem)?;
        self.set_var(name, v)
    }

    #[inline(never)]
    fn branch<'b>(&mut self, branches: &'b [(Expr, Vec<Line>)], otherwise: &'b [Line]) -> Res<&'b [Line]> {
        for (cond, body) in branches {
            match self.eval(cond)? {
                Value::Bool(true) => return Ok(body),
                Value::Bool(false) => {}
                c => return Err(format!("when needs yes or no, got {}", c.describe()).into()),
            }
        }
        Ok(otherwise)
    }

    #[inline(never)]
    fn looped(&mut self, line: &Line) -> Result<Flow, (usize, Fault)> {
        let (pg, n) = (self.page, line.n);
        let at = |e: Fault| at(pg, n, e);
        match &line.stmt {
            Stmt::ForEach(name, list, body) => {
                let l = self.eval(list).map_err(at)?;
                let Value::List(items) = l else {
                    return Err(at(format!(
                        "for each needs a list (like [1, 2], or from pages, lines or words), got {}",
                        l.describe()
                    )
                    .into()));
                };
                for item in items.v.iter() {
                    self.tick(1).map_err(at)?;
                    self.set_var(name, item.clone()).map_err(at)?;
                    if let Some(f) = self.turn(body)? {
                        return Ok(f);
                    }
                }
            }
            Stmt::Repeat(count, body) => {
                let c = self.eval(count).map_err(at)?;
                let times = match c {
                    Value::Num(v) if v >= 0.0 && v <= MAX_REPEAT && v == (v as i64) as f64 => v as i64,
                    _ => {
                        return Err(at(format!(
                            "repeat needs a whole number from 0 to {}, got {}",
                            MAX_REPEAT as i64,
                            c.describe()
                        )
                        .into()))
                    }
                };
                for _ in 0..times {
                    self.tick(1).map_err(at)?;
                    if let Some(f) = self.turn(body)? {
                        return Ok(f);
                    }
                }
            }
            Stmt::While(cond, body) => loop {
                self.tick(1).map_err(at)?;
                let c = self.eval(cond).map_err(at)?;
                let Value::Bool(c) = c else {
                    return Err(at(format!("while needs yes or no, got {}", c.describe()).into()));
                };
                if !c {
                    break;
                }
                if let Some(f) = self.turn(body)? {
                    return Ok(f);
                }
            },
            _ => {}
        }
        Ok(Flow::Next)
    }

    #[inline(never)]
    fn simple(&mut self, stmt: &Stmt) -> Res<Flow> {
        match stmt {
            Stmt::Set(t, e) => {
                let v = self.eval(e)?;
                self.change(t, Change::Assign(v))?;
            }
            Stmt::AddTo(e, t) => {
                let v = self.eval(e)?;
                self.change(t, Change::Push(v))?;
            }
            Stmt::Say(e) => {
                let v = self.eval(e)?;
                if self.out.len() >= MAX_SAY {
                    return Err(limit_fault(format!("too many say lines (at most {MAX_SAY})")));
                }
                let s = self.string(&v)?; // counted by string()
                if self.out.try_reserve(1).is_err() {
                    return Err(limit_fault("the computer has no more memory for this program".into()));
                }
                self.out.push(s);
            }
            Stmt::Stop => return Ok(Flow::Stop),
            Stmt::Skip => return Ok(Flow::Skip),
            Stmt::Quit => return Ok(Flow::Quit),
            Stmt::Do(b, args) => self.action(*b, args)?,
            Stmt::Call(e) => {
                self.eval(e)?;
            }
            Stmt::Give(e) => return Ok(Flow::Give(self.eval(e)?)),
            Stmt::Fail(e) => {
                let v = self.eval(e)?;
                let len = text_len(&v, MAX_TEXT)?;
                let mut s = String::new();
                if s.try_reserve_exact(len).is_err() {
                    return Err(limit_fault("the computer has no more memory for this program".into()));
                }
                write_text(&v, &mut s);
                return Err(Fault::new(s));
            }
            Stmt::Check(e) => self.check(e)?,
            Stmt::Pretend(_) => {} // made before the test runs
            Stmt::When(..) | Stmt::ForEach(..) | Stmt::Repeat(..) | Stmt::While(..) | Stmt::Try(..) => {}
        }
        Ok(Flow::Next)
    }

    // ------------------------------------------------------------- targets --

    fn change(&mut self, t: &Target, change: Change) -> Res<()> {
        let mut parts = Vec::new();
        for p in &t.parts {
            parts.push(match p {
                Part::Field(f) => PartV::Field(f.clone()),
                Part::At(e) => {
                    let v = self.eval(e)?;
                    match v {
                        Value::Num(x) if x >= 1.0 && x == (x as i64) as f64 && x <= MAX_LIST as f64 => PartV::At(x as usize),
                        _ => return Err(format!("at needs a whole number from 1, got {}", v.describe()).into()),
                    }
                }
            });
        }
        let new_depth = match &change {
            Change::Assign(v) => v.depth() + parts.len(),
            Change::Push(v) => v.depth() + 1 + parts.len(),
        };
        if new_depth > MAX_VALUE_DEPTH {
            return Err(too_deep());
        }
        if parts.is_empty() {
            match change {
                Change::Assign(v) => return self.set_var(&t.name, v),
                Change::Push(v) => {
                    let i = self.var_index(&t.name)?;
                    let mut cur = core::mem::replace(&mut self.vars[i].1, Value::Nothing);
                    let r = push_into(&mut cur, v, &t.name, &self.m);
                    self.vars[i].1 = cur;
                    return r;
                }
            }
        }
        let i = self.var_index(&t.name)?;
        let mut cur = core::mem::replace(&mut self.vars[i].1, Value::Nothing);
        let r = change_at(&mut cur, &parts, change, &t.name, &self.m);
        self.vars[i].1 = cur;
        r
    }

    fn var_index(&self, name: &str) -> Res<usize> {
        self.vars
            .iter()
            .position(|(n, _)| n == name)
            .ok_or_else(|| format!("{name} has no value yet (give it one with set {name} to ...)").into())
    }

    // --------------------------------------------------------------- values --

    pub(crate) fn eval(&mut self, e: &Expr) -> Res<Value> {
        if self.level >= MAX_LEVEL {
            return Err(limit_fault("the program goes too deep (tasks inside tasks inside long lines)".into()));
        }
        self.level += 1;
        let r = self.eval_here(e);
        self.level -= 1;
        r
    }

    /// Kept small (only a dispatcher): it is on the stack once for every value being worked out.
    fn eval_here(&mut self, e: &Expr) -> Res<Value> {
        self.tick(1)?;
        match e {
            Expr::Num(v) => Ok(Value::Num(*v)),
            Expr::Bool(b) => Ok(Value::Bool(*b)),
            Expr::Text(s) => text(&self.m, s),
            Expr::Var(name) => self.get(name),
            Expr::Neg(x) | Expr::Not(x) => {
                let v = self.eval(x)?;
                unary(e, v)
            }
            Expr::Bin(Op::And, l, r) | Expr::Bin(Op::Or, l, r) => {
                let is_and = matches!(e, Expr::Bin(Op::And, _, _));
                let a = self.eval(l)?;
                match logic(is_and, a)? {
                    Some(decided) => Ok(decided), // the right side is not looked at
                    None => match self.eval(r)? {
                        Value::Bool(b) => Ok(Value::Bool(b)),
                        _ => Err(both_sides(is_and)),
                    },
                }
            }
            Expr::Bin(op, l, r) => {
                let a = self.eval(l)?;
                let b = self.eval(r)?;
                self.binary(*op, a, b)
            }
            Expr::Call(b, args) => {
                let (vals, _counted) = self.values(args)?;
                self.call(*b, vals)
            }
            Expr::Task(name, args) => {
                let (vals, _counted) = self.values(args)?;
                self.call_task(name, vals)
            }
            Expr::List(items) => self.list_literal(items),
            Expr::Record(fields) => self.record_literal(fields),
            Expr::Field(x, name) => {
                let v = self.eval(x)?;
                field_value(&v, name)
            }
        }
    }

    #[inline(never)]
    /// The values of a call's inputs; their room is counted until the call ends.
    fn values(&mut self, args: &[Expr]) -> Res<(Vec<Value>, Charge)> {
        let counted = Charge::new(&self.m, args.len() * ITEM_COST)?;
        let mut vals = Vec::new();
        if vals.try_reserve_exact(args.len()).is_err() {
            return Err(limit_fault("the computer has no more memory for this program".into()));
        }
        for a in args {
            vals.push(self.eval(a)?);
        }
        Ok((vals, counted))
    }

    #[inline(never)] // keeps the recursive eval() frame small (small kernel stack)
    fn list_literal(&mut self, items: &[Expr]) -> Res<Value> {
        let mut lm = ListMaker::new(&self.m, items.len())?;
        for x in items {
            let v = self.eval(x)?;
            lm.push(v)?;
        }
        Ok(lm.finish())
    }

    #[inline(never)]
    fn record_literal(&mut self, fields: &[(String, Expr)]) -> Res<Value> {
        // counted before anything is allocated; given back if making the record fails
        let charge = Charge::new(&self.m, record_cost(fields.iter().map(|(k, _)| k.as_str())))?;
        let mut out = Vec::new();
        if out.try_reserve_exact(fields.len()).is_err() {
            return Err(limit_fault("the computer has no more memory for this program".into()));
        }
        for (k, x) in fields {
            let v = self.eval(x)?;
            let mut name = String::new();
            if name.try_reserve_exact(k.len()).is_err() {
                return Err(limit_fault("the computer has no more memory for this program".into()));
            }
            name.push_str(k);
            out.push((name, v));
        }
        record_charged(charge, out)
    }

    /// The value as text, for `say` lines and changes, which are kept to the end of the run:
    /// counted (and never given back) before it is made.
    fn string(&mut self, v: &Value) -> Res<String> {
        let len = text_len(v, MAX_TEXT)?;
        self.tick(len / TEXT_WORK)?;
        self.m.charge(len + OVERHEAD)?;
        let mut s = String::new();
        if s.try_reserve_exact(len).is_err() {
            return Err(limit_fault("the computer has no more memory for this program".into()));
        }
        write_text(v, &mut s);
        Ok(s)
    }

    #[inline(never)] // keeps the recursive eval() frame small (small kernel stack)
    pub(crate) fn binary(&mut self, op: Op, a: Value, b: Value) -> Res<Value> {
        let sym = match op {
            Op::Mul => "*",
            Op::Div => "/",
            Op::Add => "+",
            Op::Sub => "-",
            Op::Join => "&",
            Op::Is => "is",
            Op::IsNot => "is not",
            Op::Lt => "<",
            Op::Gt => ">",
            Op::Le => "<=",
            Op::Ge => ">=",
            Op::Has => "has",
            Op::And | Op::Or => "and/or",
        };
        Ok(match op {
            Op::Mul | Op::Div | Op::Add | Op::Sub => {
                let (Value::Num(x), Value::Num(y)) = (&a, &b) else {
                    let bad = if matches!(a, Value::Num(_)) { &b } else { &a };
                    let hint = if matches!(bad, Value::Text(_)) { " (number(...) turns text into a number)" } else { "" };
                    return Err(format!("{sym} needs two numbers, got {}{hint}", bad.describe()).into());
                };
                let v = match op {
                    Op::Mul => x * y,
                    Op::Add => x + y,
                    Op::Sub => x - y,
                    _ if *y == 0.0 => return Err("dividing by 0 is not possible".into()),
                    _ => x / y,
                };
                if !(v.abs() < NUM_LIMIT) {
                    return Err("that number is too big (it must stay below 1000000000000)".into());
                }
                Value::Num(v)
            }
            Op::Join => {
                let la = text_len(&a, MAX_TEXT)?;
                let lb = text_len(&b, MAX_TEXT)?;
                if la + lb > MAX_TEXT {
                    return Err(format!("that text would be too long (at most {MAX_TEXT} characters)").into());
                }
                self.tick((la + lb) / TEXT_WORK)?;
                new_text(&self.m, la + lb, |s| {
                    write_text(&a, s);
                    write_text(&b, s);
                })?
            }
            Op::Is | Op::IsNot => {
                let comparable = matches!(a, Value::Nothing)
                    || matches!(b, Value::Nothing)
                    || core::mem::discriminant(&a) == core::mem::discriminant(&b);
                if !comparable {
                    return Err(format!(
                        "{sym} compares two values of the same kind, got {} and {}{}",
                        a.kind(),
                        b.kind(),
                        if matches!((&a, &b), (Value::Num(_), Value::Text(_)) | (Value::Text(_), Value::Num(_))) {
                            " (number(...) turns text into a number)"
                        } else {
                            ""
                        }
                    )
                    .into());
                }
                let same = self.same(&a, &b)?;
                Value::Bool(same == (op == Op::Is))
            }
            Op::Lt | Op::Gt | Op::Le | Op::Ge => {
                let o = match (&a, &b) {
                    (Value::Num(x), Value::Num(y)) => x.partial_cmp(y).unwrap_or(Ordering::Equal),
                    (Value::Text(x), Value::Text(y)) => {
                        self.tick(x.s.len().min(y.s.len()) / COMPARE_WORK)?;
                        x.s.as_str().cmp(y.s.as_str())
                    }
                    _ => {
                        return Err(format!("{sym} compares two numbers or two texts, got {} and {}", a.kind(), b.kind())
                            .into())
                    }
                };
                Value::Bool(match op {
                    Op::Lt => o == Ordering::Less,
                    Op::Gt => o == Ordering::Greater,
                    Op::Le => o != Ordering::Greater,
                    _ => o != Ordering::Less,
                })
            }
            Op::Has => match (&a, &b) {
                (Value::Text(x), Value::Text(y)) => {
                    self.tick(x.s.len() / TEXT_WORK)?;
                    Value::Bool(x.s.contains(y.s.as_str()))
                }
                (Value::List(l), _) => {
                    self.tick(l.v.len())?;
                    let mut found = false;
                    for x in l.v.iter() {
                        if core::mem::discriminant(x) == core::mem::discriminant(&b) && self.same(x, &b)? {
                            found = true;
                            break;
                        }
                    }
                    Value::Bool(found)
                }
                _ => {
                    return Err(format!(
                        "has needs text or a list on the left (text on the right for text), got {} and {}",
                        a.kind(),
                        b.kind()
                    )
                    .into())
                }
            },
            Op::And | Op::Or => return Err("and/or are handled elsewhere".into()),
        })
    }

    fn page_text(&self, x: &Value) -> Res<(String, String)> {
        let p = self.path(x)?;
        match self.w.page(&p) {
            Some(t) if t.len() > MAX_TEXT => Err(format!("{} is too long for a program to read", display(&p)).into()),
            Some(t) => Ok((p, t)),
            None if self.w.is_drawer(&p) => Err(format!("{} is a drawer, not a page", display(&p)).into()),
            None => Err(format!("there is no page called {}", display(&p)).into()),
        }
    }

    fn path(&self, x: &Value) -> Res<String> {
        match x {
            Value::Text(t) => Ok(path_of(&t.s)?),
            _ => Err(format!("a page or drawer must be given as text, got {}", x.describe()).into()),
        }
    }

    fn texts(&mut self, items: Vec<&str>, what: &str) -> Res<Value> {
        if items.len() > MAX_LIST {
            return Err(format!("{what} gave more than {MAX_LIST} items").into());
        }
        self.tick(items.len())?;
        let mut lm = ListMaker::new(&self.m, items.len())?;
        for s in items {
            lm.push(text(&self.m, s)?)?;
        }
        Ok(lm.finish())
    }

    #[inline(never)] // keeps the recursive eval() frame small (small kernel stack)
    fn call(&mut self, b: B, v: Vec<Value>) -> Res<Value> {
        let text_arg = |x: &Value, what: &str| -> Res<String> {
            match x {
                Value::Text(t) => Ok(t.s.clone()),
                _ => Err(format!("{what} needs text, got {}", x.describe()).into()),
            }
        };
        let whole = |x: &Value, what: &str| -> Res<i64> {
            match x {
                Value::Num(n) if *n == (*n as i64) as f64 => Ok(*n as i64),
                _ => Err(format!("{what} needs a whole number, got {}", x.describe()).into()),
            }
        };
        Ok(match b {
            B::Pages => {
                let d = self.path(&v[0])?;
                if !self.w.is_drawer(&d) {
                    return Err(format!("there is no drawer called {}", display(&d)).into());
                }
                let mut items = self.w.pages_in(&d);
                if items.len() > MAX_LIST {
                    return Err(format!("pages(...) gave more than {MAX_LIST} items").into());
                }
                items.sort_by(|a, b| natural(a, b));
                let refs: Vec<&str> = items.iter().map(|s| s.as_str()).collect();
                self.texts(refs, "pages(...)")?
            }
            B::Read => {
                let t = self.page_text(&v[0])?.1;
                text(&self.m, &t)?
            }
            B::Field => {
                let key = text_arg(&v[1], "field(page, key): the key")?;
                let t = self.page_text(&v[0])?.1;
                text(&self.m, field_of(&t, &key))?
            }
            B::Body => {
                let t = self.page_text(&v[0])?.1;
                text(&self.m, body_of(&t))?
            }
            B::Exists => {
                let p = self.path(&v[0])?; // reads see the world as it was when the run began
                Value::Bool(self.w.is_drawer(&p) || self.w.page(&p).is_some())
            }
            B::Lines => {
                let t = text_arg(&v[0], "lines(...)")?;
                let items = t.lines().map(|s| s.trim_end_matches('\r')).collect();
                self.texts(items, "lines(...)")?
            }
            B::Words => {
                let t = text_arg(&v[0], "words(...)")?;
                let items = t.split_whitespace().collect();
                self.texts(items, "words(...)")?
            }
            B::Count => match &v[0] {
                Value::List(l) => Value::Num(l.v.len() as f64),
                Value::Text(s) => Value::Num(s.s.len() as f64),
                Value::Rec(r) => Value::Num(r.f.len() as f64),
                x => return Err(format!("count(...) needs a list, a record or text, got {}", x.describe()).into()),
            },
            B::Item => {
                let Value::List(l) = &v[0] else {
                    return Err(format!("item(list, n) needs a list first, got {}", v[0].describe()).into());
                };
                let n = whole(&v[1], "item(list, n)")?;
                if n < 1 || n as usize > l.v.len() {
                    return Err(format!("item {n} doesn't exist: the list has {} items (the first is 1)", l.v.len()).into());
                }
                l.v[n as usize - 1].clone()
            }
            B::Number => {
                let t = text_arg(&v[0], "number(...)")?;
                match number_of(&t) {
                    Some(n) => Value::Num(n),
                    None => return Err(format!("'{}' is not a number", short(t.trim())).into()),
                }
            }
            B::Lower => {
                let t = text_arg(&v[0], "lower(...)")?;
                text(&self.m, &t.to_ascii_lowercase())?
            }
            B::Today => {
                let t = self.w.today();
                text(&self.m, &t)?
            }
            B::Weekday => {
                let t = text_arg(&v[0], "weekday(...)")?;
                let (y, m, d) = parse_date(&t).ok_or_else(|| format!("'{}' is not a date like 2026-11-14", short(&t)))?;
                let w = (days_from_civil(y, m, d).rem_euclid(7) + 4) % 7;
                text(&self.m, ["sunday", "monday", "tuesday", "wednesday", "thursday", "friday", "saturday"][w as usize])?
            }
            B::AddDays => {
                let t = text_arg(&v[0], "add_days(date, n): the date")?;
                let (y, m, d) = parse_date(&t).ok_or_else(|| format!("'{}' is not a date like 2026-11-14", short(&t)))?;
                let n = whole(&v[1], "add_days(date, n)")?;
                if n.abs() > 1_000_000 {
                    return Err("add_days can move at most 1000000 days".into());
                }
                let (y, m, d) = civil_from_days(days_from_civil(y, m, d) + n);
                if !(1..=9999).contains(&y) {
                    return Err("that date is out of range".into());
                }
                text(&self.m, &format!("{y:04}-{m:02}-{d:02}"))?
            }
            B::Nothing => Value::Nothing,
            B::Write | B::Add | B::Toss | B::Send | B::Event => {
                return Err(format!("{}(...) is an action: it must be on a line of its own", builtin_name(b)).into());
            }
            other if crate::parse::is_test_only(other) => return self.test_function(other, v),
            other => return self.library(other, v),
        })
    }

    /// The planned text of a page: Some(Some(text)) written, Some(None) tossed, None unchanged.
    pub(crate) fn planned(&self, p: &str) -> Option<Option<&String>> {
        self.plan.iter().rev().find(|(q, _)| q == p).map(|(_, t)| t.as_ref())
    }

    /// Plan the new state of a page (counted before it is copied).
    fn plan_page(&mut self, p: &str, t: Option<&str>) -> Res<()> {
        self.m.charge(OVERHEAD + p.len() + t.map_or(0, |t| t.len()))?;
        let t = t.map(String::from);
        if self.plan.try_reserve(1).is_err() {
            return Err(limit_fault("the computer has no more memory for this program".into()));
        }
        self.plan.push((String::from(p), t));
        Ok(())
    }

    /// The length of a page as the planned changes so far leave it (None: no page).
    fn planned_len(&self, p: &str) -> Option<usize> {
        match self.planned(p) {
            Some(t) => t.map(|t| t.len()),
            None => self.w.page_len(p),
        }
    }

    /// Count how much a change makes the desk grow, exactly as the kernel counts it: bytes of
    /// page text, and pages (`pages` is 1 for a new page, -1 for a tossed one). A mistake if
    /// the desk would be too full. (The bookkeeping of a change counts in the memory budget.)
    fn grow(&mut self, by: i64, pages: i64) -> Res<()> {
        self.growth = self.growth.saturating_add(by);
        self.items = self.items.saturating_add(pages);
        let free = self.w.free_space().min(i64::MAX as usize) as i64;
        if self.growth > free {
            return Err("the desk would be too full: throw something away first".into());
        }
        let free = self.w.free_items().min(i64::MAX as usize) as i64;
        if self.items > free {
            return Err("the desk would hold too many pages: throw something away first".into());
        }
        Ok(())
    }

    #[inline(never)] // keeps the recursive eval() frame small (small kernel stack)
    fn action(&mut self, b: B, args: &[Expr]) -> Res<()> {
        let mut v = Vec::new();
        if v.try_reserve_exact(args.len()).is_err() {
            return Err(limit_fault("the computer has no more memory for this program".into()));
        }
        for a in args {
            v.push(self.eval(a)?);
        }
        // the texts made for a change that is refused are given back (try may go on)
        let before = self.m.used();
        let r = self.action_with(b, &v);
        if r.is_err() {
            let now = self.m.used();
            self.m.release(now.saturating_sub(before));
        }
        r
    }

    #[inline(never)]
    fn action_with(&mut self, b: B, v: &[Value]) -> Res<()> {
        if self.actions.len() >= MAX_ACTIONS {
            return Err(limit_fault(format!("too many changes (at most {MAX_ACTIONS} per run)")));
        }
        let too_long = |p: &str| -> Fault { format!("{} would be too long", display(p)).into() };
        // Strings made by string() are counted there; they stay in the action to the end.
        let action = match b {
            B::Write => {
                let page = self.path(&v[0])?;
                self.w.check_write(&page)?;
                let text = self.string(&v[1])?;
                if text.len() > self.w.max_page() {
                    return Err(too_long(&page));
                }
                let old = self.planned_len(&page);
                self.grow(text.len() as i64 - old.unwrap_or(0) as i64, old.is_none() as i64)?;
                self.plan_page(&page, Some(&text))?;
                Action::Write { page, text }
            }
            B::Add => {
                let page = self.path(&v[0])?;
                self.w.check_write(&page)?;
                let line = self.string(&v[1])?;
                let old_len = self.planned_len(&page);
                let new_len = match old_len {
                    Some(n) if n > 0 => n + 1 + line.len(),
                    _ => line.len(),
                };
                if new_len > self.w.max_page() {
                    return Err(too_long(&page));
                }
                self.grow(new_len as i64 - old_len.unwrap_or(0) as i64, old_len.is_none() as i64)?;
                // the new text of the page, counted before it is made
                let old = Charge::new(&self.m, old_len.unwrap_or(0) + OVERHEAD)?;
                let old_text = match self.planned(&page) {
                    Some(t) => t.cloned().unwrap_or_default(),
                    None => self.w.page(&page).unwrap_or_default(),
                };
                let mut new = String::new();
                let _room = Charge::new(&self.m, new_len + OVERHEAD)?;
                if new.try_reserve_exact(new_len).is_err() {
                    return Err(limit_fault("the computer has no more memory for this program".into()));
                }
                if !old_text.is_empty() {
                    new.push_str(&old_text);
                    new.push('\n');
                }
                new.push_str(&line);
                drop(old_text);
                drop(old);
                self.plan_page(&page, Some(&new))?;
                Action::Add { page, line }
            }
            B::Toss => {
                let page = self.path(&v[0])?;
                match self.planned(&page) {
                    Some(None) => return Err(format!("there is no page called {}", display(&page)).into()),
                    Some(Some(_)) => self.w.check_write(&page)?,
                    None => self.w.check_toss(&page)?,
                }
                let old = self.planned_len(&page).unwrap_or(0) as i64;
                self.grow(-old, -1)?;
                self.plan_page(&page, None)?;
                Action::Toss { page }
            }
            B::Send => {
                let (to, subject, text) = (self.string(&v[0])?, self.string(&v[1])?, self.string(&v[2])?);
                self.w.check_send(&to, &subject, &text)?;
                // the mail page: its header lines (25 bytes) and texts, as Mail writes it
                self.grow(MAIL_COST + (to.len() + subject.len() + text.len()) as i64, 1)?;
                Action::Send { to, subject, text }
            }
            B::Event => {
                let (date, time) = (self.string(&v[0])?, self.string(&v[1])?);
                let (title, with) = (self.string(&v[2])?, self.string(&v[3])?);
                self.w.check_event(&date, &time, &title, &with)?;
                // the event page: its field names (28 bytes) and texts, as Calendar writes it
                self.grow(MAIL_COST + (date.len() + time.len() + title.len() + with.len()) as i64, 1)?;
                Action::Event { date, time, title, with }
            }
            _ => return Err("only actions can stand alone on a line".into()),
        };
        self.tick(1)?;
        self.m.charge(ACTION_COST)?;
        if self.actions.try_reserve(1).is_err() {
            return Err(limit_fault("the computer has no more memory for this program".into()));
        }
        self.actions.push(action);
        Ok(())
    }
}

// ------------------------------------------------------------------ helpers --

/// What a planned change counts (as action() counted it), given back when `try` undoes it.
fn action_cost(a: &Action) -> usize {
    ACTION_COST
        + match a {
            Action::Write { text, .. } => text.len() + OVERHEAD,
            Action::Add { line, .. } => line.len() + OVERHEAD,
            Action::Toss { .. } => 0,
            Action::Send { to, subject, text } => to.len() + subject.len() + text.len() + 3 * OVERHEAD,
            Action::Event { date, time, title, with } => date.len() + time.len() + title.len() + with.len() + 4 * OVERHEAD,
        }
}

#[inline(never)]
fn unary(e: &Expr, v: Value) -> Res<Value> {
    match (e, v) {
        (Expr::Neg(_), Value::Num(x)) => Ok(Value::Num(-x)),
        (Expr::Neg(_), v) => Err(format!("- needs a number, got {}", v.describe()).into()),
        (_, Value::Bool(b)) => Ok(Value::Bool(!b)),
        (_, v) => Err(format!("not needs yes or no, got {}", v.describe()).into()),
    }
}

/// One side of `and` / `or`: Some(answer) if it decides the answer, None to look at the next.
#[inline(never)]
fn logic(is_and: bool, v: Value) -> Res<Option<Value>> {
    let word = if is_and { "and" } else { "or" };
    match v {
        Value::Bool(a) if a != is_and => Ok(Some(Value::Bool(a))),
        Value::Bool(_) => Ok(None),
        _ => Err(format!("{word} needs yes or no on both sides").into()),
    }
}

#[inline(never)]
fn both_sides(is_and: bool) -> Fault {
    format!("{} needs yes or no on both sides", if is_and { "and" } else { "or" }).into()
}

#[inline(never)]
fn field_value(v: &Value, name: &str) -> Res<Value> {
    match v {
        Value::Rec(r) => match r.get(name) {
            Some(x) => Ok(x.clone()),
            None => Err(missing_field(r, name)),
        },
        _ => Err(format!(".{name} needs a record, got {}", v.describe()).into()),
    }
}

fn missing_field(r: &RecBox, name: &str) -> Fault {
    let mut have = String::new();
    for (i, (k, _)) in r.f.iter().take(8).enumerate() {
        if i > 0 {
            have.push_str(", ");
        }
        have.push_str(k);
    }
    if r.f.is_empty() {
        format!("the record has no field {name} (it has no fields)").into()
    } else {
        format!("the record has no field {name} (it has: {have})").into()
    }
}

fn push_into(cur: &mut Value, v: Value, what: &str, m: &Rc<Meter>) -> Res<()> {
    match cur {
        Value::List(l) => list_mut(l, m)?.push(v),
        other => Err(format!("add ... to {what} needs a list there, got {}", other.describe()).into()),
    }
}

/// Make the change at the end of `parts`. Nothing changes if any part fails: every check comes
/// before the change, and copying a shared value doesn't change what it means.
fn change_at(cur: &mut Value, parts: &[PartV], change: Change, what: &str, m: &Rc<Meter>) -> Res<()> {
    let Some((first, rest)) = parts.split_first() else {
        return match change {
            Change::Assign(v) => {
                *cur = v;
                Ok(())
            }
            Change::Push(v) => push_into(cur, v, what, m),
        };
    };
    match first {
        PartV::Field(f) => {
            let Value::Rec(rc) = cur else {
                return Err(format!(".{f} needs a record, got {}", cur.describe()).into());
            };
            if rest.is_empty() {
                if let Change::Assign(v) = change {
                    return rec_mut(rc, m)?.set(f, v);
                }
            }
            if rc.get(f).is_none() {
                return Err(missing_field(rc, f));
            }
            let r = rec_mut(rc, m)?;
            let slot = r.get_mut(f).expect("checked");
            let res = change_at(slot, rest, change, what, m);
            r.fix_depth();
            res
        }
        PartV::At(n) => {
            let Value::List(rc) = cur else {
                return Err(format!("at {n} needs a list, got {}", cur.describe()).into());
            };
            if *n > rc.v.len() {
                return Err(format!("item {n} doesn't exist: the list has {} items (the first is 1)", rc.v.len()).into());
            }
            let l = list_mut(rc, m)?;
            let res = change_at(&mut l.v[*n - 1], rest, change, what, m);
            l.fix_depth();
            res
        }
    }
}

/// Are two values the same? (Different kinds inside lists and records: not the same.)
/// `work` counts the work done; None as soon as it goes over `budget`, so a huge comparison
/// stops early instead of running for a long time. Depth is bounded by MAX_VALUE_DEPTH.
fn same(a: &Value, b: &Value, work: &mut usize, budget: usize) -> Option<bool> {
    let over = |w: &usize| *w > budget;
    Some(match (a, b) {
        (Value::Num(x), Value::Num(y)) => x == y,
        (Value::Text(x), Value::Text(y)) => {
            *work += x.s.len().min(y.s.len()) / COMPARE_WORK;
            if over(work) {
                return None;
            }
            x.s == y.s
        }
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Nothing, Value::Nothing) => true,
        (Value::List(x), Value::List(y)) => {
            if Rc::ptr_eq(x, y) {
                return Some(true);
            }
            *work += x.v.len().min(y.v.len());
            if over(work) {
                return None;
            }
            if x.v.len() != y.v.len() {
                return Some(false);
            }
            for (p, q) in x.v.iter().zip(y.v.iter()) {
                if !same(p, q, work, budget)? {
                    return Some(false);
                }
            }
            true
        }
        (Value::Rec(x), Value::Rec(y)) => {
            if Rc::ptr_eq(x, y) {
                return Some(true);
            }
            if x.f.len() != y.f.len() {
                return Some(false);
            }
            for (k, v) in x.f.iter() {
                *work += y.f.len(); // finding the field
                if over(work) {
                    return None;
                }
                match y.get(k) {
                    Some(w) if same(v, w, work, budget)? => {}
                    _ => return Some(false),
                }
            }
            true
        }
        _ => false,
    })
}

/// The length of a value as text, or a mistake if it would be longer than `max`.
pub(crate) fn text_len(v: &Value, max: usize) -> Res<usize> {
    let too_long = || -> Fault { format!("that text would be too long (at most {MAX_TEXT} characters)").into() };
    let n = match v {
        Value::Num(x) => fmt_num(*x).len(),
        Value::Text(t) => t.s.len(),
        Value::Bool(b) => if *b { 3 } else { 2 },
        Value::Nothing => 7,
        Value::List(l) => {
            let mut n = 0usize;
            for (i, x) in l.v.iter().enumerate() {
                n += text_len(x, max)? + if i > 0 { 2 } else { 0 };
                if n > max {
                    return Err(too_long());
                }
            }
            n
        }
        Value::Rec(r) => {
            let mut n = 2usize;
            for (i, (k, x)) in r.f.iter().enumerate() {
                n += k.len() + 2 + text_len(x, max)? + if i > 0 { 2 } else { 0 };
                if n > max {
                    return Err(too_long());
                }
            }
            n
        }
    };
    if n > max {
        return Err(too_long());
    }
    Ok(n)
}

/// Write a value as text: lists as items joined by ", " (as chores), records as
/// {name: Mira, age: 3}, nothing as "nothing".
pub(crate) fn write_text(v: &Value, s: &mut String) {
    match v {
        Value::Num(x) => s.push_str(&fmt_num(*x)),
        Value::Text(t) => s.push_str(&t.s),
        Value::Bool(b) => s.push_str(if *b { "yes" } else { "no" }),
        Value::Nothing => s.push_str("nothing"),
        Value::List(l) => {
            for (i, x) in l.v.iter().enumerate() {
                if i > 0 {
                    s.push_str(", ");
                }
                write_text(x, s);
            }
        }
        Value::Rec(r) => {
            s.push('{');
            for (i, (k, x)) in r.f.iter().enumerate() {
                if i > 0 {
                    s.push_str(", ");
                }
                s.push_str(k);
                s.push_str(": ");
                write_text(x, s);
            }
            s.push('}');
        }
    }
}
