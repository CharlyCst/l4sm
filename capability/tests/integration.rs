//! Integration tests for the capability library.
//!
//! These tests exercise the public API (`carve`, `alias`, `revoke`, `install`, `lookup`, …)
//! as a kernel consumer would, using only the types and functions exported from `lib.rs`.

use capability::{
    alias, carve, install, lookup, new_root_cnode, new_root_untyped, revoke, Capa, CapaError,
    CapaIdx,
};
use core::ptr::NonNull;

// —————————————————————————————— Test helpers —————————————————————————————— //

/// Allocates a heap-stable backing store for a root CNode (8 slots, guard=1/1-bit, 3-bit index)
/// and returns a `Box<Capa>` containing the root capability plus the backing `Vec`.
///
/// The `Vec` must be kept alive for the duration of the test — it is the CNode's slot storage.
/// The `Box` must not be moved after `root_nn()` has been called on it.
unsafe fn make_cspace() -> (Box<Capa>, Vec<Capa>) {
    let mut slots: Vec<Capa> = (0..8).map(|_| Capa::Null).collect();
    let addr = NonNull::new(slots.as_mut_ptr()).unwrap();
    // Safety: addr valid, Null-initialised, and we keep both alive together.
    let root_capa = unsafe { new_root_cnode(addr, 3, 1, 1) };
    (Box::new(root_capa), slots)
}

/// Returns a `NonNull<Capa>` pointing to the heap-allocated `Box`.
fn root_nn(b: &mut Box<Capa>) -> NonNull<Capa> {
    NonNull::from(b.as_mut())
}

/// Returns the `CapaIdx` for `slot` in the test CNode.
///
/// The test CNode is always created with `guard=1` (1 bit) and `slots=3` bits, so the bit
/// layout of a `CapaIdx` is `[1][sss][000…0]` where `sss` is the 3-bit slot index.
fn idx(slot: usize) -> CapaIdx {
    CapaIdx((1_usize << 63) | (slot << 60))
}

// ——————————————————————————— carve integration ———————————————————————————— //

#[test]
fn carve_creates_untyped_child() {
    let (mut root_box, _slots) = unsafe { make_cspace() };
    let root = root_nn(&mut root_box);

    let src = unsafe { install(root, new_root_untyped(0x1000, 0x5000)) }.unwrap();
    let dst = idx(1);

    unsafe { carve(root, src, dst, 0x1000, 0x2000) }.unwrap();

    let child = unsafe { lookup(root, dst) }.unwrap();
    assert!(matches!(child, Capa::Untyped(_, _)));
}

#[test]
fn carve_dst_must_be_null() {
    let (mut root_box, _slots) = unsafe { make_cspace() };
    let root = root_nn(&mut root_box);

    let src = unsafe { install(root, new_root_untyped(0x1000, 0x5000)) }.unwrap();
    // Put an Untyped in dst slot to make it non-Null.
    unsafe { install(root, new_root_untyped(0x8000, 0x9000)) }.unwrap(); // slot 1
    let dst = idx(1); // now occupied

    let err = unsafe { carve(root, src, dst, 0x1000, 0x2000) }.unwrap_err();
    assert_eq!(err, CapaError::CNodeSlotOccupied);
}

#[test]
fn carve_src_must_be_untyped() {
    let (mut root_box, _slots) = unsafe { make_cspace() };
    let root = root_nn(&mut root_box);

    let src = idx(0); // slot 0 is Null
    let dst = idx(1);

    let err = unsafe { carve(root, src, dst, 0x1000, 0x2000) }.unwrap_err();
    assert_eq!(err, CapaError::InvalidCapaType);
}

#[test]
fn carve_out_of_bounds_rejected() {
    let (mut root_box, _slots) = unsafe { make_cspace() };
    let root = root_nn(&mut root_box);

    let src = unsafe { install(root, new_root_untyped(0x1000, 0x5000)) }.unwrap();
    let dst = idx(1);

    // Range starts before parent.
    let err = unsafe { carve(root, src, dst, 0x0000, 0x2000) }.unwrap_err();
    assert_eq!(err, CapaError::UntypedOutOfBounds);

    // Range extends past parent.
    let err = unsafe { carve(root, src, dst, 0x4000, 0x6000) }.unwrap_err();
    assert_eq!(err, CapaError::UntypedOutOfBounds);
}

