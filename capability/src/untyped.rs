//! Untyped Memory Capability

use crate::{FRAME_SIZE_EXPONENT, CapaError};

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
    /// The returned start address is naturally aligned to 2 ^ alignment. Size is in bytes.
    pub fn allocate(&mut self, size: usize, alignment: u8) -> Result<usize, CapaError> {
        assert!(alignment < 64, "alignment is too large");

        // We can not allocate if the capability has been split.
        if self.is_split {
            return Err(CapaError::UntypedAlreadySplit);
        }

        // Align the start of the allocation to satisfy the constaint.
        let alignment = 1usize << alignment;
        let alloc_start = (self.address + self.watermark + alignment - 1) & !(alignment - 1);

        // Check that we don't run out of space.
        if alloc_start + size > self.end() {
            return Err(CapaError::UntypedOutOfSpace);
        }

        self.watermark = (alloc_start + size) - self.address;
        Ok(alloc_start)
    }

    /// Split an untyped memory capability in two new capabilities that can be used independently.
    ///
    /// An untyped memory capability can be split only once, and can not be split once objects have
    /// been allocated.
    pub fn split(&mut self) -> Result<(UntypedCapa, UntypedCapa), CapaError> {
        // We can not split a capability twice.
        if self.is_split {
            Err(CapaError::UntypedAlreadySplit)
        }
        // We can not split a capability with already allocated objects.
        else if self.watermark != 0 {
            Err(CapaError::UntypedAlreadyInUse)
        }
        // We can not split the capability any further.
        else if self.size <= FRAME_SIZE_EXPONENT {
            Err(CapaError::UntypedCantSplitFurther)
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

    /// Returns the end of the memory range (exclusive).
    fn end(&self) -> usize {
        self.address + (1usize << self.size)
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
