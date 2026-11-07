//! Parser: a page into lines of instructions (design §4 "Lines", "Instructions").
//!
//! A line is read in this order: (1) one complete call on its own is a call line; (2) the
//! first word (or two) picks the instruction, and the rest must match its pattern; (3) anything
//! else is a mistake found before anything runs.

use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use crate::lex::{is_keyword, lex, show_tok, Tok};
use crate::limits::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum B {
    // chores
    Pages, Read, Field, Body, Exists, Lines, Words, Count, Item, Number, Lower, Today, Weekday, AddDays,
    // new in Unseen Script
    Nothing, Upper, Trim, Split, Join, Replace, Find, Slice, Starts, Ends, Newline, Text, Pad,
    Round, Floor, Ceil, Abs, Min, Max, IsNumber, Sum, Sort, Reverse, Unique, First, Last, Range,
    Position, Without, Keys, HasKey, Fields, DaysBetween, Month, Year,
    // only in tests
    PageNow, After, Sent, Events, Said,
    // actions
    Write, Add, Toss, Send, Event,
}

/// (name, built-in, number of values, is an action)
const BUILTINS: [(&str, B, usize, bool); 59] = [
    ("pages", B::Pages, 1, false),
    ("read", B::Read, 1, false),
    ("field", B::Field, 2, false),
    ("body", B::Body, 1, false),
    ("exists", B::Exists, 1, false),
    ("lines", B::Lines, 1, false),
    ("words", B::Words, 1, false),
    ("count", B::Count, 1, false),
    ("item", B::Item, 2, false),
    ("number", B::Number, 1, false),
    ("lower", B::Lower, 1, false),
    ("today", B::Today, 0, false),
    ("weekday", B::Weekday, 1, false),
    ("add_days", B::AddDays, 2, false),
    ("nothing", B::Nothing, 0, false),
    ("upper", B::Upper, 1, false),
    ("trim", B::Trim, 1, false),
    ("split", B::Split, 2, false),
    ("join", B::Join, 2, false),
    ("replace", B::Replace, 3, false),
    ("find", B::Find, 2, false),
    ("slice", B::Slice, 3, false),
    ("starts", B::Starts, 2, false),
    ("ends", B::Ends, 2, false),
    ("newline", B::Newline, 0, false),
    ("text", B::Text, 1, false),
    ("pad", B::Pad, 2, false),
    ("round", B::Round, 2, false),
    ("floor", B::Floor, 1, false),
    ("ceil", B::Ceil, 1, false),
    ("abs", B::Abs, 1, false),
    ("min", B::Min, 2, false),
    ("max", B::Max, 2, false),
    ("is_number", B::IsNumber, 1, false),
    ("sum", B::Sum, 1, false),
    ("sort", B::Sort, 1, false),
    ("reverse", B::Reverse, 1, false),
    ("unique", B::Unique, 1, false),
    ("first", B::First, 1, false),
    ("last", B::Last, 1, false),
    ("range", B::Range, 2, false),
    ("position", B::Position, 2, false),
    ("without", B::Without, 2, false),
    ("keys", B::Keys, 1, false),
    ("has_key", B::HasKey, 2, false),
    ("fields", B::Fields, 1, false),
    ("days_between", B::DaysBetween, 2, false),
    ("month", B::Month, 1, false),
    ("year", B::Year, 1, false),
    ("page_now", B::PageNow, 1, false),
    ("after", B::After, 1, false),
    ("sent", B::Sent, 0, false),
    ("events", B::Events, 0, false),
    ("said", B::Said, 0, false),
    ("write", B::Write, 2, true),
    ("add", B::Add, 2, true),
    ("toss", B::Toss, 1, true),
    ("send", B::Send, 3, true),
    ("event", B::Event, 4, true),
];

/// Built-ins that look at what a test did: only in test blocks.
pub fn is_test_only(b: B) -> bool {
    matches!(b, B::PageNow | B::After | B::Sent | B::Events | B::Said)
}

/// Built-ins that never look at the world (they may be used in fixed values).
pub fn is_pure(b: B) -> bool {
    !matches!(b, B::Pages | B::Read | B::Field | B::Body | B::Exists | B::Fields | B::Today)
        && !is_test_only(b)
        && !builtin(builtin_name(b)).map_or(true, |x| x.2)
}

/// Words that start an instruction (now or in later milestones). A task can't have one of
/// these names: `say (1)` must always be the say instruction.
pub const INSTRUCTION_WORDS: [&str; 26] = [
    "set", "add", "say", "stop", "when", "or", "otherwise", "done", "for", "repeat", "while", "skip", "quit",
    "give", "task", "fixed", "use", "app", "fail", "try", "instead", "test", "check", "pretend", "launch", "press",
];

