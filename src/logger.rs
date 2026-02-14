//! Logging backend that writes to the secure world PL011 UART.

use crate::driver::pl011::Pl011;
use core::fmt::Write;
use core::sync::atomic::{AtomicBool, Ordering};
use log::Level;
use spin::Mutex;

/// Base address of the secure world PL011 UART on the QEMU virt machine.
const UART1_BASE: usize = 0x0904_0000;

// SAFETY: this is the base address of the secure world PL011 UART on the QEMU virt machine.
static UART1: Mutex<Pl011> = Mutex::new(unsafe { Pl011::new(UART1_BASE) });
static LOGGER: Logger = Logger;
static INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Initializes the logger.
///
/// # Panics
///
/// Panics if called more than once.
pub fn init() {
    assert!(
        !INITIALIZED.swap(true, Ordering::Relaxed),
        "logger already initialized"
    );
    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(log::LevelFilter::Trace);
}

// ————————————————————————————————— Logger ————————————————————————————————— //

pub struct Logger;

impl log::Log for Logger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let mut uart = UART1.lock();
            let _ = writeln!(
                uart,
                "[{}] {}",
                level_display(record.level()),
                record.args()
            );
        }
    }

    fn flush(&self) {}
}

fn level_display(level: Level) -> &'static str {
    // We log with colors, using ANSI escape sequences
    match level {
        Level::Error => "\x1b[31;1mError\x1b[0m",
        Level::Warn => "\x1b[33;1mWarn\x1b[0m ",
        Level::Info => "\x1b[32;1mInfo\x1b[0m ",
        Level::Debug => "\x1b[34;1mDebug\x1b[0m",
        Level::Trace => "\x1b[35;1mTrace\x1b[0m",
    }
}
