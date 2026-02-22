//! CSpace (Capability Space) Capability

use crate::{Capa, CapaError};
use core::ptr;

/// Capability Space Capability.
pub struct CSpaceCapa {
    /// Number of slots, as a power of two.
    slots: u8,
    /// Start address of the CSpace object.
    ///
    /// CSpace capabilities can not be copied, therefore they uniquely own the underlying CSpace
    /// object.
    address: ptr::NonNull<Capa>,
}

impl CSpaceCapa {
    /// Create a new CSpace capability.
    ///
    /// # SAFETY:
    ///
    /// The address should point to a valid allocation capable of holding at least 2 ^ slots
    /// [Capa].
    pub unsafe fn new(address: ptr::NonNull<Capa>, slots: u8) -> Self {
        // Safety checks, so we can assume the address is valid in other methods.
        // We also limit the maximum size of a CSpace to prevent overflows in arithmetic
        // operations.
        assert!(address.is_aligned());
        assert!((slots as u32) < usize::BITS - 2);

        Self { slots, address }
    }

    /// Insert a capability in the current CSpace, returning the corresponding index.
    ///
    /// This operation performs a linear scan and selects the first free slot.
    pub fn insert(&mut self, capa: Capa) -> Result<(), CapaError> {
        for i in 0..self.nb_slots() {
            if let Ok(Capa::Null) = self.get(i) {
                // We found a free slot, let's insert the capa here.
                self.set(i, capa)?;
                return Ok(());
            }
        }

        // We could not find a free slot with a scan
        Err(CapaError::CspaceOutOfSpace)
    }

    /// Get a capability by its index within a CSpace.
    pub fn get(&self, index: usize) -> Result<Capa, CapaError> {
        // We perform a bound check first.
        self.bound_check(index)?;

        // TODO: figure stafety story --- we need to decide what the revocation policies is first
        //
        // In a nutshell, we need to ensure that the CSpace has been properly allocated and
        // initialized, and that it has not been revoked yet.
        let capa = unsafe { self.address.add(index).read() };
        Ok(capa)
    }

    /// Set a capability by its index within a CSpace.
    pub fn set(&mut self, index: usize, capa: Capa) -> Result<(), CapaError> {
        // We perform a bound check first.
        self.bound_check(index)?;

        // TODO: figure stafety story --- we need to decide what the revocation policies is first
        //
        // In a nutshell, we need to ensure that the CSpace has been properly allocated and
        // initialized, and that it has not been revoked yet.
        unsafe { self.address.add(index).write(capa) };
        Ok(())
    }

    /// Returns the number of slots in this cspace.
    const fn nb_slots(&self) -> usize {
        1usize << self.slots
    }

    /// Checks that the index is valid for this CSpace, and raises an invalid index error
    /// otherwise.
    const fn bound_check(&self, index: usize) -> Result<(), CapaError> {
        if index < self.nb_slots() {
            Ok(())
        } else {
            Err(CapaError::CSpaceInvalidIndex)
        }
    }
}
