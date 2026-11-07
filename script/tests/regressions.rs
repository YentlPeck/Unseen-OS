//! Cases found by Sol's review of M1 (6 Nov 2026). Each must stay fixed.
extern crate alloc;

#[path = "../../kernel/src/fs.rs"]
#[allow(dead_code)]
mod fs;
mod common;

use common::*;

/// A comparison of two huge shared trees: about 10^15 leaf comparisons, little memory. It must
/// stop on the step budget at once, not after the whole comparison.
#[test]
fn huge_comparisons_stop_on_the_budget() {
    let src = "set a to [0]\nset b to [0]\nrepeat 15 times\n  set a to [a,a,a,a,a,a,a,a,a,a]\n  set b to [b,b,b,b,b,b,b,b,b,b]\ndone\nsay a is b";
    let t = std::time::Instant::now();
    let e = run(src).error.expect("must stop");
    assert!(e.limit && e.message.contains("too many steps"), "{e}");
    let has = src.replace("repeat 15 times", "repeat 14 times").replace("say a is b", "say [b] has a");
    let e = run(&has).error.expect("must stop");
    assert!(e.limit, "{e}");
    assert!(t.elapsed().as_secs_f64() < 2.0, "took {:?}", t.elapsed());
}

/// Every planned add keeps its line and the page's new text: both are counted.
#[test]
fn planned_adds_are_counted() {
    let t = "x".repeat(4000);
    let src = format!("set t to '{t}' & '{t}'\nset n to 0\nrepeat 120 times\n  set n to n + 1\n  add('desk:p' & n, t)\ndone");
    let e = run(&src).error.expect("1.9 MB of planned text is over the 1 MB budget");
    assert!(e.limit && e.message.contains("too much memory"), "{e}");
}

/// A long line that is refused builds a deep tree first; freeing it must not use the stack.
#[test]
#[cfg_attr(debug_assertions, ignore = "debug frames are much bigger; the kernel is a release build")]
fn refused_deep_lines_free_without_recursion() {
    let h = std::thread::Builder::new()
        .stack_size(128 * 1024)
        .spawn(|| {
            for op in ["+", "*", "&", " and ", " or ", "-"] {
                let n = (15_990 - 4) / (op.len() + 1);
                let src = format!("say {}1", format!("1{op}").repeat(n));
                assert!(src.len() <= 16_000);
                let e = run(&src).error.expect("too complicated");
                assert!(e.message.contains("too complicated") || e.message.contains("too big"), "{e}");
            }
            let src = format!("say [{}1]", "1, ".repeat(5000));
            assert!(run(&src).error.is_some());
            let src = format!("say {}", "x.a".repeat(1) + &".a".repeat(7000));
            assert!(run(&src).error.is_some());
        })
        .unwrap();
    h.join().expect("ran out of stack");
}

/// Changes are checked against the room left on the desk, in order.
#[test]
fn desk_room_is_checked_when_planned() {
    let mut w = TestWorld::new();
    w.free = 1000;
    let o = script::run("main", &format!("write('desk:a', '{}')", "x".repeat(900)), &w);
    assert_eq!(o.error, None);
    let o = script::run("main", &format!("write('desk:a', '{}')\nwrite('desk:b', '{}')", "x".repeat(501), "y".repeat(500)), &w);
    let e = o.error.expect("too full");
    assert!(e.message.contains("too full") && e.line == 2, "{e}");
    // tossing makes room
    let o = script::run("main", &format!("toss('notes:shopping')\nwrite('desk:a', '{}')", "x".repeat(960)), &w);
    assert!(o.error.is_none(), "{:?}", o.error.map(|e| e.to_string()));
    let o = script::run("main", &format!("send('omar', 'Hi', '{}')", "x".repeat(990)), &w);
    assert!(o.error.unwrap().message.contains("too full"));
    // counted as the kernel counts it: page text only (re-check of M4)
    let o = script::run("main", &format!("write('desk:a', '{}')\nwrite('desk:b', '{}')", "x".repeat(500), "y".repeat(500)), &w);
    assert_eq!(o.error, None, "exactly full fits");
    w.free = 0;
    let o = script::run("main", "write('notes:shopping', 'eggs bread tea')", &w); // same length
    assert_eq!(o.error, None);
    let o = script::run("main", "toss('notes:shopping')\nwrite('desk:longer', 'eggs bread tea')", &w);
    assert_eq!(o.error, None, "a toss frees exactly its text");
    let o = script::run("main", "toss('notes:shopping')\nwrite('desk:longer', 'eggs bread tea!')", &w);
    assert!(o.error.unwrap().message.contains("too full"));
}

