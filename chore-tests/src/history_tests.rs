//! Host tests for kernel/src/history.rs (stamps: the desk's version history).
use crate::fs::Fs;
use crate::history::{line_diff, parse_number, History, Op, MAX_NOTE, MAX_STAMPS};

const D: &str = "2026-11-06";

fn desk(text: &str) -> Fs {
    let mut fs = Fs::from_start_files(text);
    fs.ensure_system();
    fs
}

fn start() -> (Fs, History) {
    let fs = desk("#page tools:total\nset total to 0\nsay total\n#end\n#page notes:todo\nbuy tea\n#end\n");
    let mut h = History::new();
    h.stamp(&fs, D, "the desk as it started").unwrap();
    (fs, h)
}

fn text(fs: &Fs, p: &str) -> String {
    fs.read_page(p).unwrap().to_string()
}

#[test]
fn stamp_needs_a_change_and_a_note() {
    let (mut fs, mut h) = start();
    assert!(h.stamp(&fs, D, "again").unwrap_err().contains("nothing changed since stamp #1"));
    fs.write_page("notes:todo", "buy tea\nbuy milk").unwrap();
    assert!(h.stamp(&fs, D, "  ").unwrap_err().contains("needs a note"));
    assert!(h.stamp(&fs, D, &"x".repeat(MAX_NOTE + 1)).unwrap_err().contains("too long"));
    assert_eq!(h.stamp(&fs, D, "milk").unwrap(), 2);
    assert_eq!(h.base(), Some(2));
    assert_eq!(h.stamps()[1].from, Some(1));
}

#[test]
fn app_drawers_are_not_in_the_history() {
    let (mut fs, mut h) = start();
    fs.app_write("mail:sent:m1", "to: omar").unwrap();
    assert_eq!(h.unstamped_count(&fs), 0);
    assert!(h.stamp(&fs, D, "mail only").is_err());
    assert!(h.stamps()[0].desk.keys().all(|k| !k.starts_with("mail") && !k.starts_with("system")));
}

#[test]
fn changes_show_lines_out_and_in() {
    let (mut fs, h) = start();
    fs.write_page("tools:total", "set total to 1\nsay total").unwrap();
    fs.write_page("notes:new", "a\nb").unwrap();
    fs.toss("notes:todo").unwrap();
    let out = h.changes(&fs, "").unwrap();
    let all = out.join("\n");
    assert!(out[0].starts_with("3 items changed since stamp #1"), "{all}");
    assert!(all.contains("changed  desk:tools:total  (1 line out, 1 line in)"), "{all}");
    assert!(all.contains("    1 - set total to 0"), "{all}");
    assert!(all.contains("    1 + set total to 1"), "{all}");
    assert!(all.contains("    2   say total"), "{all}");
    assert!(all.contains("new page  desk:notes:new  (2 lines)"), "{all}");
    assert!(all.contains("gone page  desk:notes:todo  (1 line)"), "{all}");
    assert!(all.contains("    1 - buy tea"), "{all}");
    // only one drawer
    let one = h.changes(&fs, "tools").unwrap().join("\n");
    assert!(one.contains("1 item changed in desk:tools since stamp #1"), "{one}");
    assert!(!one.contains("notes"), "{one}");
    // a filter that is only a prefix of a name does not match
    let none = h.changes(&fs, "tool").unwrap();
    assert!(none[0].starts_with("Nothing changed in desk:tool since"), "{none:?}");
}

#[test]
fn drawers_count_too() {
    let (mut fs, h) = start();
    fs.build_drawer("archive").unwrap();
    let all = h.changes(&fs, "").unwrap().join("\n");
    assert!(all.contains("new drawer  desk:archive"), "{all}");
}

