//! The new parts of M1 (design §4): lists, records, targets, or when, while, skip, quit,
//! nothing(), doubled quote marks, planned changes, and the budgets.
extern crate alloc;

#[path = "../../kernel/src/fs.rs"]
#[allow(dead_code)]
mod fs;
mod common;

use common::*;
use script::limits::*;
use script::Action;

#[test]
fn lists() {
    assert_eq!(say("[1, 'a', yes]"), "1, a, yes");
    assert_eq!(say("count([])"), "0");
    assert_eq!(say("count([1, [2, 3]])"), "2");
    assert_eq!(say("item([10, 20, 30], 2)"), "20");
    assert_eq!(say("item(item([1, [2, 3]], 2), 1)"), "2");
    assert_eq!(say("[1, 2] is [1, 2]"), "yes");
    assert_eq!(say("[1, 2] is [2, 1]"), "no");
    assert_eq!(say("[1, 'a'] is [1, 'b']"), "no");
    assert_eq!(say("[1] is ['1']"), "no"); // different kinds inside a list: not the same
    assert_eq!(say("[1, 2, 3] has 2"), "yes");
    assert_eq!(say("[1, 2, 3] has '2'"), "no");
    assert_eq!(say("[[1], [2]] has [2]"), "yes");
    has(&err("say [1, 2,]"), "didn't expect ']'");
    has(&err("say [1, 2"), "not closed");
    has(&err("say item([], 1)"), "has 0 items");
}

#[test]
fn records() {
    assert_eq!(say("{name: 'Mira', age: 3}"), "{name: Mira, age: 3}");
    assert_eq!(say("{}"), "{}");
    assert_eq!(say("{name: 'Mira'}.name"), "Mira");
    assert_eq!(out("set r to {Name: 'Mira'}\nsay r.NAME"), vec!["Mira"]);
    assert_eq!(say("{a: 1, b: 2} is {b: 2, a: 1}"), "yes");
    assert_eq!(say("{a: 1} is {a: 1, b: 2}"), "no");
    assert_eq!(say("count({a: 1, b: 2})"), "2");
    assert_eq!(say("{who: {name: 'Omar'}}.who.name"), "Omar");
    assert_eq!(say("[{n: 1}, {n: 2}]"), "{n: 1}, {n: 2}");
    has(&err("say {a: 1}.b"), "has no field b (it has: a)");
    has(&err("say {}.b"), "it has no fields");
    has(&err("say {a: 1, a: 2}"), "the field a is in this record twice");
    has(&err("say 5.x"), "needs a record");
    has(&err("say {a 1}"), "a record needs");
    has(&err("say {a: 1"), "not closed");
}

#[test]
fn targets() {
    assert_eq!(out("set r to {age: 3}\nset r.age to 4\nset r.city to 'Gent'\nsay r"), vec!["{age: 4, city: Gent}"]);
    assert_eq!(out("set s to {who: {name: 'a'}}\nset s.who.name to 'b'\nsay s.who.name"), vec!["b"]);
    assert_eq!(out("set l to [1, 2, 3]\nset l at 2 to 'x'\nsay l"), vec!["1, x, 3"]);
    assert_eq!(out("set s to {items: [1]}\nset s.items at 1 to 9\nadd 10 to s.items\nsay s.items"), vec!["9, 10"]);
    assert_eq!(out("set l to []\nadd 1 to l\nadd [2] to l\nsay count(l)"), vec!["2"]);
    // nothing changes if a part fails
    assert_eq!(
        run("set s to {a: {b: 1}}\nset s.x.y to 2").error.unwrap().message,
        "the record has no field x (it has: a)"
    );
    has(&err("set s to {a: 1}\nset s.a.b to 2"), "needs a record");
    has(&err("set l to [1]\nset l at 2 to 0"), "has 1 items");
    has(&err("set l to [1]\nset l at 0 to 0"), "whole number from 1");
    has(&err("set l to [1]\nset l at 1.5 to 0"), "whole number from 1");
    has(&err("set n to 1\nadd 2 to n"), "needs a list there");
    has(&err("add 2 to nope"), "nope has no value yet");
    has(&err("set nope.x to 1"), "nope has no value yet");
    has(&err("set l to [[1]]\nset l at 1 at 1 to 2"), "at N must be the last part");
    has(&err("set l to [{a: 1}]\nset l at 1.a to 2"), "");
}

#[test]
fn values_are_copies() {
    assert_eq!(out("set a to {x: 1}\nset b to a\nset b.x to 2\nsay a.x & b.x"), vec!["12"]);
    assert_eq!(out("set a to [1]\nset b to a\nadd 2 to b\nsay count(a) & count(b)"), vec!["12"]);
    assert_eq!(
        out("set a to {inner: {deep: [1]}}\nset b to a\nadd 2 to b.inner.deep\nsay a.inner.deep\nsay b.inner.deep"),
        vec!["1", "1, 2"]
    );
    assert_eq!(out("set l to [{n: 1}]\nset r to item(l, 1)\nset r.n to 5\nsay item(l, 1).n"), vec!["1"]);
    assert_eq!(out("set l to [1]\nfor each x in l\n  add x to l\ndone\nsay l"), vec!["1, 1"]);
}

#[test]
fn or_when() {
    let src = |n: i32| format!("set n to {n}\nwhen n < 0\n  say 'neg'\nor when n is 0\n  say 'zero'\nor when n < 10\n  say 'small'\notherwise\n  say 'big'\ndone");
    assert_eq!(out(&src(-1)), vec!["neg"]);
    assert_eq!(out(&src(0)), vec!["zero"]);
    assert_eq!(out(&src(5)), vec!["small"]);
    assert_eq!(out(&src(50)), vec!["big"]);
    assert_eq!(out("when no\nsay 1\nor when yes\nsay 2\ndone"), vec!["2"]);
    has(&err("when yes\notherwise\nor when yes\ndone"), "or when must come before otherwise");
    has(&err("or when yes"), "no when before it");
    has(&err("repeat 1 times\nor when yes\ndone"), "or when only belongs to when");
}

