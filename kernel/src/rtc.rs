//! Real-time clock (the CMOS chip that keeps the date and time).

use crate::port::{inb, outb};

pub struct DateTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

fn read(reg: u8) -> u8 {
    outb(0x70, reg);
    inb(0x71)
}

pub fn now() -> DateTime {
    while read(0x0A) & 0x80 != 0 {} // wait while the chip is updating
    let binary = read(0x0B) & 0x04 != 0;
    let conv = |v: u8| if binary { v } else { (v & 0x0F) + (v >> 4) * 10 };
    DateTime {
        second: conv(read(0x00)),
        minute: conv(read(0x02)),
        hour: conv(read(0x04)),
        day: conv(read(0x07)),
        month: conv(read(0x08)),
        year: 2000 + conv(read(0x09)) as u16,
    }
}
