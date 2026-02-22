//! Untyped Memory Capability

use crate::CapaError;

/// The derivation kind of an untyped capability.
#[derive(Debug, PartialEq, Eq)]
pub enum UntypedKind {
    Aliased,
    Carved,
}

/// Untyped Memory Capability.
#[derive(Debug)]
pub struct UntypedCapa {
    /// Start address of the untyped memory region (inclusive).
    start: usize,
    /// End address of the untyped memory region (exclusive).
    end: usize,
    /// Bump allocator watermark, relative to `start`.
    ///
    /// A non-zero watermark indicates the capability is in allocation mode, which is mutually
    /// exclusive with alias/carve.
    watermark: usize,
    /// How this capability was derived.
    kind: UntypedKind,
}

impl UntypedCapa {
    /// Creates a new untyped memory capability covering `[start, end)`.
    pub fn new(start: usize, end: usize, kind: UntypedKind) -> Self {
        assert!(start < end, "start must be less than end");
        Self {
            start,
            end,
            watermark: 0,
            kind,
        }
    }

    /// Allocates memory for use by L4sm objects.
    ///
    /// Size is in bytes, alignment is a power of two (as an exponent).
    ///
    /// The returned start address is naturally aligned to `2 ^ alignment`.
    ///
    /// Note: `r[untyped.allocate.mode]` — the mutual exclusion between allocation mode and
    /// alias/carve is enforced at a higher level (CDT child presence). This method only checks
    /// the watermark-level invariant.
    pub fn allocate(&mut self, size: usize, alignment: u8) -> Result<usize, CapaError> {
        assert!(alignment < 64, "alignment is too large");

        // Align the start of the allocation to satisfy the constraint.
        let alignment = 1usize << alignment;
        let alloc_start = (self.start + self.watermark + alignment - 1) & !(alignment - 1);

        // Check that we don't run out of space.
        if alloc_start + size > self.end {
            return Err(CapaError::UntypedOutOfSpace);
        }

        self.watermark = (alloc_start + size) - self.start;
        Ok(alloc_start)
    }

