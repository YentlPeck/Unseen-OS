use crate::chore::{self, Action, World, MAX_STEPS};
use crate::fs::{self, Entry, Fs};

/// A test world on top of the real file system code. The checks mirror what
/// the kernel will do (step 2 replaces them with the real Mail/Calendar ones).
pub struct TestWorld {
    pub fs: Fs,
}

const START: &str = "\
#page contacts:omar
name: Omar Haddad
mail: omar
#end
#page contacts:lina
name: Lina Berg
mail: lina
note: piano teacher
#end
#page mail:inbox:m1
from: omar
to: me
subject: Bread

Fresh bread today.
#end
#page mail:inbox:m2
from: lina
to: me
subject: Lesson

Can we move it?
#end
#page mail:inbox:m10
from: omar
to: me
subject: Closed

Closed on Monday.
#end
#page notes:shopping
eggs
bread
tea
#end
#drawer empty
";

impl TestWorld {
    pub fn new() -> Self {
        Self::from(START)
    }

    pub fn from(start: &str) -> Self {
        let mut fs = Fs::from_start_files(start);
        fs.ensure_system();
        Self { fs }
    }
}

impl World for TestWorld {
    fn page(&self, path: &str) -> Option<String> {
        self.fs.read_page(path).ok().map(|s| s.to_string())
    }
    fn is_drawer(&self, path: &str) -> bool {
        self.fs.is_drawer(path)
    }
    fn pages_in(&self, drawer: &str) -> Vec<String> {
        self.fs
            .all()
            .filter(|(p, e)| fs::parent(p) == drawer && !p.is_empty() && matches!(e, Entry::Page(_)))
            .map(|(p, _)| p.clone())
            .collect()
    }
    fn check_write(&self, path: &str) -> Result<(), String> {
        if let Some((app, _)) = fs::owner(path) {
            return Err(format!("{} is looked after by the {app} app", fs::display(path)));
        }
        if self.fs.is_drawer(path) {
            return Err(format!("{} is a drawer, not a page", fs::display(path)));
        }
        if !self.fs.is_drawer(fs::parent(path)) {
            return Err(format!("there is no drawer called {}", fs::display(fs::parent(path))));
        }
        Ok(())
    }
    fn check_toss(&self, path: &str) -> Result<(), String> {
        if fs::owner(path).is_some() {
            return Err("app page".into());
        }
        if self.fs.read_page(path).is_err() {
            return Err(format!("there is no page called {}", fs::display(path)));
        }
        Ok(())
    }
    fn check_send(&self, to: &str, subject: &str, _text: &str) -> Result<(), String> {
        if !fs::valid_name(to) || !self.fs.exists(&fs::join("contacts", to)) {
            return Err(format!("no contact has the address '{to}'"));
        }
        if subject.contains('\n') {
            return Err("a subject must be one line".into());
        }
        Ok(())
    }
    fn check_event(&self, date: &str, time: &str, title: &str, _with: &str) -> Result<(), String> {
        if date.len() != 10 || time.len() != 5 || title.is_empty() {
            return Err("bad event".into());
        }
        Ok(())
    }
    fn today(&self) -> String {
        "2026-11-12".into()
    }
}

fn run(src: &str) -> chore::Outcome {
    chore::run(src, &TestWorld::new())
}

/// Output lines of a chore that must succeed.
fn out(src: &str) -> Vec<String> {
    let o = run(src);
    assert!(o.error.is_none(), "unexpected error {:?} for:\n{src}", o.error);
    o.output
}

fn say(expr: &str) -> String {
    out(&format!("say {expr}")).remove(0)
}

/// The error of a chore that must fail; also checks nothing would change.
fn err(src: &str) -> String {
    let o = run(src);
    assert!(o.actions.is_empty());
    o.error.unwrap_or_else(|| panic!("expected an error for:\n{src}"))
}

fn has(e: &str, part: &str) {
    assert!(e.contains(part), "error {e:?} should contain {part:?}");
}

