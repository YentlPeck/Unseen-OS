//! Chores: the tiny scripting language of Unseen OS (contract: docs/v3-design.md).
//!
//! A chore is a page of instructions. `run` checks the whole page first, then
//! carries it out against a read-only view of the world. Actions (write, add,
//! toss, send, event) are only collected: the caller applies them together,
//! and only when the chore finished without an error ("all or nothing").
//!
//! Every limit gives a normal error, never a crash: sizes are checked before
//! anything is allocated and recursion depth is bounded, because the kernel
//! has a small stack and panics stop the whole OS.

use alloc::boxed::Box;
use alloc::format;
use alloc::rc::Rc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::cmp::Ordering;

use crate::fs;

pub const MAX_SOURCE: usize = 4096;
pub const MAX_LINES: usize = 120;
pub const MAX_NESTING: usize = 16;
pub const MAX_TEXT: usize = 4096;
pub const MAX_LIST: usize = 1000;
pub const MAX_OUTPUT: usize = 200;
pub const MAX_ACTIONS: usize = 200;
pub const MAX_STEPS: u32 = 50_000;
pub const MAX_NAMES: usize = 100;
pub const MAX_REPEAT: f64 = 10_000.0;
/// Brackets / `not` / unary minus inside one expression.
const MAX_BRACKETS: usize = 12;
/// Depth of an expression tree (a chain like 1+1+1+... is one level per +).
const MAX_TREE: usize = 48;
const NUM_LIMIT: f64 = 1e12;

/// What a chore may look at, and the checks its actions must pass.
/// Paths are internal paths: no "desk:" prefix, the desk itself is "".
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
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Write { page: String, text: String },
    Add { page: String, line: String },
    Toss { page: String },
    Send { to: String, subject: String, text: String },
    Event { date: String, time: String, title: String, with: String },
}

#[derive(Debug)]
pub struct Outcome {
    /// The `say` lines, in order (kept even when the chore fails).
    pub output: Vec<String>,
    /// The changes to make. Always empty when `error` is set.
    pub actions: Vec<Action>,
    pub steps: u32,
    /// "line N: ..." when the chore was refused or failed.
    pub error: Option<String>,
}

/// Check and carry out a chore. Never panics.
pub fn run<W: World>(source: &str, world: &W) -> Outcome {
    let fail = |e: String| Outcome { output: Vec::new(), actions: Vec::new(), steps: 0, error: Some(e) };
    let program = match parse(source) {
        Ok(p) => p,
        Err(e) => return fail(e),
    };
    let mut r = Runner { w: world, vars: Vec::new(), out: Vec::new(), actions: Vec::new(), steps: 0 };
    let result = r.block(&program);
    let error = match result {
        Ok(_) => None,
        Err((n, msg)) => Some(format!("line {n}: {msg}")),
    };
    let actions = if error.is_some() { Vec::new() } else { r.actions };
    Outcome { output: r.out, actions, steps: r.steps, error }
}

// ---------------------------------------------------------------- words --

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Text(String),
    Word(String),
    Sym(&'static str),
}

const KEYWORDS: [&str; 17] = [
    "set", "to", "say", "stop", "when", "otherwise", "done", "for", "each", "in", "repeat", "times", "and", "or",
    "not", "is", "has",
];

fn is_keyword(w: &str) -> bool {
    KEYWORDS.contains(&w) || w == "yes" || w == "no"
}