#[test]
fn rewind_puts_everything_back_and_keeps_later_stamps() {
    let (mut fs, mut h) = start();
    fs.write_page("tools:total", "broken").unwrap();
    fs.build_drawer("archive").unwrap();
    h.stamp(&fs, D, "broke it").unwrap();
    fs.app_write("mail:sent:m1", "to: omar").unwrap(); // app drawers are untouched by a rewind
    let msg = h.rewind(&mut fs, 1).unwrap();
    assert!(msg.contains("Rewound the desk to stamp #1"));
    assert_eq!(text(&fs, "tools:total"), "set total to 0\nsay total");
    assert!(!fs.exists("archive"));
    assert!(fs.exists("mail:sent:m1"));
    assert_eq!(h.base(), Some(1));
    assert_eq!(h.stamps().len(), 2);
    // and forward again
    h.rewind(&mut fs, 2).unwrap();
    assert_eq!(text(&fs, "tools:total"), "broken");
    // a new stamp after a rewind remembers where it was made
    h.rewind(&mut fs, 1).unwrap();
    fs.write_page("notes:todo", "other").unwrap();
    assert_eq!(h.stamp(&fs, D, "branch").unwrap(), 3);
    assert_eq!(h.stamps()[2].from, Some(1));
    assert!(h.list(&fs).iter().any(|l| l.contains("#3") && l.contains("(made on #1)")));
}

#[test]
fn rewind_refuses_unstamped_changes() {
    let (mut fs, mut h) = start();
    fs.write_page("notes:todo", "x").unwrap();
    h.stamp(&fs, D, "x").unwrap();
    fs.write_page("notes:todo", "y").unwrap();
    let e = h.rewind(&mut fs, 1).unwrap_err();
    assert!(e.contains("not stamped yet"), "{e}");
    assert_eq!(text(&fs, "notes:todo"), "y");
    assert!(h.rewind(&mut fs, 9).unwrap_err().contains("no stamp #9"));
    fs.write_page("notes:todo", "x").unwrap();
    assert!(h.rewind(&mut fs, 2).unwrap_err().contains("already on stamp #2"));
}

#[test]
fn bring_one_page_back() {
    let (mut fs, mut h) = start();
    fs.write_page("tools:total", "broken").unwrap();
    h.stamp(&fs, D, "broke it").unwrap();
    // unstamped edit of the page: refused, nothing lost
    fs.write_page("tools:total", "half fixed").unwrap();
    assert!(h.bring(&mut fs, "tools:total", 1).unwrap_err().contains("not stamped yet"));
    assert_eq!(text(&fs, "tools:total"), "half fixed");
    fs.write_page("tools:total", "broken").unwrap();
    assert!(h.bring(&mut fs, "tools:total", 1).unwrap().contains("Brought back desk:tools:total"));
    assert_eq!(text(&fs, "tools:total"), "set total to 0\nsay total");
    assert!(h.bring(&mut fs, "tools:total", 1).unwrap_err().contains("already as it was"));
    // a tossed page (and its drawer) can come back
    fs.toss("notes:todo").unwrap();
    fs.toss("notes").unwrap();
    h.bring(&mut fs, "notes:todo", 1).unwrap();
    assert_eq!(text(&fs, "notes:todo"), "buy tea");
    assert!(h.bring(&mut fs, "nothing", 1).unwrap_err().contains("no page desk:nothing in stamp #1"));
    assert!(h.bring(&mut fs, "notes", 1).unwrap_err().contains("was a drawer"));
    assert!(h.bring(&mut fs, "mail:inbox:m1", 1).unwrap_err().contains("Mail app"));
}

#[test]
fn compare_two_stamps_or_now() {
    let (mut fs, mut h) = start();
    fs.write_page("notes:todo", "buy tea\nbuy milk").unwrap();
    h.stamp(&fs, D, "milk").unwrap();
    let out = h.compare(&fs, 1, Some(2), "").unwrap().join("\n");
    assert!(out.contains("1 item changed from stamp #1 to stamp #2"), "{out}");
    assert!(out.contains("    2 + buy milk"), "{out}");
    let now = h.compare(&fs, 2, None, "").unwrap();
    assert_eq!(now[0], "No differences from stamp #2 to the desk now.");
    assert!(h.compare(&fs, 3, None, "").is_err());
}