#[test]
fn design_example() {
    let o = run("-- count Omar's mails and save the answer
set total to 0
for each m in pages('mail:inbox')
  when field(m, 'from') is 'omar'
    set total to total + 1
  done
done
say 'Omar sent ' & total & ' mails'
write('desk:answer', total)");
    assert_eq!(o.error, None);
    assert_eq!(o.output, vec!["Omar sent 2 mails"]);
    assert_eq!(o.actions, vec![Action::Write { page: "answer".into(), text: "2".into() }]);
}

#[test]
fn precedence() {
    assert_eq!(say("1 + 2 * 3"), "7");
    assert_eq!(say("(1 + 2) * 3"), "9");
    assert_eq!(say("10 - 4 - 3"), "3");
    assert_eq!(say("12 / 3 / 2"), "2");
    assert_eq!(say("-2 * 3"), "-6");
    assert_eq!(say("'a' & 1 + 2"), "a3");
    assert_eq!(say("not 1 is 2"), "yes");
    assert_eq!(say("1 is 1 and 2 is 3 or yes"), "yes");
    assert_eq!(say("no or yes and no"), "no");
    assert_eq!(say("not yes or yes"), "yes");
}

#[test]
fn numbers() {
    assert_eq!(say("10 / 4"), "2.5");
    assert_eq!(say("1 / 3"), "0.333333");
    assert_eq!(say("0.1 + 0.2"), "0.3");
    assert_eq!(say("0 - 0"), "0");
    assert_eq!(say("-0.0000001"), "0");
    assert_eq!(say("163.50 + 0"), "163.5");
    has(&err("say 1 / 0"), "dividing by 0");
    has(&err("say 999999999999 + 1"), "too big");
    has(&err("say 9999999999999"), "too big");
    assert_eq!(say("number(' 42 ') + number('-1.5')"), "40.5");
    has(&err("say number('1e5')"), "not a number");
    has(&err("say number('12,5')"), "not a number");
    has(&err("say number('')"), "not a number");
    has(&err("say number('1.')"), "not a number");
}

#[test]
fn strict_types() {
    let e = err("say 1 + 'a'");
    has(&e, "line 1:");
    has(&e, "number(...)");
    has(&err("say 1 is '1'"), "same kind");
    has(&err("when 1\nsay 'x'\ndone"), "when needs yes or no");
    has(&err("say yes and 1"), "and needs yes or no");
    has(&err("say not 'a'"), "not needs yes or no");
    has(&err("say 'a' < 1"), "two numbers or two texts");
    has(&err("say pages('mail:inbox') is 'x'"), "same kind");
    assert_eq!(say("words('a b') is words('a b')"), "yes");
    assert_eq!(say("words('a b') is words('a  c')"), "no");
    assert_eq!(say("lines('') is words('')"), "yes");
    assert_eq!(say("'2026-11-02' < '2026-11-10'"), "yes");
    assert_eq!(say("'B' is 'b'"), "no");
    assert_eq!(say("lower('B') is 'b'"), "yes");
    assert_eq!(say("'abc' & yes & 2"), "abcyes2");
}

#[test]
fn short_circuit() {
    // the right side would be an error, but it is never looked at
    assert_eq!(say("no and missing"), "no");
    assert_eq!(say("yes or missing"), "yes");
    has(&err("say yes and missing"), "missing has no value yet");
}

#[test]
fn has_operator() {
    assert_eq!(say("'neighbour, has a cat' has 'cat'"), "yes");
    assert_eq!(say("'Cat' has 'cat'"), "no");
    assert_eq!(say("words('a bb c') has 'bb'"), "yes");
    assert_eq!(say("words('a bb c') has 'b'"), "no");
    has(&err("say 1 has 'a'"), "has needs");
}

#[test]
fn blocks_and_loops() {
    assert_eq!(
        out("set n to 0\nrepeat 3 times\nset n to n + 2\ndone\nsay n\nrepeat 0 times\nsay 'never'\ndone"),
        vec!["6"]
    );
    assert_eq!(out("when 1 > 2\nsay 'a'\notherwise\nsay 'b'\ndone"), vec!["b"]);
    assert_eq!(
        out("for each w in words('x y')\n  when w is 'y'\n    say 'found ' & w\n  otherwise\n    say 'not ' & w\n  done\ndone\nsay w"),
        vec!["not x", "found y", "y"]
    );
    assert_eq!(out("SET Total TO 1\nSay TOTAL"), vec!["1"]);
    assert_eq!(out("say \"double\" & 'single'"), vec!["doublesingle"]);
    has(&err("say 'it''s'"), "didn't expect the text 's'");
    assert_eq!(out("say 5 -- a comment\n-- another\n\n   say 6"), vec!["5", "6"]);
    has(&err("for each x in 'abc'\nsay x\ndone"), "needs a list");
    has(&err("repeat 2.5 times\ndone"), "whole number");
    has(&err("repeat -1 times\ndone"), "whole number");
    has(&err("repeat 10001 times\ndone"), "whole number");
}

#[test]
fn stop_finishes_successfully() {
    let o = run("write('desk:a', 1)\nrepeat 5 times\nsay 'x'\nstop\ndone\nwrite('desk:b', 2)");
    assert_eq!(o.error, None);
    assert_eq!(o.output, vec!["x"]);
    assert_eq!(o.actions.len(), 1);
}

#[test]
fn all_or_nothing() {
    let o = run("write('desk:a', 1)\nsend('omar', 'Hi', 'hello')\nsay 'before'\nsay 1 + 'x'\nwrite('desk:b', 2)");
    has(o.error.as_deref().unwrap(), "line 4:");
    assert!(o.actions.is_empty(), "a failed chore must change nothing");
    assert_eq!(o.output, vec!["before"], "output up to the error is kept");
}

#[test]
fn parse_errors_come_first() {
    // a syntax error on the last line means not even line 1 runs
    let o = run("say 'hi'\nwrite('desk:a', 1)\nset to");
    assert!(o.output.is_empty() && o.actions.is_empty());
    has(o.error.as_deref().unwrap(), "line 3:");

    has(&err("when yes\nsay 1"), "this when has no done");
    has(&err("for each x in words('a')\nsay x"), "this for each has no done");
    has(&err("say 1\ndone"), "no block to end");
    has(&err("otherwise"), "no when before it");
    has(&err("repeat 2 times\notherwise\ndone"), "otherwise only belongs to when");
    has(&err("when yes\notherwise\notherwise\ndone"), "already has an otherwise");
    has(&err("set x = 1"), "'=' is not used here");
    has(&err("when x == 1\ndone"), "'==' is not used");
    has(&err("say foo(1)"), "no built-in called foo");
    has(&err("say field('a')"), "needs 2 values");
    has(&err("say count(1, 2)"), "needs 1 value in");
    has(&err("set done to 1"), "special word");
    has(&err("say write('desk:a', 1)"), "is an action");
    has(&err("field('notes:shopping', 'x')"), "gives a value");
    has(&err("say 'open"), "not closed");
    has(&err("say (1 + 2"), "not closed");
    has(&err("say 1 2"), "didn't expect the number 2");
    has(&err("say 1 < 2 < 3"), "one comparison at a time");
    has(&err("total to 5"), "a line must start with");
    has(&err("say 1 +"), "missing at the end");
    has(&err("say 1 ; 2"), "don't understand ';'");
    has(&err("say 'x' é"), "can't read");
    has(&err("stop now"), "alone");
    has(&err("done done"), "alone");
}

#[test]
fn reading_builtins() {
    assert_eq!(say("pages('mail:inbox')"), "mail:inbox:m1, mail:inbox:m2, mail:inbox:m10");
    assert_eq!(say("pages('desk:mail:inbox')"), "mail:inbox:m1, mail:inbox:m2, mail:inbox:m10");
    assert_eq!(say("count(pages('empty'))"), "0");
    assert_eq!(say("pages('desk') has 'answer'"), "no");
    assert_eq!(say("exists('desk')"), "yes");
    assert_eq!(say("count(pages('desk'))"), "0");
    assert_eq!(say("count(pages('mail'))"), "0");
    has(&err("say pages('nowhere')"), "no drawer called desk:nowhere");
    has(&err("say pages('bad name!')"), "not a valid");
    assert_eq!(say("field('mail:inbox:m2', 'SUBJECT')"), "Lesson");
    assert_eq!(say("field('mail:inbox:m2', 'nothing') is ''"), "yes");
    assert_eq!(say("field('contacts:lina', 'note')"), "piano teacher");
    assert_eq!(say("body('mail:inbox:m2')"), "Can we move it?");
    assert_eq!(say("body('notes:shopping') is ''"), "yes");
    assert_eq!(say("count(lines(read('notes:shopping')))"), "3");
    assert_eq!(say("item(lines(read('notes:shopping')), 2)"), "bread");
    has(&err("say item(lines(read('notes:shopping')), 4)"), "has 3 items");
    has(&err("say item(lines(read('notes:shopping')), 0)"), "the first is 1");
    has(&err("say read('notes:nothing')"), "no page called desk:notes:nothing");
    has(&err("say read('notes')"), "is a drawer");
    assert_eq!(say("exists('notes')"), "yes");
    assert_eq!(say("exists('notes:shopping')"), "yes");
    assert_eq!(say("exists('notes:nope')"), "no");
    assert_eq!(say("count('hello')"), "5");
    assert_eq!(say("today()"), "2026-11-12");
}

#[test]
fn dates() {
    assert_eq!(say("weekday('2026-11-12')"), "thursday");
    assert_eq!(say("weekday('2026-12-07')"), "monday");
    assert_eq!(say("weekday('2024-02-29')"), "thursday");
    assert_eq!(say("add_days('2026-12-28', 10)"), "2027-01-07");
    assert_eq!(say("add_days('2024-02-28', 1)"), "2024-02-29");
    assert_eq!(say("add_days('2026-03-01', -1)"), "2026-02-28");
    has(&err("say weekday('2026-02-30')"), "not a date");
    has(&err("say weekday('14 Nov')"), "not a date");
    has(&err("say add_days('2026-01-01', 1.5)"), "whole number");
    has(&err("say add_days('9999-12-31', 1)"), "out of range");
}

#[test]
fn actions_are_checked_when_called() {
    has(&err("write('mail:inbox:m1', 'x')"), "looked after by the Mail app");
    has(&err("write('notes', 'x')"), "is a drawer");
    has(&err("write('nowhere:x', 'x')"), "no drawer called desk:nowhere");
    has(&err("toss('contacts:omar')"), "app page");
    has(&err("send('stranger', 'Hi', 'x')"), "no contact");
    has(&err("event('2026-1-1', '9:00', 'x', '')"), "bad event");
    let o = run("add('notes:log', 'one')\nadd('notes:log', 2)\ntoss('notes:shopping')\nevent('2026-12-07', '10:00', 'Call', 'lina')");
    assert_eq!(o.error, None);
    assert_eq!(
        o.actions,
        vec![
            Action::Add { page: "notes:log".into(), line: "one".into() },
            Action::Add { page: "notes:log".into(), line: "2".into() },
            Action::Toss { page: "notes:shopping".into() },
            Action::Event { date: "2026-12-07".into(), time: "10:00".into(), title: "Call".into(), with: "lina".into() },
        ]
    );
}

#[test]
fn reads_see_the_world_as_it_started() {
    // the write only happens at the end, so exists() still says no
    assert_eq!(out("write('desk:new', 'x')\nsay exists('new')"), vec!["no"]);
}

#[test]
fn limits() {
    has(&err("repeat 10000 times\nrepeat 10000 times\ndone\ndone"), "too many steps");
    has(&err("repeat 201 times\nsay 1\ndone"), "too many say lines");
    has(&err("repeat 201 times\nwrite('desk:a', 1)\ndone"), "too many changes");
    has(&err("set t to 'xxxxxxxxxxxxxxxx'\nrepeat 20 times\nset t to t & t\ndone"), "too long");
    let deep = format!("{}{}", "when yes\n".repeat(17), "done\n".repeat(17));
    has(&err(&deep), "nested too deep");
    let ok_deep = format!("{}say 'deep'\n{}", "when yes\n".repeat(16), "done\n".repeat(16));
    assert_eq!(out(&ok_deep), vec!["deep"]);
    has(&err(&format!("say {}1{}", "(".repeat(13), ")".repeat(13))), "too many brackets");
    assert_eq!(say(&format!("{}1{}", "(".repeat(12), ")".repeat(12))), "1");
    has(&err(&format!("say 1{}", " + 1".repeat(60))), "too complicated");
    assert_eq!(say(&format!("1{}", " + 1".repeat(40))), "41");
    has(&err(&"say 1\n".repeat(121)), "too many lines");
    has(&err(&format!("say '{}'", "x".repeat(4100))), "too long");
    let names: String = (0..101).map(|i| format!("set v{i} to 1\n")).collect();
    has(&err(&names), "too many different names");
    let o = run("repeat 10000 times\nrepeat 10000 times\ndone\ndone");
    assert!(o.steps <= MAX_STEPS + 1);
}

#[test]
fn a_bulk_chore() {
    let o = run("for each c in pages('contacts')
  when field(c, 'note') has 'piano'
    send(field(c, 'mail'), 'Party', 'Come to my party on ' & add_days(today(), 2) & '!')
  done
done");
    assert_eq!(o.error, None);
    assert_eq!(
        o.actions,
        vec![Action::Send { to: "lina".into(), subject: "Party".into(), text: "Come to my party on 2026-11-14!".into() }]
    );
}

/// Run the deepest programs the limits allow on a 128 KB thread. Measured
/// (examples/stack.rs, release build): the worst case needs between 64 and
/// 96 KB. The kernel stack is raised to 256 KB in v0.3 to keep a wide margin.
#[test]
#[cfg_attr(debug_assertions, ignore = "debug frames are much bigger; the kernel is a release build")]
fn worst_case_fits_in_128k_stack() {
    let handle = std::thread::Builder::new()
        .stack_size(128 * 1024)
        .spawn(|| {
            let inner = format!("say {}1{} & 1{}", "(".repeat(12), ")".repeat(12), " + 1".repeat(44));
            let src = format!("{}{inner}\n{}", "when yes\n".repeat(16), "done\n".repeat(16));
            let o = run(&src);
            assert_eq!(o.error, None, "{src}");
            let nots = format!("{}{}", "when yes\n".repeat(16), "done\n".repeat(16))
                + &format!("say {}yes", "not ".repeat(12));
            let o = run(&nots);
            assert_eq!(o.error, None);
        })
        .unwrap();
    handle.join().expect("ran out of stack");
}
