//! Apps (design §5) and app tests (§6): the shopping list of §3, the step loop, the screen,
//! what each part may do, and launch/press in tests.
extern crate alloc;

#[path = "../../kernel/src/fs.rs"]
#[allow(dead_code)]
mod fs;
mod common;

use common::TestWorld;
use script::{App, Step};

const SHOP: &str = "\
app 'Shopping list'
use 'desk:apps:shop-lib'

task start()
  give {items: load_items(), picked: 1, typing: ''}
done

task key(s, k)
  when k is 'down' and s.picked < count(s.items)
    set s.picked to s.picked + 1
  or when k is 'up' and s.picked > 1
    set s.picked to s.picked - 1
  or when k is 'enter' and s.typing is not ''
    add s.typing to s.items
    set s.typing to ''
    save_items(s.items)
  or when k is 'delete' and count(s.items) > 0
    set s.items to without(s.items, s.picked)
    set s.picked to max(min(s.picked, count(s.items)), 1)
    save_items(s.items)
  or when k is 'esc'
    close()
  otherwise
    set s.typing to edit_text(s.typing, k)
  done
  give s
done

task draw(s)
  title('Shopping list - ' & count(s.items) & ' items')
  for each n in range(1, min(count(s.items), 20))
    when n is s.picked
      show(n, 2, '> ' & item(s.items, n), 'bright')
    otherwise
      show(n, 2, '  ' & item(s.items, n))
    done
  done
  show(22, 2, 'new: ' & s.typing)
  cursor(22, 7 + count(s.typing))
  help('Up/Down pick  Enter add  Delete remove')
done
";

const SHOP_LIB: &str = "\
fixed list_page is 'desk:shop-list'

task load_items()
  when exists(list_page)
    give lines(read(list_page))
  done
  give []
done

task save_items(items)
  write(list_page, join(items, newline()))
done
";

const SHOP_TESTS: &str = "\
use 'desk:apps:shop'

test 'enter adds the typed item and saves it'
  pretend 'desk:shop-list' holds 'milk'
  launch 'desk:apps:shop'
  press 'b'
  press 'enter'
  check app_state().items is ['milk', 'b']
  check page_now('desk:shop-list') is 'milk' & newline() & 'b'
  check screen_line(2) has 'b'
done

test 'the picked item is shown bright'
  pretend 'desk:shop-list' holds 'milk' & newline() & 'eggs'
  launch 'desk:apps:shop'
  press 'down'
  check screen_line(2) has '> eggs'
  check screen_colour(2, 2) is 'bright'
done

test 'up and down on an empty list change nothing'
  launch 'desk:apps:shop'
  press 'down'
  press 'up'
  check app_state().picked is 1
  press 'x'
  press 'enter'
  check app_state().items is ['x']
done
";

fn page(path: &str, text: &str) -> String {
    format!("#page {path}\n{text}\n#end\n")
}

/// The desk of the tests: the shopping list app, and `extra` pages.
fn desk(extra: &[(&str, &str)]) -> TestWorld {
    let mut start = String::from(common::START);
    start.push_str(&page("apps:shop", SHOP.trim_end()));
    start.push_str(&page("apps:shop-lib", SHOP_LIB.trim_end()));
    start.push_str(&page("apps:shop-tests", SHOP_TESTS.trim_end()));
    for (p, t) in extra {
        start.push_str(&page(p, t));
    }
    TestWorld::from(&start)
}

fn tests(w: &TestWorld, src: &str) -> Vec<String> {
    script::run_tests("tests", src, w).lines()
}

/// Run a test page and check every test passed.
fn passes(w: &TestWorld, src: &str) {
    let l = tests(w, src);
    assert!(l.last().unwrap().starts_with(&format!("{n} of {n}", n = l.len() - 1)), "{l:?}");
}

fn open(w: &TestWorld, src: &str) -> App {
    let mut app = App::load("apps:x", src, w).unwrap_or_else(|e| panic!("{e}"));
    let step = app.start(w).unwrap_or_else(|e| panic!("{e}"));
    app.keep(step);
    app
}

/// A key step that must work; gives its changes.
fn press(app: &mut App, w: &TestWorld, k: &str) -> Vec<script::Action> {
    let step: Step = app.key(k, w).unwrap_or_else(|e| panic!("{e}"));
    let a = step.actions.clone();
    app.keep(step);
    a
}

