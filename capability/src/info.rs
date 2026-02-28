//! Public capability info types.
//!
//! [`CapaInfo`] is a safe snapshot of a capability's public fields. It contains no CDT links,
//! backing-store pointers, or other internal state, making it safe to pass to userspace.

/// How an untyped capability was derived.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UntypedKind {
    /// Derived via alias; the range may overlap other aliased siblings.
    Aliased,
    /// Derived via carve; the range is exclusive among all siblings.
    Carved,
}

/// A safe, inspectable snapshot of a capability.
///
/// Contains only the fields a task needs to reason about its capabilities. All internal state
/// (CDT links, backing-store pointers, watermark, …) is omitted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapaInfo {
    Null,
    CNode {
        /// Number of slots as a power of two: this CNode holds `2^slots` capability slots.
        slots: u8,
        /// Guard value matched when entering this CNode during CSpace resolution.
        guard: usize,
        /// Number of bits in the guard.
        guard_size: u8,
    },
    Untyped {
        /// Start address of the memory region (inclusive).
        start: usize,
        /// End address of the memory region (exclusive).
        end: usize,
        /// How this capability was derived.
        kind: UntypedKind,
    },
}