pub fn is_instruction_word(w: &str) -> bool {
    INSTRUCTION_WORDS.contains(&w)
}

pub fn builtin(name: &str) -> Option<(B, usize, bool)> {
    BUILTINS.iter().find(|x| x.0 == name).map(|x| (x.1, x.2, x.3))
}

pub fn builtin_name(b: B) -> &'static str {
    BUILTINS.iter().find(|x| x.1 == b).map_or("?", |x| x.0)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Op {
    Mul,
    Div,
    Add,
    Sub,
    Join,
    Is,
    IsNot,
    Lt,
    Gt,
    Le,
    Ge,
    Has,
    And,
    Or,
}

#[derive(Debug)]
pub enum Expr {
    Num(f64),
    Text(String),
    Bool(bool),
    Var(String),
    Neg(Box<Expr>),
    Not(Box<Expr>),
    Bin(Op, Box<Expr>, Box<Expr>),
    Call(B, Vec<Expr>),
    List(Vec<Expr>),
    Record(Vec<(String, Expr)>),
    Field(Box<Expr>, String),
    /// A call to a task (checked before running: it must exist and get the right inputs).
    Task(String, Vec<Expr>),
}

fn token_cost(t: &Tok) -> usize {
    TOKEN_COST
        + 2 * match t {
            Tok::Text(s) | Tok::Word(s) => s.len(),
            _ => 0,
        }
}

/// Freed without recursion: a long rejected line like `1+1+1+...` builds a deep tree before it
/// is refused, and freeing it recursively could overflow the small kernel stack.
impl Drop for Expr {
    fn drop(&mut self) {
        let mut stack: Vec<Expr> = Vec::new();
        take_children(self, &mut stack);
        while let Some(mut e) = stack.pop() {
            take_children(&mut e, &mut stack);
        } // each `e` is dropped here, with no children left
    }
}

fn take_children(e: &mut Expr, stack: &mut Vec<Expr>) {
    let leaf = || Expr::Bool(false);
    match e {
        Expr::Neg(x) | Expr::Not(x) | Expr::Field(x, _) => stack.push(core::mem::replace(&mut **x, leaf())),
        Expr::Bin(_, l, r) => {
            stack.push(core::mem::replace(&mut **l, leaf()));
            stack.push(core::mem::replace(&mut **r, leaf()));
        }
        Expr::Call(_, args) | Expr::List(args) | Expr::Task(_, args) => stack.extend(args.drain(..)),
        Expr::Record(fields) => stack.extend(fields.drain(..).map(|(_, x)| x)),
        _ => {}
    }
}

#[derive(Debug)]
pub enum Part {
    Field(String),
    At(Expr),
}

/// `s`, `s.items`, `s.items at 2`: a name, any number of fields, at most one `at` at the end.
#[derive(Debug)]
pub struct Target {
    pub name: String,
    pub parts: Vec<Part>,
}

#[derive(Debug)]
pub enum Stmt {
    Set(Target, Expr),
    AddTo(Expr, Target),
    Say(Expr),
    Stop,
    Skip,
    Quit,
    /// `when` with its `or when` branches, then the `otherwise` lines.
    When(Vec<(Expr, Vec<Line>)>, Vec<Line>),
    ForEach(String, Expr, Vec<Line>),
    Repeat(Expr, Vec<Line>),
    While(Expr, Vec<Line>),
    Do(B, Vec<Expr>),
    /// A task called on a line of its own (what it gives is thrown away).
    Call(Expr),
    Give(Expr),
    Fail(Expr),
    /// `try` lines, the name after `instead`, the `instead` lines.
    Try(Vec<Line>, String, Vec<Line>),
    /// Only in tests.
    Check(Expr),
    /// Only at the top of a test: a page of the test world, or its date.
    Pretend(Pretend),
}

#[derive(Debug)]
pub enum Pretend {
    Page(Expr, Expr),
    Today(Expr),
}

#[derive(Debug)]
pub struct Line {
    pub n: usize,
    pub stmt: Stmt,
}

#[derive(Debug)]
pub struct Task {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Line>,
    pub line: usize,
    /// The page it is on (an index into Program::pages).
    pub page: usize,
}

#[derive(Debug)]
pub struct Fixed {
    pub name: String,
    pub value: Expr,
    pub line: usize,
    /// The page it is on (an index into Program::pages).
    pub page: usize,
}

#[derive(Debug)]
pub struct Test {
    pub title: String,
    pub body: Vec<Line>,
    pub line: usize,
    pub page: usize,
}

