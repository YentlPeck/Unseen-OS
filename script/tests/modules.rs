//! M3 (design §4 "Modules", "Mistakes and try", §6 "Tests"): use, fail, try/instead, and test
//! pages with check, pretend and the test functions.
extern crate alloc;

#[path = "../../kernel/src/fs.rs"]
#[allow(dead_code)]
mod fs;
mod common;

use common::*;

/// A desk with a few program pages on it (plus the usual test pages).
fn desk(pages: &[(&str, &str)]) -> TestWorld {
    let mut start = String::from(START);
    for (p, text) in pages {
        start.push_str(&format!("#page {p}\n{text}\n#end\n"));
    }
    TestWorld::from(&start)
}

fn run_on(w: &TestWorld, src: &str) -> script::Outcome {
    script::run("apps:main", src, w)
}

#[test]
fn use_loads_tasks_and_fixed_values() {
    let w = desk(&[
        ("lib:money", "fixed rate is 21\ntask tax(n)\n  give n * rate / 100\ndone"),
        ("lib:text", "use 'desk:lib:money'\ntask price(n)\n  give 'EUR ' & (n + tax(n))\ndone"),
    ]);
    let o = run_on(&w, "use 'desk:lib:text'\nsay price(100)\nsay rate");
    assert_eq!(o.output, vec!["EUR 121", "21"], "{:?}", o.error.map(|e| e.to_string()));
    // two pages using the same page: loaded once, no clash
    let o = run_on(&w, "use 'lib:money'\nuse 'lib:text'\nsay tax(100)");
    assert_eq!(o.output, vec!["21"], "{:?}", o.error.map(|e| e.to_string()));
}

#[test]
fn use_mistakes() {
    let w = desk(&[
        ("lib:a", "use 'lib:b'\ntask fa()\ndone"),
        ("lib:b", "use 'lib:a'\ntask fb()\ndone"),
        ("lib:self", "use 'lib:self'"),
        ("lib:loose", "task ok()\ndone\nsay 'I run by myself'"),
        ("lib:clash", "task tax(n)\n  give 0\ndone"),
        ("lib:money", "task tax(n)\n  give n / 5\ndone"),
        ("lib:bad", "task broken(\n"),
        ("lib:oops", "task half(n)\n  give n / 0\ndone"),
    ]);
    let e = |src: &str| run_on(&w, src).error.expect("must fail");
    let x = e("use 'lib:a'\nsay 1");
    assert!(x.before_running && x.message.contains("pages can't use each other in a loop"), "{x}");
    assert!(e("use 'lib:self'").message.contains("uses itself"));
    let x = e("use 'lib:nowhere'");
    assert!(x.message.contains("no page called desk:lib:nowhere") && x.line == 1, "{x}");
    let x = e("use 'lib:loose'");
    assert_eq!(x.page, "lib:loose");
    assert!(x.message.contains("may only hold use, fixed and task lines"), "{x}");
    let x = e("use 'lib:clash'\nuse 'lib:money'");
    assert!(x.message.contains("two tasks called tax"), "{x}");
    let x = e("use 'lib:bad'");
    assert_eq!(x.page, "lib:bad");
    let x = e("say 1\nuse 'lib:money'");
    assert!(x.message.contains("use must come at the top"), "{x}");
    let x = e("when yes\n  use 'lib:money'\ndone");
    assert!(x.message.contains("use must come at the top"), "{x}");
    // a mistake in a used page names that page, and the call trail names the caller's page
    let x = e("use 'lib:oops'\nsay 1\nsay half(4)");
    assert_eq!((x.page.as_str(), x.line), ("lib:oops", 2));
    assert_eq!(x.trail, vec![("apps:main".to_string(), 3)]);
    assert!(x.to_string().contains("desk:lib:oops line 2: dividing by 0") && x.to_string().contains("called from desk:apps:main line 3"));
}

