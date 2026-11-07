//! Fuzzing (design §10): many random, broken and hostile programs. None may panic, go over the
//! step or memory budget, report changes together with a mistake, or overflow the stack.
extern crate alloc;

#[path = "../../kernel/src/fs.rs"]
#[allow(dead_code)]
mod fs;
mod common;

use common::TestWorld;
use script::limits::*;

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

fn check(src: &str, world: &TestWorld) {
    let o = script::run("main", src, world);
    assert!(o.steps <= MAX_STEPS + 1, "steps {} for {src:?}", o.steps);
    if o.error.is_some() {
        assert!(o.actions.is_empty(), "mistake with changes for {src:?}");
    }
    assert!(o.output.len() <= MAX_SAY);
    assert!(o.actions.len() <= MAX_ACTIONS);
    assert!(o.peak_memory <= STEP_MEMORY, "memory {} for {src:?}", o.peak_memory);
}

const PIECES: &[&str] = &[
    "set", "to", "say", "stop", "when", "otherwise", "done", "for", "each", "in", "repeat", "times", "and", "or",
    "not", "is", "has", "yes", "no", "x", "y", "total", "(", ")", ",", "+", "-", "*", "/", "&", "<", ">", "<=",
    ">=", "=", "--", "'a'", "\"b c\"", "'", "''", "0", "1", "2.5", "999999999999", "10000", "pages('mail:inbox')",
    "read(", "field(", "body(", "exists(", "lines(", "words(", "count(", "item(", "number(", "lower(", "today()",
    "weekday(", "add_days(", "write(", "add(", "toss(", "send(", "event(", "'mail:inbox:m1'", "'desk:out'",
    "'contacts'", "'omar'", "'2026-11-12'", "\n", "\n", "\n", "é", "\t", "[", "]", "{", "}", ".", ":", "while",
    "skip", "quit", "at", "or when", "nothing()", "x.a", "{a: 1}", "[1, 2]", "add 1 to x", "set x.a to",
];

#[test]
fn random_token_soup() {
    let world = TestWorld::new();
    let mut rng = Rng(0x2545F4914F6CDD1D);
    for _ in 0..30_000 {
        let n = 1 + rng.below(40);
        let src: Vec<&str> = (0..n).map(|_| rng.pick(PIECES)).collect();
        check(&src.join(" "), &world);
    }
}

fn expr(rng: &mut Rng, depth: usize) -> String {
    if depth == 0 || rng.below(3) == 0 {
        return rng
            .pick(&[
                "1", "x", "'mail:inbox:m1'", "'a b'", "yes", "today()", "pages('mail:inbox')", "0", "-3", "y",
                "[]", "{}", "[1, 'a']", "{a: 1, b: [2]}", "nothing()", "x.a", "y.b", "[x, y]", "{a: x}",
            ])
            .to_string();
    }
    let a = expr(rng, depth - 1);
    let b = expr(rng, depth - 1);
    match rng.below(16) {
        0 => format!("({a} + {b})"),
        1 => format!("{a} & {b}"),
        2 => format!("{a} is {b}"),
        3 => format!("not {a}"),
        4 => format!("{a} has {b}"),
        5 => format!("count({a})"),
        6 => format!("field({a}, 'from')"),
        7 => format!("item({a}, {b})"),
        8 => format!("[{a}, {b}]"),
        9 => format!("{{a: {a}, b: {b}}}"),
        10 => format!("({a}).a"),
        11 => format!("words({a})"),
        12 => format!("{a} and {b}"),
        13 => format!("[{a}, {a}, {a}, {a}]"),
        14 => format!("{a} is not {b}"),
        _ => format!("{a} / {b}"),
    }
}

fn program(rng: &mut Rng, depth: usize, out: &mut String) {
    for _ in 0..1 + rng.below(4) {
        match rng.below(14) {
            0 => out.push_str(&format!("set x to {}\n", expr(rng, 3))),
            1 => out.push_str(&format!("set y to {}\n", expr(rng, 3))),
            2 => out.push_str(&format!("say {}\n", expr(rng, 3))),
            3 if depth > 0 => {
                out.push_str(&format!("when {}\n", expr(rng, 2)));
                program(rng, depth - 1, out);
                if rng.below(2) == 0 {
                    out.push_str(&format!("or when {}\n", expr(rng, 2)));
                    program(rng, depth - 1, out);
                }
                if rng.below(2) == 0 {
                    out.push_str("otherwise\n");
                    program(rng, depth - 1, out);
                }
                out.push_str("done\n");
            }
            4 if depth > 0 => {
                out.push_str(&format!("for each x in {}\n", expr(rng, 2)));
                program(rng, depth - 1, out);
                out.push_str("done\n");
            }
            5 if depth > 0 => {
                out.push_str(&format!("repeat {} times\n", rng.pick(&["0", "3", "100", "10000", "x"])));
                program(rng, depth - 1, out);
                out.push_str("done\n");
            }
            6 if depth > 0 => {
                out.push_str(&format!("while {}\n", expr(rng, 2)));
                program(rng, depth - 1, out);
                out.push_str(rng.pick(&["quit\n", "skip\n", ""]));
                out.push_str("done\n");
            }
            7 => out.push_str(&format!("write('desk:out', {})\n", expr(rng, 2))),
            8 => out.push_str(&format!("add {} to {}\n", expr(rng, 2), rng.pick(&["x", "y", "x.a", "y.b"]))),
            9 => out.push_str(&format!("set {} to {}\n", rng.pick(&["x.a", "y.b", "x at 1", "x.a at 2", "y.b.c"]), expr(rng, 2))),
            10 => out.push_str("set x to [x, x]\n"),
            11 => out.push_str("set y to {a: y, b: y}\n"),
            12 => out.push_str(&format!("add('desk:log', {})\n", expr(rng, 2))),
            _ => out.push_str("set x to x & x\n"),
        }
    }
}