/// A program: its main page's lines, and the tasks, fixed values and tests of all its pages.
#[derive(Debug, Default)]
pub struct Program {
    /// The pages, as internal paths; the main page is 0.
    pub pages: Vec<String>,
    pub main: Vec<Line>,
    pub tasks: Vec<Task>,
    pub fixed: Vec<Fixed>,
    pub tests: Vec<Test>,
    /// The `use` lines of the page just parsed: (page path as written, line).
    pub uses: Vec<(String, usize)>,
    /// What checking the pages cost (counted against the program budget).
    pub cost: usize,
}

/// A mistake found before anything runs: (line, message). Line 0: the whole page.
pub type PErr = (usize, String);

pub fn parse(source: &str) -> Result<Program, PErr> {
    if source.len() > MAX_PAGE_CHARS {
        return Err((0, format!("this page is too long (at most {MAX_PAGE_CHARS} characters)")));
    }
    let raw: Vec<&str> = source.split('\n').collect();
    if raw.len() > MAX_PAGE_LINES {
        return Err((0, format!("this page has too many lines (at most {MAX_PAGE_LINES})")));
    }
    // Checking a page counts against the program budget: every word becomes a token and at
    // most one node of the parse tree (with a copy of its text).
    let mut used = 0usize;
    let mut lines = Vec::new();
    for (i, l) in raw.iter().enumerate() {
        let toks = lex(l.trim_end_matches('\r')).map_err(|e| (i + 1, e))?;
        used += core::mem::size_of::<Line>() + toks.iter().map(token_cost).sum::<usize>();
        if used > PROGRAM_MEMORY {
            return Err((i + 1, format!("this program is too big to check (at most {} KB)", PROGRAM_MEMORY / 1024)));
        }
        if !toks.is_empty() {
            lines.push((i + 1, toks));
        }
    }
    let mut pos = 0;
    let mut cx = Cx {
        lines: &lines,
        loops: 0,
        in_task: false,
        test_depth: None,
        top_started: false,
        tasks: Vec::new(),
        fixed: Vec::new(),
        tests: Vec::new(),
        uses: Vec::new(),
    };
    let (body, end) = cx.block(&mut pos, 0)?;
    match end {
        End::Eof => Ok(Program {
            pages: Vec::new(),
            main: body,
            tasks: cx.tasks,
            fixed: cx.fixed,
            tests: cx.tests,
            uses: cx.uses,
            cost: used,
        }),
        End::Instead(n, _) => Err((n, "instead, but there is no try before it".into())),
        End::Done(n) => Err((n, "done, but there is no block to end".into())),
        End::Otherwise(n) => Err((n, "otherwise, but there is no when before it".into())),
        End::OrWhen(n, _) => Err((n, "or when, but there is no when before it".into())),
    }
}

enum End {
    Eof,
    Done(usize),
    Otherwise(usize),
    OrWhen(usize, Expr),
    Instead(usize, String),
}

struct Cx<'a> {
    lines: &'a [(usize, Vec<Tok>)],
    /// How many loops the current line is inside (for skip and quit).
    loops: usize,
    /// Inside a task (for give).
    in_task: bool,
    /// Inside a test: the depth of its lines (pretend only there, before anything else).
    test_depth: Option<usize>,
    /// Something other than `use` was seen at the top of the page (use must come first).
    top_started: bool,
    tasks: Vec<Task>,
    fixed: Vec<Fixed>,
    tests: Vec<Test>,
    uses: Vec<(String, usize)>,
}

fn start_error() -> String {
    "a line must start with set, add, say, when, for each, repeat, while, skip, quit, stop, give, task, fixed, \
     or a call like write(...)"
        .into()
}

impl<'a> Cx<'a> {
    fn nested(&mut self, pos: &mut usize, depth: usize, n: usize) -> Result<(Vec<Line>, End), PErr> {
        if depth + 1 > MAX_NESTING {
            return Err((n, format!("blocks are nested too deep (at most {MAX_NESTING})")));
        }
        self.block(pos, depth + 1)
    }

    fn loop_body(&mut self, pos: &mut usize, depth: usize, n: usize, what: &str) -> Result<Vec<Line>, PErr> {
        self.loops += 1;
        let r = self.nested(pos, depth, n);
        self.loops -= 1;
        until_done(r?, n, what)
    }

