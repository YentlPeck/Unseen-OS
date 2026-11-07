//! Values, and the memory meter (design §9).
//!
//! Every text, list and record is counted by a `Meter` when it is made, and given back when it
//! is freed, so the meter always knows the live size. The count is checked **before** the
//! memory is asked for. Values are copies: lists and records are shared (`Rc`) until one side
//! changes them, then the changed side gets its own counted copy.

use alloc::boxed::Box;
use alloc::format;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;
use core::cell::Cell;

use crate::limits::*;
use crate::util::{fmt_num, short};

/// What went wrong. Boxed, so a `Result` that may hold one stays small: results are on the
/// stack at every level of a running program, and the kernel stack is small.
#[derive(Debug, Clone, PartialEq)]
pub struct Fault(pub Box<FaultData>);

#[derive(Debug, Clone, PartialEq)]
pub struct FaultData {
    pub msg: String,
    /// A limit ran out (can never be caught by `try`).
    pub limit: bool,
    /// Where it happened, (page, line), and the (page, line)s of the task calls it happened
    /// inside, innermost first. Set by the first line it passes.
    pub place: Option<((usize, usize), Vec<(usize, usize)>)>,
    /// Not a mistake: `stop` inside a task, ending the whole run.
    pub stop: bool,
    /// A failed `check` in a test (can't be caught by `try`).
    pub check: bool,
}

impl FaultData {
    /// `try` can't catch it: a limit, a stop or a failed check.
    pub fn uncatchable(&self) -> bool {
        self.limit || self.stop || self.check
    }
}

impl core::ops::Deref for Fault {
    type Target = FaultData;
    fn deref(&self) -> &FaultData {
        &self.0
    }
}

impl core::ops::DerefMut for Fault {
    fn deref_mut(&mut self) -> &mut FaultData {
        &mut self.0
    }
}

impl Fault {
    pub fn new(msg: String) -> Fault {
        Fault(Box::new(FaultData { msg, limit: false, place: None, stop: false, check: false }))
    }

    pub fn stopped() -> Fault {
        Fault(Box::new(FaultData { msg: String::new(), limit: false, place: None, stop: true, check: false }))
    }

    pub fn check_failed(msg: String) -> Fault {
        Fault(Box::new(FaultData { msg, limit: false, place: None, stop: false, check: true }))
    }
}

pub type Res<T> = Result<T, Fault>;

impl From<String> for Fault {
    fn from(msg: String) -> Self {
        Fault::new(msg)
    }
}

impl From<&str> for Fault {
    fn from(msg: &str) -> Self {
        Fault::new(msg.into())
    }
}

pub fn limit_fault(msg: String) -> Fault {
    Fault(Box::new(FaultData { msg, limit: true, place: None, stop: false, check: false }))
}

/// The live memory of one run.
pub struct Meter {
    used: Cell<usize>,
    peak: Cell<usize>,
    limit: usize,
}

impl Meter {
    pub fn new(limit: usize) -> Rc<Meter> {
        Rc::new(Meter { used: Cell::new(0), peak: Cell::new(0), limit })
    }

    pub fn charge(&self, n: usize) -> Res<()> {
        let u = self.used.get().checked_add(n).filter(|u| *u <= self.limit).ok_or_else(|| {
            limit_fault(format!("the program needs too much memory (at most {} KB)", self.limit / 1024))
        })?;
        self.used.set(u);
        if u > self.peak.get() {
            self.peak.set(u);
        }
        Ok(())
    }

    pub fn release(&self, n: usize) {
        self.used.set(self.used.get().saturating_sub(n));
    }

    pub fn used(&self) -> usize {
        self.used.get()
    }

    pub fn peak(&self) -> usize {
        self.peak.get()
    }
}

pub struct TextBox {
    pub s: String,
    charged: usize,
    m: Rc<Meter>,
}

pub struct ListBox {
    pub v: Vec<Value>,
    charged: usize,
    depth: usize,
    m: Rc<Meter>,
}

pub struct RecBox {
    /// (field name in small letters, value), in the order the fields were made.
    pub f: Vec<(String, Value)>,
    charged: usize,
    depth: usize,
    m: Rc<Meter>,
}

impl Drop for TextBox {
    fn drop(&mut self) {
        self.m.release(self.charged)
    }
}
impl Drop for ListBox {
    fn drop(&mut self) {
        self.m.release(self.charged)
    }
}
impl Drop for RecBox {
    fn drop(&mut self) {
        self.m.release(self.charged)
    }
}

#[derive(Clone)]
pub enum Value {
    Num(f64),
    Bool(bool),
    Nothing,
    Text(Rc<TextBox>),
    List(Rc<ListBox>),
    Rec(Rc<RecBox>),
}

fn field_cost(name: &str) -> usize {
    FIELD_COST + name.len()
}

fn oom() -> Fault {
    limit_fault("the computer has no more memory for this program".into())
}