/// The mistake of a step.
fn step_error(app: &App, w: &TestWorld, k: &str) -> String {
    match app.key(k, w) {
        Ok(_) => panic!("expected a mistake for key {k}"),
        Err(e) => e.to_string(),
    }
}

/// A tiny app around one key task and one draw task.
fn tiny(key: &str, draw: &str) -> String {
    format!("app 'Tiny'\ntask start()\n  give {{n: 0}}\ndone\ntask key(s, k)\n{key}\n  give s\ndone\ntask draw(s)\n{draw}\ndone")
}

#[test]
fn the_shopping_list_tests_pass() {
    let w = desk(&[]);
    let l = tests(&w, SHOP_TESTS);
    assert_eq!(
        l,
        [
            "PASS enter adds the typed item and saves it",
            "PASS the picked item is shown bright",
            "PASS up and down on an empty list change nothing",
            "3 of 3 tests passed"
        ]
    );
}

#[test]
fn the_shopping_list_works_by_keys() {
    let w = desk(&[("shop-list", "milk\neggs")]);
    let mut app = App::load("apps:shop", SHOP, &w).unwrap();
    assert_eq!(app.title(), "Shopping list");
    let step = app.start(&w).unwrap();
    assert!(step.actions.is_empty() && !step.close);
    app.keep(step);
    let s = app.screen();
    assert_eq!(s.line(0), "Shopping list - 2 items");
    assert_eq!(s.line(1), " > milk");
    assert_eq!(s.line(2), "   eggs");
    assert_eq!(s.line(22), " new:");
    assert_eq!(s.line(24), "Up/Down pick  Enter add  Delete remove   Ctrl+Q close");
    assert_eq!(s.cell(1, 2), ('>', 1));
    assert_eq!(s.cell(2, 2), (' ', 0));
    assert_eq!(s.cursor, Some((22, 7)));
    for k in ["t", "e", "a"] {
        assert!(press(&mut app, &w, k).is_empty());
    }
    assert_eq!(app.screen().line(22), " new: tea");
    assert_eq!(app.screen().cursor, Some((22, 10)));
    let a = press(&mut app, &w, "enter");
    assert_eq!(a, [script::Action::Write { page: "shop-list".into(), text: "milk\neggs\ntea".into() }]);
    assert_eq!(app.screen().line(3), "   tea");
    press(&mut app, &w, "down");
    assert_eq!(app.screen().line(2), " > eggs");
    let a = press(&mut app, &w, "delete");
    assert_eq!(a, [script::Action::Write { page: "shop-list".into(), text: "milk\ntea".into() }]);
    assert_eq!(app.screen().line(0), "Shopping list - 2 items");
    // esc closes it: the step's changes are made, draw is skipped
    let step = app.key("esc", &w).unwrap();
    let screen = app.screen().clone();
    assert!(step.close);
    assert!(app.keep(step));
    assert_eq!(app.screen(), &screen, "draw did not run");
}

#[test]
fn a_failing_step_gives_nothing_and_keeps_the_old_state() {
    let w = desk(&[]);
    let src = tiny("  set s.n to s.n + 1\n  when k is 'x'\n    set s.n to s.n / 0\n  done", "  show(1, 1, 'n = ' & s.n)");
    let mut app = open(&w, &src);
    press(&mut app, &w, "a");
    assert_eq!(app.screen().line(1), "n = 1");
    let e = step_error(&app, &w, "x");
    assert_eq!(e, "desk:apps:x line 8: dividing by 0 is not possible");
    assert_eq!(app.screen().line(1), "n = 1");
    // a mistake in draw: the key part's changes are not given either
    let src = tiny("  write('desk:a', 'x')", "  show(1, 1, 1 + 'a')");
    let app = open_after_start(&w, &src);
    assert!(step_error(&app, &w, "a").contains("+ needs two numbers"));
}

/// An app whose draw fails only after the first key.
fn open_after_start(w: &TestWorld, src: &str) -> App {
    let src = src.replace("task draw(s)\n", "task draw(s)\n  when s.n is 0\n    give\n  done\n").replace("give {n: 0}", "give {n: 0}");
    let src = src.replace("  give s\ndone\ntask draw", "  set s.n to 1\n  give s\ndone\ntask draw");
    open(w, &src)
}