    fn block(&mut self, pos: &mut usize, depth: usize) -> Result<(Vec<Line>, End), PErr> {
        let mut out = Vec::new();
        while *pos < self.lines.len() {
            let (n, toks) = &self.lines[*pos];
            let n = *n;
            *pos += 1;
            let at = |e: String| (n, e);
            let first = match &toks[0] {
                Tok::Word(w) => w.as_str(),
                _ => return Err(at(start_error())),
            };
            let second = match toks.get(1) {
                Some(Tok::Word(w)) => Some(w.as_str()),
                _ => None,
            };
            if depth == 0 && first != "use" {
                self.top_started = true;
            }
            // (1) a call on its own line (an instruction word always starts its instruction:
            // `say (1 + 2)` and `give (x)` are not calls)
            if toks.get(1) == Some(&Tok::Sym("(")) && (builtin(first).is_some() || !is_instruction_word(first)) {
                let mut p = P::new(toks, 0);
                match p.call_line() {
                    Ok(Expr::Call(b, ref mut args)) if p.end().is_ok() => {
                        let (b, args) = (b, core::mem::take(args));
                        match builtin(first) {
                            Some((_, _, true)) => {
                                out.push(Line { n, stmt: Stmt::Do(b, args) });
                                continue;
                            }
                            _ => return Err(at(format!("{first}(...) gives a value; use it in set, say or when"))),
                        }
                    }
                    Ok(e @ Expr::Task(..)) if p.end().is_ok() => {
                        out.push(Line { n, stmt: Stmt::Call(e) });
                        continue;
                    }
                    _ => {}
                }
            }
            // (2) the first word picks the instruction
            let stmt = match (first, second) {
                ("use", _) => {
                    if depth > 0 || self.in_task || self.test_depth.is_some() || self.top_started {
                        return Err(at("use must come at the top of the page, before anything else".into()));
                    }
                    match (toks.get(1), toks.len()) {
                        (Some(Tok::Text(p)), 2) => self.uses.push((p.clone(), n)),
                        _ => return Err(at("use needs: use 'page' (the page, in quotes)".into())),
                    }
                    continue;
                }
                ("instead", _) => {
                    let mut p = P::new(toks, 1);
                    let name = p.name("instead").map_err(at)?;
                    p.end().map_err(at)?;
                    return Ok((out, End::Instead(n, name)));
                }
                ("try", _) => {
                    if toks.len() > 1 {
                        return Err(at("try must be alone on its line".into()));
                    }
                    let (body, end) = self.nested(pos, depth, n)?;
                    let name = match end {
                        End::Instead(_, name) => name,
                        End::Eof | End::Done(_) => return Err(at("this try has no instead (try ... instead problem ... done)".into())),
                        End::Otherwise(m) => return Err((m, "otherwise only belongs to when, not try".into())),
                        End::OrWhen(m, _) => return Err((m, "or when only belongs to when, not try".into())),
                    };
                    let handler = until_done(self.nested(pos, depth, n)?, n, "try")?;
                    Stmt::Try(body, name, handler)
                }
                ("fail", _) => {
                    let mut p = P::new(toks, 1);
                    let e = p.expr().map_err(at)?;
                    p.end().map_err(at)?;
                    Stmt::Fail(e)
                }
                ("test", _) => {
                    if depth > 0 || self.in_task || self.test_depth.is_some() {
                        return Err(at("a test can only be made at the top of a page, not inside a block, task or test".into()));
                    }
                    let title = match (toks.get(1), toks.len()) {
                        (Some(Tok::Text(t)), 2) => t.clone(),
                        _ => return Err(at("test needs: test 'what it checks' (a title in quotes)".into())),
                    };
                    self.test_depth = Some(depth + 1);
                    let r = self.nested(pos, depth, n);
                    self.test_depth = None;
                    let body = until_done(r?, n, "test")?;
                    // pretend lines only at the start of the test
                    let mut started = false;
                    for l in &body {
                        match l.stmt {
                            Stmt::Pretend(_) if started => {
                                return Err((l.n, "pretend must come at the top of the test, before its other lines".into()))
                            }
                            Stmt::Pretend(_) => {}
                            _ => started = true,
                        }
                    }
                    self.tests.push(Test { title, body, line: n, page: 0 });
                    continue;
                }
                ("check", _) => {
                    if self.test_depth.is_none() {
                        return Err(at("check only works inside a test".into()));
                    }
                    let mut p = P::new(toks, 1);
                    let e = p.expr().map_err(at)?;
                    p.end().map_err(at)?;
                    Stmt::Check(e)
                }
                ("pretend", _) => {
                    if self.test_depth != Some(depth) {
                        return Err(at("pretend only works at the top of a test".into()));
                    }
                    let mut p = P::new(toks, 1);
                    if second == Some("today") && matches!(toks.get(2), Some(Tok::Word(w)) if w == "is") {
                        p.i = 3;
                        let e = p.expr().map_err(at)?;
                        p.end().map_err(at)?;
                        Stmt::Pretend(Pretend::Today(e))
                    } else {
                        let path = p.expr().map_err(at)?;
                        p.word("holds", "pretend needs: pretend 'page' holds text (or: pretend today is 'date')").map_err(at)?;
                        let text = p.expr().map_err(at)?;
                        p.end().map_err(at)?;
                        Stmt::Pretend(Pretend::Page(path, text))
                    }
                }
                ("done" | "otherwise", _) => {
                    if toks.len() > 1 {
                        return Err(at(format!("{first} must be alone on its line")));
                    }
                    let end = if first == "done" { End::Done(n) } else { End::Otherwise(n) };
                    return Ok((out, end));
                }
                ("or", Some("when")) => {
                    let mut p = P::new(toks, 2);
                    let cond = p.expr().map_err(at)?;
                    p.end().map_err(at)?;
                    return Ok((out, End::OrWhen(n, cond)));
                }
                ("set", _) => {
                    let mut p = P::new(toks, 1);
                    let t = p.target("set").map_err(at)?;
                    p.word("to", "set needs: set name to value").map_err(at)?;
                    let e = p.expr().map_err(at)?;
                    p.end().map_err(at)?;
                    Stmt::Set(t, e)
                }
                ("add", _) => {
                    let mut p = P::new(toks, 1);
                    let e = p.expr().map_err(at)?;
                    p.word("to", "add needs: add value to list (or the action add(page, text))").map_err(at)?;
                    let t = p.target("add ... to").map_err(at)?;
                    p.end().map_err(at)?;
                    Stmt::AddTo(e, t)
                }
                ("say", _) => {
                    let mut p = P::new(toks, 1);
                    let e = p.expr().map_err(at)?;
                    p.end().map_err(at)?;
                    Stmt::Say(e)
                }
                ("stop" | "skip" | "quit", _) => {
                    if toks.len() > 1 {
                        return Err(at(format!("{first} must be alone on its line")));
                    }
                    match first {
                        "stop" => Stmt::Stop,
                        _ if self.loops == 0 => {
                            return Err(at(format!("{first} only works inside a loop (for each, repeat or while)")))
                        }
                        "skip" => Stmt::Skip,
                        _ => Stmt::Quit,
                    }
                }
                ("when", _) => {
                    let mut p = P::new(toks, 1);
                    let cond = p.expr().map_err(at)?;
                    p.end().map_err(at)?;
                    let mut branches = Vec::new();
                    let mut cond = cond;
                    let otherwise = loop {
                        let (body, end) = self.nested(pos, depth, n)?;
                        branches.push((cond, body));
                        match end {
                            End::Done(_) => break Vec::new(),
                            End::OrWhen(_, c) => cond = c,
                            End::Otherwise(_) => match self.nested(pos, depth, n)? {
                                (no, End::Done(_)) => break no,
                                (_, End::Otherwise(m)) => return Err((m, "this when already has an otherwise".into())),
                                (_, End::OrWhen(m, _)) => {
                                    return Err((m, "or when must come before otherwise".into()))
                                }
                                (_, End::Eof) => return Err(at("this when has no done".into())),
                                (_, End::Instead(m, _)) => return Err((m, "instead only belongs to try, not when".into())),
                            },
                            End::Eof => return Err(at("this when has no done".into())),
                            End::Instead(m, _) => return Err((m, "instead only belongs to try, not when".into())),
                        }
                    };
                    Stmt::When(branches, otherwise)
                }
                ("for", _) => {
                    let mut p = P::new(toks, 1);
                    p.word("each", "for needs: for each name in list").map_err(at)?;
                    let name = p.name("for each").map_err(at)?;
                    p.word("in", "for needs: for each name in list").map_err(at)?;
                    let list = p.expr().map_err(at)?;
                    p.end().map_err(at)?;
                    Stmt::ForEach(name, list, self.loop_body(pos, depth, n, "for each")?)
                }
                ("repeat", _) => {
                    let mut p = P::new(toks, 1);
                    let count = p.expr().map_err(at)?;
                    p.word("times", "repeat needs: repeat number times").map_err(at)?;
                    p.end().map_err(at)?;
                    Stmt::Repeat(count, self.loop_body(pos, depth, n, "repeat")?)
                }
                ("task", Some(_)) => {
                    if depth > 0 || self.in_task || self.test_depth.is_some() {
                        return Err(at("a task can only be made at the top of a page, not inside a block, task or test".into()));
                    }
                    let mut p = P::new(toks, 1);
                    let name = p.name("task").map_err(at)?;
                    if !p.peek_sym("(") {
                        return Err(at(format!("task needs: task {name}(input, ...)")));
                    }
                    p.i += 1;
                    let mut params: Vec<String> = Vec::new();
                    if !p.peek_sym(")") {
                        loop {
                            let x = p.name("task (").map_err(at)?;
                            if params.contains(&x) {
                                return Err(at(format!("the input {x} is in this task twice")));
                            }
                            params.push(x);
                            if p.peek_sym(",") {
                                p.i += 1;
                            } else {
                                break;
                            }
                        }
                    }
                    if !p.peek_sym(")") {
                        return Err(at(format!("task {name}( is not closed with )")));
                    }
                    p.i += 1;
                    p.end().map_err(at)?;
                    let (loops, in_task) = (self.loops, self.in_task);
                    self.loops = 0;
                    self.in_task = true;
                    let r = self.nested(pos, depth, n);
                    self.loops = loops;
                    self.in_task = in_task;
                    let body = until_done(r?, n, "task")?;
                    self.tasks.push(Task { name, params, body, line: n, page: 0 });
                    continue;
                }
                ("fixed", Some(_)) => {
                    if depth > 0 || self.in_task || self.test_depth.is_some() {
                        return Err(at("a fixed value can only be made at the top of a page, not inside a block, task or test".into()));
                    }
                    let mut p = P::new(toks, 1);
                    let name = p.name("fixed").map_err(at)?;
                    p.word("is", "fixed needs: fixed name is value").map_err(at)?;
                    let value = p.expr().map_err(at)?;
                    p.end().map_err(at)?;
                    self.fixed.push(Fixed { name, value, line: n, page: 0 });
                    continue;
                }
                ("give", _) => {
                    if !self.in_task {
                        return Err(at("give only works inside a task".into()));
                    }
                    let mut p = P::new(toks, 1);
                    let e = if toks.len() == 1 { Expr::Call(B::Nothing, Vec::new()) } else { p.expr().map_err(at)? };
                    p.end().map_err(at)?;
                    Stmt::Give(e)
                }
                ("while", _) => {
                    let mut p = P::new(toks, 1);
                    let cond = p.expr().map_err(at)?;
                    p.end().map_err(at)?;
                    Stmt::While(cond, self.loop_body(pos, depth, n, "while")?)
                }
                _ => {
                    if toks.get(1) == Some(&Tok::Sym("(")) {
                        // looks like a call but isn't one complete call: show why
                        let mut p = P::new(toks, 0);
                        p.call_line().map_err(at)?;
                        p.end().map_err(at)?;
                    }
                    return Err(at(start_error()));
                }
            };
            out.push(Line { n, stmt });
        }
        Ok((out, End::Eof))
    }
}

