//! Serial port COM1: a hidden text channel to the outside world.
//! Used for boot logs now, and later by the test harness to inspect the OS.

use crate::port::{inb, outb};
use core::fmt;

const COM1: u16 = 0x3F8;

pub fn init() {
    outb(COM1 + 1, 0x00); // no interrupts
    outb(COM1 + 3, 0x80); // set baud rate divisor
    outb(COM1, 0x01); // 115200 baud
    outb(COM1 + 1, 0x00);
    outb(COM1 + 3, 0x03); // 8 bits, no parity, 1 stop bit
    outb(COM1 + 2, 0xC7); // enable FIFO
    outb(COM1 + 4, 0x0B);
}

pub fn write_byte(b: u8) {
    while inb(COM1 + 5) & 0x20 == 0 {}
    outb(COM1, b);
}

/// A byte from the outside, if one is waiting.
pub fn read_byte() -> Option<u8> {
    (inb(COM1 + 5) & 0x01 != 0).then(|| inb(COM1))
}

pub struct Serial;

impl fmt::Write for Serial {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for b in s.bytes() {
            if b == b'\n' {
                write_byte(b'\r');
            }
            write_byte(b);
        }
        Ok(())
    }
}

/// Like `Serial`, but never waits: if the port is busy (e.g. nobody is
/// listening), the byte is dropped. Used for logs, so they can never stall
/// the OS and make it miss key presses.
pub struct Log;

impl fmt::Write for Log {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        fn try_write(b: u8) -> bool {
            for _ in 0..1000 {
                if inb(COM1 + 5) & 0x20 != 0 {
                    outb(COM1, b);
                    return true;
                }
            }
            false
        }
        for b in s.bytes() {
            if (b == b'\n' && !try_write(b'\r')) || !try_write(b) {
                break; // port busy: drop the rest of this log line
            }
        }
        Ok(())
    }
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        let _ = writeln!($crate::serial::Log, $($arg)*);
    }};
}
