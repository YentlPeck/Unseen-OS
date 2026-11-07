//! "Old chores keep working" (design §2, §10): many random chores run through the chore engine
//! (kernel/src/chore.rs, unchanged) and through Unseen Script. Every chore that finishes in the
//! chore engine must finish in Unseen Script with the same say lines and the same changes. And
//! a chore that stops with a mistake (not a limit) must stop in Unseen Script too.
extern crate alloc;

#[path = "../../kernel/src/fs.rs"]
#[allow(dead_code)]
mod fs;
#[path = "../../kernel/src/chore.rs"]
#[allow(dead_code)]
mod chore;
mod common;

use common::TestWorld;

impl chore::World for TestWorld {
    fn page(&self, path: &str) -> Option<String> {
        script::World::page(self, path)
    }
    fn is_drawer(&self, path: &str) -> bool {
        script::World::is_drawer(self, path)
    }
    fn pages_in(&self, drawer: &str) -> Vec<String> {
        script::World::pages_in(self, drawer)
    }
    fn check_write(&self, path: &str) -> Result<(), String> {
        script::World::check_write(self, path)
    }
    fn check_toss(&self, path: &str) -> Result<(), String> {
        script::World::check_toss(self, path)
    }
    fn check_send(&self, to: &str, subject: &str, text: &str) -> Result<(), String> {
        script::World::check_send(self, to, subject, text)
    }
    fn check_event(&self, date: &str, time: &str, title: &str, with: &str) -> Result<(), String> {
        script::World::check_event(self, date, time, title, with)
    }
    fn today(&self) -> String {
        script::World::today(self)
    }
}

fn same_action(a: &chore::Action, b: &script::Action) -> bool {
    use chore::Action as C;
    use script::Action as S;
    match (a, b) {
        (C::Write { page, text }, S::Write { page: p, text: t }) => page == p && text == t,
        (C::Add { page, line }, S::Add { page: p, line: l }) => page == p && line == l,
        (C::Toss { page }, S::Toss { page: p }) => page == p,
        (C::Send { to, subject, text }, S::Send { to: t, subject: s, text: x }) => to == t && subject == s && text == x,
        (C::Event { date, time, title, with }, S::Event { date: d, time: t, title: ti, with: w }) => {
            date == d && time == t && title == ti && with == w
        }
        _ => false,
    }
}

/// Limits that are bigger in Unseen Script: a chore that only failed on one of these may finish.
fn chore_limit(e: &str) -> bool {
    ["too many steps", "too many say lines", "too many changes", "too long", "too many different names",
     "too many lines", "items"]
        .iter()
        .any(|k| e.contains(k))
}

/// Compare one chore; returns true if it finished in the chore engine.
/// Make a chore's changes the way the kernel does (kernel/src/chores.rs): a chore whose changes
/// can't all be made changes nothing and counts as stopped.
fn commit(world: &TestWorld, actions: &[chore::Action]) -> Result<(), String> {
    let mut copy = world.fs.clone();
    for a in actions {
        match a {
            chore::Action::Write { page, text } => {
                if text.len() > 16 * 1024 {
                    return Err("too long".into());
                }
                copy.write_page(page, text)?;
            }
            chore::Action::Add { page, line } => {
                let old = copy.read_page(page).map(|t| t.to_string()).unwrap_or_default();
                let new = if old.is_empty() { line.clone() } else { format!("{old}
{line}") };
                if new.len() > 16 * 1024 {
                    return Err("too long".into());
                }
                copy.write_page(page, &new)?;
            }
            chore::Action::Toss { page } => copy.toss(page)?,
            _ => {}
        }
    }
    Ok(())
}

fn compare(src: &str, world: &TestWorld) -> bool {
    let mut c = chore::run(src, world);
    if c.error.is_none() {
        if let Err(e) = commit(world, &c.actions) {
            c.error = Some(format!("its changes can't be made: {e}"));
            c.actions.clear();
        }
    }
    let s = script::run("main", src, world);
    match &c.error {
        None => {
            assert!(s.error.is_none(), "chore finished but script failed: {:?}\n--\n{src}", s.error.map(|e| e.to_string()));
            assert_eq!(c.output, s.output, "say lines differ for:\n{src}");
            assert_eq!(c.actions.len(), s.actions.len(), "changes differ for:\n{src}");
            for (a, b) in c.actions.iter().zip(s.actions.iter()) {
                assert!(same_action(a, b), "change {a:?} vs {b:?} for:\n{src}");
            }
            true
        }
        Some(e) if !chore_limit(e) && !e.contains("I don't understand") && !e.contains("didn't expect the text")
            && !e.contains("a line must start with") && !e.contains("I didn't expect")
            // deliberate extension: a list has any value (chores: text only)
            && !e.contains("has needs") =>
        {
            // a real mistake in a chore is a mistake in Unseen Script too (new syntax aside)
            assert!(s.error.is_some(), "chore failed ({e}) but script finished:\n{src}");
            false
        }
        Some(_) => false,
    }
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    fn pick<'a>(&mut self, xs: &[&'a str]) -> &'a str {
        xs[self.below(xs.len())]
    }
}

