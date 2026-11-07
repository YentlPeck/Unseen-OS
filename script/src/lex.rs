//! Words of one line. As chores, plus: `[ ] { } . :` and a doubled quote mark inside a text
//! (`'it''s'`) for one quote mark.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::util::parse_num;

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    Num(f64),
    Text(String),
    /// Always in small letters (capitals don't matter in words and names).
    Word(String),
    Sym(&'static str),
}

/// The chore keywords: they can't be names. The new instruction words (`while`, `skip`,
/// `task`, ...) are not keywords: they are only special as the first word of a line.
pub const KEYWORDS: [&str; 19] = [
    "set", "to", "say", "stop", "when", "otherwise", "done", "for", "each", "in", "repeat", "times", "and", "or",
    "not", "is", "has", "yes", "no",
];

pub fn is_keyword(w: &str) -> bool {
    KEYWORDS.contains(&w)
}

pub fn lex(line: &str) -> Result<Vec<Tok>, String> {
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
            let mut text = String::new();
            let mut j = i + 1;
            loop {
                match line[j..].find(c as char) {
                    None => return Err(format!("the text started with {} is not closed on this line", c as char)),
                    Some(k) => {
                        text.push_str(&line[j..j + k]);
                        j += k + 1;
                        if b.get(j) == Some(&c) {
                            text.push(c as char); // a doubled quote mark is one quote mark
                            j += 1;
                        } else {
                            break;
                        }
                    }
                }
            }
            out.push(Tok::Text(text));
            i = j;
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
                (b'[', _) => ("[", 1),
                (b']', _) => ("]", 1),
                (b'{', _) => ("{", 1),
                (b'}', _) => ("}", 1),
                (b',', _) => (",", 1),
                (b'.', _) => (".", 1),
                (b':', _) => (":", 1),
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

pub fn show_tok(t: &Tok) -> String {
    match t {
        Tok::Num(v) => format!("the number {}", crate::util::fmt_num(*v)),
        Tok::Text(s) => format!("the text '{}'", crate::util::short(s)),
        Tok::Word(w) => format!("the word {w}"),
        Tok::Sym(s) => format!("'{s}'"),
    }
}

#[allow(dead_code)]
pub fn word(t: &Tok) -> Option<&str> {
    match t {
        Tok::Word(w) => Some(w.as_str()),
        _ => None,
    }
}

#[allow(dead_code)]
pub fn text_tok(s: &str) -> Tok {
    Tok::Text(s.to_string())
}