fn lex(line: &str) -> Result<Vec<Tok>, String> {
    let b = line.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if c == b' ' || c == b'\t' {
            i += 1;
        } else if c == b'-' && b.get(i + 1) == Some(&b'-') {
            break; // comment
        } else if c.is_ascii_digit() {
            let start = i;
            while i < b.len() && b[i].is_ascii_digit() {
                i += 1;
            }
            if i + 1 < b.len() && b[i] == b'.' && b[i + 1].is_ascii_digit() {
                i += 1;
                while i < b.len() && b[i].is_ascii_digit() {
                    i += 1;
                }
            }
            let v = parse_num(&line[start..i]).ok_or_else(|| format!("the number {} is too big", &line[start..i]))?;
            out.push(Tok::Num(v));
        } else if c == b'\'' || c == b'"' {
            let end = line[i + 1..].find(c as char).ok_or_else(|| {
                format!("the text started with {} is not closed on this line", c as char)
            })?;
            out.push(Tok::Text(line[i + 1..i + 1 + end].to_string()));
            i += end + 2;
        } else if c.is_ascii_alphabetic() || c == b'_' {
            let start = i;
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                i += 1;
            }
            out.push(Tok::Word(line[start..i].to_ascii_lowercase()));
        } else {
            let next = b.get(i + 1).copied().unwrap_or(0);
            let (sym, len) = match (c, next) {
                (b'<', b'=') => ("<=", 2),
                (b'>', b'=') => (">=", 2),
                (b'=', b'=') | (b'!', b'=') | (b'<', b'>') => {
                    return Err(format!("'{}{}' is not used here: compare with is or is not", c as char, next as char))
                }
                (b'=', _) => return Err("'=' is not used here: compare with is, or give a value with set x to ...".into()),
                (b'(', _) => ("(", 1),
                (b')', _) => (")", 1),
                (b',', _) => (",", 1),
                (b'+', _) => ("+", 1),
                (b'-', _) => ("-", 1),
                (b'*', _) => ("*", 1),
                (b'/', _) => ("/", 1),
                (b'&', _) => ("&", 1),
                (b'<', _) => ("<", 1),
                (b'>', _) => (">", 1),
                _ if c.is_ascii_graphic() => return Err(format!("I don't understand '{}'", c as char)),
                _ => return Err("this line has a character I can't read".into()),
            };
            out.push(Tok::Sym(sym));
            i += len;
        }
    }
    Ok(out)
}

fn parse_num(s: &str) -> Option<f64> {
    let v: f64 = s.parse().ok()?;
    (v.abs() < NUM_LIMIT).then_some(v)
}

// -------------------------------------------------------------- program --

#[derive(Debug, Clone, Copy, PartialEq)]
enum B {
    Pages,
    Read,
    Field,
    Body,
    Exists,
    Lines,
    Words,
    Count,
    Item,
    Number,
    Lower,
    Today,
    Weekday,
    AddDays,
    Write,
    Add,
    Toss,
    Send,
    Event,
}

/// (name, built-in, number of values, is an action)
const BUILTINS: [(&str, B, usize, bool); 19] = [
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
    ("write", B::Write, 2, true),
    ("add", B::Add, 2, true),
    ("toss", B::Toss, 1, true),
    ("send", B::Send, 3, true),
    ("event", B::Event, 4, true),
];

