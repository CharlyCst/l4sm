#![no_std]
#![no_main]

mod arch;
mod driver;
mod logger;
mod platform;

use core::arch::global_asm;

const STACK_SIZE: usize = 16 * 1024;

// ———————————————————————————— Rust Entry Point ———————————————————————————— //

#[unsafe(no_mangle)]
fn main() -> ! {
    logger::init();
    log::info!("Hello, world!");
    arch::feature::log_features();

    if !arch::feature::has_rme() {
        panic!("Hardware does not support RME");
    }

    platform::exit_success();
}

// ————————————————————————————— Panic Handler —————————————————————————————— //

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    log::error!("{info}");
    platform::exit_failure();
}

// —————————————————————————— Assembly Entry Point —————————————————————————— //

// The global entry point.
//
// This is the first code that runs at EL3, it is responsible for setting up a suitable environment
// for the Rust code (stack, BSS) before jumping into main.
global_asm!(
r#"
.text
.global _start
_start:
    // Mask all exceptions (Debug, SError, IRQ, FIQ) inherited from previous boot stage.
    msr DAIFSet, #0xf

    // Set up the stack.
    // The stack grows downward, so sp = _stack_start + STACK_SIZE.
    ldr x0, =_stack_start
    ldr x1, ={stack_size}
    add x1, x0, x1
    mov sp, x1

    // Fill the stack with a known pattern to help detect overflows.
    ldr x2, ={stack_pattern}
stack_fill_loop:
    cmp x0, x1
    b.hs stack_fill_done
    str x2, [x0], #8
    b stack_fill_loop
stack_fill_done:

    // Zero-out the BSS section.
    ldr x0, =_bss_start
    ldr x1, =_bss_stop
zero_bss_loop:
    cmp x0, x1
    b.hs zero_bss_done
    stp xzr, xzr, [x0], #16
    b zero_bss_loop
zero_bss_done:

    // Jump into Rust code.
    b {main}
"#,
    main = sym main,
    stack_size = const STACK_SIZE,
    stack_pattern = const 0x0BAD_BED0_0BAD_BED0_u64,
);
