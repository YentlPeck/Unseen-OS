//! M2 (design §4 "Tasks", "Fixed values", "Built-in library"): tasks, give, fixed values,
//! clashes found before running, the call trail in mistakes, and the new library functions.
extern crate alloc;

#[path = "../../kernel/src/fs.rs"]
#[allow(dead_code)]
mod fs;
mod common;

use common::*;
use script::Action;

#[test]
fn tasks_and_give() {
    let src = "task double(n)\n  give n * 2\ndone\nsay double(21)";
    assert_eq!(out(src), vec!["42"]);
    // a task can be used before the line that makes it
    assert_eq!(out("say twice('ab')\ntask twice(t)\n  give t & t\ndone"), vec!["abab"]);
    // no give: nothing()
    assert_eq!(out("task quiet()\n  set x to 1\ndone\nsay quiet() is nothing()"), vec!["yes"]);
    // give alone gives nothing()
    assert_eq!(out("task early(n)\n  when n > 0\n    give\n  done\n  give n\ndone\nsay early(5)\nsay early(0)"), vec!["nothing", "0"]);
    // give from inside loops ends the task at once
    let src = "task first_big(l)\n  for each x in l\n    when x > 10\n      give x\n    done\n  done\n  give 0\ndone\nsay first_big([1, 20, 30])";
    assert_eq!(out(src), vec!["20"]);
    // a task on a line of its own; its changes are planned like any other
    let o = run("task log(t)\n  add('desk:log', t)\ndone\nlog('a')\nlog('b')");
    assert_eq!(o.error, None);
    assert_eq!(o.actions.len(), 2);
    assert!(matches!(&o.actions[1], Action::Add { line, .. } if line == "b"));
}

#[test]
fn tasks_have_their_own_names() {
    // the task can't see the caller's names
    has(&err("set secret to 1\ntask peek()\n  give secret\ndone\nsay peek()"), "secret has no value yet");
    // and its names don't leak out
    has(&err("task t()\n  set inside to 1\ndone\nt()\nsay inside"), "inside has no value yet");
    // inputs are copies
    let src = "task grow(l)\n  add 9 to l\n  give l\ndone\nset a to [1]\nset b to grow(a)\nsay a\nsay b";
    assert_eq!(out(src), vec!["1", "1, 9"]);
    let src = "task rename(r)\n  set r.name to 'B'\n  give r\ndone\nset p to {name: 'A'}\nset q to rename(p)\nsay p.name & q.name";
    assert_eq!(out(src), vec!["AB"]);
}

#[test]
fn recursion() {
    let src = "task fact(n)\n  when n <= 1\n    give 1\n  done\n  give n * fact(n - 1)\ndone\nsay fact(10)";
    assert_eq!(out(src), vec!["3628800"]);
    let src = "task down(n)\n  when n is 0\n    give 0\n  done\n  give down(n - 1)\ndone\nsay down(39)";
    assert_eq!(out(src), vec!["0"]);
    let e = run("task down(n)\n  when n is 0\n    give 0\n  done\n  give down(n - 1)\ndone\nsay down(100)").error.unwrap();
    assert!(e.limit && e.message.contains("too deep"), "{e}");
    // a loop that calls a task many times gives the names' memory back each time
    let src = "task f(a, b, c)\n  set d to a + b + c\n  give d\ndone\nset t to 0\nrepeat 10000 times\n  set t to f(t, 1, 0)\ndone\nsay t";
    let o = run(src);
    assert_eq!(o.output, vec!["10000"], "{:?}", o.error.map(|e| e.to_string()));
    assert!(o.peak_memory < 10_000, "peak {}", o.peak_memory);
}

#[test]
fn stop_inside_a_task_ends_the_run() {
    let o = run("task halt()\n  say 'stopping'\n  stop\ndone\nwrite('desk:a', 1)\nhalt()\nsay 'never'");
    assert_eq!(o.error, None);
    assert_eq!(o.output, vec!["stopping"]);
    assert_eq!(o.actions.len(), 1, "stop finishes: the changes are made");
}

#[test]
fn mistakes_show_where_the_task_was_called() {
    let src = "task inner(x)\n  give x + 'a'\ndone\ntask outer(x)\n  give inner(x)\ndone\nsay 1\nsay outer(1)";
    let e = run(src).error.unwrap();
    assert_eq!(e.line, 2);
    assert_eq!(e.trail, vec![("main".to_string(), 5), ("main".to_string(), 8)]);
    let shown = e.to_string();
    assert!(shown.starts_with("desk:main line 2: + needs two numbers"), "{shown}");
    assert!(shown.contains("called from desk:main line 5\n  called from desk:main line 8"), "{shown}");
}