#[test]
fn what_each_part_may_do() {
    let w = desk(&[]);
    let cases = [
        (tiny("  say 'hi'", "  show(1, 1, 'x')"), "key can't use say"),
        (tiny("  show(1, 1, 'x')", "  show(1, 1, 'x')"), "show(...) only works in an app's draw"),
        (tiny("  stop", "  show(1, 1, 'x')"), "stop doesn't work in an app: close() closes it"),
        (tiny("", "  close()"), "draw can't use close()"),
        (tiny("", "  write('desk:a', 'x')"), "draw can't use write(...): only start and key can change pages"),
        (tiny("", "  show(1, 1, read('notes:shopping'))"), "draw can't use read(...)"),
        (tiny("", "  show(1, 1, today())"), "draw can't use today(...)"),
        (tiny("", "  show(24, 1, 'x')"), "the row must be a whole number from 1 to 23"),
        (tiny("", "  show(1, 79, 'x')"), "the column must be a whole number from 1 to 78"),
        (tiny("", "  show(1, 1, 'a' & newline() & 'b')"), "can't hold a line break"),
        (tiny("", "  show(1, 1, 'x', 'pink')"), "the text 'pink' is not a colour"),
        (tiny("", "  fill(1, 'red')"), "is not a colour"),
    ];
    for (src, want) in cases {
        let app = open_after_start(&w, &src);
        let e = step_error(&app, &w, "a");
        assert!(e.contains(want), "{e:?} should contain {want:?}\n{src}");
    }
    // start must give, and so must key
    let src = tiny("", "").replace("  give {n: 0}\n", "");
    let app = App::load("apps:x", &src, &w).unwrap();
    let e = app.start(&w).err().expect("no give").to_string();
    assert_eq!(e, "desk:apps:x line 2: start ended without give: it must give the app's state (give s)");
    let src = tiny("  when k is 'q'\n    give s\n  done\n  set s.n to 1", "").replace("  set s.n to 1\n  give s\ndone", "  set s.n to 1\ndone");
    let app = open(&w, &src);
    assert!(step_error(&app, &w, "a").contains("key ended without give"));
    // helpers are checked for the part that runs them
    let src = "app 'H'\ntask note()\n  say 'x'\ndone\ntask start()\n  note()\n  give 1\ndone\ntask key(s, k)\n  give s\ndone\ntask draw(s)\ndone";
    let app = App::load("apps:x", src, &w).unwrap();
    let e = app.start(&w).err().unwrap().to_string();
    assert!(e.starts_with("desk:apps:x line 3: start can't use say") && e.contains("called from desk:apps:x line 6"), "{e}");
}

#[test]
fn app_pages_are_checked_before_they_open() {
    let w = desk(&[]);
    let load = |src: &str| App::load("apps:x", src, &w).err().map(|e| e.to_string()).unwrap_or_default();
    assert_eq!(load("task start()\n  give 1\ndone"), "desk:apps:x: this page is not an app: an app page starts with app 'TITLE'");
    assert!(load("app 'A'\ntask start()\n  give 1\ndone\ntask draw(s)\ndone").contains("an app needs the task key: task key(state, k)"));
    assert!(load("app 'A'\ntask start()\n  give 1\ndone\ntask key(s)\n  give s\ndone\ntask draw(s)\ndone").contains("desk:apps:x line 5: key must have 2 inputs"));
    assert!(load("app 'A'\nsay 1").contains("line 2: an app page may only hold app, use, fixed and task lines"));
    let e = script::run("main", "set x to 1\napp 'A'", &w).error.unwrap().to_string();
    assert!(e.contains("line 2: app must be the first line of the page"), "{e}");
    let e = script::run("main", "app A", &w).error.unwrap().to_string();
    assert!(e.contains("app needs: app 'TITLE'"), "{e}");
    let e = script::run("apps:shop", SHOP, &w).error.unwrap().to_string();
    assert!(e.contains("desk:apps:shop: this page is an app: open it with open"), "{e}");
    // launch and press only in tests, screen functions only on their own line
    assert!(script::run("main", "launch 'desk:apps:shop'", &w).error.unwrap().message.contains("only works inside a test"));
    assert!(script::run("main", "set x to show(1, 1, 'a')", &w).error.unwrap().message.contains("must be on a line of its own"));
    assert!(script::run("main", "task f()\n  show(1, 1)\ndone", &w).error.unwrap().message.contains("needs 3 or 4 values"));
    // a used app page: its app line is ignored, its tasks can be called (and checked when they run)
    let o = script::run("main", "use 'desk:apps:shop-lib'\nsay count(load_items())", &w);
    assert_eq!(o.output, ["0"]);
    let o = script::run("main", "use 'desk:apps:shop'\nsay start().picked\ndraw(start())", &w);
    assert_eq!(o.output, ["1"]);
    assert!(o.error.unwrap().message.contains("title(...) only works in an app's draw"));
}