/// One comparison at a time, as chores (the design's table says so now).
#[test]
fn one_comparison_at_a_time() {
    has(&err("say [yes] has 1 < 2"), "one comparison at a time");
    assert_eq!(say("[yes] has (1 < 2)"), "yes");
}

/// An instruction word followed by a bracket is still that instruction (found by the deep task
/// stack test while building M2): `say (1 + 2)` worked in chores and must keep working.
#[test]
fn instruction_words_are_not_task_calls() {
    assert_eq!(out("say (1 + 2) * 2"), vec!["6"]);
    assert_eq!(out("when (1 < 2)\n  say 'yes'\ndone"), vec!["yes"]);
    assert_eq!(out("set x to 1\nwhile (x < 3)\n  set x to x + 1\ndone\nsay (x)"), vec!["3"]);
    assert_eq!(out("task f(x)\n  give (x + 1) * 2\ndone\nsay f(1)"), vec!["4"]);
    assert_eq!(out("set l to []\nadd (1 + 1) to l\nsay l"), vec!["2"]);
    has(&err("task say(x)\ndone"), "special word"); // say is also a chore keyword
    has(&err("task give(x)\ndone"), "that word starts an instruction");
}

// ---- Sol's review of M2 (6 Nov 2026) ----

/// The room for a call's inputs is counted while the call runs.
#[test]
fn argument_lists_are_counted() {
    let params: Vec<String> = (0..1700).map(|i| format!("a{i}")).collect();
    let zeros = vec!["0"; 1699].join(",");
    let src = format!("task g({})\ndone\ntask f()\n  give g({zeros},f())\ndone\nsay f()", params.join(","));
    assert!(src.len() < 16_000);
    let o = run(&src);
    let e = o.error.expect("must run out of memory or depth");
    assert!(e.limit, "{e}");
    assert!(o.peak_memory > 0);
}

/// Sorting long texts with a long common start counts the comparisons' work.
#[test]
fn sorting_long_texts_counts_the_work() {
    let src = "set l to []\nset n to 0\nrepeat 100 times\n  set n to n + 1\n  add pad('', 7995) & n to l\ndone\nrepeat 10000 times\n  set s to sort(l)\ndone";
    let t = std::time::Instant::now();
    let e = run(src).error.expect("too many steps");
    assert!(e.limit && e.message.contains("too many steps"), "{e}");
    assert!(t.elapsed().as_secs_f64() < 2.0, "took {:?}", t.elapsed());
}

/// Fixed values are counted by the program budget, not by the run's.
#[test]
fn fixed_values_have_their_own_budget() {
    let fixed: String = (0..100).map(|i| format!("fixed k{i} is pad('', 8000)\n")).collect();
    let src = format!("{fixed}set l to []\nrepeat 40 times\n  add pad('', 8000) to l\ndone\nsay count(l)");
    let o = run(&src);
    assert_eq!(o.output, vec!["40"], "{:?}", o.error.map(|e| e.to_string()));
    // and changing a copy of a fixed value is counted by the run
    assert_eq!(out("fixed base is [1, 2]\nset c to base\nadd 3 to c\nsay c\nsay base"), vec!["1, 2, 3", "1, 2"]);
}

/// Texts with characters that take more than one place.
#[test]
fn slice_never_cuts_a_character() {
    assert_eq!(say("slice('éa', 1, 1)"), "é");
    assert_eq!(say("slice('aé', 2, 2)"), "é");
}

