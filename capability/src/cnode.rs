//! CNode (Capability Node) Capability

use core::ptr;

use crate::{Capa, CapaError, CapaIdx};

// r[cnode.structure]
// r[cnode.invariant.unique-owner]
/// Capability Node Capability.
#[derive(Debug)]
pub struct CNodeCapa {
    /// Number of slots as a power of two: this CNode holds `2^slots` capability slots.
    slots: u8,
    /// Guard value matched when entering this CNode during CSpace resolution.
    ///
    /// Must be non-zero: a zero guard makes slot 0 of this CNode unreachable.
    guard: usize,
    /// Number of bits in the guard.
    guard_size: u8,
    /// Backing array of `2^slots` [Capa] values.
    ///
    /// CNode capabilities cannot be copied; this CNodeCapa uniquely owns the backing array.
    address: ptr::NonNull<Capa>,
}

impl CNodeCapa {
    /// Creates a new CNode capability.
    ///
    /// # Safety
    ///
    /// `address` must point to a valid allocation of at least `2^slots` [Capa] values, all
    /// initialized to [Capa::Null]. The allocation must remain valid for the lifetime of this
    /// capability and will not be freed by [CNodeCapa] itself — it is freed at revocation time
    /// by the caller.
    pub unsafe fn new(
        address: ptr::NonNull<Capa>,
        slots: u8,
        guard: usize,
        guard_size: u8,
    ) -> Self {
        assert!(address.is_aligned());
        assert!(slots >= 1, "slots must be at least 1");
        assert!(
            (slots as u32) < usize::BITS,
            "slots must be less than pointer width"
        );
        // r[cnode.invariant.guard-nonzero]
        assert!(
            guard_size >= 1,
            "guard_size must be at least 1 (r[cnode.invariant.guard-nonzero])"
        );
        assert!(
            (guard_size as u32) < usize::BITS,
            "guard_size must be less than pointer width"
        );
        assert!(
            (guard_size as u32) + (slots as u32) <= usize::BITS,
            "guard_size + slots must not exceed pointer width"
        );
        assert!(
            guard != 0,
            "guard must be non-zero (r[cnode.invariant.guard-nonzero])"
        );
        // r[cnode.invariant.guard-fits]
        assert!(
            guard < (1usize << guard_size),
            "guard must fit within guard_size bits (r[cnode.invariant.guard-fits])"
        );

        Self {
            slots,
            guard,
            guard_size,
            address,
        }
    }

    // r[cnode.get]
    // r[cnode.bounds]
    /// Returns a shared reference to the capability at `index`.
    ///
    /// The reference lifetime is tied to the shared borrow of this `CNodeCapa`.
    pub fn get(&self, index: usize) -> Result<&Capa, CapaError> {
        self.bound_check(index)?;
        // SAFETY: index is within bounds (verified above); the backing array is valid and
        // uniquely owned for the lifetime of this CNodeCapa.
        unsafe { Ok(self.address.add(index).as_ref()) }
    }

    // r[cnode.get_mut]
    /// Returns an exclusive mutable reference to the capability at `index`.
    ///
    /// The reference lifetime is tied to the exclusive borrow of this `CNodeCapa`.
    pub fn get_mut(&mut self, index: usize) -> Result<&mut Capa, CapaError> {
        self.bound_check(index)?;
        // SAFETY: index is within bounds (verified above); the exclusive borrow of self ensures
        // no other references to this slot exist.
        unsafe { Ok(self.address.add(index).as_mut()) }
    }

    // r[cnode.insert]
    /// Inserts a capability into the first free (`Null`) slot, returning the slot index.
    ///
    /// Returns `CspaceOutOfSpace` if no free slot exists.
    pub fn insert(&mut self, capa: Capa) -> Result<usize, CapaError> {
        for i in 0..self.nb_slots() {
            let slot = self.get_mut(i)?;
            if matches!(slot, Capa::Null) {
                *slot = capa;
                return Ok(i);
            }
        }
        Err(CapaError::CNodeOutOfSpace)
    }

    // r[cspace.resolve]
    // r[cspace.resolve.uniqueness]
    /// Resolves a `CapaIdx` to a shared reference to a capability slot.
    pub fn resolve(&self, idx: CapaIdx) -> Result<&Capa, CapaError> {
        self.resolve_inner(idx.0)
    }