#[test]
fn the_screen() {
    let w = desk(&[]);
    let draw = "  fill(3, 'select')\n  show(3, 2, 'picked')\n  show(3, 10, 'red', 'warn')\n  show(5, 75, 'abcdefgh')\n  show(6, 1, 'é' & 3)\n  show(7, 1, [1, 2])";
    let app = open(&w, &tiny("", draw));
    let s = app.screen();
    assert_eq!(s.line(0), "Tiny", "the default title");
    assert_eq!(s.line(24), "Ctrl+Q close", "the default help");
    assert_eq!(s.cell(3, 1), (' ', 4), "fill colours the whole row");
    assert_eq!(s.cell(3, 2), ('p', 4), "show without a colour keeps the row's colour");
    assert_eq!(s.cell(3, 10), ('r', 3));
    assert_eq!(s.cell(3, 78), (' ', 4));
    assert_eq!(s.line(5), format!("{}abcd", " ".repeat(74)), "cut at column 78");
    assert_eq!(s.line(6), "é3");
    assert_eq!(s.line(7), "1, 2");
    assert_eq!(s.cursor, None, "hidden unless cursor(...) is called");
    // help that already has Ctrl+Q is kept; a long one is cut so Ctrl+Q close fits
    let app = open(&w, &tiny("", "  help('Enter go  Ctrl+Q close')"));
    assert_eq!(app.screen().line(24), "Enter go  Ctrl+Q close");
    let app = open(&w, &tiny("", "  help('Ctrl+Q quit  Enter go')"));
    assert_eq!(app.screen().line(24), "Ctrl+Q quit  Enter go   Ctrl+Q close", "mentioning Ctrl+Q is not enough");
    // the hint must still be there after the help is cut (review of M5)
    let app = open(&w, &tiny("", "  help(pad('', 78) & 'Ctrl+Q close')"));
    assert_eq!(app.screen().line(24), "Ctrl+Q close");
    let app = open(&w, &tiny("", "  help(pad('x', 90))"));
    assert!(app.screen().line(24).ends_with("x   Ctrl+Q close") && app.screen().line(24).chars().count() <= 78);
    // the screen is empty at the start of every draw
    let src = tiny("  set s.n to s.n + 1", "  when s.n is 0\n    show(1, 1, 'first')\n  done\n  show(2, 1, s.n)");
    let mut app = open(&w, &src);
    assert_eq!(app.screen().line(1), "first");
    press(&mut app, &w, "a");
    assert_eq!(app.screen().line(1), "");
    assert_eq!(app.screen().line(2), "1");
}

#[test]
fn try_undoes_screen_output_and_close() {
    let w = desk(&[]);
    let draw = "  try\n    show(1, 1, 'lost')\n    fail 'no'\n  instead p\n    show(2, 1, 'kept ' & p)\n  done";
    let app = open(&w, &tiny("", draw));
    assert_eq!(app.screen().line(1), "");
    assert_eq!(app.screen().line(2), "kept no");
    let key = "  try\n    close()\n    fail 'no'\n  instead p\n  done";
    let app = open(&w, &tiny(key, "  show(1, 1, 'still open')"));
    let step = app.key("a", &w).unwrap();
    assert!(!step.close);
}