#[test]
fn checked_before_running() {
    for (src, want) in [
        ("say nope(1)", "no task or built-in called nope"),
        ("task f(a)\ndone\nsay f()", "needs 1 value in its brackets, got 0"),
        ("task f(a, b)\ndone\nf(1)", "needs 2 values in its brackets, got 1"),
        ("task f()\ndone\ntask f()\ndone", "two tasks called f"),
        ("task count(x)\ndone", "that is a built-in function"),
        ("task f(a, a)\ndone", "the input a is in this task twice"),
        ("when yes\n  task f()\n  done\ndone", "only be made at the top of a page"),
        ("task f()\n  task g()\n  done\ndone", "only be made at the top of a page"),
        ("give 1", "give only works inside a task"),
        ("task f()\n  skip\ndone", "only works inside a loop"),
        ("task f(", "task ( needs a name here"),
        ("task f(a", "is not closed"),
        ("task f", "task needs: task f(input, ...)"),
        ("task f()\nsay 1", "this task has no done"),
        ("fixed rate is 21\nset rate to 3", "rate is a fixed value: it can't be changed"),
        ("fixed rate is 21\nadd 1 to rate", "rate is a fixed value"),
        ("fixed rate is 21\nfor each rate in [1]\ndone", "rate is a fixed value"),
        ("fixed rate is 21\ntask f(rate)\ndone", "the input rate has the name of a fixed value"),
        ("fixed a is 1\nfixed a is 2", "two fixed values called a"),
        ("fixed sum is 1", "that is a built-in function"),
        ("fixed a is b\nfixed b is 1", "only use earlier fixed values"),
        ("fixed a is read('notes:shopping')", "can't use read(...)"),
        ("fixed a is today()", "can't use today(...)"),
        ("task f()\n  give 1\ndone\nfixed a is f()", "can't call a task"),
        ("when yes\n  fixed a is 1\ndone", "only be made at the top of a page"),
    ] {
        let o = run(src);
        let e = o.error.unwrap_or_else(|| panic!("should be refused: {src}"));
        assert!(e.before_running, "{src}: found while running: {e}");
        assert!(e.to_string().contains(want), "{src}: {e} should contain {want}");
    }
    // a checked mistake means nothing at all ran
    let o = run("say 'hi'\nsay nope()");
    assert!(o.output.is_empty());
}

#[test]
fn fixed_values() {
    let src = "fixed rate is 21\nfixed names is ['a', 'b']\nfixed info is {rate: rate, line: upper('x') & newline() & 'y'}\ntask tax(n)\n  give n * rate / 100\ndone\nsay tax(200)\nsay count(names)\nsay info.rate\nsay lines(info.line)";
    assert_eq!(out(src), vec!["42", "2", "21", "X, y"]);
    // nothing inside a fixed value can be changed, even through a copy
    let src = "fixed base is {items: [1]}\nset c to base\nadd 2 to c.items\nsay base.items\nsay c.items";
    assert_eq!(out(src), vec!["1", "1, 2"]);
    // a mistake while working out a fixed value names its line
    let e = run("say 1\nfixed bad is 1 / 0").error.unwrap();
    assert_eq!(e.line, 2);
    assert!(e.message.contains("dividing by 0"));
}

#[test]
fn library_text() {
    assert_eq!(say("upper('aBc')"), "ABC");
    assert_eq!(say("'[' & trim('  hi  ') & ']'"), "[hi]");
    assert_eq!(say("split('a,b,,c', ',')"), "a, b, , c");
    assert_eq!(say("count(split('abc', ','))"), "1");
    has(&err("say split('abc', '')"), "needs something to split on");
    assert_eq!(say("join(['a', 1, yes], '-')"), "a-1-yes");
    assert_eq!(say("join([], '-') is ''"), "yes");
    assert_eq!(say("replace('a-b-c', '-', '+')"), "a+b+c");
    assert_eq!(say("replace('aaa', 'a', 'bb')"), "bbbbbb");
    has(&err("say replace('a', '', 'x')"), "not empty");
    assert_eq!(say("find('hello', 'l')"), "3");
    assert_eq!(say("find('hello', 'z')"), "0");
    assert_eq!(say("slice('hello', 2, 4)"), "ell");
    assert_eq!(say("slice('hello', 0, 99)"), "hello");
    assert_eq!(say("slice('hello', 4, 2) is ''"), "yes");
    assert_eq!(say("slice([1, 2, 3, 4], 2, 3)"), "2, 3");
    assert_eq!(say("starts('hello', 'he') and ends('hello', 'lo')"), "yes");
    assert_eq!(say("count(newline())"), "1");
    assert_eq!(say("text([1, {a: 2}])"), "1, {a: 2}");
    assert_eq!(say("'[' & pad('ab', 4) & ']'"), "[ab  ]");
    assert_eq!(say("'[' & pad('ab', -4) & ']'"), "[  ab]");
    assert_eq!(say("pad('abcdef', 3)"), "abcdef");
}

