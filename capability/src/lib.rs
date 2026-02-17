//! Work-In-Progress: Capabilities for L4sm
//!
//! L4sm is inspired by seL4"s design.

const FRAME_SIZE_EXPONENT: u8 = 12;

/// A capability index, represents an address in capability space (CSpace).
#[repr(transparent)]
pub struct CapaIdx(usize);

/// A capability, as stored in a CSpace.
pub enum Capa {
    Null,
    CSpace(CSpaceCapa),
    Untyped(UntypedCapa),
}

/// Capability Space Capability.
pub struct CSpaceCapa {
    /// Number of slots, as a power of two.
    slots: u8,
    /// Start address of the CSpace object.
    address: usize,
}

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
    /// Whether the untyped capability has been split in two child capabilities.
    is_split: bool,
}

impl UntypedCapa {
    /// Crates a new untyped memory capability.
    ///
    /// Size is interpreted as a power of two.
    pub fn new(address: usize, size: u8) -> Self {
        assert!(size < 64, "size exponent too large");
        assert!(size >= FRAME_SIZE_EXPONENT, "size exponent is too small");
        assert!(
            address & ((1usize << size) - 1) == 0,
            "address must be aligned to the untyped size"
        );
        Self {
            size,
            address,
            watermark: 0,
            is_split: false,
        }
    }

    /// Allocates memory for use by L4sm objects.
    ///
    /// Size is in bytes, alignment a power of two.
    ///
    /// The returned address is naturally aligned to `size`. Size is in bytes.
    pub fn allocate(&mut self, size: usize, alignment: u8) -> Result<usize, ()> {
        assert!(alignment < 64, "alignment is too large");

        if self.is_split {
            return Err(());
        }

        let align = 1usize << alignment;
        let aligned = (self.watermark + align - 1) & !(align - 1);
        let end = 1usize << self.size;

        if aligned + size > end {
            return Err(());
        }

        self.watermark = aligned + size;
        Ok(self.address + aligned)
    }

    pub fn split(&mut self) -> Result<(UntypedCapa, UntypedCapa), ()> {
        // We can not split a capability twice.
        if self.is_split {
            Err(())
        }
        // We can not split a capability with already allocated objects.
        else if self.watermark != 0 {
            Err(())
        }
        // We can not split the capability any further.
        else if self.size <= FRAME_SIZE_EXPONENT {
            Err(())
        }
        // All good, we can split.
        else {
            self.is_split = true;
            let size = self.size - 1;
            let address = self.address;
            let left = Self::new(address, size);
            let right = Self::new(address + (1 << size), size);
            Ok((left, right))
        }
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
        let mut ut = UntypedCapa::new(0x1000, 12); // 4 KiB
        ut.allocate(2048, 0).unwrap();
        assert!(ut.allocate(4096, 0).is_err());

        // Out of memory due to alignment padding.
        let mut ut = UntypedCapa::new(0x1000, 12); // 4 KiB
        ut.allocate(1, 0).unwrap();
        assert!(ut.allocate(2048, 12).is_err());

        // Exact fit.
        let mut ut = UntypedCapa::new(0x1000, 12); // 4 KiB
        let addr = ut.allocate(4096, 0).unwrap();
        assert_eq!(addr, 0x1000);
        assert!(ut.allocate(1, 0).is_err());
    }

    #[test]
    fn split() {
        // Splitting produces two children of half the size.
        let mut ut = UntypedCapa::new(0x4000, 14); // 16 KiB at 0x4000
        let (left, right) = ut.split().unwrap();
        assert_eq!(left.address, 0x4000);
        assert_eq!(right.address, 0x6000);
        assert_eq!(left.size, 13);
        assert_eq!(right.size, 13);

        // Cannot split twice.
        assert!(ut.split().is_err());

        // Cannot split after allocating.
        let mut ut = UntypedCapa::new(0x4000, 14);
        ut.allocate(64, 0).unwrap();
        assert!(ut.split().is_err());

        // Cannot split at minimum size (FRAME_SIZE_EXPONENT).
        let mut ut = UntypedCapa::new(0x1000, FRAME_SIZE_EXPONENT);
        assert!(ut.split().is_err());

        // Children can be split recursively.
        let mut ut = UntypedCapa::new(0x4000, 14);
        let (mut left, _right) = ut.split().unwrap();
        let (ll, lr) = left.split().unwrap();
        assert_eq!(ll.address, 0x4000);
        assert_eq!(lr.address, 0x5000);
        assert_eq!(ll.size, 12);
        assert_eq!(lr.size, 12);

        // Children can be allocated from.
        let mut ut = UntypedCapa::new(0x4000, 14);
        let (mut left, _right) = ut.split().unwrap();
        let addr = left.allocate(128, 0).unwrap();
        assert_eq!(addr, 0x4000);
    }
}