fn until_done(r: (Vec<Line>, End), n: usize, what: &str) -> Result<Vec<Line>, PErr> {
    match r {
        (body, End::Done(_)) => Ok(body),
        (_, End::Otherwise(m)) => Err((m, format!("otherwise only belongs to when, not {what}"))),
        (_, End::OrWhen(m, _)) => Err((m, format!("or when only belongs to when, not {what}"))),
        (_, End::Eof) => Err((n, format!("this {what} has no done"))),
        (_, End::Instead(m, _)) => Err((m, format!("instead only belongs to try, not {what}"))),
    }
}

/// Expression parser for one line.
struct P<'a> {
    t: &'a [Tok],
    i: usize,
    brackets: usize,
}

impl<'a> P<'a> {
    fn new(t: &'a [Tok], i: usize) -> Self {
        Self { t, i, brackets: 0 }
    }

    fn peek(&self) -> Option<&Tok> {
        self.t.get(self.i)
    }

    fn peek_word(&self, w: &str) -> bool {
        matches!(self.peek(), Some(Tok::Word(x)) if x == w)
    }

    fn peek_sym(&self, s: &str) -> bool {
        matches!(self.peek(), Some(Tok::Sym(x)) if *x == s)
    }

    fn word(&mut self, w: &str, help: &str) -> Result<(), String> {
        if self.peek_word(w) {
            self.i += 1;
            Ok(())
        } else {
            Err(help.into())
        }
    }

