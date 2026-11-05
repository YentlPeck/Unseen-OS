//! Calculator: type a sum like (12 + 3) * 2 and press Enter.

use crate::keyboard::Key;
use crate::screen::{Screen, ACCENT, NORMAL, WARN};
use crate::ui::{self, Edit, LineEdit};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

pub struct Calc {
    input: LineEdit,
    history: Vec<String>,
    last: String,
    message: String,
}

impl Calc {
    pub fn new() -> Self {
        Self { input: LineEdit::new("", 60), history: Vec::new(), last: String::new(), message: String::new() }
    }

    /// Returns true when the app should close.
    pub fn key(&mut self, key: Key) -> bool {
        self.message.clear();
        match key {
            Key::Escape => return true,
            Key::Enter => {
                let expr = self.input.text.trim().to_string();
                if expr.is_empty() {
                    return false;
                }
                if self.input.overflow {
                    self.message = ui::OVERFLOW_MSG.into();
                    return false;
                }
                match evaluate(&expr).and_then(show) {
                    Ok(result) => {
                        self.history.push(expr.clone());
                        self.history.push(format!("  = {result}"));
                        self.last = expr;
                        self.input = LineEdit::new("", 60);
                    }
                    Err(e) => self.message = format!("Can't work that out: {e}."),
                }
            }
            Key::Up => self.input = LineEdit::new(&self.last, 60),
            _ => {
                if self.input.key(key) == Edit::Full {
                    self.message = "The sum is too long (60 characters).".into();
                }
            }
        }
        false
    }

    pub fn draw(&self, screen: &mut Screen) {
        ui::bars(screen, "Calculator", "+ - * / ( )", "type a sum, Enter works it out   Up: last sum   Esc leave", "");
        ui::clear_body(screen);
        let shown = &self.history[self.history.len().saturating_sub(18)..];
        for (i, line) in shown.iter().enumerate() {
            screen.put_str(2, 2 + i, line, NORMAL);
        }
        if self.history.is_empty() {
            screen.put_str(2, 2, "Example: (12 + 3) * 2   then press Enter", NORMAL);
        }
        if !self.message.is_empty() {
            screen.put_str(2, 21, &self.message, WARN);
        }
        screen.put_str(2, 22, "> ", ACCENT);
        let col = self.input.draw(screen, 4, 22, 70, NORMAL);
        screen.set_cursor(col, 22);
    }
}

const LIMIT: f64 = 1e12;

/// Show a result exactly when possible. Whole numbers are shown in full;
/// other numbers with up to 6 decimals, marked "about" if that rounds them.
/// Numbers of a million million or more are refused instead of shown wrong.
fn show(v: f64) -> Result<String, &'static str> {
    let size = if v < 0.0 { -v } else { v };
    if !(size < LIMIT) {
        return Err("the answer is too big (it must be below 1000000000000)");
    }
    let whole = v as i64; // exact: |v| < 1e12
    if whole as f64 == v {
        return Ok(format!("{whole}"));
    }
    let s = format!("{v:.6}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    let s = if s == "-0" { "0" } else { s };
    let exact = s.parse::<f64>().map_or(false, |back| back == v);
    Ok(if exact { s.to_string() } else { format!("about {s}") })
}

pub fn evaluate(expr: &str) -> Result<f64, &'static str> {
    let mut p = Parser { s: expr.as_bytes(), i: 0 };
    let v = p.sum()?;
    p.space();
    if p.i < p.s.len() {
        return Err("unexpected character");
    }
    Ok(v)
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
}

impl Parser<'_> {
    fn space(&mut self) {
        while self.s.get(self.i) == Some(&b' ') {
            self.i += 1;
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.space();
        self.s.get(self.i).copied()
    }

    fn sum(&mut self) -> Result<f64, &'static str> {
        let mut v = self.product()?;
        while let Some(op @ (b'+' | b'-')) = self.peek() {
            self.i += 1;
            let r = self.product()?;
            v = if op == b'+' { v + r } else { v - r };
        }
        Ok(v)
    }

    fn product(&mut self) -> Result<f64, &'static str> {
        let mut v = self.factor()?;
        while let Some(op @ (b'*' | b'/' | b'x')) = self.peek() {
            self.i += 1;
            let r = self.factor()?;
            if op == b'/' {
                if r == 0.0 {
                    return Err("division by zero");
                }
                v /= r;
            } else {
                v *= r;
            }
        }
        Ok(v)
    }

    fn factor(&mut self) -> Result<f64, &'static str> {
        match self.peek() {
            Some(b'-') => {
                self.i += 1;
                Ok(-self.factor()?)
            }
            Some(b'(') => {
                self.i += 1;
                let v = self.sum()?;
                if self.peek() != Some(b')') {
                    return Err("missing )");
                }
                self.i += 1;
                Ok(v)
            }
            Some(c) if c.is_ascii_digit() || c == b'.' => {
                let start = self.i;
                while self.s.get(self.i).is_some_and(|c| c.is_ascii_digit() || *c == b'.') {
                    self.i += 1;
                }
                let text = core::str::from_utf8(&self.s[start..self.i]).unwrap_or("");
                let v = text.parse::<f64>().map_err(|_| "bad number")?;
                if v >= LIMIT {
                    return Err("numbers must be below 1000000000000");
                }
                Ok(v)
            }
            _ => Err("expected a number"),
        }
    }
}