fn builtin(name: &str) -> Option<(B, usize, bool)> {
    BUILTINS.iter().find(|x| x.0 == name).map(|x| (x.1, x.2, x.3))
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Op {
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
enum Expr {
    Num(f64),
    Text(String),
    Bool(bool),
    Var(String),
    Neg(Box<Expr>),
    Not(Box<Expr>),
    Bin(Op, Box<Expr>, Box<Expr>),
    Call(B, Vec<Expr>),
}

#[derive(Debug)]
enum Stmt {
    Set(String, Expr),
    Say(Expr),
    Stop,
    When(Expr, Vec<Line>, Vec<Line>),
    ForEach(String, Expr, Vec<Line>),
    Repeat(Expr, Vec<Line>),
    Do(B, Vec<Expr>),
}

#[derive(Debug)]
struct Line {
    n: usize,
    stmt: Stmt,
}

fn parse(source: &str) -> Result<Vec<Line>, String> {
    if source.len() > MAX_SOURCE {
        return Err(format!("this chore is too long (at most {MAX_SOURCE} characters)"));
    }
    let raw: Vec<&str> = source.split('\n').collect();
    if raw.len() > MAX_LINES {
        return Err(format!("this chore has too many lines (at most {MAX_LINES})"));
    }
    let mut lines = Vec::new();
    for (i, l) in raw.iter().enumerate() {
        let toks = lex(l.trim_end_matches('\r')).map_err(|e| format!("line {}: {e}", i + 1))?;
        if !toks.is_empty() {
            lines.push((i + 1, toks));
        }
    }
    let mut pos = 0;
    let (body, end) = block(&lines, &mut pos, 0)?;
    match end {
        End::Eof => Ok(body),
        End::Done(n) => Err(format!("line {n}: done, but there is no block to end")),
        End::Otherwise(n) => Err(format!("line {n}: otherwise, but there is no when before it")),
    }
}

enum End {
    Eof,
    Done(usize),
    Otherwise(usize),
}

fn block(lines: &[(usize, Vec<Tok>)], pos: &mut usize, depth: usize) -> Result<(Vec<Line>, End), String> {
    let mut out = Vec::new();
    while *pos < lines.len() {
        let (n, toks) = &lines[*pos];
        let n = *n;
        *pos += 1;
        let at = |e: String| format!("line {n}: {e}");
        let first = match &toks[0] {
            Tok::Word(w) => w.as_str(),
            _ => return Err(at(start_error())),
        };
        let nested = |pos: &mut usize| -> Result<(Vec<Line>, End), String> {
            if depth + 1 > MAX_NESTING {
                return Err(at(format!("blocks are nested too deep (at most {MAX_NESTING})")));
            }
            block(lines, pos, depth + 1)
        };
        let stmt = match first {
            "done" | "otherwise" => {
                if toks.len() > 1 {
                    return Err(at(format!("{first} must be alone on its line")));
                }
                let end = if first == "done" { End::Done(n) } else { End::Otherwise(n) };
                return Ok((out, end));
            }
            "set" => {
                let mut p = P::new(toks, 1);
                let name = p.name("set").map_err(at)?;
                p.word("to", "set needs: set name to value").map_err(at)?;
                let e = p.expr().map_err(at)?;
                p.end().map_err(at)?;
                Stmt::Set(name, e)
            }
            "say" => {
                let mut p = P::new(toks, 1);
                let e = p.expr().map_err(at)?;
                p.end().map_err(at)?;
                Stmt::Say(e)
            }
            "stop" => {
                if toks.len() > 1 {
                    return Err(at("stop must be alone on its line".into()));
                }
                Stmt::Stop
            }
            "when" => {
                let mut p = P::new(toks, 1);
                let cond = p.expr().map_err(at)?;
                p.end().map_err(at)?;
                let (yes, end) = nested(pos)?;
                let no = match end {
                    End::Done(_) => Vec::new(),
                    End::Otherwise(_) => match nested(pos)? {
                        (no, End::Done(_)) => no,
                        (_, End::Otherwise(m)) => return Err(format!("line {m}: this when already has an otherwise")),
                        (_, End::Eof) => return Err(at("this when has no done".into())),
                    },
                    End::Eof => return Err(at("this when has no done".into())),
                };
                Stmt::When(cond, yes, no)
            }
            "for" => {
                let mut p = P::new(toks, 1);
                p.word("each", "for needs: for each name in list").map_err(at)?;
                let name = p.name("for each").map_err(at)?;
                p.word("in", "for needs: for each name in list").map_err(at)?;
                let list = p.expr().map_err(at)?;
                p.end().map_err(at)?;
                Stmt::ForEach(name, list, body_until_done(nested(pos)?, n, "for each")?)
            }
            "repeat" => {
                let mut p = P::new(toks, 1);
                let count = p.expr().map_err(at)?;
                p.word("times", "repeat needs: repeat number times").map_err(at)?;
                p.end().map_err(at)?;
                Stmt::Repeat(count, body_until_done(nested(pos)?, n, "repeat")?)
            }
            w => match builtin(w) {
                Some((b, _, true)) if toks.get(1) == Some(&Tok::Sym("(")) => {
                    let mut p = P::new(toks, 0);
                    let e = p.expr_allowing_action().map_err(at)?;
                    p.end().map_err(at)?;
                    match e {
                        Expr::Call(_, args) => Stmt::Do(b, args),
                        _ => return Err(at(start_error())),
                    }
                }
                Some((_, _, false)) if toks.get(1) == Some(&Tok::Sym("(")) => {
                    return Err(at(format!("{w}(...) gives a value; use it in set, say or when")))
                }
                _ => return Err(at(start_error())),
            },
        };
        out.push(Line { n, stmt });
    }
    Ok((out, End::Eof))
}

fn body_until_done(r: (Vec<Line>, End), n: usize, what: &str) -> Result<Vec<Line>, String> {
    match r {
        (body, End::Done(_)) => Ok(body),
        (_, End::Otherwise(m)) => Err(format!("line {m}: otherwise only belongs to when, not {what}")),
        (_, End::Eof) => Err(format!("line {n}: this {what} has no done")),
    }
}

fn start_error() -> String {
    "a line must start with set, say, when, for each, repeat, stop, or an action like write(...)".into()
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

    fn expr_allowing_action(&mut self) -> Result<Expr, String> {
        let e = self.call(true)?;
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
        self.call(false)
    }

    fn call(&mut self, action_ok: bool) -> Result<Expr, String> {
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
            Tok::Word(w) if w == "yes" => Ok(Expr::Bool(true)),
            Tok::Word(w) if w == "no" => Ok(Expr::Bool(false)),
            Tok::Word(w) if self.peek_sym("(") => {
                let (b, argc, action) =
                    builtin(&w).ok_or_else(|| format!("there is no built-in called {w}(...)"))?;
                if action && !action_ok {
                    return Err(format!("{w}(...) is an action: it must be on a line of its own"));
                }
                self.i += 1;
                self.deeper()?;
                let mut args = Vec::new();
                if !self.peek_sym(")") {
                    loop {
                        args.push(self.or()?);
                        if self.peek_sym(",") {
                            self.i += 1;
                        } else {
                            break;
                        }
                    }
                }
                self.brackets -= 1;
                if !self.peek_sym(")") {
                    return Err(format!("{w}( is not closed with )"));
                }
                self.i += 1;
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

fn show_tok(t: &Tok) -> String {
    match t {
        Tok::Num(v) => format!("the number {}", fmt_num(*v)),
        Tok::Text(s) => format!("the text '{s}'"),
        Tok::Word(w) => format!("the word {w}"),
        Tok::Sym(s) => format!("'{s}'"),
    }
}

/// Refuse expression trees deeper than MAX_TREE (evaluation is recursive).
/// Iterative, so checking a huge tree can't use up the stack either.
fn check_tree(e: &Expr) -> Result<(), String> {
    let mut stack = Vec::new();
    stack.push((e, 1usize));
    while let Some((e, d)) = stack.pop() {
        if d > MAX_TREE {
            return Err("this line is too complicated: split it with set".into());
        }
        match e {
            Expr::Neg(x) | Expr::Not(x) => stack.push((x, d + 1)),
            Expr::Bin(_, l, r) => {
                stack.push((l, d + 1));
                stack.push((r, d + 1));
            }
            Expr::Call(_, args) => args.iter().for_each(|a| stack.push((a, d + 1))),
            _ => {}
        }
    }
    Ok(())
}

// --------------------------------------------------------------- values --

#[derive(Debug, Clone)]
enum Value {
    Num(f64),
    Text(String),
    Bool(bool),
    List(Rc<Vec<String>>),
}

impl Value {
    fn kind(&self) -> &'static str {
        match self {
            Value::Num(_) => "a number",
            Value::Text(_) => "text",
            Value::Bool(_) => "yes/no",
            Value::List(_) => "a list",
        }
    }

    fn describe(&self) -> String {
        match self {
            Value::Num(v) => format!("the number {}", fmt_num(*v)),
            Value::Text(s) if short(s).len() == s.len() => format!("the text '{s}'"),
            Value::Text(s) => format!("the text '{}...'", short(s)),
            Value::Bool(b) => String::from(if *b { "yes" } else { "no" }),
            Value::List(l) => format!("a list of {} items", l.len()),
        }
    }
}

/// Numbers as the chore shows and writes them: whole numbers in full, others
/// with up to 6 decimals (rounded there), never "-0".
pub fn fmt_num(v: f64) -> String {
    let whole = v as i64; // |v| < 1e12, so exact for whole numbers
    if whole as f64 == v {
        return format!("{whole}");
    }
    let s = format!("{v:.6}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else { s.into() }
}

fn to_text(v: &Value) -> Result<String, String> {
    let s = match v {
        Value::Num(n) => fmt_num(*n),
        Value::Text(s) => s.clone(),
        Value::Bool(b) => String::from(if *b { "yes" } else { "no" }),
        Value::List(l) => {
            let len = l.iter().map(|s| s.len() + 2).sum::<usize>();
            if len > MAX_TEXT + 2 {
                return Err(format!("that text would be too long (at most {MAX_TEXT} characters)"));
            }
            l.join(", ")
        }
    };
    Ok(s)
}

fn days_in_month(y: i64, m: i64) -> i64 {
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31][(m - 1) as usize]
}

/// (year, month, day) of a real date written YYYY-MM-DD.
fn parse_date(s: &str) -> Option<(i64, i64, i64)> {
    let b = s.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    let num = |r: core::ops::Range<usize>| -> Option<i64> {
        let part = &s[r];
        part.bytes().all(|c| c.is_ascii_digit()).then(|| part.parse().ok()).flatten()
    };
    let (y, m, d) = (num(0..4)?, num(5..7)?, num(8..10)?);
    (y >= 1 && (1..=12).contains(&m) && d >= 1 && d <= days_in_month(y, m)).then_some((y, m, d))
}

/// Days since 1970-01-01 (Howard Hinnant's algorithm).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + if m <= 2 { 1 } else { 0 }, m, d)
}

/// Natural order: "m2" before "m10".
fn natural(a: &str, b: &str) -> Ordering {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        if a[i].is_ascii_digit() && b[j].is_ascii_digit() {
            let (si, sj) = (i, j);
            while i < a.len() && a[i].is_ascii_digit() {
                i += 1;
            }
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }
            let x = core::str::from_utf8(&a[si..i]).unwrap_or("").trim_start_matches('0');
            let y = core::str::from_utf8(&b[sj..j]).unwrap_or("").trim_start_matches('0');
            let o = x.len().cmp(&y.len()).then(x.cmp(y));
            if o != Ordering::Equal {
                return o;
            }
        } else {
            if a[i] != b[j] {
                return a[i].cmp(&b[j]);
            }
            i += 1;
            j += 1;
        }
    }
    (a.len() - i).cmp(&(b.len() - j)).then(a.cmp(b))
}

/// A path as a chore writes it ("mail:inbox:m1" or "desk:notes"), turned into
/// an internal path. Paths always start at the desk.
fn path_of(v: &Value) -> Result<String, String> {
    let Value::Text(s) = v else {
        return Err(format!("a page or drawer must be given as text, got {}", v.describe()));
    };
    let s = s.trim();
    let rest = if s == "desk" { "" } else { s.strip_prefix("desk:").unwrap_or(s) };
    let mut path = String::new();
    for part in rest.split(':') {
        if !fs::valid_name(part) {
            return Err(format!("'{s}' is not a valid page or drawer path"));
        }
        path = fs::join(&path, part);
    }
    Ok(path)
}

fn field_of<'a>(text: &'a str, key: &str) -> &'a str {
    for line in text.lines() {
        if let Some((k, v)) = line.split_once(':') {
            if k.trim().eq_ignore_ascii_case(key) {
                return v.trim();
            }
        }
        if line.is_empty() {
            break; // the header ends at the first blank line
        }
    }
    ""
}