#[test]
fn library_numbers_and_lists() {
    assert_eq!(say("round(2.345, 2)"), "2.35");
    assert_eq!(say("round(-2.5, 0)"), "-3");
    assert_eq!(say("floor(2.7) & ' ' & floor(-2.2) & ' ' & ceil(2.2) & ' ' & abs(-3)"), "2 -3 3 3");
    assert_eq!(say("min(3, 5) & max(3, 5) & min('b', 'a')"), "35a");
    has(&err("say min(1, 'a')"), "two numbers or two texts");
    assert_eq!(say("is_number('12.5') and not is_number('x')"), "yes");
    assert_eq!(say("sum([1, 2, 3.5])"), "6.5");
    assert_eq!(say("sum([])"), "0");
    has(&err("say sum([1, 'a'])"), "item 2 is the text 'a'");
    assert_eq!(say("sort([3, 1, 2])"), "1, 2, 3");
    assert_eq!(say("sort(['pear', 'apple'])"), "apple, pear");
    has(&err("say sort([1, 'a'])"), "only numbers or only texts");
    assert_eq!(say("reverse([1, 2, 3])"), "3, 2, 1");
    assert_eq!(say("unique([1, 2, 1, '1', [1], [1]])"), "1, 2, 1, 1");
    assert_eq!(say("count(unique([1, 2, 1, '1', [1], [1]]))"), "4");
    assert_eq!(say("first([4, 5]) & last([4, 5])"), "45");
    has(&err("say first([])"), "the list is empty");
    assert_eq!(say("range(1, 5)"), "1, 2, 3, 4, 5");
    assert_eq!(say("count(range(5, 1))"), "0");
    has(&err("say range(1, 3000)"), "more than 2000 items");
    assert_eq!(say("position(['a', 'b'], 'b') & position(['a'], 'z')"), "20");
    assert_eq!(say("without([1, 2, 3], 2)"), "1, 3");
    has(&err("say without([1], 2)"), "has 1 items");
}

#[test]
fn library_records_and_dates() {
    assert_eq!(say("keys({b: 1, a: 2})"), "b, a");
    assert_eq!(say("has_key({name: 1}, 'NAME') and not has_key({}, 'x')"), "yes");
    assert_eq!(say("fields('mail:inbox:m2')"), "{from: lina, to: me, subject: Lesson}");
    assert_eq!(say("fields('mail:inbox:m2').to"), "me");
    assert_eq!(say("fields('contacts:lina').note"), "piano teacher");
    assert_eq!(say("{to: 1, in: 2}.in"), "2");
    assert_eq!(out("set m to {}\nset m.to to 'x'\nsay m"), vec!["{to: x}"]);
    assert_eq!(say("days_between('2026-11-12', '2026-12-25')"), "43");
    assert_eq!(say("days_between('2026-11-12', '2026-11-01')"), "-11");
    assert_eq!(say("month('2026-11-12') & '/' & year('2026-11-12')"), "11/2026");
    has(&err("say month('soon')"), "not a date");
}

#[test]
fn new_words_still_work_as_names() {
    // old chores may use these as names; they still work
    assert_eq!(out("set task to 1\nset give to 2\nset fixed to 3\nsay task + give + fixed"), vec!["6"]);
    // a name and a task may share a name: total is the name, total(...) the task
    assert_eq!(out("task total(l)\n  give sum(l)\ndone\nset total to total([1, 2])\nsay total"), vec!["3"]);
}

#[test]
fn a_long_trail_is_shown_short() {
    let src = "task down(n)\n  when n is 0\n    give 1 / 0\n  done\n  give down(n - 1)\ndone\nsay down(30)";
    let e = run(src).error.unwrap();
    assert_eq!(e.trail.len(), 31);
    assert_eq!(e.to_string(), "desk:main line 3: dividing by 0 is not possible\n  called from desk:main line 5 (30 times)\n  called from desk:main line 7");
}