#[test]
fn fail_and_try() {
    assert_eq!(
        out("try\n  say 'in try'\n  fail 'no ' & 'luck'\n  say 'never'\ninstead problem\n  say 'caught: ' & problem\ndone\nsay 'after'"),
        vec!["caught: no luck", "after"]
    );
    // everything the try part did is undone: names, say lines, planned changes
    let o = run("set x to 1\ntry\n  set x to 2\n  set y to 3\n  say 'gone'\n  write('desk:a', 1)\n  fail 'stop here'\ninstead p\n  say 'x is ' & x\ndone\nsay exists('desk:a')\ntry\n  say y\ninstead p2\n  say p2\ndone");
    assert_eq!(o.output, vec!["x is 1", "no", "y has no value yet (give it one with set y to ...)"], "{:?}", o.error);
    assert!(o.actions.is_empty());
    // a try part that works keeps everything
    let o = run("try\n  write('desk:a', 1)\n  set x to 5\ninstead p\n  say p\ndone\nsay x");
    assert_eq!(o.output, vec!["5"]);
    assert_eq!(o.actions.len(), 1);
    // mistakes inside tasks are caught too, with their message only
    assert_eq!(out("task risky(n)\n  give 10 / n\ndone\ntry\n  say risky(0)\ninstead p\n  say 'no: ' & p\ndone"), vec!["no: dividing by 0 is not possible"]);
    // nested: the inner try catches first; the outer undo then also undoes what the inner said
    assert_eq!(out("try\n  try\n    fail 'inner'\n  instead a\n    say 'A ' & a\n  done\n  fail 'outer'\ninstead b\n  say 'B ' & b\ndone"), vec!["B outer"]);
    // a mistake in the instead part is a normal mistake
    has(&err("try\n  fail 'x'\ninstead p\n  say 1 / 0\ndone"), "dividing by 0");
    // fail without try ends the program with that message
    let e = run("say 1\nfail 'the list is empty'").error.unwrap();
    assert_eq!((e.line, e.message.as_str()), (2, "the list is empty"));
    // limits and stop are never caught
    let e = run("try\n  while yes\n  done\ninstead p\n  say 'caught'\ndone").error.unwrap();
    assert!(e.limit);
    let o = run("try\n  stop\ninstead p\n  say 'caught'\ndone\nsay 'after'");
    assert!(o.error.is_none() && o.output.is_empty());
    // skip and quit pass through try
    assert_eq!(out("for each x in [1, 2, 3]\n  try\n    when x is 2\n      skip\n    done\n    say x\n  instead p\n  done\ndone"), vec!["1", "3"]);
    // try in a loop, failing every time, doesn't use up memory
    let o = run("set n to 0\nrepeat 1000 times\n  try\n    set big to pad('', 2000)\n    write('desk:x', big)\n    send('nobody', 'Hi', big)\n  instead p\n    set n to n + 1\n  done\ndone\nsay n");
    assert_eq!(o.output, vec!["1000"], "{:?}", o.error.map(|e| e.to_string()));
    assert!(o.peak_memory < 50_000, "peak {}", o.peak_memory);
    has(&err("try\n  say 1\ndone"), "this try has no instead");
    has(&err("instead p"), "no try before it");
    has(&err("when yes\ninstead p\ndone"), "instead only belongs to try");
    has(&err("fixed p is 1\ntry\n  say 1\ninstead p\ndone"), "p is a fixed value");
}

fn tests_on(w: &TestWorld, src: &str) -> script::TestReport {
    script::run_tests("apps:shop-tests", src, w)
}

#[test]
fn test_pages() {
    let w = desk(&[(
        "apps:shop-lib",
        "fixed list_page is 'desk:shop:list'\ntask load_items()\n  when exists(list_page)\n    give lines(read(list_page))\n  done\n  give []\ndone\ntask save_items(items)\n  write(list_page, join(items, newline()))\ndone",
    )]);
    let src = "use 'desk:apps:shop-lib'

test 'nothing saved yet'
  check load_items() is []
done

test 'it reads the saved list'
  pretend 'desk:shop:list' holds 'milk' & newline() & 'eggs'
  check load_items() is ['milk', 'eggs']
done

test 'saving plans a write'
  pretend 'desk:shop:list' holds 'old'
  save_items(['tea'])
  check page_now('desk:shop:list') is 'old'
  check after('desk:shop:list') is 'tea'
  check after('desk:shop:other') is nothing()
done

test 'a wrong check fails'
  check count(load_items()) is 3
done

test 'no checks'
  say 'hi'
done

test 'a mistake fails'
  check 1 / 0 is 1
done";
    let r = tests_on(&w, src);
    assert!(r.error.is_none(), "{:?}", r.error.map(|e| e.to_string()));
    let lines = r.lines();
    assert_eq!(lines[0], "PASS nothing saved yet");
    assert_eq!(lines[1], "PASS it reads the saved list");
    assert_eq!(lines[2], "PASS saving plans a write");
    assert_eq!(lines[3], "FAIL a wrong check fails: line 21: check failed: got 0, expected 3");
    assert_eq!(lines[4], "FAIL no checks: this test has no check");
    assert_eq!(lines[5], "FAIL a mistake fails: line 29: dividing by 0 is not possible");
    assert_eq!(lines[6], "3 of 6 tests passed");
    assert!(!r.all_passed());
}