fn body_of(text: &str) -> &str {
    let mut at = 0;
    for line in text.split('\n') {
        at += line.len() + 1;
        if line.trim_end_matches('\r').is_empty() {
            return text.get(at.min(text.len())..).unwrap_or("");
        }
    }
    ""
}

// --------------------------------------------------------------- runner --

enum Flow {
    Next,
    Stop,
}

struct Runner<'a, W: World> {
    w: &'a W,
    vars: Vec<(String, Value)>,
    out: Vec<String>,
    actions: Vec<Action>,
    steps: u32,
}

type Res<T> = Result<T, String>;

impl<'a, W: World> Runner<'a, W> {
    fn tick(&mut self, n: usize) -> Res<()> {
        self.steps = self.steps.saturating_add(n.min(u32::MAX as usize) as u32);
        if self.steps > MAX_STEPS {
            self.steps = MAX_STEPS + 1;
            Err(format!("the chore took too many steps (at most {MAX_STEPS}); is a loop too big?"))
        } else {
            Ok(())
        }
    }

    fn set(&mut self, name: &str, v: Value) -> Res<()> {
        if let Some(slot) = self.vars.iter_mut().find(|(n, _)| n == name) {
            slot.1 = v;
            return Ok(());
        }
        if self.vars.len() >= MAX_NAMES {
            return Err(format!("too many different names (at most {MAX_NAMES})"));
        }
        self.vars.push((name.to_string(), v));
        Ok(())
    }

