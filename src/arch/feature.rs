//! Hardware feature detection via AArch64 system registers.
//!
//! Reference: ID_AA64PFR0_EL1, AArch64 Processor Feature Register 0.

use core::arch::asm;

/// Returns the value of `ID_AA64PFR0_EL1`.
fn id_aa64pfr0() -> u64 {
    let value: u64;
    unsafe { asm!("mrs {}, ID_AA64PFR0_EL1", out(reg) value) };
    value
}

/// Extracts a 4-bit field from a register value.
fn field(reg: u64, shift: u32) -> u64 {
    (reg >> shift) & 0xF
}

/// Returns `true` if the Realm Management Extension (RME) is implemented.
pub fn has_rme() -> bool {
    field(id_aa64pfr0(), 52) != 0
}

/// Logs the features reported by `ID_AA64PFR0_EL1`.
pub fn log_features() {
    let pfr0 = id_aa64pfr0();

    log::info!("ID_AA64PFR0_EL1: {pfr0:#018x}");

    // EL0-EL3 support
    let el0 = field(pfr0, 0);
    let el1 = field(pfr0, 4);
    let el2 = field(pfr0, 8);
    let el3 = field(pfr0, 12);
    log::info!(
        "  EL0: {} | EL1: {} | EL2: {} | EL3: {}",
        el_description(el0),
        el_description(el1),
        el_description(el2),
        el_description(el3),
    );

    // Floating point and SIMD
    let fp = field(pfr0, 16);
    let advsimd = field(pfr0, 20);
    log::info!(
        "  FP: {} | AdvSIMD: {}",
        fp_description(fp),
        fp_description(advsimd),
    );

    // GIC
    let gic = field(pfr0, 24);
    log::info!("  GIC: {}", match gic {
        0b0000 => "none",
        0b0001 => "v3.0/v4.0",
        0b0011 => "v4.1",
        _ => "unknown",
    });

    // RAS
    let ras = field(pfr0, 28);
    log::info!("  RAS: {}", match ras {
        0b0000 => "none",
        0b0001 => "v1",
        0b0010 => "v1.1",
        0b0011 => "v2",
        _ => "unknown",
    });

    // SVE
    let sve = field(pfr0, 32);
    log::info!("  SVE: {}", if sve != 0 { "yes" } else { "no" });

    // Secure EL2
    let sel2 = field(pfr0, 36);
    log::info!("  Secure EL2: {}", if sel2 != 0 { "yes" } else { "no" });

    // MPAM
    let mpam = field(pfr0, 40);
    log::info!("  MPAM: v{mpam}");

    // AMU
    let amu = field(pfr0, 44);
    log::info!("  AMU: {}", match amu {
        0b0000 => "none",
        0b0001 => "v1",
        0b0010 => "v1.1",
        _ => "unknown",
    });

    // DIT
    let dit = field(pfr0, 48);
    log::info!("  DIT: {}", if dit != 0 { "yes" } else { "no" });

    // RME
    let rme = field(pfr0, 52);
    log::info!("  RME: {}", match rme {
        0b0000 => "none",
        0b0001 => "v1",
        0b0010 => "v1 + GPC2",
        0b0011 => "v1 + GPC2 + GPC3",
        _ => "unknown",
    });

    // CSV2
    let csv2 = field(pfr0, 56);
    log::info!("  CSV2: {}", match csv2 {
        0b0000 => "none",
        0b0001 => "v1",
        0b0010 => "v2",
        0b0011 => "v3",
        _ => "unknown",
    });

    // CSV3
    let csv3 = field(pfr0, 60);
    log::info!("  CSV3: {}", if csv3 != 0 { "yes" } else { "no" });
}

fn el_description(val: u64) -> &'static str {
    match val {
        0b0000 => "none",
        0b0001 => "AArch64",
        0b0010 => "AArch64+AArch32",
        _ => "unknown",
    }
}

fn fp_description(val: u64) -> &'static str {
    match val {
        0b0000 => "yes",
        0b0001 => "yes (FP16)",
        0b1111 => "no",
        _ => "unknown",
    }
}
