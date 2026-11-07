//! The built-in functions that are new in Unseen Script (design §4 "Built-in library").
//! The chore functions are in run.rs. Every function counts its work as steps and its result
//! as memory before making it.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::cmp::Ordering;

use crate::limits::*;
use crate::parse::B;
use crate::run::{text_len, write_text, Runner, World};
use crate::util::*;
use crate::value::*;

fn targ<'v>(x: &'v Value, what: &str) -> Res<&'v str> {
    match x {
        Value::Text(t) => Ok(&t.s),
        _ => Err(format!("{what} needs text, got {}", x.describe()).into()),
    }
}

fn whole(x: &Value, what: &str) -> Res<i64> {
    match x {
        Value::Num(n) if *n == (*n as i64) as f64 => Ok(*n as i64),
        _ => Err(format!("{what} needs a whole number, got {}", x.describe()).into()),
    }
}

fn num(x: &Value, what: &str) -> Res<f64> {
    match x {
        Value::Num(n) => Ok(*n),
        Value::Text(_) => Err(format!("{what} needs a number, got {} (number(...) turns text into a number)", x.describe()).into()),
        _ => Err(format!("{what} needs a number, got {}", x.describe()).into()),
    }
}

fn list<'v>(x: &'v Value, what: &str) -> Res<&'v [Value]> {
    match x {
        Value::List(l) => Ok(&l.v),
        _ => Err(format!("{what} needs a list, got {}", x.describe()).into()),
    }
}

fn record<'v>(x: &'v Value, what: &str) -> Res<&'v RecBox> {
    match x {
        Value::Rec(r) => Ok(r),
        _ => Err(format!("{what} needs a record, got {}", x.describe()).into()),
    }
}

fn date(x: &Value, what: &str) -> Res<i64> {
    let t = targ(x, what)?;
    let (y, m, d) = parse_date(t).ok_or_else(|| format!("'{}' is not a date like 2026-11-14", short(t)))?;
    Ok(days_from_civil(y, m, d))
}

fn checked(v: f64) -> Res<Value> {
    if !(v.abs() < NUM_LIMIT) {
        return Err("that number is too big (it must stay below 1000000000000)".into());
    }
    Ok(Value::Num(v))
}

/// Positions `from` to `to` (both counted from 1, both included) of something `len` long,
/// moved inside it: a range of 0-based offsets (empty if from > to).
fn span(from: i64, to: i64, len: usize) -> (usize, usize) {
    let a = from.max(1) as usize - 1;
    let b = (to.min(len as i64)).max(0) as usize;
    if a >= b { (0, 0) } else { (a, b) }
}

impl<'a, W: World> Runner<'a, W> {
    fn list_of(&mut self, items: impl Iterator<Item = Value>, n: usize) -> Res<Value> {
        self.tick(n)?;
        let mut lm = ListMaker::new(&self.m, n)?;
        for x in items {
            lm.push(x)?;
        }
        Ok(lm.finish())
    }