    fn name(&mut self, after: &str) -> Result<String, String> {
        match self.peek() {
            Some(Tok::Word(w)) if !is_keyword(w) => {
                let w = w.clone();
                self.i += 1;
                Ok(w)
            }
            Some(Tok::Word(w)) => Err(format!("'{w}' is a special word and can't be used as a name")),
            _ => Err(format!("{after} needs a name here")),
        }
    }

    /// A field name: any word, even a keyword (mail pages have a `to:` field).
    fn field_name(&mut self) -> Result<String, String> {
        match self.peek() {
            Some(Tok::Word(w)) => {
                let w = w.clone();
                self.i += 1;
                Ok(w)
            }
            _ => Err("a field name is missing (after . or in a record)".into()),
        }
    }

    fn target(&mut self, after: &str) -> Result<Target, String> {
        let name = self.name(after)?;
        let mut parts = Vec::new();
        loop {
            if self.peek_sym(".") {
                self.i += 1;
                parts.push(Part::Field(self.field_name()?));
            } else if self.peek_word("at") && !matches!(self.t.get(self.i + 1), Some(Tok::Word(w)) if w == "to") {
                self.i += 1;
                parts.push(Part::At(self.expr()?));
                if self.peek_sym(".") || self.peek_word("at") {
                    return Err("at N must be the last part of what you change (use a name in between)".into());
                }
                break;
            } else {
                break;
            }
            if parts.len() > MAX_VALUE_DEPTH {
                return Err("this goes too deep into a value".into());
            }
        }
        Ok(Target { name, parts })
    }

