//! Real memory (Sol's review of M1): a counting allocator watches every allocation while hostile
//! programs run. The heap a run really uses must stay within its budgets (step + program) with
//! a margin, whatever the program does, so the kernel's 8 MB heap can never run out.
extern crate alloc;

#[path = "../../kernel/src/fs.rs"]
#[allow(dead_code)]
mod fs;
mod common;

use common::TestWorld;
use script::limits::*;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
use std::sync::Mutex;

struct Counting;
static NOW: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
/// Tests in one file run on several threads; this keeps the measurements apart.
static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let n = NOW.fetch_add(l.size(), SeqCst) + l.size();
        PEAK.fetch_max(n, SeqCst);
        System.alloc(l)
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        NOW.fetch_sub(l.size(), SeqCst);
        System.dealloc(p, l)
    }
}

#[global_allocator]
static A: Counting = Counting;

/// The most heap one run uses on top of what was in use before it.
fn real_peak(src: &str, world: &TestWorld) -> (usize, script::Outcome) {
    let before = NOW.load(SeqCst);
    PEAK.store(before, SeqCst);
    let o = script::run("main", src, world);
    (PEAK.load(SeqCst) - before, o)
}

#[test]
fn hostile_programs_stay_within_their_budgets() {
    let _one = ONE_AT_A_TIME.lock().unwrap();
    let world = TestWorld::new();
    let t = "x".repeat(3990);
    let programs = [
        format!("set l to []\nrepeat 2000 times\n  add '{t}' & '{t}' to l\ndone"),
        format!("set r to {{}}\nset n to 0\nrepeat 2000 times\n  set n to n + 1\n  set r.a to '{t}' & n\ndone"),
        "set l to []\nrepeat 2000 times\n  add [1, 2, 3, 4, 5, 6, 7, 8] to l\ndone".into(),
        "set l to []\nrepeat 2000 times\n  add {a: 1, b: 2, c: 3, d: 4} to l\ndone".into(),
        "set x to [1]\nrepeat 30 times\n  set x to [x, x]\ndone".into(),
        "set l to [1]\nrepeat 2000 times\n  set m to l\n  add 1 to m\n  set l to m\ndone".into(),
        format!("set n to 0\nrepeat 300 times\n  set n to n + 1\n  add('desk:p' & n, '{t}' & '{t}')\ndone"),
        format!("set n to 0\nrepeat 300 times\n  set n to n + 1\n  write('desk:p' & n, '{t}' & '{t}')\ndone"),
        format!("repeat 500 times\n  say '{t}' & '{t}'\ndone"),
        format!("set l to []\nrepeat 2000 times\n  add words('{}') to l\ndone", "a ".repeat(1000)),
        format!("say {}", "[1, 2, 3, 4], ".repeat(1000) + "1"),
        format!("{}say 1", "set aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa to 1\n".repeat(390)),
    ];
    let budget = STEP_MEMORY + PROGRAM_MEMORY;
    for src in &programs {
        let (peak, o) = real_peak(src, &world);
        let first = src.lines().next().unwrap_or("");
        println!("real {peak:>8}  counted {:>8}  {:?}  {}", o.peak_memory, o.error.as_ref().map(|e| &e.message), &first[..first.len().min(50)]);
        assert!(peak <= budget * 3 / 2, "real heap {peak} for: {}", &src[..src.len().min(80)]);
    }
}

#[test]
fn counted_memory_is_close_to_real_memory() {
    // For ordinary values, what the meter counts must not be far below the real heap.
    let _one = ONE_AT_A_TIME.lock().unwrap();
    let world = TestWorld::new();
    for src in [
        "set l to []\nrepeat 1000 times\n  add 'abc' & '' to l\ndone",
        "set l to []\nrepeat 1000 times\n  add {a: 1} to l\ndone",
        "set l to []\nrepeat 1000 times\n  add [1, 2] to l\ndone",
    ] {
        let (peak, o) = real_peak(src, &world);
        println!("real {peak:>8}  counted {:>8}  {src:?}", o.peak_memory);
        assert!(peak <= o.peak_memory * 2 + 64 * 1024, "real {peak} vs counted {}", o.peak_memory);
    }
}