    fn texts_of<'s>(&mut self, items: impl Iterator<Item = &'s str>, n: usize, what: &str) -> Res<Value> {
        if n > MAX_LIST {
            return Err(format!("{what} would give more than {MAX_LIST} items").into());
        }
        self.tick(n)?;
        let mut lm = ListMaker::new(&self.m, n)?;
        for s in items {
            lm.push(text(&self.m, s)?)?;
        }
        Ok(lm.finish())
    }

    #[inline(never)] // keeps the recursive eval() frame small (small kernel stack)
    pub(crate) fn library(&mut self, b: B, v: Vec<Value>) -> Res<Value> {
        Ok(match b {
            // ---- text
            B::Upper => {
                let t = targ(&v[0], "upper(...)")?;
                self.tick(t.len() / TEXT_WORK)?;
                new_text(&self.m, t.len(), |s| s.extend(t.chars().map(|c| c.to_ascii_uppercase())))?
            }
            B::Trim => text(&self.m, targ(&v[0], "trim(...)")?.trim())?,
            B::Split => {
                let t = targ(&v[0], "split(text, by): the text")?;
                let by = targ(&v[1], "split(text, by): what to split on")?;
                if by.is_empty() {
                    return Err("split(text, by) needs something to split on (words(...) splits on spaces)".into());
                }
                self.tick(t.len() / TEXT_WORK)?;
                let n = t.matches(by).count() + 1;
                self.texts_of(t.split(by), n, "split(...)")?
            }
            B::Join => {
                let items = list(&v[0], "join(list, by): the list")?;
                let by = targ(&v[1], "join(list, by): what goes between")?;
                let mut len = by.len() * items.len().saturating_sub(1);
                for x in items {
                    len += text_len(x, MAX_TEXT)?;
                    if len > MAX_TEXT {
                        return Err(format!("that text would be too long (at most {MAX_TEXT} characters)").into());
                    }
                }
                self.tick(items.len() + len / TEXT_WORK)?;
                new_text(&self.m, len, |s| {
                    for (i, x) in items.iter().enumerate() {
                        if i > 0 {
                            s.push_str(by);
                        }
                        write_text(x, s);
                    }
                })?
            }
            B::Replace => {
                let t = targ(&v[0], "replace(text, old, new): the text")?;
                let old = targ(&v[1], "replace(text, old, new): the old part")?;
                let new = targ(&v[2], "replace(text, old, new): the new part")?;
                if old.is_empty() {
                    return Err("replace(text, old, new) needs an old part that is not empty".into());
                }
                self.tick(t.len() / TEXT_WORK)?;
                let n = t.matches(old).count();
                let len = (t.len() - n * old.len()).saturating_add(n.saturating_mul(new.len()));
                if len > MAX_TEXT {
                    return Err(format!("that text would be too long (at most {MAX_TEXT} characters)").into());
                }
                new_text(&self.m, len, |s| {
                    let mut rest = t;
                    while let Some(i) = rest.find(old) {
                        s.push_str(&rest[..i]);
                        s.push_str(new);
                        rest = &rest[i + old.len()..];
                    }
                    s.push_str(rest);
                })?
            }
            B::Find => {
                let t = targ(&v[0], "find(text, part): the text")?;
                let part = targ(&v[1], "find(text, part): the part")?;
                self.tick(t.len() / TEXT_WORK)?;
                Value::Num(t.find(part).map_or(0, |i| i + 1) as f64)
            }
            B::Slice => {
                let from = whole(&v[1], "slice(x, from, to): from")?;
                let to = whole(&v[2], "slice(x, from, to): to")?;
                match &v[0] {
                    Value::Text(t) => {
                        let (mut a, mut b) = span(from, to, t.s.len());
                        // never cut inside a character that takes more than one place
                        while a > 0 && !t.s.is_char_boundary(a) {
                            a -= 1;
                        }
                        while b < t.s.len() && !t.s.is_char_boundary(b) {
                            b += 1;
                        }
                        let part = &t.s[a..b.max(a)];
                        self.tick(part.len() / TEXT_WORK)?;
                        text(&self.m, part)?
                    }
                    Value::List(l) => {
                        let (a, b) = span(from, to, l.v.len());
                        self.list_of(l.v[a..b].iter().cloned(), b - a)?
                    }
                    x => return Err(format!("slice(x, from, to) needs text or a list, got {}", x.describe()).into()),
                }
            }
            B::Starts => Value::Bool(targ(&v[0], "starts(text, part)")?.starts_with(targ(&v[1], "starts(text, part)")?)),
            B::Ends => Value::Bool(targ(&v[0], "ends(text, part)")?.ends_with(targ(&v[1], "ends(text, part)")?)),
            B::Newline => text(&self.m, "\n")?,
            B::Text => {
                if let Value::Text(_) = &v[0] {
                    return Ok(v[0].clone());
                }
                let len = text_len(&v[0], MAX_TEXT)?;
                self.tick(len / TEXT_WORK)?;
                new_text(&self.m, len, |s| write_text(&v[0], s))?
            }
            B::Pad => {
                let t = targ(&v[0], "pad(text, width): the text")?;
                let w = whole(&v[1], "pad(text, width): the width")?;
                if w.unsigned_abs() as usize > MAX_TEXT {
                    return Err(format!("pad(...) can make a text at most {MAX_TEXT} characters long").into());
                }
                let width = w.unsigned_abs() as usize;
                let len = t.len().max(width);
                let spaces = len - t.len();
                self.tick(len / TEXT_WORK)?;
                new_text(&self.m, len, |s| {
                    if w < 0 {
                        s.extend(core::iter::repeat(' ').take(spaces));
                        s.push_str(t);
                    } else {
                        s.push_str(t);
                        s.extend(core::iter::repeat(' ').take(spaces));
                    }
                })?
            }
            // ---- numbers
            B::Round => {
                let n = num(&v[0], "round(number, decimals)")?;
                let d = whole(&v[1], "round(number, decimals): the decimals")?;
                if !(0..=6).contains(&d) {
                    return Err("round(number, decimals) can round to 0 to 6 decimals".into());
                }
                let f = [1.0, 10.0, 100.0, 1e3, 1e4, 1e5, 1e6][d as usize];
                if n == (n as i64) as f64 || (n * f).abs() >= 4.5e15 {
                    checked(n)? // whole already, or too big to have that many decimals
                } else {
                    checked(libm_round(n * f) / f)?
                }
            }
            B::Floor => checked(libm_floor(num(&v[0], "floor(...)")?))?,
            B::Ceil => checked(-libm_floor(-num(&v[0], "ceil(...)")?))?,
            B::Abs => checked(num(&v[0], "abs(...)")?.abs())?,
            B::Min | B::Max => {
                let o = match (&v[0], &v[1]) {
                    (Value::Num(x), Value::Num(y)) => x.partial_cmp(y).unwrap_or(Ordering::Equal),
                    (Value::Text(x), Value::Text(y)) => x.s.cmp(&y.s),
                    (x, y) => {
                        return Err(format!(
                            "{}(a, b) needs two numbers or two texts, got {} and {}",
                            if b == B::Min { "min" } else { "max" },
                            x.kind(),
                            y.kind()
                        )
                        .into())
                    }
                };
                let first = if b == B::Min { o != Ordering::Greater } else { o != Ordering::Less };
                if first { v[0].clone() } else { v[1].clone() }
            }
            B::IsNumber => Value::Bool(number_of(targ(&v[0], "is_number(...)")?).is_some()),
            // ---- lists
            B::Sum => {
                let items = list(&v[0], "sum(list)")?;
                self.tick(items.len())?;
                let mut total = 0.0;
                for (i, x) in items.iter().enumerate() {
                    match x {
                        Value::Num(n) => total += n,
                        _ => return Err(format!("sum(list) needs a list of numbers, but item {} is {}", i + 1, x.describe()).into()),
                    }
                }
                checked(total)?
            }
            B::Sort => {
                let items = list(&v[0], "sort(list)")?;
                let nums = items.iter().all(|x| matches!(x, Value::Num(_)));
                let texts = items.iter().all(|x| matches!(x, Value::Text(_)));
                if !nums && !texts {
                    return Err("sort(list) needs a list of only numbers or only texts".into());
                }
                let n = items.len();
                let log = usize::BITS as usize - n.leading_zeros() as usize;
                let longest = items.iter().map(|x| x.as_text().map_or(0, str::len)).max().unwrap_or(0);
                // every comparison may look at the whole of the longest text
                self.tick(n.saturating_mul(log.max(1)).saturating_mul(1 + longest / COMPARE_WORK))?;
                let _counted = Charge::new(&self.m, OVERHEAD + n * core::mem::size_of::<usize>())?;
                let mut idx: Vec<usize> = Vec::new();
                if idx.try_reserve_exact(n).is_err() {
                    return Err(limit_fault("the computer has no more memory for this program".into()));
                }
                idx.extend(0..n);
                // sorted in place (no hidden working memory); ties keep their order by index
                idx.sort_unstable_by(|&i, &j| match (&items[i], &items[j]) {
                    (Value::Num(x), Value::Num(y)) => x.partial_cmp(y).unwrap_or(Ordering::Equal),
                    (Value::Text(x), Value::Text(y)) => x.s.cmp(&y.s),
                    _ => Ordering::Equal,
                }.then(i.cmp(&j)));
                self.list_of(idx.iter().map(|&i| items[i].clone()), n)?
            }
            B::Reverse => {
                let items = list(&v[0], "reverse(list)")?;
                self.list_of(items.iter().rev().cloned(), items.len())?
            }
            B::Unique => {
                let items = list(&v[0], "unique(list)")?;
                let mut keep: Vec<usize> = Vec::new();
                if keep.try_reserve_exact(items.len()).is_err() {
                    return Err(limit_fault("the computer has no more memory for this program".into()));
                }
                for (i, x) in items.iter().enumerate() {
                    let mut seen = false;
                    for &k in &keep {
                        let y = &items[k];
                        if core::mem::discriminant(x) == core::mem::discriminant(y) && self.same(x, y)? {
                            seen = true;
                            break;
                        }
                    }
                    self.tick(keep.len())?;
                    if !seen {
                        keep.push(i);
                    }
                }
                self.list_of(keep.iter().map(|&i| items[i].clone()), keep.len())?
            }
            B::First | B::Last => {
                let items = list(&v[0], if b == B::First { "first(list)" } else { "last(list)" })?;
                let x = if b == B::First { items.first() } else { items.last() };
                x.cloned().ok_or("the list is empty, so it has no first or last item")?
            }
            B::Range => {
                let a = whole(&v[0], "range(from, to): from")?;
                let z = whole(&v[1], "range(from, to): to")?;
                let n = if z < a { 0 } else { (z - a + 1) as u64 };
                if n > MAX_LIST as u64 {
                    return Err(format!("range(...) would give more than {MAX_LIST} items").into());
                }
                self.list_of((0..n as i64).map(|i| Value::Num((a + i) as f64)), n as usize)?
            }
            B::Position => {
                let items = list(&v[0], "position(list, value)")?;
                self.tick(items.len())?;
                let mut at = 0;
                for (i, x) in items.iter().enumerate() {
                    if core::mem::discriminant(x) == core::mem::discriminant(&v[1]) && self.same(x, &v[1])? {
                        at = i + 1;
                        break;
                    }
                }
                Value::Num(at as f64)
            }
            B::Without => {
                let items = list(&v[0], "without(list, n)")?;
                let n = whole(&v[1], "without(list, n)")?;
                if n < 1 || n as usize > items.len() {
                    return Err(format!("item {n} doesn't exist: the list has {} items (the first is 1)", items.len()).into());
                }
                let n = n as usize - 1;
                let rest = items.iter().enumerate().filter(|(i, _)| *i != n).map(|(_, x)| x.clone());
                self.list_of(rest, items.len() - 1)?
            }
            // ---- records
            B::Keys => {
                let r = record(&v[0], "keys(record)")?;
                let n = r.f.len();
                self.texts_of(r.f.iter().map(|(k, _)| k.as_str()), n, "keys(...)")?
            }
            B::HasKey => {
                let r = record(&v[0], "has_key(record, key)")?;
                let k = targ(&v[1], "has_key(record, key): the key")?;
                Value::Bool(r.f.iter().any(|(f, _)| f.eq_ignore_ascii_case(k)))
            }
            B::Fields => {
                let p = match &v[0] {
                    Value::Text(t) => path_of(&t.s)?,
                    x => return Err(format!("fields(page) needs a page given as text, got {}", x.describe()).into()),
                };
                let t = match self.w.page(&p) {
                    Some(t) if t.len() > MAX_TEXT => return Err(format!("{} is too long for a program to read", display(&p)).into()),
                    Some(t) => t,
                    None if self.w.is_drawer(&p) => return Err(format!("{} is a drawer, not a page", display(&p)).into()),
                    None => return Err(format!("there is no page called {}", display(&p)).into()),
                };
                let mut pairs: Vec<(String, &str)> = Vec::new();
                for line in t.lines() {
                    if line.trim_end_matches('\r').is_empty() {
                        break;
                    }
                    let Some((k, val)) = line.split_once(':') else { continue };
                    let k = k.trim().to_ascii_lowercase();
                    let ok = k.bytes().next().map_or(false, |c| c.is_ascii_alphabetic() || c == b'_')
                        && k.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_');
                    if ok && !pairs.iter().any(|(q, _)| *q == k) {
                        pairs.push((k, val.trim()));
                    }
                }
                self.tick(pairs.len())?;
                let charge = Charge::new(&self.m, record_cost(pairs.iter().map(|(k, _)| k.as_str())))?;
                let mut out = Vec::new();
                if out.try_reserve_exact(pairs.len()).is_err() {
                    return Err(limit_fault("the computer has no more memory for this program".into()));
                }
                for (k, val) in pairs {
                    out.push((k, text(&self.m, val)?));
                }
                record_charged(charge, out)?
            }
            // ---- dates
            B::DaysBetween => {
                let a = date(&v[0], "days_between(a, b): a")?;
                let z = date(&v[1], "days_between(a, b): b")?;
                Value::Num((z - a) as f64)
            }
            B::Month | B::Year => {
                let t = targ(&v[0], "month(date) and year(date)")?;
                let (y, m, _) = parse_date(t).ok_or_else(|| format!("'{}' is not a date like 2026-11-14", short(t)))?;
                Value::Num(if b == B::Month { m } else { y } as f64)
            }
            _ => return Err("internal: not a library function".into()),
        })
    }
}

/// Rounding half away from zero, without the standard library (the kernel has none).
fn libm_round(x: f64) -> f64 {
    if !(x.abs() < 4.5e15) {
        return x; // whole already
    }
    let t = libm_floor(x.abs() + 0.5);
    if x < 0.0 { -t } else { t }
}

fn libm_floor(x: f64) -> f64 {
    // |x| < 1e12 here (and 2^52 > 1e12), so going through i64 is exact
    if !(x.abs() < 4.5e15) {
        return x;
    }
    let i = x as i64 as f64;
    if i > x { i - 1.0 } else { i }
}