#[test]
fn random_structured_programs() {
    let world = TestWorld::new();
    let mut rng = Rng(0x9E3779B97F4A7C15);
    for _ in 0..30_000 {
        let mut src = String::new();
        src.push_str(rng.pick(&["set x to [1]\nset y to {b: [1]}\n", "set x to {a: [1, 2]}\nset y to [{b: 1}]\n", ""]));
        program(&mut rng, 4, &mut src);
        check(&src, &world);
    }
}

#[test]
fn hostile_programs() {
    let world = TestWorld::new();
    let t = "x".repeat(7990);
    let hostile = [
        "(".repeat(5000),
        "[".repeat(5000),
        "{a: ".repeat(3000),
        format!("say {}", "not ".repeat(3000)),
        format!("say {}", "-".repeat(3000)),
        format!("say x{}", ".a".repeat(3000)),
        "when yes\n".repeat(399),
        "done\n".repeat(399),
        format!("say {}1", "1 + ".repeat(1000)),
        "'".repeat(15000),
        "x".repeat(15999),
        "set x to 'a'\nrepeat 10000 times\nset x to x & x\ndone".into(),
        "set x to [1]\nrepeat 10000 times\nset x to [x, x]\ndone".into(),
        "set x to {a: 1}\nrepeat 10000 times\nset x to {a: x, b: x}\ndone".into(),
        "set x to [1]\nrepeat 10000 times\nadd x to x\ndone".into(),
        // doubling: each step doubles the work of comparing and turning into text
        "set x to [1]\nrepeat 15 times\nset x to [x, x]\ndone\nrepeat 10000 times\nset y to x is x\nset z to x & ''\ndone".into(),
        format!("set t to '{t}'\nset l to []\nrepeat 2000 times\nadd t & '' to l\ndone"),
        format!("set t to '{t}'\nset l to []\nrepeat 2000 times\nadd t to l\ndone\nrepeat 10000 times\nsay l is l\ndone"),
        format!("set t to '{t}'\nset l to []\nrepeat 2000 times\nadd t to l\ndone\nsay l"),
        format!("set t to '{t}'\nset r to {{}}\nrepeat 2000 times\nset r.a to t & ''\ndone"),
        "while yes\ndone".into(),
        "set l to []\nwhile yes\nadd l to l\ndone".into(),
        "repeat 10000 times\nsay 1\ndone".into(),
        "repeat 10000 times\nadd('desk:log', 'x')\ndone".into(),
        format!("say '{}' has 'b'", "a".repeat(7900)),
        "\r\n\r\nsay 1\r\n".into(),
        "\0\0\0".into(),
        "say 1\n".repeat(400),
    ];
    for src in hostile {
        check(&src, &world);
    }
}

/// The deepest programs the limits allow must fit in a 128 KB stack (release build; the
/// kernel stack is 256 KB). Debug frames are much bigger, so this runs in release only.
#[test]
#[cfg_attr(debug_assertions, ignore = "debug frames are much bigger; the kernel is a release build")]
fn worst_case_fits_in_128k_stack() {
    let handle = std::thread::Builder::new()
        .stack_size(128 * 1024)
        .spawn(|| {
            let world = TestWorld::new();
            let blocks = |inner: &str, n: usize| format!("{}{inner}\n{}", "when yes\n".repeat(n), "done\n".repeat(n));
            // deepest expression in the deepest blocks
            let e = format!("say {}1{} & 1{}", "(".repeat(12), ")".repeat(12), " + 1".repeat(44));
            let o = script::run("main", &blocks(&e, 16), &world);
            assert_eq!(o.error, None);
            // deepest value: compared, turned into text, copied and freed
            let v = "set l to 1\nrepeat 8 times\nset l to [l, {a: l}]\ndone\nset m to l\nadd 1 to m\nsay l is m\nsay count(m)";
            let o = script::run("main", &blocks(v, 15), &world);
            assert_eq!(o.error, None);
            assert_eq!(o.output, vec!["no", "3"]);
            let v = "set r to {a: 1}\nrepeat 15 times\nset r to {a: r}\ndone\nset s to r\nset s.a.a.a.a.a.a.a.a.a.a.a.a.a.a.a to 2\nsay r is s";
            let o = script::run("main", &blocks(v, 15), &world);
            assert_eq!(o.error, None);
            assert_eq!(o.output, vec!["no"]);
        })
        .unwrap();
    handle.join().expect("ran out of stack");
}