fn expr(rng: &mut Rng, depth: usize) -> String {
    if depth == 0 || rng.below(3) == 0 {
        return rng
            .pick(&[
                "1", "x", "'mail:inbox:m1'", "'a b'", "yes", "today()", "pages('mail:inbox')", "0", "-3", "y",
                "2.5", "'contacts:lina'", "'notes:shopping'", "lines(read('notes:shopping'))", "'2026-11-12'", "no",
            ])
            .to_string();
    }
    let a = expr(rng, depth - 1);
    let b = expr(rng, depth - 1);
    match rng.below(20) {
        0 => format!("({a} + {b})"),
        1 => format!("{a} & {b}"),
        2 => format!("{a} is {b}"),
        3 => format!("not {a}"),
        4 => format!("{a} has {b}"),
        5 => format!("count({a})"),
        6 => format!("field({a}, 'from')"),
        7 => format!("item({a}, {b})"),
        8 => format!("{a} / {b}"),
        9 => format!("words({a})"),
        10 => format!("{a} and {b}"),
        11 => format!("add_days({a}, {b})"),
        12 => format!("{a} < {b}"),
        13 => format!("({a} * {b})"),
        14 => format!("lower({a})"),
        15 => format!("{a} or {b}"),
        16 => format!("number({a})"),
        17 => format!("weekday({a})"),
        18 => format!("body({a})"),
        _ => format!("exists({a})"),
    }
}

fn program(rng: &mut Rng, depth: usize, out: &mut String) {
    for _ in 0..1 + rng.below(4) {
        match rng.below(11) {
            0 => out.push_str(&format!("set x to {}\n", expr(rng, 3))),
            1 => out.push_str(&format!("set y to {}\n", expr(rng, 3))),
            2 => out.push_str(&format!("say {}\n", expr(rng, 3))),
            3 => out.push_str(&format!("say ({})\n", expr(rng, 3))),
            4 if depth > 0 => {
                out.push_str(&format!("when ({})\n", expr(rng, 2)));
                program(rng, depth - 1, out);
                if rng.below(2) == 0 {
                    out.push_str("otherwise\n");
                    program(rng, depth - 1, out);
                }
                out.push_str("done\n");
            }
            5 if depth > 0 => {
                out.push_str(&format!("for each x in {}\n", expr(rng, 2)));
                program(rng, depth - 1, out);
                out.push_str("done\n");
            }
            6 if depth > 0 => {
                out.push_str(&format!("repeat {} times\n", rng.pick(&["0", "3", "100", "x"])));
                program(rng, depth - 1, out);
                out.push_str("done\n");
            }
            7 => out.push_str(&format!("write('desk:out', {})\n", expr(rng, 2))),
            8 => out.push_str(&format!("add('notes:log', {})\n", expr(rng, 2))),
            9 => out.push_str(&format!("send('omar', 'Hi', {})\n", expr(rng, 2))),
            _ => out.push_str(rng.pick(&["set x to x & x\n", "stop\n", "toss('notes:shopping')\n", "set y to 0\n"])),
        }
    }
}

#[test]
fn random_chores_give_the_same_result() {
    let world = TestWorld::new();
    let mut rng = Rng(0x9E3779B97F4A7C15);
    let mut finished = 0;
    for _ in 0..60_000 {
        let mut src = String::new();
        // most random chores fail early because x has no value; give x and y one first
        src.push_str(rng.pick(&["set x to 1\nset y to 2\n", "set x to 'notes:shopping'\nset y to 'a'\n",
            "set x to words('a b c')\nset y to 2\n", "set x to yes\nset y to no\n", ""]));
        program(&mut rng, 4, &mut src);
        if compare(&src, &world) {
            finished += 1;
        }
    }
    // the comparison is only worth something if many chores finish
    assert!(finished > 8_000, "only {finished} random chores finished");
}

#[test]
fn hand_written_chores_give_the_same_result() {
    let world = TestWorld::new();
    let chores = [
        "set total to 0\nfor each m in pages('mail:inbox')\n  when field(m, 'from') is 'omar'\n    set total to total + 1\n  done\ndone\nsay 'Omar sent ' & total & ' mails'\nwrite('desk:answer', total)",
        "for each c in pages('contacts')\n  add('desk:phones', field(c, 'name') & ': ' & field(c, 'phone'))\ndone",
        "for each m in pages('mail:inbox')\n  when lower(field(m, 'subject')) has 'lesson'\n    say field(m, 'from') & ' - ' & field(m, 'subject')\n  done\ndone",
        "set d to today()\nset added to 0\nrepeat 21 times\n  set d to add_days(d, 1)\n  when weekday(d) is 'friday' and added < 3\n    event(d, '17:00', 'Drinks', '')\n    set added to added + 1\n  done\ndone",
        "set skip to 1\nset quit to 2\nset while to 3\nset task to 4\nset give to 5\nset nothing to 6\nset at to 7\nsay skip + quit + while + task + give + nothing + at",
        "set add to 'x'\nadd('desk:log', add)\nsay add",
        "say pages('mail:inbox')\nsay words('a b') & '!'\nsay lines(read('notes:shopping'))",
        "set n to 0\nfor each w in words('a b c')\n  set n to n + 1\ndone\nsay w & n",
    ];
    for c in chores {
        assert!(compare(c, &world), "chore did not finish:\n{c}");
    }
    // Sol's review: comparing big texts many times must not use up the (bigger) step budget
    let big = format!("set x to '{}'\nrepeat 2000 times\n  set y to x is x\ndone\nsay y", "a".repeat(4000));
    assert!(compare(&big, &world), "big comparison chore did not finish");
    let big = format!("set x to '{}'\nrepeat 500 times\n  set y to x has 'b'\n  set z to x < x\ndone\nsay y", "a".repeat(4000));
    assert!(compare(&big, &world), "big has chore did not finish");
}