impl Value {
    pub fn kind(&self) -> &'static str {
        match self {
            Value::Num(_) => "a number",
            Value::Text(_) => "text",
            Value::Bool(_) => "yes/no",
            Value::List(_) => "a list",
            Value::Rec(_) => "a record",
            Value::Nothing => "nothing",
        }
    }

    pub fn describe(&self) -> String {
        match self {
            Value::Num(v) => format!("the number {}", fmt_num(*v)),
            Value::Text(t) if short(&t.s).len() == t.s.len() => format!("the text '{}'", t.s),
            Value::Text(t) => format!("the text '{}...'", short(&t.s)),
            Value::Bool(b) => String::from(if *b { "yes" } else { "no" }),
            Value::List(l) => format!("a list of {} items", l.v.len()),
            Value::Rec(r) => format!("a record with {} fields", r.f.len()),
            Value::Nothing => "nothing".into(),
        }
    }

    /// How deep lists and records are nested: 0 for a plain value, 1 for a list of plain values.
    pub fn depth(&self) -> usize {
        match self {
            Value::List(l) => l.depth,
            Value::Rec(r) => r.depth,
            _ => 0,
        }
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            Value::Text(t) => Some(&t.s),
            _ => None,
        }
    }
}

/// A new text of at most `len` bytes, counted before it is made. `build` must not write more.
pub fn new_text(m: &Rc<Meter>, len: usize, build: impl FnOnce(&mut String)) -> Res<Value> {
    if len > MAX_TEXT {
        return Err(format!("that text would be too long (at most {MAX_TEXT} characters)").into());
    }
    let cost = len + OVERHEAD;
    m.charge(cost)?;
    let mut s = String::new();
    if s.try_reserve_exact(len).is_err() {
        m.release(cost);
        return Err(oom());
    }
    build(&mut s);
    debug_assert!(s.len() <= len);
    Ok(Value::Text(Rc::new(TextBox { s, charged: cost, m: m.clone() })))
}

pub fn text(m: &Rc<Meter>, s: &str) -> Res<Value> {
    new_text(m, s.len(), |t| t.push_str(s))
}

/// Builds a list of at most `cap` items; the room is counted (and reserved) first.
pub struct ListMaker {
    v: Vec<Value>,
    cap: usize,
    charged: usize,
    depth: usize,
    m: Rc<Meter>,
}

impl ListMaker {
    pub fn new(m: &Rc<Meter>, cap: usize) -> Res<ListMaker> {
        if cap > MAX_LIST {
            return Err(format!("a list can have at most {MAX_LIST} items").into());
        }
        let cost = OVERHEAD + cap * ITEM_COST;
        m.charge(cost)?;
        let mut v = Vec::new();
        if v.try_reserve_exact(cap).is_err() {
            m.release(cost);
            return Err(oom());
        }
        Ok(ListMaker { v, cap, charged: cost, depth: 1, m: m.clone() })
    }

    pub fn push(&mut self, x: Value) -> Res<()> {
        if self.v.len() >= self.cap {
            return Err("internal: list maker is full".into());
        }
        let d = x.depth() + 1;
        if d > MAX_VALUE_DEPTH {
            return Err(too_deep());
        }
        self.depth = self.depth.max(d);
        self.v.push(x);
        Ok(())
    }

    pub fn finish(mut self) -> Value {
        let v = core::mem::take(&mut self.v);
        let b = ListBox { v, charged: self.charged, depth: self.depth, m: self.m.clone() };
        self.charged = 0;
        Value::List(Rc::new(b))
    }
}

impl Drop for ListMaker {
    fn drop(&mut self) {
        self.m.release(self.charged)
    }
}

pub fn too_deep() -> Fault {
    format!("lists and records can be nested at most {MAX_VALUE_DEPTH} deep").into()
}

/// Memory counted ahead of making something; given back when dropped unless it is used.
pub struct Charge {
    m: Rc<Meter>,
    n: usize,
}

impl Charge {
    pub fn new(m: &Rc<Meter>, n: usize) -> Res<Charge> {
        m.charge(n)?;
        Ok(Charge { m: m.clone(), n })
    }

    /// Keep the memory counted (it now belongs to what was made).
    pub fn keep(mut self) -> usize {
        core::mem::take(&mut self.n)
    }
}

impl Drop for Charge {
    fn drop(&mut self) {
        self.m.release(self.n)
    }
}

/// What a record with these field names counts.
pub fn record_cost<'a>(names: impl Iterator<Item = &'a str>) -> usize {
    OVERHEAD + names.map(field_cost).sum::<usize>()
}