    fn block(&mut self, lines: &[Line]) -> Result<Flow, (usize, String)> {
        for line in lines {
            if let Flow::Stop = self.line(line)? {
                return Ok(Flow::Stop);
            }
        }
        Ok(Flow::Next)
    }

    fn line(&mut self, line: &Line) -> Result<Flow, (usize, String)> {
        let n = line.n;
        let at = |e: String| (n, e);
        self.tick(1).map_err(at)?;
        match &line.stmt {
            Stmt::Set(name, e) => {
                let v = self.eval(e).map_err(at)?;
                self.set(name, v).map_err(at)?;
            }
            Stmt::Say(e) => {
                let v = self.eval(e).map_err(at)?;
                let s = to_text(&v).map_err(at)?;
                if self.out.len() >= MAX_OUTPUT {
                    return Err(at(format!("too many say lines (at most {MAX_OUTPUT})")));
                }
                self.out.push(s);
            }
            Stmt::Stop => return Ok(Flow::Stop),
            Stmt::When(cond, yes, no) => {
                let c = self.eval(cond).map_err(at)?;
                let Value::Bool(c) = c else {
                    return Err(at(format!("when needs yes or no, got {}", c.describe())));
                };
                return self.block(if c { yes } else { no });
            }
            Stmt::ForEach(name, list, body) => {
                let l = self.eval(list).map_err(at)?;
                let Value::List(items) = l else {
                    return Err(at(format!(
                        "for each needs a list (from pages, lines or words), got {}",
                        l.describe()
                    )));
                };
                for item in items.iter() {
                    self.tick(1).map_err(at)?;
                    self.set(name, Value::Text(item.clone())).map_err(at)?;
                    if let Flow::Stop = self.block(body)? {
                        return Ok(Flow::Stop);
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
                        )))
                    }
                };
                for _ in 0..times {
                    self.tick(1).map_err(at)?;
                    if let Flow::Stop = self.block(body)? {
                        return Ok(Flow::Stop);
                    }
                }
            }
            Stmt::Do(b, args) => self.action(*b, args).map_err(at)?,
        }
        Ok(Flow::Next)
    }

    fn eval(&mut self, e: &Expr) -> Res<Value> {
        self.tick(1)?;
        Ok(match e {
            Expr::Num(v) => Value::Num(*v),
            Expr::Text(s) => Value::Text(s.clone()),
            Expr::Bool(b) => Value::Bool(*b),
            Expr::Var(name) => match self.vars.iter().find(|(n, _)| n == name) {
                Some((_, v)) => v.clone(),
                None => return Err(format!("{name} has no value yet (give it one with set {name} to ...)")),
            },
            Expr::Neg(x) => match self.eval(x)? {
                Value::Num(v) => Value::Num(-v),
                v => return Err(format!("- needs a number, got {}", v.describe())),
            },
            Expr::Not(x) => match self.eval(x)? {
                Value::Bool(b) => Value::Bool(!b),
                v => return Err(format!("not needs yes or no, got {}", v.describe())),
            },
            Expr::Bin(Op::And, l, r) | Expr::Bin(Op::Or, l, r) => {
                let is_and = matches!(e, Expr::Bin(Op::And, _, _));
                let word = if is_and { "and" } else { "or" };
                let Value::Bool(a) = self.eval(l)? else {
                    return Err(format!("{word} needs yes or no on both sides"));
                };
                if a != is_and {
                    return Ok(Value::Bool(a)); // decided already: the right side is not looked at
                }
                match self.eval(r)? {
                    Value::Bool(b) => Value::Bool(b),
                    _ => return Err(format!("{word} needs yes or no on both sides")),
                }
            }
            Expr::Bin(op, l, r) => {
                let a = self.eval(l)?;
                let b = self.eval(r)?;
                self.binary(*op, a, b)?
            }
            Expr::Call(b, args) => {
                let mut vals = Vec::with_capacity(args.len());
                for a in args {
                    vals.push(self.eval(a)?);
                }
                self.call(*b, vals)?
            }
        })
    }

    #[inline(never)] // keeps the recursive eval() frame small (small kernel stack)
    fn binary(&mut self, op: Op, a: Value, b: Value) -> Res<Value> {
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
                    return Err(format!("{sym} needs two numbers, got {}{hint}", bad.describe()));
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
                let (x, y) = (to_text(&a)?, to_text(&b)?);
                if x.len() + y.len() > MAX_TEXT {
                    return Err(format!("that text would be too long (at most {MAX_TEXT} characters)"));
                }
                self.tick((x.len() + y.len()) / 64)?;
                Value::Text(x + &y)
            }
            Op::Is | Op::IsNot => {
                let same = match (&a, &b) {
                    (Value::Num(x), Value::Num(y)) => x == y,
                    (Value::Text(x), Value::Text(y)) => x == y,
                    (Value::Bool(x), Value::Bool(y)) => x == y,
                    (Value::List(_), _) | (_, Value::List(_)) => {
                        return Err(format!("{sym} can't compare lists (use has, or count(...))"))
                    }
                    _ => {
                        return Err(format!(
                            "{sym} compares two values of the same kind, got {} and {}{}",
                            a.kind(),
                            b.kind(),
                            if matches!((&a, &b), (Value::Num(_), Value::Text(_)) | (Value::Text(_), Value::Num(_))) {
                                " (number(...) turns text into a number)"
                            } else {
                                ""
                            }
                        ))
                    }
                };
                Value::Bool(same == (op == Op::Is))
            }
            Op::Lt | Op::Gt | Op::Le | Op::Ge => {
                let o = match (&a, &b) {
                    (Value::Num(x), Value::Num(y)) => x.partial_cmp(y).unwrap_or(Ordering::Equal),
                    (Value::Text(x), Value::Text(y)) => x.as_str().cmp(y.as_str()),
                    _ => {
                        return Err(format!(
                            "{sym} compares two numbers or two texts, got {} and {}",
                            a.kind(),
                            b.kind()
                        ))
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
                    self.tick(x.len() / 64)?;
                    Value::Bool(x.contains(y.as_str()))
                }
                (Value::List(l), Value::Text(y)) => {
                    self.tick(l.len())?;
                    Value::Bool(l.iter().any(|s| s == y))
                }
                _ => return Err(format!("has needs text or a list on the left and text on the right, got {} and {}", a.kind(), b.kind())),
            },
            Op::And | Op::Or => return Err("and/or are handled elsewhere".into()),
        })
    }

    #[inline(never)] // keeps the recursive eval() frame small (small kernel stack)
    fn call(&mut self, b: B, v: Vec<Value>) -> Res<Value> {
        let text = |x: &Value, what: &str| -> Res<String> {
            match x {
                Value::Text(s) => Ok(s.clone()),
                _ => Err(format!("{what} needs text, got {}", x.describe())),
            }
        };
        let whole = |x: &Value, what: &str| -> Res<i64> {
            match x {
                Value::Num(n) if *n == (*n as i64) as f64 => Ok(*n as i64),
                _ => Err(format!("{what} needs a whole number, got {}", x.describe())),
            }
        };
        let page = |me: &Self, x: &Value| -> Res<(String, String)> {
            let p = path_of(x)?;
            match me.w.page(&p) {
                Some(t) if t.len() > MAX_TEXT => Err(format!("{} is too long for a chore to read", fs::display(&p))),
                Some(t) => Ok((p, t)),
                None if me.w.is_drawer(&p) => Err(format!("{} is a drawer, not a page", fs::display(&p))),
                None => Err(format!("there is no page called {}", fs::display(&p))),
            }
        };
        let list = |me: &mut Self, items: Vec<String>, what: &str| -> Res<Value> {
            if items.len() > MAX_LIST {
                return Err(format!("{what} gave more than {MAX_LIST} items"));
            }
            me.tick(items.len())?;
            Ok(Value::List(Rc::new(items)))
        };
        Ok(match b {
            B::Pages => {
                let d = path_of(&v[0])?;
                if !self.w.is_drawer(&d) {
                    return Err(format!("there is no drawer called {}", fs::display(&d)));
                }
                let mut items = self.w.pages_in(&d);
                if items.len() > MAX_LIST {
                    return Err(format!("pages(...) gave more than {MAX_LIST} items"));
                }
                items.sort_by(|a, b| natural(a, b));
                list(self, items, "pages(...)")?
            }
            B::Read => Value::Text(page(self, &v[0])?.1),
            B::Field => {
                let key = text(&v[1], "field(page, key): the key")?;
                Value::Text(field_of(&page(self, &v[0])?.1, &key).to_string())
            }
            B::Body => Value::Text(body_of(&page(self, &v[0])?.1).to_string()),
            B::Exists => {
                let p = path_of(&v[0])?;
                Value::Bool(self.w.is_drawer(&p) || self.w.page(&p).is_some())
            }
            B::Lines => {
                let t = text(&v[0], "lines(...)")?;
                let items = t.lines().map(|s| s.trim_end_matches('\r').to_string()).collect();
                list(self, items, "lines(...)")?
            }
            B::Words => {
                let t = text(&v[0], "words(...)")?;
                let items = t.split_whitespace().map(|s| s.to_string()).collect();
                list(self, items, "words(...)")?
            }
            B::Count => match &v[0] {
                Value::List(l) => Value::Num(l.len() as f64),
                Value::Text(s) => Value::Num(s.len() as f64),
                x => return Err(format!("count(...) needs a list or text, got {}", x.describe())),
            },
            B::Item => {
                let Value::List(l) = &v[0] else {
                    return Err(format!("item(list, n) needs a list first, got {}", v[0].describe()));
                };
                let n = whole(&v[1], "item(list, n)")?;
                if n < 1 || n as usize > l.len() {
                    return Err(format!("item {n} doesn't exist: the list has {} items (the first is 1)", l.len()));
                }
                Value::Text(l[n as usize - 1].clone())
            }
            B::Number => {
                let t = text(&v[0], "number(...)")?;
                let s = t.trim();
                let digits = s.strip_prefix('-').unwrap_or(s);
                let ok = {
                    let mut parts = digits.splitn(2, '.');
                    let a = parts.next().unwrap_or("");
                    let b = parts.next();
                    !a.is_empty()
                        && a.bytes().all(|c| c.is_ascii_digit())
                        && b.map_or(true, |b| !b.is_empty() && b.bytes().all(|c| c.is_ascii_digit()))
                };
                let n = if ok && s.len() <= 30 { parse_num(digits) } else { None };
                match n {
                    Some(n) => Value::Num(if s.starts_with('-') { -n } else { n }),
                    None => return Err(format!("'{}' is not a number", short(s))),
                }
            }
            B::Lower => Value::Text(text(&v[0], "lower(...)")?.to_ascii_lowercase()),
            B::Today => Value::Text(self.w.today()),
            B::Weekday => {
                let t = text(&v[0], "weekday(...)")?;
                let (y, m, d) = parse_date(&t).ok_or_else(|| format!("'{}' is not a date like 2026-11-14", short(&t)))?;
                let w = (days_from_civil(y, m, d).rem_euclid(7) + 4) % 7;
                Value::Text(["sunday", "monday", "tuesday", "wednesday", "thursday", "friday", "saturday"][w as usize].into())
            }
            B::AddDays => {
                let t = text(&v[0], "add_days(date, n): the date")?;
                let (y, m, d) = parse_date(&t).ok_or_else(|| format!("'{}' is not a date like 2026-11-14", short(&t)))?;
                let n = whole(&v[1], "add_days(date, n)")?;
                if n.abs() > 1_000_000 {
                    return Err("add_days can move at most 1000000 days".into());
                }
                let (y, m, d) = civil_from_days(days_from_civil(y, m, d) + n);
                if !(1..=9999).contains(&y) {
                    return Err("that date is out of range".into());
                }
                Value::Text(format!("{y:04}-{m:02}-{d:02}"))
            }
            B::Write | B::Add | B::Toss | B::Send | B::Event => {
                return Err("an action can't be used as a value".into());
            }
        })
    }

    #[inline(never)] // keeps the recursive eval() frame small (small kernel stack)
    fn action(&mut self, b: B, args: &[Expr]) -> Res<()> {
        let mut v = Vec::with_capacity(args.len());
        for a in args {
            v.push(self.eval(a)?);
        }
        if self.actions.len() >= MAX_ACTIONS {
            return Err(format!("too many changes (at most {MAX_ACTIONS} actions per chore)"));
        }
        let action = match b {
            B::Write => {
                let page = path_of(&v[0])?;
                self.w.check_write(&page)?;
                Action::Write { page, text: to_text(&v[1])? }
            }
            B::Add => {
                let page = path_of(&v[0])?;
                self.w.check_write(&page)?;
                let line = to_text(&v[1])?;
                Action::Add { page, line }
            }
            B::Toss => {
                let page = path_of(&v[0])?;
                self.w.check_toss(&page)?;
                Action::Toss { page }
            }
            B::Send => {
                let (to, subject, text) = (to_text(&v[0])?, to_text(&v[1])?, to_text(&v[2])?);
                self.w.check_send(&to, &subject, &text)?;
                Action::Send { to, subject, text }
            }
            B::Event => {
                let (date, time, title, with) = (to_text(&v[0])?, to_text(&v[1])?, to_text(&v[2])?, to_text(&v[3])?);
                self.w.check_event(&date, &time, &title, &with)?;
                Action::Event { date, time, title, with }
            }
            _ => return Err("only actions can stand alone on a line".into()),
        };
        self.tick(1)?;
        self.actions.push(action);
        Ok(())
    }
}

/// At most the first 20 characters (for error messages).
fn short(s: &str) -> &str {
    s.char_indices().nth(20).map_or(s, |(i, _)| &s[..i])
}
