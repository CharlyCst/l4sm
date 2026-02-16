//! Work-In-Progress: Capabilities for L4sm
//!
//! L4sm is inspired by seL4"s design.

/// Capability Space Capability.
pub struct CSpaceCapa {}

/// Untyped Memory Capability.
pub struct UntypedCapa {
    /// Size of the untyped memory, as a power of two.
    size: u8,
    /// Start address of the untyped memory.
    ///
    /// Note: this should be page-aligned, we could use the remaining bits to optimize the
    /// capability size.
    address: usize,
    /// How much of the untyped memory has been allocated for kernel objects.
    ///
    /// This can be seen as a simple bump allocator.
    watermark: usize,
}

impl UntypedCapa {
    /// Crates a new untyped memory capability.
    ///
    /// Size is interpreted as a power of two.
    pub fn new(address: usize, size: u8) -> Self {
        assert!(size < 64, "size exponent too large");
        assert!(
            address & ((1usize << size) - 1) == 0,
            "address must be aligned to the untyped size"
        );
        Self {
            size,
            address,
            watermark: 0,
        }
    }

    /// Allocates memory for use by L4sm objects.
    ///
    /// Size is in bytes, alignment a power of two.
    ///
    /// The returned address is naturally aligned to `size`. Size is in bytes.
    pub fn allocate(&mut self, size: usize, alignment: u8) -> Result<usize, ()> {
        assert!(alignment < 64, "alignment is too large");

        let align = 1usize << alignment;
        let aligned = (self.watermark + align - 1) & !(align - 1);
        let end = 1usize << self.size;

        if aligned + size > end {
            return Err(());
        }

        self.watermark = aligned + size;
        Ok(self.address + aligned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocate() {
        // Simple allocation.
        let mut ut = UntypedCapa::new(0x1000, 12); // 4 KiB region
        let addr = ut.allocate(64, 0).unwrap();
        assert_eq!(addr, 0x1000);
        assert_eq!(ut.watermark, 64);

        // Aligned allocation: 256-byte alignment bumps the watermark.
        let mut ut = UntypedCapa::new(0x1000, 12);
        ut.allocate(8, 0).unwrap();
        let addr = ut.allocate(64, 8).unwrap();
        assert_eq!(addr, 0x1100);

        // Multiple sequential allocations.
        let mut ut = UntypedCapa::new(0x1000, 12); // 4 KiB
        let a = ut.allocate(128, 0).unwrap();
        let b = ut.allocate(256, 0).unwrap();
        assert_eq!(a, 0x1000);
        assert_eq!(b, 0x1080);

        // Out of memory.
        let mut ut = UntypedCapa::new(0x1000, 4); // 16 bytes
        ut.allocate(8, 0).unwrap();
        assert!(ut.allocate(16, 0).is_err());

        // Out of memory due to alignment padding.
        let mut ut = UntypedCapa::new(0x1000, 4); // 16 bytes
        ut.allocate(1, 0).unwrap();
        assert!(ut.allocate(8, 4).is_err());

        // Exact fit.
        let mut ut = UntypedCapa::new(0x1000, 4); // 16 bytes
        let addr = ut.allocate(16, 0).unwrap();
        assert_eq!(addr, 0x1000);
        assert!(ut.allocate(1, 0).is_err());
    }
}
