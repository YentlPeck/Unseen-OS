//! Run a test page on a desk built from start files, on the PC (a fast check while writing
//! tasks; the real grading runs in QEMU).
//!
//!   cargo run --release --example runtests -- START_FILE TEST_PAGE
//!
//! Prints the report lines, as the `test` command shows them.

extern crate alloc;

#[path = "../../kernel/src/fs.rs"]
#[allow(dead_code)]
mod fs;

use fs::Fs;

struct Desk(Fs);

impl script::World for Desk {
    fn page(&self, p: &str) -> Option<String> {
        self.0.read_page(p).ok().map(|t| t.to_string())
    }
    fn is_drawer(&self, p: &str) -> bool {
        self.0.is_drawer(p)
    }
    fn pages_in(&self, d: &str) -> Vec<String> {
        self.0.children(d).into_iter().filter(|(_, dr)| !dr).map(|(n, _)| fs::join(d, &n)).collect()
    }
    fn check_write(&self, _: &str) -> Result<(), String> {
        Ok(())
    }
    fn check_toss(&self, _: &str) -> Result<(), String> {
        Ok(())
    }
    fn check_send(&self, _: &str, _: &str, _: &str) -> Result<(), String> {
        Ok(())
    }
    fn check_event(&self, _: &str, _: &str, _: &str, _: &str) -> Result<(), String> {
        Ok(())
    }
    fn today(&self) -> String {
        "2026-11-20".into()
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let start = std::fs::read_to_string(&args[1]).expect("start file");
    let page = args[2].strip_prefix("desk:").unwrap_or(&args[2]).to_string();
    let mut desk = Fs::from_start_files(&start);
    desk.ensure_system();
    let desk = Desk(desk);
    let source = script::World::page(&desk, &page).expect("no such test page");
    for l in script::run_tests(&page, &source, &desk).lines() {
        println!("{l}");
    }
}
