//! Apps (design §5): an app page has `app 'TITLE'` and the tasks start(), key(state, k) and
//! draw(state). The OS runs it one step at a time:
//!
//! - the start step: `start()`, then `draw` with the state it gave;
//! - a key step, for every key: `key(state, k)`, then `draw` with the state it gave (unless
//!   `close()` was called).
//!
//! One step is the task and its draw together: one budget, one read view of the world. A step
//! gives back its planned changes, the new state and the new screen; the caller makes the
//! changes and only then keeps the state and the screen (`App::keep`). A step with a mistake
//! gives nothing: the caller closes the app and shows the mistake.

use alloc::format;
use alloc::rc::Rc;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use crate::limits::*;
use crate::parse::{Program, B};
use crate::run::{mistake, prepare, Action, Mistake, Runner, Running, World};
use crate::value::*;

/// The app's part of the screen: rows 1 to 23, columns 1 to 78.
pub const APP_ROWS: usize = 23;
pub const APP_COLS: usize = 78;
/// The colours, by number.
pub const COLOURS: [&str; 5] = ["normal", "bright", "accent", "warn", "select"];
/// What the bottom bar always has.
pub const CLOSE_HELP: &str = "Ctrl+Q close";

/// The keys an app can get (besides one character, and 'ctrl+a' to 'ctrl+z').
const KEY_NAMES: [&str; 11] = ["enter", "esc", "backspace", "tab", "delete", "up", "down", "left", "right", "home", "end"];

/// Is this a key's name, as key(state, k) gets it?
pub fn is_key(k: &str) -> bool {
    let b = k.as_bytes();
    (b.len() == 1 && (0x20..0x7f).contains(&b[0]))
        || KEY_NAMES.contains(&k)
        || (b.len() == 6 && k.starts_with("ctrl+") && b[5].is_ascii_lowercase())
}

/// One screen function call of a draw, kept in order (so `try` can undo the last ones).
#[derive(Debug, Clone)]
pub(crate) enum ScreenOp {
    Title(String),
    Help(String),
    /// row and col from 1; colour None: keep the colour the place has (normal, or a fill's).
    Show(usize, usize, String, Option<u8>),
    Fill(usize, u8),
    Cursor(usize, usize),
}

impl ScreenOp {
    /// The memory its text was counted with (by Runner::string).
    pub(crate) fn cost(&self) -> usize {
        match self {
            ScreenOp::Title(t) | ScreenOp::Help(t) | ScreenOp::Show(_, _, t, _) => t.len() + OVERHEAD,
            _ => 0,
        }
    }
}

fn whole(v: &Value, what: &str, max: usize) -> Res<usize> {
    match v {
        Value::Num(n) if *n >= 1.0 && *n <= max as f64 && *n == (*n as i64) as f64 => Ok(*n as usize),
        _ => Err(format!("{what} must be a whole number from 1 to {max}, got {}", v.describe()).into()),
    }
}

fn colour(v: &Value) -> Res<u8> {
    let bad = || -> Fault {
        format!("{} is not a colour (normal, bright, accent, warn or select)", v.describe()).into()
    };
    match v {
        Value::Text(t) => COLOURS.iter().position(|c| *c == t.s).map(|i| i as u8).ok_or_else(bad),
        _ => Err(bad()),
    }
}

fn one_line(t: &str, what: &str) -> Res<()> {
    if t.contains('\n') {
        return Err(format!("{what} can't hold a line break: each row is shown with its own call").into());
    }
    Ok(())
}