#[test]
fn keys_and_edit_text() {
    for k in ["a", "Z", "7", " ", ".", "enter", "esc", "backspace", "tab", "delete", "up", "down", "left", "right", "home", "end", "ctrl+a", "ctrl+z"] {
        assert!(script::is_key(k), "{k}");
    }
    for k in ["", "ab", "Enter", "ctrl+1", "ctrl+", "f1", "é"] {
        assert!(!script::is_key(k), "{k}");
    }
    let w = desk(&[]);
    let say = |e: &str| script::run("main", &format!("say {e}"), &w).output.join("|");
    assert_eq!(say("edit_text('te', 'a')"), "tea");
    assert_eq!(say("edit_text('tea', ' ')"), "tea ");
    assert_eq!(say("edit_text('tea', 'backspace')"), "te");
    assert_eq!(say("edit_text('', 'backspace')"), "");
    assert_eq!(say("edit_text('é', 'backspace')"), "");
    assert_eq!(say("edit_text('tea', 'up')"), "tea");
    assert_eq!(say("edit_text('tea', 'ctrl+a')"), "tea");
}

#[test]
fn a_too_big_state_is_a_mistake() {
    let w = desk(&[]);
    let key = "  set s to []\n  repeat 40 times\n    add pad('', 8000) to s\n  done";
    let app = open(&w, &tiny(key, ""));
    let e = step_error(&app, &w, "a");
    assert!(e.contains("the app's state is too big to keep (at most 256 KB)"), "{e}");
    // a big state that fits is fine, and the next step starts from it
    let key = "  set s.n to s.n + 1\n  set s.big to []\n  repeat 20 times\n    add pad('', 8000) to s.big\n  done";
    let mut app = open(&w, &tiny(key, "  show(1, 1, count(s))"));
    press(&mut app, &w, "a");
    press(&mut app, &w, "a");
    assert_eq!(app.screen().line(1), "2");
}

#[test]
fn app_tests_see_their_steps_and_their_mistakes() {
    let w = desk(&[("apps:mailer", "app 'Mailer'\ntask start()\n  give 0\ndone\ntask key(s, k)\n  when k is 'm'\n    send('omar', 'Hi', 'Hello')\n  done\n  when k is 'x'\n    give 1 / 0\n  done\n  when k is 'q'\n    close()\n  done\n  give s + 1\ndone\ntask draw(s)\n  show(1, 1, 'keys: ' & s)\ndone")]);
    passes(
        &w,
        "\
test 'a mail sent by a step'
  pretend 'contacts:omar' holds 'name: Omar' & newline() & 'mail: omar'
  launch 'desk:apps:mailer'
  press 'm'
  check count(sent()) is 1
  check item(sent(), 1).to is 'omar'
  check page_now('mail:sent:s1') has 'subject: Hi'
  check screen_line(1) is 'keys: 1'
done
test 'a mistake closes the app'
  launch 'desk:apps:mailer'
  press 'a'
  press 'x'
  check not app_open()
  check app_mistake() has 'dividing by 0'
  check app_state() is 1
  check screen_line(1) is 'keys: 1'
done
test 'a mail to someone unknown is a mistake of the step'
  launch 'desk:apps:mailer'
  press 'm'
  check app_mistake() has 'no contact'
  check sent() is []
done
test 'close and ctrl+q'
  launch 'desk:apps:mailer'
  check app_open()
  press 'q'
  check not app_open()
  check app_mistake() is ''
  launch 'desk:apps:mailer'
  press 'ctrl+q'
  check not app_open()
done
test 'the test sees its own planned changes apart'
  launch 'desk:apps:mailer'
  write('desk:note', 'x')
  check after('desk:note') is 'x'
  check page_now('desk:note') is nothing()
done",
    );
    let l = tests(&w, "test 'press after close'\n  launch 'desk:apps:mailer'\n  press 'q'\n  press 'a'\n  check yes\ndone");
    assert_eq!(l[0], "FAIL press after close: line 4: press 'a', but the app has closed");
    let l = tests(&w, "test 'caught'\n  launch 'desk:apps:mailer'\n  try\n    press 'a'\n  instead p\n  done\n  check yes\ndone");
    assert!(l[0].contains("line 4: press can't be in the try part of a try"), "{l:?}");
    // in the instead part it is fine (that part is not undone)
    let l = tests(&w, "test 'instead'\n  launch 'desk:apps:mailer'\n  try\n    fail 'x'\n  instead p\n    press 'a'\n  done\n  check screen_line(1) is 'keys: 1'\ndone");
    assert_eq!(l[0], "PASS instead");
    let l = tests(&w, "test 'no app'\n  press 'a'\n  check yes\ndone");
    assert!(l[0].contains("no app was launched"), "{l:?}");
    let l = tests(&w, "test 'bad key'\n  launch 'desk:apps:mailer'\n  press 'shift'\n  check yes\ndone");
    assert!(l[0].contains("'shift' is not a key"), "{l:?}");
    let l = tests(&w, "test 'no state'\n  check app_state() is 1\ndone");
    assert!(l[0].contains("app_state() needs an app: launch 'PAGE' first"), "{l:?}");
    let l = tests(&w, "test 'missing'\n  launch 'desk:apps:nope'\n  check yes\ndone");
    assert!(l[0].contains("there is no page called desk:apps:nope"), "{l:?}");
    let l = tests(&w, "test 'not an app'\n  launch 'desk:apps:shop-lib'\n  check not app_open()\n  check app_mistake() has 'is not an app'\ndone");
    assert_eq!(l[0], "PASS not an app");
    let l = tests(&w, "test 'after a mistake'\n  launch 'desk:apps:mailer'\n  press 'x'\n  press 'a'\n  check yes\ndone");
    assert!(l[0].contains("but the app has closed after a mistake: desk:apps:mailer line 10: dividing by 0"), "{l:?}");
}