/// round() leaves whole and very big numbers alone.
#[test]
fn round_keeps_whole_numbers() {
    assert_eq!(say("round(4500000000, 6)"), "4500000000");
    assert_eq!(say("round(4500000000, 6) is 4500000000"), "yes");
    assert_eq!(say("round(-4500000000, 6)"), "-4500000000");
    has(&err("say round(999999999999.5, 0)"), "too big"); // it rounds to 10^12, over the limit
    assert_eq!(say("round(2.5, 0) & ' ' & round(-2.5, 0) & ' ' & round(0.125, 2)"), "3 -3 0.13");
}

/// A task and a fixed value can't share a name.
#[test]
fn task_and_fixed_names_clash() {
    has(&err("task f()\n  give 1\ndone\nfixed f is 2"), "is the name of a task");
}

// ---- Sol's review of M3 (6 Nov 2026) ----

/// Nested instead parts inside recursion stay within a 128 KB stack.
#[test]
#[cfg_attr(debug_assertions, ignore = "debug frames are much bigger; the kernel is a release build")]
fn nested_instead_parts_fit_the_stack() {
    let h = std::thread::Builder::new()
        .stack_size(128 * 1024)
        .spawn(|| {
            let open = "try\n  fail 'x'\ninstead p\n".repeat(15);
            let close = "done\n".repeat(15);
            let src = format!("task down(n)\n  when n is 0\n    give 0\n  done\n{open}give down(n - 1)\n{close}done\nsay down(39)");
            let o = run(&src);
            assert!(o.error.is_none() || o.error.as_ref().unwrap().limit, "{:?}", o.error);
        })
        .unwrap();
    h.join().expect("ran out of stack");
}

fn tests(src: &str) -> Vec<String> {
    script::run_tests("tests", src, &TestWorld::new()).lines()
}

/// A pretend with a very long path counts its drawers.
#[test]
fn pretend_counts_paths_and_drawers() {
    let l = tests("test 'deep'\n  pretend replace(pad('', 3999), ' ', 'a:') & 'a' holds ''\n  check yes\ndone");
    assert!(l[0].starts_with("FAIL deep: line 2: the test world would be too full"), "{l:?}");
}

/// A check that can't be worked out fails the test, even inside try.
#[test]
fn a_check_that_fails_to_work_out_is_not_a_pass() {
    let l = tests("test 'false pass'\n  try\n    check 1 / 0 is 0\n  instead p\n  done\ndone");
    assert!(l[0].starts_with("FAIL false pass: line 3: dividing by 0"), "{l:?}");
}

/// Caught mistakes don't leak memory.
#[test]
fn many_caught_mistakes_do_not_leak() {
    let o = run("repeat 4 times\n  repeat 10000 times\n    try\n      fail 'x'\n    instead p\n    done\n  done\ndone\nsay 'ok'");
    assert_eq!(o.output, vec!["ok"], "{:?}", o.error.map(|e| e.to_string()));
    assert!(o.peak_memory < 2_000, "peak {}", o.peak_memory);
}

/// The pretend lines and the test share one step budget.
#[test]
fn pretends_share_the_step_budget() {
    let work = "task work()\n  repeat 10000 times\n    repeat 10 times\n      set x to 1\n    done\n  done\n  give 'x'\ndone\n";
    let pretends = "  pretend 'p' holds work()\n".repeat(16);
    let l = tests(&format!("{work}test 'slow'\n{pretends}  check yes\ndone"));
    assert!(l[0].contains("too many steps"), "{l:?}");
}

/// Fixed values and the checked code share one program budget.
#[test]
fn fixed_values_and_code_share_the_program_budget() {
    let fixed: String = (0..125).map(|i| format!("fixed k{i} is pad('', 8000)\n")).collect();
    let e = run(&format!("{fixed}say 'ok'")).error.expect("over the program budget");
    assert!(e.limit && e.message.contains("too much memory"), "{e}");
}

/// use must come before calls too.
#[test]
fn use_comes_before_calls() {
    has(&err("write('desk:a', 'x')\nuse 'notes:shopping'"), "use must come at the top");
}