#[test]
fn loops() {
    assert_eq!(out("set n to 0\nwhile n < 5\n  set n to n + 1\ndone\nsay n"), vec!["5"]);
    assert_eq!(out("while no\nsay 'never'\ndone\nsay 'ok'"), vec!["ok"]);
    assert_eq!(
        out("for each x in [1, 2, 3, 4]\n  when x is 2\n    skip\n  done\n  when x is 4\n    quit\n  done\n  say x\ndone"),
        vec!["1", "3"]
    );
    // quit ends only the innermost loop
    assert_eq!(
        out("repeat 2 times\n  repeat 5 times\n    say 'in'\n    quit\n  done\n  say 'out'\ndone"),
        vec!["in", "out", "in", "out"]
    );
    assert_eq!(out("set n to 0\nwhile yes\n  set n to n + 1\n  when n is 3\n    quit\n  done\ndone\nsay n"), vec!["3"]);
    has(&err("skip"), "only works inside a loop");
    has(&err("when yes\nquit\ndone"), "only works inside a loop");
    has(&err("while 1\ndone"), "while needs yes or no");
    has(&err("while yes\ndone"), "too many steps");
    has(&err("skip now"), "alone");
}

#[test]
fn new_words_are_names_too() {
    assert_eq!(out("set skip to 1\nset while to 2\nsay skip + while"), vec!["3"]);
    assert_eq!(out("set at to [5]\nset at at 1 to 6\nsay at"), vec!["6"]);
    assert_eq!(out("set add to [1]\nadd 2 to add\nadd('desk:x', add)\nsay add"), vec!["1, 2"]);
}

#[test]
fn nothing_value() {
    assert_eq!(say("nothing()"), "nothing");
    assert_eq!(say("nothing() is nothing()"), "yes");
    assert_eq!(say("5 is nothing()"), "no");
    assert_eq!(say("nothing() is not 'a'"), "yes");
    assert_eq!(say("[nothing()]"), "nothing");
    has(&err("say nothing() + 1"), "needs two numbers");
}

#[test]
fn quote_marks() {
    assert_eq!(say("'it''s'"), "it's");
    assert_eq!(say("\"say \"\"hi\"\"\""), "say \"hi\"");
    assert_eq!(say("''''"), "'");
    assert_eq!(say("'' is ''"), "yes");
    assert_eq!(say("'a -- b'"), "a -- b");
}

#[test]
fn planned_changes_in_order() {
    let o = run("write('desk:new', 'a')\nadd('desk:new', 'b')\ntoss('desk:new')\nwrite('desk:new', 'c')");
    assert_eq!(o.error, None);
    assert_eq!(o.actions.len(), 4);
    has(&err("toss('notes:shopping')\ntoss('notes:shopping')"), "no page called desk:notes:shopping");
    let o = run("set t to ''\nrepeat 3 times\n  add('desk:big', 'xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx')\ndone");
    assert_eq!(o.error, None);
    let half = "x".repeat(3500);
    let e = err(&format!("set t to '{half}' & '{half}'\nrepeat 3 times\n  add('desk:big', t)\ndone"));
    has(&e, "would be too long");
    has(&e, "line 3");
    assert_eq!(
        run("write('desk:a', [1, 2])").actions,
        vec![Action::Write { page: "a".into(), text: "1, 2".into() }]
    );
}

#[test]
fn budgets() {
    // nesting of values
    let deep = "set l to 1\nrepeat 16 times\n  set l to [l]\ndone\nsay count(l)";
    assert_eq!(out(deep), vec!["1"]);
    let too_deep = "set l to 1\nrepeat 17 times\n  set l to [l]\ndone";
    has(&err(too_deep), "nested at most 16 deep");
    has(&err("set r to {a: 1}\nrepeat 17 times\n  set r to {a: r}\ndone"), "nested at most 16 deep");
    has(&err("set l to 1\nrepeat 16 times\n  set l to [l]\ndone\nset m to []\nadd l to m"), "nested at most 16 deep");
    // items
    has(&err("set l to []\nrepeat 2001 times\n  add 1 to l\ndone"), "at most 2000 items");
    // memory: 300 different texts of 8000 characters is far more than 1 MB
    let t = "x".repeat(4000);
    let e = run(&format!("set l to []\nrepeat 300 times\n  add '{t}' & '{t}' to l\ndone")).error.unwrap();
    has(&e.message, "too much memory");
    assert!(e.limit);
    // the same text shared 2000 times is cheap
    let o = run(&format!("set t to '{t}' & '{t}'\nset l to []\nrepeat 2000 times\n  add t to l\ndone\nsay count(l)"));
    assert_eq!(o.output, vec!["2000"]);
    assert!(o.peak_memory < STEP_MEMORY / 2, "peak {}", o.peak_memory);
    // memory is given back: a long loop that keeps replacing a big value is fine
    let o = run(&format!("repeat 2000 times\n  set x to '{t}' & '{t}'\ndone\nsay 'ok'"));
    assert_eq!(o.error, None);
    assert!(o.peak_memory < 40_000, "peak {}", o.peak_memory);
}

#[test]
fn mistakes_name_the_page_and_line() {
    let o = script::run("apps:shop", "say 1\nsay 1 + 'x'", &TestWorld::new());
    assert_eq!(o.error.unwrap().to_string(), "desk:apps:shop line 2: + needs two numbers, got the text 'x' (number(...) turns text into a number)");
}