/// Tasks calling tasks as deep as allowed, each call from inside deep blocks and a long line.
#[test]
#[cfg_attr(debug_assertions, ignore = "debug frames are much bigger; the kernel is a release build")]
fn deepest_tasks_fit_in_128k_stack() {
    let handle = std::thread::Builder::new()
        .stack_size(128 * 1024)
        .spawn(|| {
            let world = TestWorld::new();
            for brackets in [0usize, 5, 11] {
                let open = "(1 + ".repeat(brackets);
                let close = ")".repeat(brackets);
                let whens = "when yes\n".repeat(14);
                let dones = "done\n".repeat(14);
                let src = format!(
                    "task down(n)\n{whens}when n is 0\ngive 0\ndone\ngive {open}down(n - 1){close}\n{dones}done\nsay down(39)"
                );
                let o = script::run("main", &src, &world);
                let ok = o.error.is_none() || o.error.as_ref().unwrap().limit;
                assert!(ok, "{brackets}: {:?}", o.error);
                println!("brackets {brackets}: {:?}", o.error.map(|e| e.message));
            }
        })
        .unwrap();
    handle.join().expect("ran out of stack");
}

/// Random programs with tasks (calling each other, recursion), give and fixed values.
#[test]
fn random_programs_with_tasks() {
    let world = TestWorld::new();
    let mut rng = Rng(0xD1B54A32D192ED03);
    for _ in 0..15_000 {
        let mut src = String::new();
        src.push_str(rng.pick(&["fixed k is 3\n", "fixed k is [1, {a: 2}]\n", "fixed k is upper('a') & newline()\n", ""]));
        let tasks = ["f", "g", "h"];
        for t in tasks.iter().take(1 + rng.below(3)) {
            src.push_str(&format!("task {t}(x, y)\n"));
            let mut body = String::new();
            program(&mut rng, 2, &mut body);
            src.push_str(&body);
            match rng.below(5) {
                0 => src.push_str("give x\n"),
                1 => src.push_str(&format!("give {}(y, x)\n", rng.pick(&tasks))),
                2 => src.push_str(&format!("when count([x]) > 0\n  give [x, {}(x, y)]\ndone\n", rng.pick(&tasks))),
                3 => src.push_str("give\n"),
                _ => {}
            }
            src.push_str("done\n");
        }
        let mut main = String::new();
        program(&mut rng, 3, &mut main);
        src.push_str(&main);
        for _ in 0..1 + rng.below(3) {
            let t = rng.pick(&tasks);
            src.push_str(&match rng.below(3) {
                0 => format!("say {t}(x, y)\n"),
                1 => format!("{t}(1, [k])\n"),
                _ => format!("set x to {t}(x, {t}(y, x))\n"),
            });
        }
        check(&src, &world);
    }
}

/// Deep try blocks inside deep task calls (a try is a block level too).
#[test]
#[cfg_attr(debug_assertions, ignore = "debug frames are much bigger; the kernel is a release build")]
fn deepest_tries_fit_in_128k_stack() {
    let handle = std::thread::Builder::new()
        .stack_size(128 * 1024)
        .spawn(|| {
            let world = TestWorld::new();
            let tries = "try\n".repeat(7);
            let insteads = "instead p\ndone\n".repeat(7);
            let src = format!("task down(n)\n{tries}when n is 0\ngive 0\ndone\ngive 1 + down(n - 1)\n{insteads}done\nsay down(39)");
            let o = script::run("main", &src, &world);
            assert!(o.error.is_none() || o.error.as_ref().unwrap().limit, "{:?}", o.error);
            // failing at the bottom: every try catches and fails again
            let src = format!("task down(n)\n  when n is 0\n    fail 'bottom'\n  done\n  try\n    give down(n - 1)\n  instead p\n    fail p & '.'\n  done\ndone\ntry\n  say down(39)\ninstead p\n  say count(p)\ndone");
            let o = script::run("main", &src, &world);
            assert!(o.error.is_none() || o.error.as_ref().unwrap().limit, "{:?}", o.error);
            println!("{:?} {:?}", o.output, o.error.map(|e| e.message));
        })
        .unwrap();
    handle.join().expect("ran out of stack");
}

/// Random programs with try, instead and fail.
#[test]
fn random_programs_with_try() {
    let world = TestWorld::new();
    let mut rng = Rng(0xA0761D6478BD642F);
    for _ in 0..15_000 {
        let mut src = String::from("set x to [1]\nset y to {b: 1}\n");
        let mut body = String::new();
        program(&mut rng, 3, &mut body);
        for line in body.lines() {
            match rng.below(8) {
                0 => src.push_str(&format!("try\n{line}\n  fail 'x'\ninstead p\n  say p\ndone\n")),
                1 => src.push_str(&format!("try\n{line}\ninstead p\n  add p to x\ndone\n")),
                2 => src.push_str("fail [x, y]\n"),
                _ => {
                    src.push_str(line);
                    src.push('\n');
                }
            }
        }
        check(&src, &world);
    }
}