#[test]
fn unchanged_text_is_shared_between_stamps() {
    let big = "x".repeat(500_000);
    let mut fs = desk(&format!("#page big\n{big}\n#end\n"));
    let mut h = History::new();
    h.stamp(&fs, D, "one").unwrap();
    // 5 more stamps that each change a tiny page: the big page is not copied again
    for i in 0..5 {
        fs.write_page("small", &format!("{i}")).unwrap();
        h.stamp(&fs, D, "small").unwrap();
    }
    let p0 = h.stamps()[0].desk.get("big").unwrap().as_ref().unwrap().as_ptr();
    let p5 = h.stamps()[5].desk.get("big").unwrap().as_ref().unwrap().as_ptr();
    assert!(h.used() < 520_000, "{}", h.used());
    assert_eq!(p0, p5);
    // but 5 different big versions fill the 2 MB history
    let mut full = false;
    for i in 0..6 {
        fs.write_page("big", &format!("{i}{big}")).unwrap();
        if let Err(e) = h.stamp(&fs, D, "big") {
            assert!(e.contains("history is full"), "{e}");
            full = true;
            break;
        }
    }
    assert!(full);
}

#[test]
fn at_most_max_stamps() {
    let (mut fs, mut h) = start();
    for i in 1..MAX_STAMPS {
        fs.write_page("n", &format!("{i}")).unwrap();
        h.stamp(&fs, D, "n").unwrap();
    }
    fs.write_page("n", "last").unwrap();
    assert!(h.stamp(&fs, D, "n").unwrap_err().contains("at most 99 stamps"));
}

#[test]
fn start_files_can_hold_a_history() {
    let text = "#page tools:total\nv1\n#end\n#stamp 2026-11-01 first version\n\
                #page tools:total\nv2\n#end\n#page tools:extra\ne\n#end\n#stamp second\n\
                #toss tools:extra\n#page tools:total\nv3\n#end\n";
    let mut h = History::new();
    let mut fs = Fs::load_start_files(text, |fs, note| {
        h.start_stamp(fs, note, D).unwrap();
    });
    fs.ensure_system();
    assert_eq!(h.stamps().len(), 2);
    assert_eq!(h.stamps()[0].date, "2026-11-01");
    assert_eq!(h.stamps()[0].note, "first version");
    assert_eq!(h.stamps()[1].date, D);
    assert_eq!(h.stamps()[1].note, "second");
    assert_eq!(h.base(), Some(2));
    // what comes after the last #stamp is an unstamped change
    assert_eq!(text_of(&fs, "tools:total"), "v3");
    assert!(!fs.exists("tools:extra"));
    let ch = h.changes(&fs, "").unwrap().join("\n");
    assert!(ch.contains("gone page  desk:tools:extra"), "{ch}");
    assert!(ch.contains("    1 + v3"), "{ch}");
}

fn text_of(fs: &Fs, p: &str) -> String {
    text(fs, p)
}

#[test]
fn stamp_numbers() {
    assert_eq!(parse_number("#3"), Some(3));
    assert_eq!(parse_number("12"), Some(12));
    assert_eq!(parse_number("#"), None);
    assert_eq!(parse_number("x3"), None);
    assert_eq!(parse_number("99999"), Some(99999));
    assert_eq!(parse_number("123456789012345678901234567890"), Some(usize::MAX));
}