    fn end(&self) -> Result<(), String> {
        match self.peek() {
            None => Ok(()),
            Some(t) => Err(format!("I didn't expect {} here", show_tok(t))),
        }
    }

    fn deeper(&mut self) -> Result<(), String> {
        self.brackets += 1;
        if self.brackets > MAX_BRACKETS {
            Err("this line has too many brackets inside each other".into())
        } else {
            Ok(())
        }
    }

    fn expr(&mut self) -> Result<Expr, String> {
        let e = self.or()?;
        check_tree(&e)?;
        Ok(e)
    }

    fn call_line(&mut self) -> Result<Expr, String> {
        let e = self.primary(true)?;
        check_tree(&e)?;
        Ok(e)
    }

    fn or(&mut self) -> Result<Expr, String> {
        let mut l = self.and()?;
        while self.peek_word("or") {
            self.i += 1;
            l = Expr::Bin(Op::Or, Box::new(l), Box::new(self.and()?));
        }
        Ok(l)
    }

    fn and(&mut self) -> Result<Expr, String> {
        let mut l = self.not()?;
        while self.peek_word("and") {
            self.i += 1;
            l = Expr::Bin(Op::And, Box::new(l), Box::new(self.not()?));
        }
        Ok(l)
    }

    fn not(&mut self) -> Result<Expr, String> {
        if self.peek_word("not") {
            self.i += 1;
            self.deeper()?;
            let e = self.not()?;
            self.brackets -= 1;
            return Ok(Expr::Not(Box::new(e)));
        }
        self.cmp()
    }

    fn cmp(&mut self) -> Result<Expr, String> {
        let l = self.join()?;
        let op = match self.peek() {
            Some(Tok::Word(w)) if w == "is" => {
                self.i += 1;
                if self.peek_word("not") {
                    self.i += 1;
                    Op::IsNot
                } else {
                    Op::Is
                }
            }
            Some(Tok::Word(w)) if w == "has" => {
                self.i += 1;
                Op::Has
            }
            Some(Tok::Sym("<")) => {
                self.i += 1;
                Op::Lt
            }
            Some(Tok::Sym(">")) => {
                self.i += 1;
                Op::Gt
            }
            Some(Tok::Sym("<=")) => {
                self.i += 1;
                Op::Le
            }
            Some(Tok::Sym(">=")) => {
                self.i += 1;
                Op::Ge
            }
            _ => return Ok(l),
        };
        let r = self.join()?;
        if matches!(self.peek(), Some(Tok::Word(w)) if w == "is" || w == "has")
            || matches!(self.peek(), Some(Tok::Sym(s)) if ["<", ">", "<=", ">="].contains(s))
        {
            return Err("only one comparison at a time: use and / or to combine them".into());
        }
        Ok(Expr::Bin(op, Box::new(l), Box::new(r)))
    }

    fn join(&mut self) -> Result<Expr, String> {
        let mut l = self.sum()?;
        while self.peek_sym("&") {
            self.i += 1;
            l = Expr::Bin(Op::Join, Box::new(l), Box::new(self.sum()?));
        }
        Ok(l)
    }

    fn sum(&mut self) -> Result<Expr, String> {
        let mut l = self.product()?;
        loop {
            let op = if self.peek_sym("+") {
                Op::Add
            } else if self.peek_sym("-") {
                Op::Sub
            } else {
                return Ok(l);
            };
            self.i += 1;
            l = Expr::Bin(op, Box::new(l), Box::new(self.product()?));
        }
    }

    fn product(&mut self) -> Result<Expr, String> {
        let mut l = self.unary()?;
        loop {
            let op = if self.peek_sym("*") {
                Op::Mul
            } else if self.peek_sym("/") {
                Op::Div
            } else {
                return Ok(l);
            };
            self.i += 1;
            l = Expr::Bin(op, Box::new(l), Box::new(self.unary()?));
        }
    }

    fn unary(&mut self) -> Result<Expr, String> {
        if self.peek_sym("-") {
            self.i += 1;
            self.deeper()?;
            let e = self.unary()?;
            self.brackets -= 1;
            return Ok(Expr::Neg(Box::new(e)));
        }
        self.postfix()
    }

    /// Level 2: `.field` after a level-1 form, any number of times.
    fn postfix(&mut self) -> Result<Expr, String> {
        let mut e = self.primary(false)?;
        while self.peek_sym(".") {
            self.i += 1;
            let f = self.field_name()?;
            e = Expr::Field(Box::new(e), f);
        }
        Ok(e)
    }