/// Check a screen call's values and make it. `texts` are its text values, already as text.
pub(crate) fn screen_op(b: B, v: &[Value], mut texts: Vec<String>) -> Res<ScreenOp> {
    let mut text = || texts.pop().unwrap_or_default();
    Ok(match b {
        B::Title => {
            let t = text();
            one_line(&t, "title(...)")?;
            ScreenOp::Title(t)
        }
        B::Help => {
            let t = text();
            one_line(&t, "help(...)")?;
            ScreenOp::Help(t)
        }
        B::Show => {
            let row = whole(&v[0], "show(row, col, text): the row", APP_ROWS)?;
            let col = whole(&v[1], "show(row, col, text): the column", APP_COLS)?;
            let c = match v.get(3) {
                Some(c) => Some(colour(c)?),
                None => None,
            };
            let t = text();
            one_line(&t, "show(...)")?;
            ScreenOp::Show(row, col, t, c)
        }
        B::Fill => ScreenOp::Fill(whole(&v[0], "fill(row, colour): the row", APP_ROWS)?, colour(&v[1])?),
        B::Cursor => ScreenOp::Cursor(
            whole(&v[0], "cursor(row, col): the row", APP_ROWS)?,
            whole(&v[1], "cursor(row, col): the column", APP_COLS)?,
        ),
        _ => return Err("internal: not a screen function".into()),
    })
}

/// What an app shows: its bars, its rows, and where the cursor is (None: hidden).
#[derive(Debug, Clone, PartialEq)]
pub struct AppScreen {
    pub title: String,
    /// The bottom bar, with Ctrl+Q close always in it.
    pub help: String,
    /// (character, colour) for each place, row by row.
    cells: Vec<(char, u8)>,
    /// (row, col), from 1.
    pub cursor: Option<(usize, usize)>,
}

/// Cut a text to the width of the screen.
fn cut(t: &str) -> String {
    t.chars().take(APP_COLS).map(|c| if c.is_control() { ' ' } else { c }).collect()
}

fn with_close(help: &str) -> String {
    let c = cut(help);
    if c.contains(CLOSE_HELP) {
        return c;
    }
    // the help is cut so that Ctrl+Q close always fits
    let room = APP_COLS - CLOSE_HELP.len() - 3;
    let h: String = c.chars().take(room).collect();
    let h = h.trim_end();
    if h.trim().is_empty() {
        CLOSE_HELP.to_string()
    } else {
        format!("{h}   {CLOSE_HELP}")
    }
}

/// What the screen holds for the memory budget (made once per draw).
pub(crate) const SCREEN_COST: usize = APP_ROWS * APP_COLS * 8 + 4 * OVERHEAD;

impl AppScreen {
    /// An empty screen with these bars.
    pub fn new(title: &str) -> Self {
        AppScreen { title: cut(title), help: CLOSE_HELP.into(), cells: vec![(' ', 0); APP_ROWS * APP_COLS], cursor: None }
    }

    fn render(title: &str, ops: &[ScreenOp]) -> Self {
        let mut s = AppScreen::new(title);
        for op in ops {
            match op {
                ScreenOp::Title(t) => s.title = cut(t),
                ScreenOp::Help(t) => s.help = with_close(t),
                ScreenOp::Show(row, col, t, c) => {
                    for (i, ch) in t.chars().enumerate() {
                        if col + i > APP_COLS {
                            break;
                        }
                        let cell = &mut s.cells[(row - 1) * APP_COLS + col - 1 + i];
                        *cell = (if ch.is_control() { ' ' } else { ch }, c.unwrap_or(cell.1));
                    }
                }
                ScreenOp::Fill(row, c) => {
                    for cell in &mut s.cells[(row - 1) * APP_COLS..row * APP_COLS] {
                        cell.1 = *c;
                    }
                }
                ScreenOp::Cursor(row, col) => s.cursor = Some((*row, *col)),
            }
        }
        s
    }

    /// The character and colour (a number into COLOURS) at row 1-23, col 1-78.
    pub fn cell(&self, row: usize, col: usize) -> (char, u8) {
        self.cells[(row - 1) * APP_COLS + col - 1]
    }

    /// A row as text: 0 is the title bar, 24 the help bar, 1-23 the app's rows (from column 1,
    /// without the spaces at the end).
    pub fn line(&self, row: usize) -> String {
        match row {
            0 => self.title.clone(),
            24 => self.help.clone(),
            _ => {
                let s: String = (1..=APP_COLS).map(|c| self.cell(row, c).0).collect();
                s.trim_end().to_string()
            }
        }
    }
}

/// An open app: its checked program, the state kept between steps and its last screen.
pub struct App {
    program: Program,
    title: String,
    state: Value,
    screen: AppScreen,
    started: bool,
}