#[test]
fn app_steps_in_a_test_share_its_budget() {
    let slow = "app 'Slow'\ntask start()\n  give 0\ndone\ntask key(s, k)\n  repeat 400 times\n    repeat 400 times\n      set x to 1\n    done\n  done\n  give s\ndone\ntask draw(s)\ndone";
    let w = desk(&[("apps:slow", slow)]);
    let presses = "  press 'a'\n".repeat(12);
    let l = tests(&w, &format!("test 'slow'\n  launch 'desk:apps:slow'\n{presses}  check yes\ndone"));
    assert!(l[0].contains("too many steps in all"), "{l:?}");
}

/// The deepest app steps the limits allow (in key and draw, and pressed from a test) stay
/// within a 128 KB stack (the kernel's is 256 KB; tools/test_apps_os.py measures it there).
#[test]
#[cfg_attr(debug_assertions, ignore = "debug frames are much bigger; the kernel is a release build")]
fn deep_app_steps_fit_the_stack() {
    const DEEPEST: usize = 7;
    let h = std::thread::Builder::new()
        .stack_size(128 * 1024)
        .spawn(|| {
            let mut down = String::from("task down(n)\n");
            down.push_str(&"when yes\n".repeat(14));
            down.push_str("when n is 0\ngive 0\ndone\ngive ");
            down.push_str(&"(1 + ".repeat(10));
            down.push_str("down(n - 1)");
            down.push_str(&")".repeat(10));
            down.push('\n');
            down.push_str(&"done\n".repeat(15));
            let deep = |n: usize| format!("app 'Deep'\nuse 'deeplib:down'\ntask start()\n  give 0\ndone\ntask key(s, k)\n  give down({n})\ndone\ntask draw(s)\n  show(1, 1, 'deep: ' & down({n}))\ndone");
            let w = desk(&[("deeplib:down", down.trim_end())]);
            // the deepest n that works in a key step and its draw (one more is over the limits)
            let works = |n: usize| {
                let Ok(mut app) = App::load("apps:deep", &deep(n), &w) else { return false };
                let Ok(step) = app.start(&w) else { return false };
                app.keep(step);
                app.key("a", &w).is_ok()
            };
            let n = (1..40).rev().find(|&n| works(n)).expect("some depth works");
            assert_eq!(n, DEEPEST, "tools/test_apps_os.py uses this depth");
            let e = App::load("apps:deep", &deep(n + 1), &w).unwrap().start(&w).err().unwrap();
            assert!(e.limit, "{e}");
            let w = desk(&[("deeplib:down", down.trim_end()), ("apps:deep", &deep(n))]);
            let src = format!("use 'deeplib:down'\ntest 'deep'\n  when yes\n    when yes\n      launch 'desk:apps:deep'\n      press 'a'\n    done\n  done\n  check down({n}) is {}\n  check app_open()\ndone", n * 10);
            assert_eq!(tests(&w, &src), ["PASS deep", "1 of 1 tests passed"]);
        })
        .unwrap();
    h.join().expect("ran out of stack");
}

// ---- Sol's review of M5 (7 Nov 2026) ----