#[test]
fn carve_overlap_rejected() {
    let (mut root_box, _slots) = unsafe { make_cspace() };
    let root = root_nn(&mut root_box);

    let src = unsafe { install(root, new_root_untyped(0x1000, 0x5000)) }.unwrap();
    let dst1 = idx(1);
    let dst2 = idx(2);

    unsafe { carve(root, src, dst1, 0x1000, 0x3000) }.unwrap();

    // Second carve overlaps the first child.
    let err = unsafe { carve(root, src, dst2, 0x2000, 0x4000) }.unwrap_err();
    assert_eq!(err, CapaError::UntypedOverlap);
}

#[test]
fn carve_non_overlapping_siblings() {
    let (mut root_box, _slots) = unsafe { make_cspace() };
    let root = root_nn(&mut root_box);

    let src = unsafe { install(root, new_root_untyped(0x1000, 0x5000)) }.unwrap();
    let dst1 = idx(1);
    let dst2 = idx(2);

    unsafe { carve(root, src, dst1, 0x1000, 0x2000) }.unwrap();
    unsafe { carve(root, src, dst2, 0x2000, 0x3000) }.unwrap();

    assert!(matches!(
        unsafe { lookup(root, dst1) }.unwrap(),
        Capa::Untyped(_, _)
    ));
    assert!(matches!(
        unsafe { lookup(root, dst2) }.unwrap(),
        Capa::Untyped(_, _)
    ));
}

// ——————————————————————————— alias integration ———————————————————————————— //

#[test]
fn alias_creates_untyped_child() {
    let (mut root_box, _slots) = unsafe { make_cspace() };
    let root = root_nn(&mut root_box);

    let src = unsafe { install(root, new_root_untyped(0x1000, 0x5000)) }.unwrap();
    let dst = idx(1);

    unsafe { alias(root, src, dst, 0x1000, 0x3000) }.unwrap();

    let child = unsafe { lookup(root, dst) }.unwrap();
    assert!(matches!(child, Capa::Untyped(_, _)));
}

#[test]
fn alias_allows_overlap_with_alias() {
    let (mut root_box, _slots) = unsafe { make_cspace() };
    let root = root_nn(&mut root_box);

    let src = unsafe { install(root, new_root_untyped(0x1000, 0x5000)) }.unwrap();
    let dst1 = idx(1);
    let dst2 = idx(2);

    unsafe { alias(root, src, dst1, 0x1000, 0x3000) }.unwrap();
    // Same range aliased again is allowed (aliased siblings may overlap).
    unsafe { alias(root, src, dst2, 0x1000, 0x3000) }.unwrap();

    assert!(matches!(
        unsafe { lookup(root, dst2) }.unwrap(),
        Capa::Untyped(_, _)
    ));
}

#[test]
fn alias_rejects_overlap_with_carved() {
    let (mut root_box, _slots) = unsafe { make_cspace() };
    let root = root_nn(&mut root_box);

    let src = unsafe { install(root, new_root_untyped(0x1000, 0x5000)) }.unwrap();
    let dst1 = idx(1);
    let dst2 = idx(2);

    unsafe { carve(root, src, dst1, 0x2000, 0x4000) }.unwrap();

    let err = unsafe { alias(root, src, dst2, 0x1000, 0x3000) }.unwrap_err();
    assert_eq!(err, CapaError::UntypedOverlap);
}

// ——————————————————————————— revoke integration ——————————————————————————— //

#[test]
fn revoke_deletes_direct_children() {
    let (mut root_box, _slots) = unsafe { make_cspace() };
    let root = root_nn(&mut root_box);

    let src = unsafe { install(root, new_root_untyped(0x1000, 0x5000)) }.unwrap();
    let dst1 = idx(1);
    let dst2 = idx(2);

    unsafe { carve(root, src, dst1, 0x1000, 0x2000) }.unwrap();
    unsafe { carve(root, src, dst2, 0x2000, 0x3000) }.unwrap();

    // Revoking the parent removes all descendants.
    unsafe { revoke(root, src) }.unwrap();

    assert!(matches!(
        unsafe { lookup(root, dst1) }.unwrap(),
        Capa::Null
    ));
    assert!(matches!(
        unsafe { lookup(root, dst2) }.unwrap(),
        Capa::Null
    ));
    // Parent itself is kept.
    assert!(matches!(
        unsafe { lookup(root, src) }.unwrap(),
        Capa::Untyped(_, _)
    ));
}

