#![no_std]
#![no_main]

#[unsafe(no_mangle)]
fn _start() {
    loop {
        core::hint::spin_loop();
    }
}

// ————————————————————————————— Panic Handler —————————————————————————————— //

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
