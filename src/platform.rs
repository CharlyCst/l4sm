//! Platform-specific constants and configuration.
//!
//! All hardware-specific values (base addresses, memory layout, etc.) should be defined in this
//! module to make porting to a new platform straightforward.

/// Base address of the secure world PL011 UART (UART1).
pub const UART1_BASE: usize = 0x0904_0000;
