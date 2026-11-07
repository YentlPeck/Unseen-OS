//! Unseen OS kernel.

#![no_std]
#![no_main]

extern crate alloc;

mod boot;
mod calc;
mod calendar;
mod chore;
mod chores;
mod contacts;
mod files;
mod fs;
mod history;
mod inspector;
mod keyboard;
mod mail;
mod notes;
mod programs;
mod port;
mod rtc;
mod screen;
mod serial;
mod settings;
mod shell;
mod ui;

use core::panic::PanicInfo;
use linked_list_allocator::LockedHeap;

/// Memory for files and apps (the "heap"): 16 MB. The desk (1 MB, and a copy while changes
/// are made), the history (2 MB) and a running program (at most about 5 MB, see
/// docs/script-design.md §9) together stay well below it.
const HEAP_SIZE: usize = 16 * 1024 * 1024;
static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

/// Bytes of the heap in use now (for the inspector).
pub fn heap_used() -> usize {
    ALLOCATOR.lock().used()
}

extern "C" {
    /// The bottom of the kernel stack (boot.rs). The stack grows down towards it.
    static boot_stack: u8;
}
const STACK_SIZE: usize = 256 * 1024;

/// What the free part of the stack is painted with at boot.
const PAINT: u8 = 0xA5;

/// Paint the stack below the current frame (less a margin), so stack_used can see how deep
/// it ever went.
#[inline(never)]
fn paint_stack() {
    let here = 0u8;
    let sp = core::ptr::addr_of!(here) as usize;
    let base = core::ptr::addr_of!(boot_stack) as usize;
    let end = sp.saturating_sub(4096).max(base);
    for a in base..end {
        unsafe { (a as *mut u8).write_volatile(PAINT) };
    }
}

/// A watermark of the stack: how far down a byte of the painted part was ever changed. This
/// is the deepest place written, not a proof of the peak (a frame that is reserved but never
/// written to is not seen; it also can't harm the memory below the stack).
pub fn stack_used() -> usize {
    let base = core::ptr::addr_of!(boot_stack);
    let mut untouched = 0;
    while untouched < STACK_SIZE && unsafe { base.add(untouched).read_volatile() } == PAINT {
        untouched += 1;
    }
    STACK_SIZE - untouched
}

/// Memory size in MB, from the multiboot info the loader hands us.
fn memory_mb(info: u32) -> u32 {
    if info == 0 {
        return 0;
    }
    let p = info as usize as *const u32;
    unsafe {
        if p.read_volatile() & 1 == 0 {
            return 0; // loader gave no memory info
        }
        let upper_kb = p.add(2).read_volatile();
        (upper_kb + 1024) / 1024
    }
}

/// The task's start files, if the harness passed them as a multiboot module.
fn start_files(info: u32) -> Option<&'static str> {
    if info == 0 {
        return None;
    }
    let p = info as usize as *const u32;
    unsafe {
        let flags = p.read_volatile();
        let count = p.add(5).read_volatile();
        if flags & (1 << 3) == 0 || count == 0 {
            return None;
        }
        let module = p.add(6).read_volatile() as usize as *const u32;
        let start = module.read_volatile() as usize;
        let end = module.add(1).read_volatile() as usize;
        let bytes = core::slice::from_raw_parts(start as *const u8, end - start);
        core::str::from_utf8(bytes).ok()
    }
}

#[no_mangle]
extern "C" fn kernel_main(multiboot_info: u32) -> ! {
    paint_stack();
    serial::init();
    log!("Unseen OS booting");
    unsafe { ALLOCATOR.lock().init(core::ptr::addr_of_mut!(HEAP) as *mut u8, HEAP_SIZE) };

    let mut screen = screen::Screen::new();
    let mut keyboard = keyboard::Keyboard::new();
    let start = start_files(multiboot_info);
    // A random code the harness puts in the start files, so it knows it is
    // talking to the right machine.
    let session = start
        .and_then(|t| t.lines().find_map(|l| l.strip_prefix("#session ")))
        .map_or("none", |s| s.trim());
    // A frozen date for the experiment ("#today 2026-11-12"); else the PC's clock.
    let today = start
        .and_then(|t| t.lines().find_map(|l| l.strip_prefix("#today ")))
        .map(|s| s.trim())
        .filter(|s| calendar::parse_date(s).is_some())
        .map(|s| alloc::string::String::from(s));
    let date = today.clone().unwrap_or_else(|| {
        let t = rtc::now();
        alloc::format!("{:04}-{:02}-{:02}", t.year, t.month, t.day)
    });
    // Stamps written in the start files ("#stamp <note>") become the history;
    // without any, the desk as it starts is stamp #1.
    let mut stamps = history::History::new();
    let desk = match start {
        Some(text) => {
            log!("loading start files ({} bytes)", text.len());
            fs::Fs::load_start_files(text, |fs, note| {
                if let Err(e) = stamps.start_stamp(fs, note, &date) {
                    log!("START FILES ERROR: {}", e);
                }
            })
        }
        None => fs::Fs::sample(),
    };
    let mut desk = desk;
    if desk.skipped > 0 {
        log!("START FILES ERROR: {} drawers/pages left out", desk.skipped);
        stamps.start_error(&alloc::format!(
            "{} drawers or pages were left out (more than {} in all, or a path over {} characters)",
            desk.skipped,
            fs::MAX_ITEMS,
            fs::MAX_PATH
        ));
    }
    desk.ensure_system();
    if stamps.stamps().is_empty() {
        if let Err(e) = stamps.stamp(&desk, &date, "the desk as it started") {
            log!("START FILES ERROR: first stamp: {}", e);
            stamps.start_error(&alloc::format!("first stamp: {e}"));
        }
    }
    let mut shell = shell::Shell::new(desk, stamps, memory_mb(multiboot_info), today);
    let mut inspector = inspector::Inspector::new();
    let mut keys_handled: u64 = 0;
    shell.start(&mut screen);
    log!("ready");

    loop {
        inspector.poll(&inspector::State {
            screen: &screen,
            fs: shell.fs(),
            history: shell.history(),
            keys_handled,
            session,
        });
        if let Some(key) = keyboard.poll() {
            shell.key(&mut screen, key);
            keys_handled += 1;
        } else {
            core::hint::spin_loop();
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    log!("KERNEL PANIC: {}", info);
    loop {
        unsafe { core::arch::asm!("cli; hlt") }
    }
}