/// What one step gives: the changes to make, then (with App::keep) the new state and screen.
pub struct Step {
    pub actions: Vec<Action>,
    /// close() was called: after the changes are made, the app closes.
    pub close: bool,
    pub steps: u64,
    state: Value,
    screen: Option<AppScreen>,
}

/// The tasks an app must have: (name, inputs, what it is for).
const APP_TASKS: [(&str, usize, &str); 3] = [
    ("start", 0, "task start() gives the first state"),
    ("key", 2, "task key(state, k) gets every key and gives the new state"),
    ("draw", 1, "task draw(state) shows the state with show(...)"),
];

impl App {
    /// Check an app page (and the pages it uses). It is not started yet.
    pub fn load<W: World>(page: &str, source: &str, world: &W) -> Result<App, Mistake> {
        let program = prepare(page, source, world)?;
        let before = |line: usize, message: String| Mistake {
            page: page.into(),
            line,
            message,
            limit: false,
            before_running: true,
            trail: Vec::new(),
        };
        let Some((title, _)) = program.app.clone() else {
            return Err(before(0, "this page is not an app: an app page starts with app 'TITLE'".into()));
        };
        if let Some(first) = program.main.first() {
            return Err(before(first.n, "an app page may only hold app, use, fixed and task lines".into()));
        }
        for (name, inputs, what) in APP_TASKS {
            match program.tasks.iter().find(|t| t.name == name) {
                None => return Err(before(0, format!("an app needs the task {name}: {what}"))),
                Some(t) if t.params.len() != inputs => {
                    let s = if inputs == 1 { "" } else { "s" };
                    return Err(Mistake {
                        page: program.pages[t.page].clone(),
                        ..before(t.line, format!("{name} must have {inputs} input{s}: {what}"))
                    });
                }
                Some(_) => {}
            }
        }
        let screen = AppScreen::new(&title);
        Ok(App { program, title, state: Value::Nothing, screen, started: false })
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    /// The screen after the last step that was kept.
    pub fn screen(&self) -> &AppScreen {
        &self.screen
    }

    /// The state after the last step that was kept (nothing before the start step).
    pub fn state(&self) -> &Value {
        &self.state
    }

    /// The start step.
    pub fn start<W: World>(&self, world: &W) -> Result<Step, Mistake> {
        self.step(None, world, MAX_STEPS).0
    }

    /// A key step. `key` must be a key's name (is_key).
    pub fn key<W: World>(&self, key: &str, world: &W) -> Result<Step, Mistake> {
        self.key_within(key, world, MAX_STEPS).0
    }

    /// The start step with at most `budget` steps (a test's steps share one budget); also
    /// gives the steps it took, even when it failed.
    pub(crate) fn start_within<W: World>(&self, world: &W, budget: u64) -> (Result<Step, Mistake>, u64) {
        self.step(None, world, budget)
    }

    pub(crate) fn key_within<W: World>(&self, key: &str, world: &W, budget: u64) -> (Result<Step, Mistake>, u64) {
        if !self.started {
            return (Err(Mistake {
                page: self.program.pages[0].clone(),
                line: 0,
                message: "the app has not started".into(),
                limit: false,
                before_running: false,
                trail: Vec::new(),
            }), 0);
        }
        self.step(Some(key), world, budget)
    }

    /// Keep what a step gave, once its changes are made. Gives true if the app closes.
    pub fn keep(&mut self, step: Step) -> bool {
        self.state = step.state;
        self.started = true;
        if let Some(s) = step.screen {
            self.screen = s;
        }
        step.close
    }

    /// One step with at most `budget` steps; gives the steps it took (also when it failed).
    fn step<W: World>(&self, key: Option<&str>, world: &W, budget: u64) -> (Result<Step, Mistake>, u64) {
        let m = Meter::new(STEP_MEMORY);
        let mut r = Runner::new(world, &self.program, &m);
        let start = MAX_STEPS - budget.min(MAX_STEPS);
        let res = self.step_in(&mut r, &m, key, start);
        let used = r.steps.saturating_sub(start);
        (res.map(|s| Step { steps: used, ..s }), used)
    }

    fn step_in<W: World>(&self, r: &mut Runner<'_, W>, m: &Rc<Meter>, key: Option<&str>, start: u64) -> Result<Step, Mistake> {
        let p = &self.program;
        r.fixed_values().map_err(|(n, f)| mistake(p, n, f))?;
        r.steps = start; // (working out the fixed values doesn't count)
        let (task, args) = match key {
            None => ("start", Vec::new()),
            Some(k) => ("key", vec![self.state.clone(), text(m, k).map_err(|f| mistake(p, 0, f))?]),
        };
        let t = p.tasks.iter().find(|t| t.name == task).expect("checked by load");
        let place = (t.page, t.line);
        let at_task = |mut f: Fault| {
            if f.place.is_none() {
                f.place = Some((place, Vec::new()));
            }
            mistake(p, 0, f)
        };
        r.part = if key.is_none() { Running::Start } else { Running::Key };
        let state = r.call_task(task, args).map_err(at_task)?;
        if !r.gave {
            return Err(at_task(format!("{task} ended without give: it must give the app's state (give s)").into()));
        }
        let screen = if r.close {
            None
        } else {
            r.part = Running::Draw;
            let room = Charge::new(m, SCREEN_COST).map_err(at_task)?;
            r.call_task("draw", vec![state.clone()]).map_err(at_task)?;
            let s = AppScreen::render(&self.title, &r.shows);
            drop(room);
            Some(s)
        };
        // the state is kept apart from the step (which is then given back in full)
        let kept = Meter::new(APP_STATE);
        let state = deep_copy(&state, &kept, &mut 0).map_err(|_| {
            at_task(limit_fault(format!("the app's state is too big to keep (at most {} KB)", APP_STATE / 1024)))
        })?;
        Ok(Step { actions: core::mem::take(&mut r.actions), close: r.close, steps: r.steps, state, screen })
    }
}

/// A copy of a value made of new parts counted by `m` (depth is at most MAX_VALUE_DEPTH).
/// `work` counts the parts and characters copied.
pub(crate) fn deep_copy(v: &Value, m: &Rc<Meter>, work: &mut usize) -> Res<Value> {
    *work += 1;
    Ok(match v {
        Value::Text(t) => {
            *work += t.s.len() / TEXT_WORK;
            text(m, &t.s)?
        }
        Value::List(l) => {
            let mut lm = ListMaker::new(m, l.v.len())?;
            for x in l.v.iter() {
                lm.push(deep_copy(x, m, work)?)?;
            }
            lm.finish()
        }
        Value::Rec(r) => {
            let charge = Charge::new(m, record_cost(r.f.iter().map(|(k, _)| k.as_str())))?;
            let mut fields = Vec::new();
            if fields.try_reserve_exact(r.f.len()).is_err() {
                return Err(limit_fault("the computer has no more memory for this program".into()));
            }
            for (k, x) in r.f.iter() {
                fields.push((k.clone(), deep_copy(x, m, work)?));
            }
            record_charged(charge, fields)?
        }
        other => other.clone(),
    })
}

impl<'a, W: World> Runner<'a, W> {
    /// `edit_text(text, key)`: typing into a text.
    pub(crate) fn edit_text(&mut self, v: &[Value]) -> Res<Value> {
        let (Value::Text(t), Value::Text(k)) = (&v[0], &v[1]) else {
            return Err(format!("edit_text(text, key) needs two texts, got {} and {}", v[0].describe(), v[1].describe()).into());
        };
        let (t, k) = (&t.s, &k.s);
        self.tick(t.len() / TEXT_WORK)?;
        if k == "backspace" {
            let end = t.char_indices().last().map_or(0, |(i, _)| i);
            return text(&self.m, &t[..end]);
        }
        let b = k.as_bytes();
        if b.len() == 1 && (0x20..0x7f).contains(&b[0]) {
            if t.len() + 1 > MAX_TEXT {
                return Err(format!("that text would be too long (at most {MAX_TEXT} characters)").into());
            }
            return new_text(&self.m, t.len() + 1, |s| {
                s.push_str(t);
                s.push_str(k);
            });
        }
        Ok(v[0].clone())
    }
}