#[test]
fn line_diff_is_minimal() {
    let a = ["a", "b", "c", "d"];
    let b = ["a", "x", "c", "d", "e"];
    let ops = line_diff(&a, &b);
    let outs = ops.iter().filter(|o| matches!(o, Op::Out(_))).count();
    let ins = ops.iter().filter(|o| matches!(o, Op::In(_))).count();
    assert_eq!((outs, ins), (1, 2));
    // rebuild both sides from the ops
    let old: Vec<&str> = ops.iter().filter_map(|o| match o { Op::Same(i, _) | Op::Out(i) => Some(a[*i]), _ => None }).collect();
    let new: Vec<&str> = ops.iter().filter_map(|o| match o { Op::Same(_, j) => Some(b[*j]), Op::In(j) => Some(b[*j]), _ => None }).collect();
    assert_eq!(old, a);
    assert_eq!(new, b);
}

#[test]
fn line_diff_random_rebuilds_both_sides() {
    let mut seed = 7u64;
    let mut rnd = |n: u64| {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed % n
    };
    let words = ["a", "b", "c", "d", ""];
    for _ in 0..2000 {
        let la = rnd(12) as usize;
        let lb = rnd(12) as usize;
        let a: Vec<&str> = (0..la).map(|_| words[rnd(5) as usize]).collect();
        let b: Vec<&str> = (0..lb).map(|_| words[rnd(5) as usize]).collect();
        let ops = line_diff(&a, &b);
        let old: Vec<&str> = ops.iter().filter_map(|o| match o { Op::Same(i, _) | Op::Out(i) => Some(a[*i]), _ => None }).collect();
        let new: Vec<&str> = ops.iter().filter_map(|o| match o { Op::Same(_, j) | Op::In(j) => Some(b[*j]), _ => None }).collect();
        assert_eq!(old, a);
        assert_eq!(new, b);
        for o in &ops {
            if let Op::Same(i, j) = o {
                assert_eq!(a[*i], b[*j]);
            }
        }
    }
}

#[test]
fn a_huge_change_still_works() {
    let a: Vec<String> = (0..2000).map(|i| format!("a{i}")).collect();
    let b: Vec<String> = (0..2000).map(|i| format!("b{i}")).collect();
    let a: Vec<&str> = a.iter().map(|s| s.as_str()).collect();
    let b: Vec<&str> = b.iter().map(|s| s.as_str()).collect();
    let ops = line_diff(&a, &b);
    assert_eq!(ops.len(), 4000);
}

// ---- memory and review cases ---------------------------------------------------

use crate::fs;
use crate::history::{MAX_HISTORY, MAX_LINES};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

/// Counts the bytes each test thread has allocated (now and at its peak).
struct Counting;

thread_local! {
    static NOW: Cell<isize> = const { Cell::new(0) };
    static PEAK: Cell<isize> = const { Cell::new(0) };
}

fn count(d: isize) {
    let _ = NOW.try_with(|n| {
        let v = n.get() + d;
        n.set(v);
        let _ = PEAK.try_with(|p| p.set(p.get().max(v)));
    });
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let p = System.alloc(l);
        if !p.is_null() {
            count(l.size() as isize);
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        System.dealloc(p, l);
        count(-(l.size() as isize));
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, new: usize) -> *mut u8 {
        let q = System.realloc(p, l, new);
        if !q.is_null() {
            count(new as isize - l.size() as isize);
        }
        q
    }
}

#[global_allocator]
static COUNTING: Counting = Counting;

/// Run f; return its result, the peak extra bytes while it ran, and the extra bytes still held.
fn measure<T>(f: impl FnOnce() -> T) -> (T, isize, isize) {
    let start = NOW.with(|n| n.get());
    PEAK.with(|p| p.set(start));
    let r = f();
    (r, PEAK.with(|p| p.get()) - start, NOW.with(|n| n.get()) - start)
}

#[test]
fn huge_page_listing_stays_small() {
    let (mut fs, h) = start();
    let big = "\n".repeat(200_000);
    fs.write_page("big", &big).unwrap();
    let (out, peak, _) = measure(|| h.changes(&fs, "").unwrap());
    assert!(peak < 1_500_000, "peak {peak}");
    assert!(out.len() <= MAX_LINES + 4);
    assert!(out.iter().any(|l| l.contains("the rest is not shown")));
    fs.write_page("tools:total", &big).unwrap();
    let (out, peak, _) = measure(|| h.changes(&fs, "tools").unwrap());
    assert!(peak < 1_500_000, "peak {peak}");
    let all = out.join("\n");
    assert!(all.contains("too long to compare line by line: 2 lines before, 200001 lines now"), "{all}");
}