#[test]
fn tests_start_fresh() {
    let w = desk(&[]);
    let src = "test 'one'
  pretend 'desk:notes:x' holds 'a'
  set n to 1
  check exists('notes:x')
  check not exists('notes:shopping')
done
test 'two'
  check not exists('notes:x')
  try
    say n
  instead p
    say p
  done
  check said() is ['n has no value yet (give it one with set n to ...)']
  check today() is '2026-11-14'
  check count(pages('mail:inbox')) is 0
done
test 'three'
  pretend today is '2027-01-01'
  check today() is '2027-01-01'
done";
    let r = tests_on(&w, src);
    assert!(r.all_passed(), "{:?}", r.lines());
}

#[test]
fn test_functions_see_what_was_planned() {
    let w = desk(&[]);
    let src = "test 'mail and events'
  pretend 'contacts:mira' holds 'name: Mira'
  send('mira', 'Hi', 'Hello Mira')
  event('2026-12-01', '10:00', 'Dentist', '')
  say 'done'
  check sent() is [{to: 'mira', subject: 'Hi', text: 'Hello Mira'}]
  check first(events()).title is 'Dentist'
  check said() is ['done']
  check count(sent()) is 1
done
test 'the real desk is not seen'
  check not exists('contacts:omar')
done
test 'refused send fails the test'
  send('nobody', 'Hi', 'x')
  check yes
done";
    let r = tests_on(&w, src);
    let lines = r.lines();
    assert_eq!(lines[0], "PASS mail and events", "{lines:?}");
    assert_eq!(lines[1], "PASS the real desk is not seen");
    assert!(lines[2].starts_with("FAIL refused send fails the test: line 15: no contact has the address 'nobody'"), "{lines:?}");
}

#[test]
fn test_page_mistakes() {
    let w = desk(&[]);
    let e = |src: &str| tests_on(&w, src).error.expect("must be refused").to_string();
    assert!(e("say 1\ntest 'x'\n  check yes\ndone").contains("may only hold use, fixed, task and test lines"));
    assert!(e("task f()\ndone").contains("has no tests"));
    assert!(e("test 'x'\n  check yes\n  pretend 'a' holds 'b'\ndone").contains("pretend must come at the top of the test"));
    assert!(e("test 'x'\n  when yes\n    pretend 'a' holds 'b'\n  done\ndone").contains("pretend only works at the top of a test"));
    assert!(e("test x\ndone").contains("test needs: test 'what it checks'"));
    assert!(e("test 'x'\n  test 'y'\n  done\ndone").contains("only be made at the top"));
    // check, pretend and the test functions only work in tests
    has(&err("check yes"), "check only works inside a test");
    has(&err("say said()"), "said(...) only works inside a test");
    has(&err("task f()\n  give sent()\ndone"), "sent(...) only works inside a test");
    has(&err("pretend 'a' holds 'b'"), "pretend only works at the top of a test");
    // a pretend that can't be made fails that test only
    let r = tests_on(&w, "test 'bad'\n  pretend 'mail' holds 'x'\n  check yes\ndone\ntest 'good'\n  check yes\ndone");
    let lines = r.lines();
    assert!(lines[0].starts_with("FAIL bad: line 2: desk:mail is a drawer"), "{lines:?}");
    assert_eq!(lines[1], "PASS good");
    // stop and limits fail the test
    let r = tests_on(&w, "test 'stops'\n  check yes\n  stop\ndone\ntest 'loops'\n  while yes\n  done\n  check yes\ndone");
    let lines = r.lines();
    assert_eq!(lines[0], "FAIL stops: stop ended the test before its done");
    assert!(lines[1].contains("too many steps"), "{lines:?}");
    // a failed check can't be caught by try
    let r = tests_on(&w, "test 'caught?'\n  try\n    check 1 is 2\n  instead p\n    say p\n  done\n  check yes\ndone");
    assert!(r.lines()[0].starts_with("FAIL caught?: line 3: check failed"), "{:?}", r.lines());
}