/// app_state() is a copy counted by the test: keeping old states can't use memory uncounted.
#[test]
fn kept_app_states_are_counted_by_the_test() {
    let big = "app 'Big'\ntask start()\n  set l to []\n  repeat 20 times\n    add pad('', 7990) & count(l) to l\n  done\n  give l\ndone\ntask key(s, k)\n  give s\ndone\ntask draw(s)\ndone";
    let w = desk(&[("apps:big", big)]);
    let held = "  set held to []\n  repeat 128 times\n    add app_state() to held\n    press 'a'\n  done\n  check yes";
    let l = tests(&w, &format!("test 'held'\n  launch 'desk:apps:big'\n{held}\ndone"));
    assert!(l[0].contains("too much memory"), "{l:?}");
}

/// Failed steps count too, and so do the test's own lines: one budget for the whole test.
#[test]
fn every_step_of_a_test_counts_even_when_it_fails() {
    let failing = "app 'F'\ntask start()\n  repeat 400 times\n    repeat 400 times\n      set x to 1\n    done\n  done\n  give 1 / 0\ndone\ntask key(s, k)\n  give s\ndone\ntask draw(s)\ndone";
    let w = desk(&[("apps:failing", failing)]);
    let launches = "  launch 'desk:apps:failing'\n".repeat(20);
    let l = tests(&w, &format!("test 'many'\n{launches}  check yes\ndone"));
    assert!(l[0].contains("this test took too many steps in all"), "{l:?}");
    // the test's own work and the app's steps together
    let slow = "app 'Slow'\ntask start()\n  give 0\ndone\ntask key(s, k)\n  repeat 400 times\n    repeat 400 times\n      set x to 1\n    done\n  done\n  give s + 1\ndone\ntask draw(s)\ndone";
    let w = desk(&[("apps:slow", slow)]);
    let own = "  repeat 400 times\n    repeat 400 times\n      set x to 1\n    done\n  done\n";
    let presses = "  press 'a'\n".repeat(10);
    let l = tests(&w, &format!("test 'both'\n{own}  launch 'desk:apps:slow'\n{presses}  check yes\ndone"));
    assert!(l[0].contains("this test took too many steps in all"), "{l:?}");
    let l = tests(&w, &format!("test 'own lines last'\n  launch 'desk:apps:slow'\n{presses}{own}  check yes\ndone"));
    assert!(l[0].contains("this test took too many steps in all"), "{l:?}");
    let presses = "  press 'a'\n".repeat(10);
    let l = tests(&w, &format!("test 'fits'\n  launch 'desk:apps:slow'\n{presses}  check app_state() is 10\ndone"));
    assert_eq!(l[0], "PASS fits");
}

/// The test world holds at most as many pages as the desk.
#[test]
fn the_test_world_holds_as_many_pages_as_the_desk() {
    let maker = "app 'M'\ntask start()\n  give 0\ndone\ntask key(s, k)\n  repeat 300 times\n    set s to s + 1\n    write('desk:p' & s, '')\n  done\n  give s\ndone\ntask draw(s)\ndone";
    let w = desk(&[("apps:maker", maker)]);
    let presses = "  press 'a'\n".repeat(17);
    let l = tests(&w, &format!("test 'many pages'\n  launch 'desk:apps:maker'\n{presses}  check not app_open()\n  check app_mistake() has 'too many pages'\n  check page_now('desk:p4800') is ''\n  check page_now('desk:p5100') is nothing()\ndone"));
    assert_eq!(l[0], "PASS many pages");
}

/// Mail and Calendar pick the first number no page or drawer has.
#[test]
fn made_mails_and_events_skip_drawers() {
    let app = "app 'S'\ntask start()\n  send('omar', 'Hi', 'x')\n  event('2026-12-01', '10:00', 'Tea', '')\n  give 0\ndone\ntask key(s, k)\n  give s\ndone\ntask draw(s)\ndone";
    let w = desk(&[("apps:sender", app)]);
    passes(&w, "test 'skip'\n  pretend 'contacts:omar' holds 'mail: omar'\n  pretend 'mail:sent:s1:child' holds 'x'\n  pretend 'calendar:e1:child' holds 'x'\n  launch 'desk:apps:sender'\n  check app_open()\n  check page_now('mail:sent:s2') has 'subject: Hi'\n  check page_now('calendar:e2') has 'title: Tea'\ndone");
}
