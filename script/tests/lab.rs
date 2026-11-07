//! The lab (design §7): one line at a time, names kept, changes only shown.
extern crate alloc;

#[path = "../../kernel/src/fs.rs"]
#[allow(dead_code)]
mod fs;
mod common;

use common::TestWorld;
use script::Lab;

fn world() -> TestWorld {
    let mut start = String::from(common::START);
    start.push_str("#page lib:money\nfixed rate is 21\ntask tax(amount)\n  give amount * rate / 100\ndone\n#end\n");
    start.push_str("#page lib:bad\ntask half(n)\n  give n / 0\ndone\n#end\n");
    TestWorld::from(&start)
}

/// Type the lines; gives what the last one showed.
fn typed<W: script::World>(lab: &mut Lab, w: &W, lines: &[&str]) -> Vec<String> {
    let mut last = Vec::new();
    for l in lines {
        last = lab.line(l, w);
    }
    last
}

#[test]
fn values_are_shown_as_you_would_type_them() {
    let w = world();
    let mut lab = Lab::new();
    assert_eq!(lab.line("1 + 2", &w), ["3"]);
    assert_eq!(lab.line("'it''s'", &w), ["'it''s'"]);
    assert_eq!(lab.line("[1, 'a', yes, [], {}]", &w), ["[1, 'a', yes, [], {}]"]);
    assert_eq!(lab.line("{name: 'Mira', age: 3}", &w), ["{name: 'Mira', age: 3}"]);
    assert_eq!(lab.line("nothing()", &w), ["nothing()"]);
    assert_eq!(lab.line("count(pages('mail:inbox'))", &w), ["3"]);
    assert_eq!(lab.line("-4 * 2", &w), ["-8"]);
}

#[test]
fn names_tasks_and_uses_are_kept() {
    let w = world();
    let mut lab = Lab::new();
    assert!(lab.line("set items to ['milk']", &w).is_empty());
    assert!(lab.line("add 'tea' to items", &w).is_empty());
    assert_eq!(lab.line("items", &w), ["['milk', 'tea']"]);
    // a task, typed over several lines
    assert!(lab.line("task twice(x)", &w).is_empty());
    assert!(lab.in_block());
    assert!(lab.line("  give x * 2", &w).is_empty());
    assert!(lab.line("done", &w).is_empty());
    assert!(!lab.in_block());
    assert_eq!(lab.line("twice(21)", &w), ["42"]);
    // use is kept too, wherever it is typed
    assert!(lab.line("use 'lib:money'", &w).is_empty());
    assert_eq!(lab.line("tax(100) + rate", &w), ["42"]);
    assert_eq!(lab.line("twice(count(items))", &w), ["4"]);
}

#[test]
fn say_lines_and_changes_are_shown_once() {
    let w = world();
    let mut lab = Lab::new();
    assert_eq!(lab.line("say 'hello'", &w), ["hello"]);
    // the earlier say line is not shown again
    assert_eq!(lab.line("say 'again'", &w), ["again"]);
    assert_eq!(lab.line("write('desk:list', 'tea')", &w), ["would write desk:list"]);
    assert_eq!(lab.line("add('notes:shopping', 'jam')", &w), ["would add a line to desk:notes:shopping"]);
    assert_eq!(lab.line("send('omar', 'Hi', 'Hello')", &w), ["would send a mail to omar: Hi"]);
    assert_eq!(lab.line("1", &w), ["1"]);
    // a block shows what it said when it is done
    typed(&mut lab, &w, &["for each n in [1, 2]"]);
    assert_eq!(typed(&mut lab, &w, &["  say n * 10", "done"]), ["10", "20"]);
    // nothing is ever made
    assert!(!w.fs.exists("list"));
}

#[test]
fn a_mistake_keeps_nothing() {
    let w = world();
    let mut lab = Lab::new();
    lab.line("set x to 1", &w);
    assert_eq!(lab.line("set x to x / 0", &w), ["mistake: dividing by 0 is not possible"]);
    assert_eq!(lab.line("x", &w), ["1"]);
    assert_eq!(lab.line("y", &w)[0], "mistake: y has no value yet (give it one with set y to ...)");
    // a say before the mistake is shown
    assert_eq!(lab.line("say 'a'", &w), ["a"]);
    let r = typed(&mut lab, &w, &["when yes", "  say 'b'", "  say 1 + 'c'", "done"]);
    assert_eq!(r[0], "b");
    assert!(r[1].starts_with("mistake: line 3 of the block: "), "{r:?}");
    // the say line inside the failed block is not kept
    assert_eq!(lab.line("say 'd'", &w), ["d"]);
    // a mistake on a used page names it
    lab.line("use 'lib:bad'", &w);
    let r = lab.line("half(4)", &w);
    assert_eq!(r, ["mistake: desk:lib:bad line 2: dividing by 0 is not possible"]);
    assert_eq!(lab.line("x + 1", &w), ["2"]);
}

#[test]
fn checks_and_stop_and_odd_lines() {
    let w = world();
    let mut lab = Lab::new();
    let r = lab.line("frobnicate x", &w);
    assert!(r[0].starts_with("mistake: "), "{r:?}");
    let r = lab.line("done", &w);
    assert!(r[0].starts_with("mistake: done, but there is no block"), "{r:?}");
    let r = lab.line("stop", &w);
    assert!(r[0].starts_with("(stopped"), "{r:?}");
    assert_eq!(lab.line("say 'still here'", &w), ["still here"]);
    // a value line with a mistake found before running
    let r = lab.line("count()", &w);
    assert!(r[0].starts_with("mistake: count"), "{r:?}");
    // a block can be forgotten
    lab.line("when yes", &w);
    assert!(lab.in_block());
    lab.forget_block();
    assert_eq!(lab.line("2", &w), ["2"]);
}