    /// Resolves a `CapaIdx` to an exclusive mutable reference to a capability slot.
    pub fn resolve_mut(&mut self, idx: CapaIdx) -> Result<&mut Capa, CapaError> {
        self.resolve_mut_inner(idx.0)
    }

    /// Decodes one level of a `CapaIdx` against this CNode.
    ///
    /// Checks the guard, extracts the slot index, and returns `(slot_idx, remaining)`.
    fn decode(&self, mut idx: usize) -> Result<(usize, usize), CapaError> {
        let guard_shift = usize::BITS as u8 - self.guard_size;
        // r[cspace.resolve.guard]
        if (idx >> guard_shift) != self.guard {
            return Err(CapaError::CNodeGuardMismatch);
        }
        idx <<= self.guard_size;
        // r[cspace.resolve.index]
        let slot_idx = idx >> (usize::BITS as u8 - self.slots);
        idx <<= self.slots;
        Ok((slot_idx, idx))
    }

    fn resolve_inner(&self, idx: usize) -> Result<&Capa, CapaError> {
        let (slot_idx, remaining) = self.decode(idx)?;
        let capa = self.get(slot_idx)?;
        if remaining == 0 {
            return Ok(capa); // r[cspace.resolve.stop]
        }
        match capa {
            Capa::CNode(child, _) => child.resolve_inner(remaining), // r[cspace.resolve.descend]
            _ => Err(CapaError::CNodeInvalidIndex),                  // r[cspace.resolve.error]
        }
    }

    fn resolve_mut_inner(&mut self, idx: usize) -> Result<&mut Capa, CapaError> {
        let (slot_idx, remaining) = self.decode(idx)?;
        let capa = self.get_mut(slot_idx)?;
        if remaining == 0 {
            return Ok(capa);
        }
        match capa {
            Capa::CNode(child, _) => child.resolve_mut_inner(remaining),
            _ => Err(CapaError::CNodeInvalidIndex),
        }
    }

    pub(crate) fn slots(&self) -> u8 {
        self.slots
    }

    pub(crate) fn guard(&self) -> usize {
        self.guard
    }

    pub(crate) fn guard_size(&self) -> u8 {
        self.guard_size
    }

    /// Returns the number of slots.
    const fn nb_slots(&self) -> usize {
        1usize << self.slots
    }

    /// Returns the `CapaIdx` that addresses `slot_idx` in this single-level CNode.
    ///
    /// Bit layout (MSB first): `[guard (guard_size bits)] [slot_idx (slots bits)] [zeros]`
    pub(crate) fn capaidx_for(&self, slot_idx: usize) -> CapaIdx {
        let guard_shift = usize::BITS as u8 - self.guard_size;
        let slot_shift = usize::BITS as u8 - self.guard_size - self.slots;
        CapaIdx((self.guard << guard_shift) | (slot_idx << slot_shift))
    }