/// A record whose memory was counted ahead (with `record_cost`).
pub fn record_charged(charge: Charge, fields: Vec<(String, Value)>) -> Res<Value> {
    let depth = 1 + fields.iter().map(|(_, v)| v.depth()).max().unwrap_or(0);
    if fields.len() > MAX_LIST {
        return Err(format!("a record can have at most {MAX_LIST} fields").into());
    }
    if depth > MAX_VALUE_DEPTH {
        return Err(too_deep());
    }
    let m = charge.m.clone();
    let charged = charge.keep();
    Ok(Value::Rec(Rc::new(RecBox { f: fields, charged, depth, m })))
}

pub fn new_record(m: &Rc<Meter>, fields: Vec<(String, Value)>) -> Res<Value> {
    let mut cost = OVERHEAD;
    let mut depth = 1;
    for (k, v) in &fields {
        cost += field_cost(k);
        depth = depth.max(v.depth() + 1);
    }
    if fields.len() > MAX_LIST {
        return Err(format!("a record can have at most {MAX_LIST} fields").into());
    }
    if depth > MAX_VALUE_DEPTH {
        return Err(too_deep());
    }
    m.charge(cost)?;
    Ok(Value::Rec(Rc::new(RecBox { f: fields, charged: cost, depth, m: m.clone() })))
}

/// The list in `rc`, ready to be changed: copied first if shared, and the copy counted by `m`
/// (the run that changes it; a shared value may belong to the fixed values).
pub fn list_mut<'r>(rc: &'r mut Rc<ListBox>, m: &Rc<Meter>) -> Res<&'r mut ListBox> {
    if Rc::get_mut(rc).is_none() {
        let old = &**rc;
        let n = old.v.len();
        let cost = OVERHEAD + n * ITEM_COST;
        m.charge(cost)?;
        let mut v = Vec::new();
        if v.try_reserve_exact(n).is_err() {
            m.release(cost);
            return Err(oom());
        }
        v.extend(old.v.iter().cloned());
        *rc = Rc::new(ListBox { v, charged: cost, depth: old.depth, m: m.clone() });
    }
    Ok(Rc::get_mut(rc).expect("just made unique"))
}

pub fn rec_mut<'r>(rc: &'r mut Rc<RecBox>, m: &Rc<Meter>) -> Res<&'r mut RecBox> {
    if Rc::get_mut(rc).is_none() {
        let old = &**rc;
        let cost = OVERHEAD + old.f.iter().map(|(k, _)| field_cost(k)).sum::<usize>();
        m.charge(cost)?;
        let mut f = Vec::new();
        if f.try_reserve_exact(old.f.len()).is_err() {
            m.release(cost);
            return Err(oom());
        }
        f.extend(old.f.iter().cloned());
        *rc = Rc::new(RecBox { f, charged: cost, depth: old.depth, m: m.clone() });
    }
    Ok(Rc::get_mut(rc).expect("just made unique"))
}

impl ListBox {
    /// Add an item at the end (room is counted before it is asked for).
    pub fn push(&mut self, x: Value) -> Res<()> {
        if self.v.len() >= MAX_LIST {
            return Err(format!("a list can have at most {MAX_LIST} items").into());
        }
        let d = x.depth() + 1;
        if d > MAX_VALUE_DEPTH {
            return Err(too_deep());
        }
        if self.v.len() == self.v.capacity() {
            let cap = self.v.capacity();
            let new_cap = (cap * 2).max(4).min(MAX_LIST);
            let cost = (new_cap - cap) * ITEM_COST;
            self.m.charge(cost)?;
            if self.v.try_reserve_exact(new_cap - self.v.len()).is_err() {
                self.m.release(cost);
                return Err(oom());
            }
            self.charged += cost;
        }
        self.depth = self.depth.max(d);
        self.v.push(x);
        Ok(())
    }

    pub fn fix_depth(&mut self) {
        self.depth = 1 + self.v.iter().map(Value::depth).max().unwrap_or(0);
    }
}

impl RecBox {
    pub fn get(&self, name: &str) -> Option<&Value> {
        self.f.iter().find(|(k, _)| k == name).map(|(_, v)| v)
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut Value> {
        self.f.iter_mut().find(|(k, _)| k == name).map(|(_, v)| v)
    }

    /// Change a field, or make it.
    pub fn set(&mut self, name: &str, x: Value) -> Res<()> {
        if x.depth() + 1 > MAX_VALUE_DEPTH {
            return Err(too_deep());
        }
        if let Some(slot) = self.get_mut(name) {
            *slot = x;
        } else {
            if self.f.len() >= MAX_LIST {
                return Err(format!("a record can have at most {MAX_LIST} fields").into());
            }
            let cost = field_cost(name);
            self.m.charge(cost)?;
            if self.f.try_reserve_exact(1).is_err() {
                self.m.release(cost);
                return Err(oom());
            }
            self.charged += cost;
            self.f.push((String::from(name), x));
        }
        self.fix_depth();
        Ok(())
    }

    pub fn fix_depth(&mut self) {
        self.depth = 1 + self.f.iter().map(|(_, v)| v.depth()).max().unwrap_or(0);
    }
}