#[test]
fn worst_line_diff_stays_small() {
    // two 3000-line versions with long lines, all different
    let a: Vec<String> = (0..3000).map(|i| format!("a{i} {}", "x".repeat(300))).collect();
    let b: Vec<String> = (0..3000).map(|i| format!("b{i} {}", "y".repeat(300))).collect();
    let mut fs = desk(&format!("#page p\n{}\n#end\n", a.join("\n")));
    let mut h = History::new();
    h.stamp(&fs, D, "a").unwrap();
    fs.write_page("p", &b.join("\n")).unwrap();
    let (out, peak, _) = measure(|| h.changes(&fs, "").unwrap());
    assert!(peak < 2_000_000, "peak {peak}");
    assert!(out.iter().map(|l| l.len()).sum::<usize>() < 450_000);
}

#[test]
fn many_pages_fill_the_history_not_the_heap() {
    let mut text = String::new();
    for i in 0..2000 {
        text.push_str(&format!("#page drawer{}:page{i}\n#end\n", i % 20));
    }
    let mut fs = desk(&text);
    let mut h = History::new();
    let (stamps, _, live) = measure(|| {
        let mut i = 0;
        loop {
            fs.write_page("counter", &format!("{i}")).unwrap();
            match h.stamp(&fs, D, "n") {
                Ok(_) => i += 1,
                Err(e) => {
                    assert!(e.contains("history is full"), "{e}");
                    return i;
                }
            }
        }
    });
    assert!(stamps > 5 && stamps < 99, "{stamps} stamps");
    assert!(h.used() <= MAX_HISTORY);
    assert!(live < 2_600_000, "the history really holds {live} bytes ({stamps} stamps)");
}

#[test]
fn renamed_pages_share_their_text() {
    let big = "y".repeat(800_000);
    let mut fs = desk(&format!("#page a\n{big}\n#end\n"));
    let mut h = History::new();
    h.stamp(&fs, D, "a").unwrap();
    fs.rename("a", "b").unwrap();
    h.stamp(&fs, D, "b").unwrap();
    fs.rename("b", "c").unwrap();
    h.stamp(&fs, D, "c").unwrap();
    assert!(h.used() < 900_000, "{}", h.used());
}

#[test]
fn bring_never_puts_a_page_under_a_page() {
    let mut fs = desk("#page a:b:p\nhello\n#end\n");
    let mut h = History::new();
    h.stamp(&fs, D, "tree").unwrap();
    fs.toss("a:b:p").unwrap();
    fs.toss("a:b").unwrap();
    fs.toss("a").unwrap();
    fs.write_page("a", "now a page").unwrap();
    h.stamp(&fs, D, "page").unwrap();
    let e = h.bring(&mut fs, "a:b:p", 1).unwrap_err();
    assert!(e.contains("desk:a is a page now"), "{e}");
    assert!(!fs.exists("a:b"));
}

#[test]
fn a_failed_bring_changes_nothing() {
    let mut fs = desk("#page old:sub:p\nhello\n#end\n");
    let mut h = History::new();
    h.stamp(&fs, D, "one").unwrap();
    fs.toss("old:sub:p").unwrap();
    fs.toss("old:sub").unwrap();
    fs.toss("old").unwrap();
    let room = fs::MAX_DESK - fs.total_bytes();
    fs.write_page("filler", &"z".repeat(room)).unwrap();
    h.stamp(&fs, D, "full").unwrap();
    let e = h.bring(&mut fs, "old:sub:p", 1).unwrap_err();
    assert!(e.contains("desk is full"), "{e}");
    assert!(!fs.exists("old"));
}

