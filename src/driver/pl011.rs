//! Minimal driver for the ARM PL011 UART.

use core::fmt;
use core::ptr;

const UARTDR: usize = 0x00;
const UARTFR: usize = 0x18;
const UARTFR_TXFF: u32 = 1 << 5;

/// A PL011 UART, accessed through memory-mapped I/O.
pub struct Pl011 {
    base: usize,
}

impl Pl011 {
    /// Creates a new PL011 driver for the UART at the given MMIO base address.
    ///
    /// # Safety
    ///
    /// `base` must be the base address of a valid PL011 UART and must remain mapped for the
    /// lifetime of the returned driver.
    pub const unsafe fn new(base: usize) -> Self {
        Self { base }
    }

    /// Writes a single byte to the UART, blocking until the TX FIFO has space.
    pub fn putc(&self, c: u8) {
        while self.is_tx_busy() {
            core::hint::spin_loop();
        }
        unsafe {
            ptr::write_volatile((self.base + UARTDR) as *mut u32, c as u32);
        }
    }

    fn is_tx_busy(&self) -> bool {
        let fr = unsafe { ptr::read_volatile((self.base + UARTFR) as *const u32) };
        fr & UARTFR_TXFF != 0
    }
}

impl fmt::Write for Pl011 {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.bytes() {
            self.putc(c);
        }
        Ok(())
    }
}
