//! Host tests for kernel/src/chore.rs (run: cargo test, in this folder).
extern crate alloc;

#[path = "../../kernel/src/fs.rs"]
#[allow(dead_code)]
pub mod fs;

#[path = "../../kernel/src/history.rs"]
pub mod history;

#[path = "../../kernel/src/chore.rs"]
pub mod chore;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod fuzz;

#[cfg(test)]
mod history_tests;
