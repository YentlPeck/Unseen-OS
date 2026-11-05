//! Fuzz tests: thousands of random and hostile chores must never panic, never
//! exceed the step limit, and never report changes together with an error.
use crate::chore::{self, MAX_STEPS};
use crate::tests::TestWorld;

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

const PIECES: &[&str] = &[
    "set", "to", "say", "stop", "when", "otherwise", "done", "for", "each", "in", "repeat", "times", "and", "or",
    "not", "is", "has", "yes", "no", "x", "y", "total", "(", ")", ",", "+", "-", "*", "/", "&", "<", ">", "<=",
    ">=", "=", "--", "'a'", "\"b c\"", "'", "0", "1", "2.5", "999999999999", "10000", "pages('mail:inbox')",
    "read(", "field(", "body(", "exists(", "lines(", "words(", "count(", "item(", "number(", "lower(", "today()",
    "weekday(", "add_days(", "write(", "add(", "toss(", "send(", "event(", "'mail:inbox:m1'", "'desk:out'",
    "'contacts'", "'omar'", "'2026-11-12'", "\n", "\n", "\n", "é", "\t",
];

fn check(src: &str, world: &TestWorld) {
    let o = chore::run(src, world);
    assert!(o.steps <= MAX_STEPS + 1, "steps {} for {src:?}", o.steps);
    if o.error.is_some() {
        assert!(o.actions.is_empty(), "error with actions for {src:?}");
    }
    assert!(o.output.len() <= chore::MAX_OUTPUT);
    assert!(o.actions.len() <= chore::MAX_ACTIONS);
}

#[test]
fn random_token_soup() {
    let world = TestWorld::new();
    let mut rng = Rng(0x2545F4914F6CDD1D);
    for _ in 0..20_000 {
        let n = 1 + rng.below(40);
        let src: Vec<&str> = (0..n).map(|_| rng.pick(PIECES)).collect();
        check(&src.join(" "), &world);
    }
}

fn expr(rng: &mut Rng, depth: usize) -> String {
    if depth == 0 || rng.below(3) == 0 {
        return rng
            .pick(&["1", "x", "'mail:inbox:m1'", "'a b'", "yes", "today()", "pages('mail:inbox')", "0", "-3", "y"])
            .to_string();
    }
    let a = expr(rng, depth - 1);
    let b = expr(rng, depth - 1);
    match rng.below(12) {
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
        _ => format!("add_days({a}, {b})"),
    }
}

fn program(rng: &mut Rng, depth: usize, out: &mut String) {
    for _ in 0..1 + rng.below(4) {
        match rng.below(9) {
            0 => out.push_str(&format!("set x to {}\n", expr(rng, 3))),
            1 => out.push_str(&format!("set y to {}\n", expr(rng, 3))),
            2 => out.push_str(&format!("say {}\n", expr(rng, 3))),
            3 if depth > 0 => {
                out.push_str(&format!("when {}\n", expr(rng, 2)));
                program(rng, depth - 1, out);
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
            6 => out.push_str(&format!("write('desk:out', {})\n", expr(rng, 2))),
            7 => out.push_str(&format!("send('omar', 'Hi', {})\n", expr(rng, 2))),
            _ => out.push_str("set x to x & x\n"),
        }
    }
}

#[test]
fn random_structured_programs() {
    let world = TestWorld::new();
    let mut rng = Rng(0x9E3779B97F4A7C15);
    for _ in 0..20_000 {
        let mut src = String::new();
        program(&mut rng, 4, &mut src);
        check(&src, &world);
    }
}

#[test]
fn hostile_programs() {
    let world = TestWorld::new();
    let hostile = [
        "(".repeat(5000),
        format!("say {}", "not ".repeat(3000)),
        format!("say {}", "-".repeat(3000)),
        "when yes\n".repeat(119),
        "done\n".repeat(119),
        format!("say {}1", "1 + ".repeat(1000)),
        "'".repeat(4096),
        "x".repeat(4096),
        "set x to 'a'\nrepeat 10000 times\nset x to x & x\ndone".into(),
        "set x to pages('mail:inbox')\nrepeat 10000 times\nset y to x & x\ndone".into(),
        "repeat 10000 times\nsay 1\ndone".into(),
        "repeat 10000 times\nadd('desk:log', 'x')\ndone".into(),
        format!("say '{}' has 'b'", "a".repeat(4000)),
        "\r\n\r\nsay 1\r\n".into(),
        "\0\0\0".into(),
        "say 1\n".repeat(120),
    ];
    for src in hostile {
        check(&src, &world);
    }
}
