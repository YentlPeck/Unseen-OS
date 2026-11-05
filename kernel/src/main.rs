//! Unseen OS kernel.

#![no_std]
#![no_main]

extern crate alloc;

mod boot;
mod calc;
mod calendar;
#[allow(dead_code)] // wired into the shell in v0.3 step 2
mod chore;
mod contacts;
mod files;
mod fs;
mod inspector;
mod keyboard;
mod mail;
mod notes;
mod port;
mod rtc;
mod screen;
mod serial;
mod settings;
mod shell;
mod ui;

use core::panic::PanicInfo;
use linked_list_allocator::LockedHeap;

/// Memory for files and apps (the "heap"): 8 MB.
const HEAP_SIZE: usize = 8 * 1024 * 1024;
static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

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
    let desk = match start {
        Some(text) => {
            log!("loading start files ({} bytes)", text.len());
            fs::Fs::from_start_files(text)
        }
        None => fs::Fs::sample(),
    };
    let mut desk = desk;
    desk.ensure_system();
    let mut shell = shell::Shell::new(desk, memory_mb(multiboot_info));
    let mut inspector = inspector::Inspector::new();
    let mut keys_handled: u64 = 0;
    shell.start(&mut screen);
    log!("ready");

    loop {
        inspector.poll(&inspector::State { screen: &screen, fs: shell.fs(), keys_handled, session });
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
