//! Platform-specific constants and configuration.
//!
//! All hardware-specific values (base addresses, memory layout, etc.) should be defined in this
//! module to make porting to a new platform straightforward.

use core::arch::asm;

/// Base address of the secure world PL011 UART (UART1).
pub const UART1_BASE: usize = 0x0904_0000;

/// Exits the emulator with a success.
pub fn exit_success() -> ! {
    semihosting_exit(true);
}

/// Exits the emulator with a failure.
pub fn exit_failure() -> ! {
    semihosting_exit(false);
}

/// Exits via ARM semihosting.
fn semihosting_exit(success: bool) -> ! {
    // ARM semihosting constants.
    const SYS_EXIT: u64 = 0x18;
    const ADP_STOPPED_APPLICATION_EXIT: u64 = 0x20026;

    let code = if success { 0u64 } else { 1u64 };
    let params: [u64; 2] = [ADP_STOPPED_APPLICATION_EXIT, code];
    unsafe {
        asm!(
            "hlt #0xf000",
            in("x0") SYS_EXIT,
            in("x1") params.as_ptr(),
            // No "options(noreturn)" here in case semihosting is not enabled
        );
    }

    // Semihosting is not enabled, let's spin here forever.
    if success {
        log::info!("Exit success, spinning forever...")
    } else {
        log::info!("Exit failure, spinning forever...")
    }
    loop {
        core::hint::spin_loop();
    }
}