#[test]
fn rewind_keeps_the_desk_size_limit() {
    let mut fs = desk(&format!("#page big\n{}\n#end\n", "b".repeat(900_000)));
    let mut h = History::new();
    h.stamp(&fs, D, "big").unwrap();
    fs.toss("big").unwrap();
    h.stamp(&fs, D, "no big").unwrap();
    fs.app_write("mail:inbox:huge", &"m".repeat(900_000)).unwrap();
    let e = h.rewind(&mut fs, 1).unwrap_err();
    assert!(e.contains("no longer fits"), "{e}");
    assert!(!fs.exists("big"));
    assert_eq!(h.base(), Some(2));
}

#[test]
fn long_listings_say_how_to_see_less() {
    let a: Vec<String> = (0..2500).map(|i| format!("old line {i} {}", "x".repeat(100))).collect();
    let b: Vec<String> = (0..2500).map(|i| format!("new line {i} {}", "x".repeat(100))).collect();
    let mut fs = desk(&format!("#page x:p\n{}\n#end\n#page y\nsmall\n#end\n", a.join("\n")));
    let mut h = History::new();
    h.stamp(&fs, D, "a").unwrap();
    fs.write_page("x:p", &b.join("\n")).unwrap();
    fs.write_page("y", "changed").unwrap();
    h.stamp(&fs, D, "b").unwrap();
    let all = h.compare(&fs, 1, Some(2), "").unwrap().join("\n");
    assert!(all.contains("the rest is not shown"), "not cut off");
    assert!(all.contains("name one page or drawer: compare 1 2 <page or drawer>"));
    let one = h.compare(&fs, 1, Some(2), "y").unwrap().join("\n");
    assert!(one.contains("1 item changed in desk:y from stamp #1 to stamp #2"), "{one}");
    assert!(one.contains("    1 + changed"), "{one}");
}

#[test]
fn bad_start_stamps_are_reported() {
    let text = format!("#page p\nv1\n#end\n#stamp {}\n#page p\nv2\n#end\n#stamp ok\n", "n".repeat(61));
    let mut h = History::new();
    let _fs = Fs::load_start_files(&text, |fs, note| {
        let _ = h.start_stamp(fs, note, D);
    });
    assert_eq!(h.start_errors.len(), 1);
    assert!(h.start_errors[0].contains("too long"), "{:?}", h.start_errors);
}

#[test]
fn the_desk_has_a_limit_on_items_and_paths() {
    use crate::fs::{MAX_ITEMS, MAX_PATH};
    let (mut fs, mut h) = start();
    let mut i = 0;
    let err = loop {
        match fs.write_page(&format!("p{i:05}"), "") {
            Ok(()) => i += 1,
            Err(e) => break e,
        }
    };
    assert!(err.contains(&format!("at most {MAX_ITEMS} drawers and pages")), "{err}");
    assert_eq!(fs.count(), MAX_ITEMS);
    assert!(fs.build_drawer("more").is_err());
    assert!(fs.app_write("mail:sent:s1", "x").is_err());
    // a full desk can still be stamped and listed, with little memory
    h.stamp(&fs, D, "full").unwrap();
    fs.toss("p00000").unwrap();
    let (out, peak, _) = measure(|| h.changes(&fs, "").unwrap());
    assert!(peak < 200_000, "peak {peak}");
    assert!(out[0].starts_with("1 item changed"), "{}", out[0]);
    let (n, peak, _) = measure(|| h.unstamped_count(&fs));
    assert_eq!((n, peak), (1, 0));
    // paths
    let long = format!("desk:{}", ["abcdefghij"; 12].join(":"));
    assert!(fs.resolve("", &long).unwrap_err().contains(&format!("at most {MAX_PATH} characters")));
}

