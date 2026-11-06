//! The inspector: a hidden channel on the serial port for the test harness.
//! The AI player never sees this; it only gets the screen and keyboard.
//!
//! Every reply is "<NAME> <length>\n" followed by exactly <length> bytes, so
//! text on the screen or in pages can never be mistaken for the end of a reply.
//!
//! Commands (one per line):
//!   PING    -> PONG  <session code from the start files>
//!   KEYS    -> KEYS  <number of key presses handled so far>
//!   SCREEN  -> SCREEN <the 25 screen rows as text>
//!   CURSOR  -> CURSOR <row> <column> (counted from 1, like the screen rows)
//!   DUMP    -> DUMP  <every drawer and page, for scoring a task>
//!   STAMPS  -> STAMPS <the history: "BASE <n>", then one "S <n> <from> <date> <note>" line
//!              per stamp (from is "-" for none), then "E <problem>" for each #stamp line of
//!              the start files that failed>

use crate::fs::{self, Entry, Fs};
use crate::history::History;
use crate::screen::{Screen, COLS, ROWS};
use crate::serial::{self, Serial};
use alloc::format;
use alloc::string::String;
use core::fmt::Write;

pub struct Inspector {
    line: String,
}

pub struct State<'a> {
    pub screen: &'a Screen,
    pub fs: &'a Fs,
    pub history: &'a History,
    pub keys_handled: u64,
    pub session: &'a str,
}

impl Inspector {
    pub fn new() -> Self {
        Self { line: String::new() }
    }

    /// Check the serial port; answer a command if a full line arrived.
    pub fn poll(&mut self, state: &State) {
        while let Some(b) = serial::read_byte() {
            match b {
                b'\r' => {}
                b'\n' => {
                    let cmd = core::mem::take(&mut self.line);
                    answer(cmd.trim(), state);
                }
                _ if self.line.len() < 64 => self.line.push(b as char),
                _ => {}
            }
        }
    }
}

fn reply(name: &str, payload: &str) {
    let _ = writeln!(Serial, "{name} {}", payload.len());
    let _ = Serial.write_str(payload);
}

fn answer(cmd: &str, state: &State) {
    match cmd {
        "PING" => reply("PONG", state.session),
        "KEYS" => reply("KEYS", &format!("{}", state.keys_handled)),
        "SCREEN" => {
            let mut text = String::new();
            let mut row = [0u8; COLS];
            for r in 0..ROWS {
                state.screen.text_row(r, &mut row);
                text.push_str(core::str::from_utf8(&row).unwrap_or("").trim_end());
                text.push('\n');
            }
            reply("SCREEN", &text);
        }
        "CURSOR" => {
            let (col, row) = state.screen.cursor();
            reply("CURSOR", &format!("{} {}", row + 1, col + 1));
        }
        "DUMP" => {
            // "D <path>" for a drawer; "P <path> <n>" then n bytes of text for a page.
            let mut text = String::new();
            for (path, entry) in state.fs.all() {
                match entry {
                    Entry::Drawer => text.push_str(&format!("D {}\n", fs::display(path))),
                    Entry::Page(t) => text.push_str(&format!("P {} {}\n{t}\n", fs::display(path), t.len())),
                }
            }
            reply("DUMP", &text);
        }
        "STAMPS" => {
            let h = state.history;
            let mut text = match h.base() {
                Some(b) => format!("BASE {b}\n"),
                None => String::from("BASE -\n"),
            };
            for s in h.stamps() {
                let from = s.from.map_or(String::from("-"), |f| format!("{f}"));
                text.push_str(&format!("S {} {from} {} {}\n", s.number, s.date, s.note));
            }
            for e in &h.start_errors {
                text.push_str(&format!("E {e}\n"));
            }
            reply("STAMPS", &text);
        }
        "" => {}
        _ => reply("ERROR", "unknown inspector command"),
    }
}