    /// Bounds-checks `index`, returning `CNodeInvalidIndex` if out of range.
    const fn bound_check(&self, index: usize) -> Result<(), CapaError> {
        if index < self.nb_slots() {
            Ok(())
        } else {
            Err(CapaError::CNodeInvalidIndex)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CdtNode;

    // ——————————————————————————— Test helpers ————————————————————————————— //

    /// Allocates a leaked backing array and wraps it in a CNodeCapa.
    ///
    /// Uses guard=1 (guard_size=1) and `slots` bits of slot space (2^slots entries).
    fn make_cnode(slots: u8) -> CNodeCapa {
        let n = 1usize << slots;
        let mut v: Vec<Capa> = (0..n).map(|_| Capa::Null).collect();
        let ptr = ptr::NonNull::new(v.as_mut_ptr()).unwrap();
        std::mem::forget(v);
        // Safety: ptr is valid, all slots initialised to Null, memory is intentionally leaked.
        unsafe { CNodeCapa::new(ptr, slots, 1, 1) }
    }

    /// CapaIdx for slot `i` in a single-level CNode with guard=1 (1 bit) and 3-bit slots.
    ///
    /// Bit layout (64 bits, MSB first): `1 | iii | 0…0`
    fn idx1(i: usize) -> CapaIdx {
        CapaIdx((1 << 63) | (i << 60))
    }

    /// CapaIdx for slot `j` in a child CNode reached via slot `i` of the parent.
    ///
    /// Both CNodes use guard=1 (1 bit) and 3-bit slots.
    /// Bit layout: `1 | iii | 1 | jjj | 0…0`
    fn idx2(i: usize, j: usize) -> CapaIdx {
        CapaIdx((1 << 63) | (i << 60) | (1 << 59) | (j << 56))
    }

    // ————————————————————————————————— get ———————————————————————————————— //

    #[test]
    fn get_returns_null_for_empty_slot() {
        let cnode = make_cnode(3);
        assert!(matches!(cnode.get(0).unwrap(), Capa::Null));
        assert!(matches!(cnode.get(7).unwrap(), Capa::Null));
    }

    #[test]
    fn get_out_of_bounds() {
        let cnode = make_cnode(3); // 8 slots
        assert_eq!(cnode.get(8).unwrap_err(), CapaError::CNodeInvalidIndex);
        assert_eq!(
            cnode.get(usize::MAX).unwrap_err(),
            CapaError::CNodeInvalidIndex
        );
    }

    // ———————————————————————————————— insert —————————————————————————————— //

    #[test]
    fn insert_basic() {
        let mut cnode = make_cnode(3);
        let child = make_cnode(1);
        let idx = cnode
            .insert(Capa::CNode(child, CdtNode::unlinked(0)))
            .unwrap();
        assert_eq!(idx, 0);
    }

    #[test]
    fn insert_fills_sequentially() {
        let mut cnode = make_cnode(2); // 4 slots
        for expected in 0..4usize {
            let child = make_cnode(1);
            let idx = cnode
                .insert(Capa::CNode(child, CdtNode::unlinked(0)))
                .unwrap();
            assert_eq!(idx, expected);
        }
        let child = make_cnode(1);
        assert_eq!(
            cnode
                .insert(Capa::CNode(child, CdtNode::unlinked(0)))
                .unwrap_err(),
            CapaError::CNodeOutOfSpace
        );
    }

    #[test]
    fn get_mut_write_through() {
        let mut cnode = make_cnode(3);
        *cnode.get_mut(2).unwrap() = Capa::Null; // write Null → Null (sanity check)
        assert!(matches!(cnode.get(2).unwrap(), Capa::Null));
    }

    // ——————————————————————————————— resolve —————————————————————————————— //

    #[test]
    fn resolve_single_level() {
        let cnode = make_cnode(3); // guard=1 (1 bit), 8 slots
        // idx1(5): guard=1, index=5 → slot 5, remaining=0 → stop
        let capa = cnode.resolve(idx1(5)).unwrap();
        assert!(matches!(capa, Capa::Null));
    }

    #[test]
    fn resolve_guard_mismatch() {
        let cnode = make_cnode(3); // guard=1
        // Flip the guard bit: use 0 instead of 1 in the top bit.
        let bad_idx = CapaIdx(0 << 63); // guard bit = 0, mismatch
        assert_eq!(
            cnode.resolve(bad_idx).unwrap_err(),
            CapaError::CNodeGuardMismatch
        );
    }

    #[test]
    fn resolve_non_cnode_mid_walk() {
        let cnode = make_cnode(3);
        // idx2(5, 2): expects slot 5 to be a CNode for descent, but it's Null.
        assert_eq!(
            cnode.resolve(idx2(5, 2)).unwrap_err(),
            CapaError::CNodeInvalidIndex
        );
    }

    #[test]
    fn resolve_two_levels() {
        let mut parent = make_cnode(3); // guard=1 (1 bit), 8 slots
        let child = make_cnode(3); // guard=1 (1 bit), 8 slots

        // Place child CNode at parent slot 0.
        *parent.get_mut(0).unwrap() = Capa::CNode(child, CdtNode::unlinked(0));

        // idx2(0, 5): parent[0] → child[5], remaining=0 → stop at child[5] (Null).
        let capa = parent.resolve(idx2(0, 5)).unwrap();
        assert!(matches!(capa, Capa::Null));

        // idx1(0): parent[0] → stop here (the CNode capability itself).
        let capa = parent.resolve(idx1(0)).unwrap();
        assert!(matches!(capa, Capa::CNode(_, _)));
    }

    #[test]
    fn resolve_mut_two_levels() {
        let mut parent = make_cnode(3);
        let child = make_cnode(3);
        *parent.get_mut(0).unwrap() = Capa::CNode(child, CdtNode::unlinked(0));

        // Write through the mutable resolve to child[3].
        *parent.resolve_mut(idx2(0, 3)).unwrap() = Capa::Null;

        // Confirm the write landed at child[3].
        let capa = parent.resolve(idx2(0, 3)).unwrap();
        assert!(matches!(capa, Capa::Null));
    }
}