    /// Derives a new aliased child capability covering `[start, end)`.
    ///
    /// Aliased children may overlap with other aliased children, but not with carved children.
    ///
    /// - `r[untyped.alias.mode]`: rejected if `watermark > 0` (allocation mode active).
    /// - `r[untyped.alias.bounds]`: `[start, end)` must be within `[self.start, self.end)`.
    /// - `r[untyped.alias.no-overlap-carved]`: rejected if any carved child overlaps `[start, end)`.
    pub fn alias<'a>(
        &mut self,
        start: usize,
        end: usize,
        children: impl Iterator<Item = &'a UntypedCapa>,
    ) -> Result<UntypedCapa, CapaError> {
        // r[untyped.alias.mode]
        if self.watermark > 0 {
            return Err(CapaError::UntypedWrongMode);
        }
        // r[untyped.alias.bounds]
        if start < self.start || end > self.end || start >= end {
            return Err(CapaError::UntypedOutOfBounds);
        }
        // r[untyped.alias.no-overlap-carved]
        for child in children {
            if child.kind == UntypedKind::Carved && Self::overlaps(start, end, child.start, child.end) {
                return Err(CapaError::UntypedOverlap);
            }
        }
        Ok(UntypedCapa {
            start,
            end,
            watermark: 0,
            kind: UntypedKind::Aliased,
        })
    }

    /// Derives a new carved (exclusive) child capability covering `[start, end)`.
    ///
    /// Carved children may not overlap with any other child, aliased or carved.
    ///
    /// - `r[untyped.carve.mode]`: rejected if `watermark > 0` (allocation mode active).
    /// - `r[untyped.carve.bounds]`: `[start, end)` must be within `[self.start, self.end)`.
    /// - `r[untyped.carve.no-overlap]`: rejected if any existing child overlaps `[start, end)`.
    pub fn carve<'a>(
        &mut self,
        start: usize,
        end: usize,
        children: impl Iterator<Item = &'a UntypedCapa>,
    ) -> Result<UntypedCapa, CapaError> {
        // r[untyped.carve.mode]
        if self.watermark > 0 {
            return Err(CapaError::UntypedWrongMode);
        }
        // r[untyped.carve.bounds]
        if start < self.start || end > self.end || start >= end {
            return Err(CapaError::UntypedOutOfBounds);
        }
        // r[untyped.carve.no-overlap]
        for child in children {
            if Self::overlaps(start, end, child.start, child.end) {
                return Err(CapaError::UntypedOverlap);
            }
        }
        Ok(UntypedCapa {
            start,
            end,
            watermark: 0,
            kind: UntypedKind::Carved,
        })
    }

    /// Returns true if the two ranges `[a_start, a_end)` and `[b_start, b_end)` overlap.
    fn overlaps(a_start: usize, a_end: usize, b_start: usize, b_end: usize) -> bool {
        a_start < b_end && b_start < a_end
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocate() {
        // Simple allocation.
        let mut ut = UntypedCapa::new(0x1000, 0x2000, UntypedKind::Carved); // 4 KiB region
        let addr = ut.allocate(64, 0).unwrap();
        assert_eq!(addr, 0x1000);
        assert_eq!(ut.watermark, 64);

        // Aligned allocation: 256-byte alignment bumps the watermark.
        let mut ut = UntypedCapa::new(0x1000, 0x2000, UntypedKind::Carved);
        ut.allocate(8, 0).unwrap();
        let addr = ut.allocate(64, 8).unwrap();
        assert_eq!(addr, 0x1100);

        // Multiple sequential allocations.
        let mut ut = UntypedCapa::new(0x1000, 0x2000, UntypedKind::Carved); // 4 KiB
        let a = ut.allocate(128, 0).unwrap();
        let b = ut.allocate(256, 0).unwrap();
        assert_eq!(a, 0x1000);
        assert_eq!(b, 0x1080);

        // Out of memory.
        let mut ut = UntypedCapa::new(0x1000, 0x2000, UntypedKind::Carved); // 4 KiB
        ut.allocate(2048, 0).unwrap();
        assert!(ut.allocate(4096, 0).is_err());

        // Out of memory due to alignment padding.
        let mut ut = UntypedCapa::new(0x1000, 0x2000, UntypedKind::Carved); // 4 KiB
        ut.allocate(1, 0).unwrap();
        assert!(ut.allocate(2048, 12).is_err());

        // Exact fit.
        let mut ut = UntypedCapa::new(0x1000, 0x2000, UntypedKind::Carved); // 4 KiB
        let addr = ut.allocate(4096, 0).unwrap();
        assert_eq!(addr, 0x1000);
        assert!(ut.allocate(1, 0).is_err());
    }

    #[test]
    fn alias_basic() {
        let mut parent = UntypedCapa::new(0x1000, 0x5000, UntypedKind::Carved);
        let child = parent.alias(0x1000, 0x3000, std::iter::empty()).unwrap();
        assert_eq!(child.start, 0x1000);
        assert_eq!(child.end, 0x3000);
        assert_eq!(child.kind, UntypedKind::Aliased);
    }

    #[test]
    fn alias_out_of_bounds() {
        let mut parent = UntypedCapa::new(0x1000, 0x5000, UntypedKind::Carved);
        // start before parent
        assert_eq!(
            parent.alias(0x0000, 0x2000, std::iter::empty()).unwrap_err(),
            CapaError::UntypedOutOfBounds
        );
        // end after parent
        assert_eq!(
            parent.alias(0x1000, 0x6000, std::iter::empty()).unwrap_err(),
            CapaError::UntypedOutOfBounds
        );
        // start >= end
        assert_eq!(
            parent.alias(0x3000, 0x2000, std::iter::empty()).unwrap_err(),
            CapaError::UntypedOutOfBounds
        );
    }

    #[test]
    fn alias_overlap_carved_rejected() {
        let mut parent = UntypedCapa::new(0x1000, 0x5000, UntypedKind::Carved);
        let carved = UntypedCapa::new(0x2000, 0x3000, UntypedKind::Carved);
        let children = [carved];
        assert_eq!(
            parent.alias(0x2000, 0x4000, children.iter()).unwrap_err(),
            CapaError::UntypedOverlap
        );
    }

    #[test]
    fn alias_overlap_aliased_allowed() {
        let mut parent = UntypedCapa::new(0x1000, 0x5000, UntypedKind::Carved);
        let existing = UntypedCapa::new(0x2000, 0x3000, UntypedKind::Aliased);
        let children = [existing];
        // Aliased children may overlap with other aliased children.
        let child = parent.alias(0x2000, 0x4000, children.iter()).unwrap();
        assert_eq!(child.kind, UntypedKind::Aliased);
    }

    #[test]
    fn alias_wrong_mode() {
        let mut parent = UntypedCapa::new(0x1000, 0x5000, UntypedKind::Carved);
        parent.allocate(64, 0).unwrap();
        assert_eq!(
            parent.alias(0x1000, 0x3000, std::iter::empty()).unwrap_err(),
            CapaError::UntypedWrongMode
        );
    }

    #[test]
    fn carve_basic() {
        let mut parent = UntypedCapa::new(0x1000, 0x5000, UntypedKind::Carved);
        let child = parent.carve(0x1000, 0x3000, std::iter::empty()).unwrap();
        assert_eq!(child.start, 0x1000);
        assert_eq!(child.end, 0x3000);
        assert_eq!(child.kind, UntypedKind::Carved);
    }

    #[test]
    fn carve_out_of_bounds() {
        let mut parent = UntypedCapa::new(0x1000, 0x5000, UntypedKind::Carved);
        assert_eq!(
            parent.carve(0x0000, 0x2000, std::iter::empty()).unwrap_err(),
            CapaError::UntypedOutOfBounds
        );
        assert_eq!(
            parent.carve(0x1000, 0x6000, std::iter::empty()).unwrap_err(),
            CapaError::UntypedOutOfBounds
        );
    }

    #[test]
    fn carve_overlap_carved_rejected() {
        let mut parent = UntypedCapa::new(0x1000, 0x5000, UntypedKind::Carved);
        let existing = UntypedCapa::new(0x2000, 0x3000, UntypedKind::Carved);
        let children = [existing];
        assert_eq!(
            parent.carve(0x2000, 0x4000, children.iter()).unwrap_err(),
            CapaError::UntypedOverlap
        );
    }

    #[test]
    fn carve_overlap_aliased_rejected() {
        let mut parent = UntypedCapa::new(0x1000, 0x5000, UntypedKind::Carved);
        let existing = UntypedCapa::new(0x2000, 0x3000, UntypedKind::Aliased);
        let children = [existing];
        assert_eq!(
            parent.carve(0x2000, 0x4000, children.iter()).unwrap_err(),
            CapaError::UntypedOverlap
        );
    }

    #[test]
    fn carve_non_overlapping() {
        let mut parent = UntypedCapa::new(0x1000, 0x5000, UntypedKind::Carved);
        let first = parent.carve(0x1000, 0x2000, std::iter::empty()).unwrap();
        let children = [first];
        let second = parent.carve(0x2000, 0x3000, children.iter()).unwrap();
        assert_eq!(second.start, 0x2000);
        assert_eq!(second.end, 0x3000);
    }

    #[test]
    fn carve_wrong_mode() {
        let mut parent = UntypedCapa::new(0x1000, 0x5000, UntypedKind::Carved);
        parent.allocate(64, 0).unwrap();
        assert_eq!(
            parent.carve(0x1000, 0x3000, std::iter::empty()).unwrap_err(),
            CapaError::UntypedWrongMode
        );
    }
}