#[test]
fn moving_a_drawer_cannot_make_paths_too_long() {
    let mut fs = desk("#page a:b:c:d:e:page\nx\n#end\n#drawer deep1234567890123456789:deep1234567890123456789:deep1234567890123456789:deep1234567890123456789:deep1234567890123456789\n");
    let e = fs.move_into("a", "deep1234567890123456789:deep1234567890123456789:deep1234567890123456789:deep1234567890123456789:deep1234567890123456789").unwrap_err();
    assert!(e.contains("paths inside would get too long"), "{e}");
    assert!(fs.exists("a:b:c:d:e:page"));
}

#[test]
fn start_files_leave_out_what_does_not_fit() {
    use crate::fs::MAX_ITEMS;
    let mut text = String::new();
    for i in 0..6000 {
        text.push_str(&format!("#page p{i:05}\n#end\n"));
    }
    text.push_str(&format!("#page {}\n#end\n", ["abcdefghij"; 13].join(":")));
    let mut fs = Fs::from_start_files(&text);
    fs.ensure_system();
    assert!(fs.count() <= MAX_ITEMS);
    assert!(fs.skipped > 1000, "{}", fs.skipped);
}

#[test]
fn start_errors_are_short_and_few() {
    let mut h = History::new();
    let note = "n".repeat(4_200_000);
    let fs = desk("");
    let (_, peak, _) = measure(|| {
        for _ in 0..50 {
            let _ = h.start_stamp(&fs, &note, D);
        }
    });
    assert_eq!(h.start_errors.len(), 21);
    assert!(h.start_errors.iter().all(|e| e.len() < 200));
    assert!(peak < 100_000, "peak {peak}");
}

#[test]
fn the_same_new_text_is_counted_once() {
    let mut fs = desk("");
    let mut h = History::new();
    for gen in 0..3 {
        let body = format!("{gen}{}", "t".repeat(10_000));
        for i in 0..100 {
            fs.write_page(&format!("p{i}"), &body).unwrap();
        }
        h.stamp(&fs, D, "gen").unwrap();
    }
    assert!(h.used() < 100_000, "{}", h.used());
}

#[test]
fn the_history_counts_at_least_what_it_holds() {
    let mut text = String::new();
    for i in 0..3000 {
        text.push_str(&format!("#page d{}:p{i}\nline {i}\n#end\n", i % 30));
    }
    let mut fs = desk(&text);
    let mut h = History::new();
    let (_, _, live) = measure(|| {
        for i in 0..99 {
            fs.write_page("counter", &format!("{i}")).unwrap();
            if h.stamp(&fs, D, "n").is_err() {
                break;
            }
        }
    });
    assert!(h.stamps().len() > 3);
    assert!((live as usize) < h.used(), "holds {live}, counted {}", h.used());
}

#[test]
fn start_files_never_hold_more_text_than_fits() {
    let text = format!("#page huge\n{}\n#end\n#page small\nok\n#end\n", "z".repeat(9 * 1024 * 1024));
    let (fs, peak, _) = measure(|| Fs::from_start_files(&text));
    assert!(!fs.exists("huge"));
    assert_eq!(fs.read_page("small").unwrap(), "ok");
    assert_eq!(fs.skipped, 1);
    assert!(peak < 3_000_000, "peak {peak}");
}

#[test]
fn apps_keep_the_text_limit() {
    let mut fs = desk("");
    let room = fs::MAX_DESK - fs.total_bytes();
    fs.write_page("filler", &"f".repeat(room - 1)).unwrap();
    let e = fs.app_write("mail:sent:s1", "from: me").unwrap_err();
    assert!(e.contains("desk is full"), "{e}");
    assert!(!fs.exists("mail:sent:s1"));
}

#[test]
fn writes_keep_the_path_limit() {
    let mut fs = desk("");
    let long = ["abcdefghij"; 11].join(":");
    fs.ensure_drawers(&long);
    let e = fs.write_page(&format!("{long}:pp"), "x").unwrap_err();
    assert!(e.contains("too long"), "{e}");
    assert!(fs.build_drawer(&format!("{long}:dd")).is_err());
}