#[test]
fn revoke_deletes_grandchildren() {
    let (mut root_box, _slots) = unsafe { make_cspace() };
    let root = root_nn(&mut root_box);

    let src = unsafe { install(root, new_root_untyped(0x1000, 0x5000)) }.unwrap();
    let dst1 = idx(1);
    let dst2 = idx(2);

    // Carve a child, then carve a grandchild from it.
    unsafe { carve(root, src, dst1, 0x1000, 0x3000) }.unwrap();
    unsafe { carve(root, dst1, dst2, 0x1000, 0x2000) }.unwrap();

    // Revoking the grandparent must wipe both dst1 and dst2.
    unsafe { revoke(root, src) }.unwrap();

    assert!(matches!(
        unsafe { lookup(root, dst1) }.unwrap(),
        Capa::Null
    ));
    assert!(matches!(
        unsafe { lookup(root, dst2) }.unwrap(),
        Capa::Null
    ));
}

#[test]
fn revoke_resets_watermark_allows_recarve() {
    let (mut root_box, _slots) = unsafe { make_cspace() };
    let root = root_nn(&mut root_box);

    let src = unsafe { install(root, new_root_untyped(0x1000, 0x5000)) }.unwrap();
    let dst = idx(1);

    unsafe { carve(root, src, dst, 0x1000, 0x2000) }.unwrap();

    // Revoking the parent resets its watermark and deletes the child.
    unsafe { revoke(root, src) }.unwrap();
    assert!(matches!(
        unsafe { lookup(root, dst) }.unwrap(),
        Capa::Null
    ));

    // The same range can now be carved again.
    unsafe { carve(root, src, dst, 0x1000, 0x2000) }.unwrap();
    assert!(matches!(
        unsafe { lookup(root, dst) }.unwrap(),
        Capa::Untyped(_, _)
    ));
}

#[test]
fn revoke_stops_at_sibling() {
    let (mut root_box, _slots) = unsafe { make_cspace() };
    let root = root_nn(&mut root_box);

    let src = unsafe { install(root, new_root_untyped(0x1000, 0x5000)) }.unwrap();
    let dst1 = idx(1);
    let dst2 = idx(2);

    // Two siblings at depth 1.
    unsafe { carve(root, src, dst1, 0x1000, 0x2000) }.unwrap();
    unsafe { carve(root, src, dst2, 0x2000, 0x3000) }.unwrap();

    // Revoking dst1 must not touch the sibling dst2.
    unsafe { revoke(root, dst1) }.unwrap();

    // dst1 is kept (revoke removes descendants, not the target itself).
    assert!(matches!(
        unsafe { lookup(root, dst1) }.unwrap(),
        Capa::Untyped(_, _)
    ));
    // dst2 (sibling) is unaffected.
    assert!(matches!(
        unsafe { lookup(root, dst2) }.unwrap(),
        Capa::Untyped(_, _)
    ));
}

#[test]
fn revoke_parent_after_revoking_middle_child() {
    // Regression: unlinking a middle child must leave the sibling list intact so that a
    // subsequent revoke of the grandparent still walks all remaining children.
    let (mut root_box, _slots) = unsafe { make_cspace() };
    let root = root_nn(&mut root_box);

    let src = unsafe { install(root, new_root_untyped(0x1000, 0x5000)) }.unwrap();
    let dst1 = idx(1);
    let dst2 = idx(2);
    let dst3 = idx(3);

    unsafe { carve(root, src, dst1, 0x1000, 0x2000) }.unwrap();
    unsafe { carve(root, src, dst2, 0x2000, 0x3000) }.unwrap();
    unsafe { carve(root, src, dst3, 0x3000, 0x4000) }.unwrap();

    // Revoke the middle sibling (no children, so this is a no-op for the subtree,
    // but exercises the CDT list at depth 1).
    unsafe { revoke(root, dst2) }.unwrap();

    // Now revoking src must walk and delete dst1, dst2, and dst3.
    unsafe { revoke(root, src) }.unwrap();

    assert!(matches!(
        unsafe { lookup(root, dst1) }.unwrap(),
        Capa::Null
    ));
    assert!(matches!(
        unsafe { lookup(root, dst2) }.unwrap(),
        Capa::Null
    ));
    assert!(matches!(
        unsafe { lookup(root, dst3) }.unwrap(),
        Capa::Null
    ));
}
