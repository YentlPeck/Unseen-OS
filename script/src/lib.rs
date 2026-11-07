//! Unseen Script: the programming language of Unseen OS (design: docs/script-design.md).
//!
//! `no_std` with `alloc`: the kernel uses it, and the tests in `tests/` run it on the PC.
//! Milestones M1 to M5: expressions and blocks, tasks and the library, modules and tests,
//! programs and the lab in the OS, and apps (design §11).
#![no_std]

extern crate alloc;

mod app;
mod check;
mod lab;
pub mod lex;
mod library;
mod program;
mod testing;
pub mod parse;
mod run;
pub mod util;
pub mod value;

pub use run::{run, Action, Mistake, Outcome, World};
pub use testing::{run_tests, TestReport, TestResult};
pub use lab::{Lab, LAB_PAGE};
pub use app::{is_key, App, AppScreen, Step, APP_COLS, APP_ROWS, CLOSE_HELP, COLOURS};

/// Limits (design §9). Going over one is a mistake that ends the run, never a crash.
pub mod limits {
    pub const MAX_PAGE_CHARS: usize = 16_000;
    pub const MAX_PAGE_LINES: usize = 400;
    pub const MAX_NESTING: usize = 16;
    pub const MAX_BRACKETS: usize = 12;
    /// Depth of an expression tree (a chain like 1+1+1+... is one level per +).
    pub const MAX_TREE: usize = 48;
    pub const MAX_TEXT: usize = 8_000;
    pub const MAX_LIST: usize = 2_000;
    pub const MAX_VALUE_DEPTH: usize = 16;
    pub const MAX_SAY: usize = 500;
    pub const MAX_ACTIONS: usize = 300;
    pub const MAX_STEPS: u64 = 500_000;
    /// Pages in one program (the main page and the pages it uses).
    pub const MAX_PAGES: usize = 16;
    /// Tasks calling tasks.
    pub const MAX_CALLS: usize = 40;
    /// Values being worked out inside each other, over all running tasks (bounds the stack).
    pub const MAX_LEVEL: usize = 256;
    /// How many levels one task call counts for (it puts several frames on the stack).
    pub const CALL_LEVELS: usize = 4;
    pub const MAX_REPEAT: f64 = 10_000.0;
    pub const STEP_MEMORY: usize = 1024 * 1024;
    /// An app's state, kept between its steps.
    pub const APP_STATE: usize = 256 * 1024;
    /// Screen calls (show, fill, ...) in one draw.
    pub const MAX_SHOWS: usize = 2_000;
    /// All the steps of one test together: its own lines and its app's steps.
    pub const MAX_TEST_STEPS: u64 = 5_000_000;
    /// The checked program: tokens and the parse tree of all its pages.
    pub const PROGRAM_MEMORY: usize = 1024 * 1024;
    /// Counted for every token (and the parse-tree node it may become), plus twice its text.
    pub const TOKEN_COST: usize = 96;
    pub const NUM_LIMIT: f64 = 1e12;
    /// Counted for every text, list and record, on top of its contents.
    pub const OVERHEAD: usize = 32;
    /// Counted for every item of a list.
    pub const ITEM_COST: usize = 32;
    /// Counted for every field of a record (plus the length of its name).
    pub const FIELD_COST: usize = 48;
    /// Counted for every name that has a value.
    pub const NAME_COST: usize = 48;
    /// Counted for every planned change (on top of its texts).
    pub const ACTION_COST: usize = 96;
    /// What a mail page adds to the desk's text besides the texts given to send(...): its
    /// header lines and line ends, as Mail writes them (from: me, to:, subject:, a blank line).
    pub const MAIL_COST: i64 = 25;
    /// What an event page adds besides the texts given to event(...): its field names and
    /// line ends, as Calendar writes them (date:, time:, title:, with:).
    pub const EVENT_COST: i64 = 28;
    /// Text work: one step per this many characters (copying, comparing, searching).
    pub const TEXT_WORK: usize = 64;
    /// Comparing two texts: one step per this many characters (a fast memory compare).
    pub const COMPARE_WORK: usize = 256;
}
