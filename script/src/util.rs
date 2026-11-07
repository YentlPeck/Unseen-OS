//! Small helpers shared by the engine. Numbers, dates, natural order, paths and page fields
//! work exactly as in the chore engine (kernel/src/chore.rs), so old chores give the same
//! results.

use alloc::format;
use alloc::string::{String, ToString};
use core::cmp::Ordering;

use crate::limits::NUM_LIMIT;

/// Numbers as shown and written: whole numbers in full, others with up to 6 decimals
/// (rounded there), never "-0".
pub fn fmt_num(v: f64) -> String {
    let whole = v as i64; // |v| < 1e12, so exact for whole numbers
    if whole as f64 == v {
        return format!("{whole}");
    }
    let s = format!("{v:.6}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else { s.into() }
}

pub fn parse_num(s: &str) -> Option<f64> {
    let v: f64 = s.parse().ok()?;
    (v.abs() < NUM_LIMIT).then_some(v)
}

/// `number(...)`: digits, an optional point with digits, an optional minus in front.
pub fn number_of(t: &str) -> Option<f64> {
    let s = t.trim();
    let digits = s.strip_prefix('-').unwrap_or(s);
    let mut parts = digits.splitn(2, '.');
    let a = parts.next().unwrap_or("");
    let b = parts.next();
    let ok = !a.is_empty()
        && a.bytes().all(|c| c.is_ascii_digit())
        && b.map_or(true, |b| !b.is_empty() && b.bytes().all(|c| c.is_ascii_digit()));
    let n = if ok && s.len() <= 30 { parse_num(digits) } else { None }?;
    Some(if s.starts_with('-') { -n } else { n })
}

pub fn days_in_month(y: i64, m: i64) -> i64 {
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31][(m - 1) as usize]
}

/// (year, month, day) of a real date written YYYY-MM-DD.
pub fn parse_date(s: &str) -> Option<(i64, i64, i64)> {
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
pub fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

pub fn civil_from_days(z: i64) -> (i64, i64, i64) {
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
pub fn natural(a: &str, b: &str) -> Ordering {
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

// ---- paths (as kernel/src/fs.rs; internal paths have no "desk:" and the desk is "") ----

pub fn display(path: &str) -> String {
    if path.is_empty() { "desk".to_string() } else { format!("desk:{path}") }
}

pub fn parent(path: &str) -> &str {
    path.rfind(':').map_or("", |i| &path[..i])
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

/// A path as a program writes it ("mail:inbox:m1" or "desk:notes"), as an internal path.
/// Paths always start at the desk.
pub fn path_of(s: &str) -> Result<String, String> {
    let s = s.trim();
    if s == "desk" {
        return Ok(String::new());
    }
    let rest = s.strip_prefix("desk:").unwrap_or(s);
    let mut path = String::new();
    for part in rest.split(':') {
        if !valid_name(part) {
            return Err(format!("'{}' is not a valid page or drawer path", short(s)));
        }
        path = join(&path, part);
    }
    Ok(path)
}

/// The value of a `key: value` line at the top of a page (up to the first empty line).
pub fn field_of<'a>(text: &'a str, key: &str) -> &'a str {
    for line in text.lines() {
        if let Some((k, v)) = line.split_once(':') {
            if k.trim().eq_ignore_ascii_case(key) {
                return v.trim();
            }
        }
        if line.is_empty() {
            break;
        }
    }
    ""
}

/// The text after the first empty line.
pub fn body_of(text: &str) -> &str {
    let mut at = 0;
    for line in text.split('\n') {
        at += line.len() + 1;
        if line.trim_end_matches('\r').is_empty() {
            return text.get(at.min(text.len())..).unwrap_or("");
        }
    }
    ""
}

/// At most the first 20 characters (for mistake messages).
pub fn short(s: &str) -> &str {
    s.char_indices().nth(20).map_or(s, |(i, _)| &s[..i])
}