#[test]
fn big_values_are_cut_and_the_lab_fills_up() {
    let w = world();
    let mut lab = Lab::new();
    lab.line("set t to ''", &w);
    lab.line("repeat 600 times", &w);
    lab.line("  set t to t & 'ab'", &w);
    lab.line("done", &w);
    let r = lab.line("[t, t, t]", &w);
    assert!(r[0].len() < 700 && r[0].ends_with(" ..."), "{}", r[0].len());
    let mut lab = Lab::new();
    let mut last = Vec::new();
    for i in 0..500 {
        last = lab.line(&format!("set n{i} to {i}"), &w);
        if !last.is_empty() {
            break;
        }
    }
    assert!(last[0].starts_with("mistake: the lab is full"), "{last:?}");
    assert_eq!(lab.line("n3", &w), ["3"]);
}

#[test]
fn the_steps_of_all_kept_lines_count() {
    let w = world();
    let mut lab = Lab::new();
    lab.line("set n to 0", &w);
    lab.line("while n < 100000", &w);
    lab.line("  set n to n + 1", &w);
    let r = lab.line("done", &w);
    assert!(r[0].contains("too many steps"), "{r:?}");
    assert_eq!(lab.line("n", &w), ["0"]);
}

// ---- found in the M4 review ----

#[test]
fn a_line_that_changes_the_earlier_lines_is_not_kept() {
    let w = world();
    let mut lab = Lab::new();
    let r = typed(&mut lab, &w, &["try", "  say x", "  say 'extra'", "instead e", "  say 'missing'", "done"]);
    assert_eq!(r, ["missing"]);
    // a fixed value is seen from every line: the try above would now take the other way
    let r = lab.line("fixed x is 1", &w);
    assert!(r[0].starts_with("mistake: this line changes what the lines before it did"), "{r:?}");
    assert_eq!(lab.line("say 'next'", &w), ["next"]);
    // a use line that brings in a name seen earlier, the same
    let mut lab = Lab::new();
    typed(&mut lab, &w, &["try", "  say rate", "instead e", "  say 'no rate'", "done"]);
    let r = lab.line("use 'lib:money'", &w);
    assert!(r[0].starts_with("mistake: this line changes"), "{r:?}");
    // a fixed value nothing used before is fine
    assert!(lab.line("fixed y is 2", &w).is_empty());
    assert_eq!(lab.line("y", &w), ["2"]);
}

#[test]
fn names_that_are_instruction_words_are_values() {
    let w = world();
    let mut lab = Lab::new();
    assert!(lab.line("set while to {done: 3}", &w).is_empty());
    assert_eq!(lab.line("while.done", &w), ["3"]);
    assert!(!lab.in_block());
    // a real header still starts a block
    lab.line("while no", &w);
    assert!(lab.in_block());
    assert!(lab.line("done", &w).is_empty());
    lab.line("try", &w);
    assert!(lab.in_block());
    lab.forget_block();
}

#[test]
fn a_stop_shows_the_changes_planned_before_it() {
    let w = world();
    let mut lab = Lab::new();
    let r = typed(&mut lab, &w, &["when yes", "  write('desk:x', 'a')", "  stop", "done"]);
    assert_eq!(r[0], "would write desk:x");
    assert!(r[1].starts_with("(stopped"), "{r:?}");
}

#[test]
fn a_different_date_is_noticed() {
    struct Dated(TestWorld, &'static str);
    impl script::World for Dated {
        fn page(&self, p: &str) -> Option<String> { script::World::page(&self.0, p) }
        fn is_drawer(&self, p: &str) -> bool { script::World::is_drawer(&self.0, p) }
        fn pages_in(&self, d: &str) -> Vec<String> { script::World::pages_in(&self.0, d) }
        fn check_write(&self, p: &str) -> Result<(), String> { script::World::check_write(&self.0, p) }
        fn check_toss(&self, p: &str) -> Result<(), String> { script::World::check_toss(&self.0, p) }
        fn check_send(&self, a: &str, b: &str, c: &str) -> Result<(), String> { script::World::check_send(&self.0, a, b, c) }
        fn check_event(&self, a: &str, b: &str, c: &str, d: &str) -> Result<(), String> { script::World::check_event(&self.0, a, b, c, d) }
        fn today(&self) -> String { self.1.into() }
    }
    let before = Dated(world(), "2026-11-06");
    let after = Dated(world(), "2026-11-07");
    let mut lab = Lab::new();
    assert_eq!(typed(&mut lab, &before, &["when today() is '2026-11-06'", "  say 'old'", "done"]), ["old"]);
    let r = lab.line("say 'new'", &after);
    assert!(r[0].starts_with("mistake: this line changes"), "{r:?}");
}

// ---- found in the M4 re-check ----

#[test]
fn default_is_the_same_as_new() {
    let w = world();
    let mut lab = Lab::default();
    assert_eq!(lab.line("1 + 2", &w), ["3"]);
    assert_eq!(lab.line("say 'x'", &w), ["x"]);
}

#[test]
fn a_name_called_use_is_a_value() {
    let w = world();
    let mut lab = Lab::new();
    assert!(lab.line("set use to {done: 3}", &w).is_empty());
    assert_eq!(lab.line("use.done", &w), ["3"]);
    // a real use line still works, and a broken one still says what use needs
    assert!(lab.line("use 'lib:money'", &w).is_empty());
    assert_eq!(lab.line("use", &w), ["{done: 3}"]); // the name, since there is one
    let r = Lab::new().line("use", &w);
    assert!(r[0].starts_with("mistake: "), "{r:?}");
}