    /// The items of `( ... )`, `[ ... ]`: values split by commas, up to `close`.
    fn items(&mut self, close: &'static str, what: &str) -> Result<Vec<Expr>, String> {
        let mut args = Vec::new();
        if !self.peek_sym(close) {
            loop {
                args.push(self.or()?);
                if self.peek_sym(",") {
                    self.i += 1;
                } else {
                    break;
                }
            }
        }
        if !self.peek_sym(close) {
            return Err(format!("{what} is not closed with {close}"));
        }
        self.i += 1;
        Ok(args)
    }

    fn primary(&mut self, action_ok: bool) -> Result<Expr, String> {
        let tok = self.peek().cloned().ok_or_else(|| String::from("a value is missing at the end of the line"))?;
        self.i += 1;
        match tok {
            Tok::Num(v) => Ok(Expr::Num(v)),
            Tok::Text(s) => Ok(Expr::Text(s)),
            Tok::Sym("(") => {
                self.deeper()?;
                let e = self.or()?;
                self.brackets -= 1;
                if !self.peek_sym(")") {
                    return Err("a bracket ( is not closed".into());
                }
                self.i += 1;
                Ok(e)
            }
            Tok::Sym("[") => {
                self.deeper()?;
                let items = self.items("]", "a list [")?;
                self.brackets -= 1;
                if items.len() > MAX_LIST {
                    return Err(format!("a list can have at most {MAX_LIST} items"));
                }
                Ok(Expr::List(items))
            }
            Tok::Sym("{") => {
                self.deeper()?;
                let mut fields: Vec<(String, Expr)> = Vec::new();
                if !self.peek_sym("}") {
                    loop {
                        let f = self.field_name()?;
                        if !self.peek_sym(":") {
                            return Err(format!("a record needs: {{{f}: value, ...}}"));
                        }
                        self.i += 1;
                        if fields.iter().any(|(g, _)| *g == f) {
                            return Err(format!("the field {f} is in this record twice"));
                        }
                        fields.push((f, self.or()?));
                        if self.peek_sym(",") {
                            self.i += 1;
                        } else {
                            break;
                        }
                    }
                }
                if !self.peek_sym("}") {
                    return Err("a record { is not closed with }".into());
                }
                self.i += 1;
                self.brackets -= 1;
                Ok(Expr::Record(fields))
            }
            Tok::Word(w) if w == "yes" => Ok(Expr::Bool(true)),
            Tok::Word(w) if w == "no" => Ok(Expr::Bool(false)),
            Tok::Word(w) if self.peek_sym("(") && builtin(&w).is_none() => {
                if is_keyword(&w) {
                    return Err(format!("I didn't expect the word {w} here"));
                }
                self.i += 1;
                self.deeper()?;
                let args = self.items(")", &format!("{w}("))?;
                self.brackets -= 1;
                Ok(Expr::Task(w, args))
            }
            Tok::Word(w) if self.peek_sym("(") => {
                let (b, argc, action) = builtin(&w).expect("checked");
                if action && !action_ok {
                    return Err(format!("{w}(...) is an action: it must be on a line of its own"));
                }
                self.i += 1;
                self.deeper()?;
                let args = self.items(")", &format!("{w}("))?;
                self.brackets -= 1;
                if args.len() != argc {
                    let s = if argc == 1 { "" } else { "s" };
                    return Err(format!("{w}(...) needs {argc} value{s} in its brackets, got {}", args.len()));
                }
                Ok(Expr::Call(b, args))
            }
            Tok::Word(w) if is_keyword(&w) => Err(format!("I didn't expect the word {w} here")),
            Tok::Word(w) => Ok(Expr::Var(w)),
            t => Err(format!("I didn't expect {} here", show_tok(&t))),
        }
    }
}

/// Refuse expression trees deeper than MAX_TREE (evaluation is recursive). Iterative, so
/// checking a huge tree can't use up the stack either.
fn check_tree(e: &Expr) -> Result<(), String> {
    let mut stack = Vec::new();
    stack.push((e, 1usize));
    while let Some((e, d)) = stack.pop() {
        if d > MAX_TREE {
            return Err("this line is too complicated: split it with set".into());
        }
        match e {
            Expr::Neg(x) | Expr::Not(x) | Expr::Field(x, _) => stack.push((x, d + 1)),
            Expr::Bin(_, l, r) => {
                stack.push((l, d + 1));
                stack.push((r, d + 1));
            }
            Expr::Call(_, args) | Expr::List(args) | Expr::Task(_, args) => {
                args.iter().for_each(|a| stack.push((a, d + 1)))
            }
            Expr::Record(fields) => fields.iter().for_each(|(_, a)| stack.push((a, d + 1))),
            _ => {}
        }
    }
    Ok(())
}
