//! Work-In-Progress: Capabilities for L4sm
//!
//! L4sm is inspired by seL4's design.

use thiserror::Error;

mod cnode;
mod untyped;

use cnode::CNodeCapa;
use untyped::UntypedCapa;

// ————————————————————————————————— Errors ————————————————————————————————— //

/// Capability operation error.
#[derive(Error, Debug, PartialEq, Eq)]
pub enum CapaError {
    // CNode
    // r[cspace.error.index]
    #[error("invalid cnode index")]
    CNodeInvalidIndex,
    // r[cspace.error.guard]
    #[error("guard bits do not match during CSpace resolution")]
    CNodeGuardMismatch,
    #[error("cnode is full")]
    CNodeOutOfSpace,
    #[error("destination slot is already occupied")]
    CNodeSlotOccupied,

    // Untyped Memory
    #[error("untyped memory does not have enough free space")]
    UntypedOutOfSpace,
    #[error("proposed range is not within the parent's range")]
    UntypedOutOfBounds,
    #[error("proposed range overlaps a conflicting sibling")]
    UntypedOverlap,
    #[error("operation rejected due to implicit mode (watermark > 0)")]
    UntypedWrongMode,

    // Misc
    #[error("capability has the wrong type for this operation")]
    InvalidCapaType,
}

// —————————————————————————————— Capabilities —————————————————————————————— //

// r[cspace.capaidx]
/// A capability index, represents an address in capability space (CSpace).
#[repr(transparent)]
pub struct CapaIdx(usize);

// r[cdt.structure.node]
/// Capability Derivation Tree Node
#[derive(Debug)]
pub struct CdtNode {
    pub(crate) prev: *mut Capa,
    pub(crate) next: *mut Capa,
}

impl CdtNode {
    /// Creates a new CDT node with null pointers, not yet linked into the tree.
    pub(crate) fn unlinked() -> Self {
        Self {
            prev: core::ptr::null_mut(),
            next: core::ptr::null_mut(),
        }
    }
}

// r[cdt.structure.embedded]
// r[cdt.structure.capa]
/// A capability, as stored in a CNode.
#[derive(Debug)]
pub enum Capa {
    // r[cdt.null.no-cdt-node]
    Null,
    CNode(CNodeCapa, CdtNode),
    Untyped(UntypedCapa, CdtNode),
}

// ——————————————————————————— CSpace Operations ———————————————————————————— //

/// Extracts a `&mut CNodeCapa` from a raw `*mut Capa`.
///
/// # Safety
///
/// `ptr` must be a valid, non-aliased pointer to a `Capa` that outlives the returned reference.
unsafe fn as_cnode<'a>(ptr: *mut Capa) -> Result<&'a mut CNodeCapa, CapaError> {
    match unsafe { &mut *ptr } {
        Capa::CNode(cnode, _) => Ok(cnode),
        _ => Err(CapaError::InvalidCapaType),
    }
}

/// Extracts a `&mut UntypedCapa` from a raw `*mut Capa`.
///
/// # Safety
///
/// `ptr` must be a valid, non-aliased pointer to a `Capa` that outlives the returned reference.
unsafe fn as_untyped<'a>(ptr: *mut Capa) -> Result<&'a mut UntypedCapa, CapaError> {
    match unsafe { &mut *ptr } {
        Capa::Untyped(untyped, _) => Ok(untyped),
        _ => Err(CapaError::InvalidCapaType),
    }
}

// r[op.carve]
/// Derives an exclusive `Carved` child untyped capability covering `[start, end)` from the
/// untyped capability at `src`, and writes it to the `Null` slot at `dst`.
///
/// # Safety
///
/// `root` must be a valid pointer to a `Capa::CNode`. The caller is responsible for providing
/// CDT children once CDT wiring is implemented.
pub unsafe fn carve(
    root: *mut Capa,
    src: CapaIdx,
    dst: CapaIdx,
    start: usize,
    end: usize,
) -> Result<(), CapaError> {
    // r[op.root]
    let root_cnode = unsafe { as_cnode(root) }?;
    let src_ptr: *mut Capa = root_cnode.resolve_mut(src)?;
    let dst_ptr: *mut Capa = root_cnode.resolve_mut(dst)?;

    // r[op.dst]: dst must be a Null slot; check before deriving to avoid discarding the child.
    if unsafe { !matches!(*dst_ptr, Capa::Null) } {
        return Err(CapaError::CNodeSlotOccupied);
    }

    // r[op.src]
    let untyped = unsafe { as_untyped(src_ptr) }?;
    // r[op.carve.delegate]
    // TODO: pass CDT children once CDT wiring is implemented (r[untyped.carve.no-overlap])
    let child = untyped.carve(start, end, core::iter::empty())?;

    unsafe { dst_ptr.write(Capa::Untyped(child, CdtNode::unlinked())) };
    Ok(())
}

// r[op.alias]
/// Derives a shared `Aliased` child untyped capability covering `[start, end)` from the
/// untyped capability at `src`, and inserts it into the `Null` slot at `dst`.
///
/// # Safety
///
/// `root` must be a valid pointer to a `Capa::CNode`. The caller is responsible for providing
/// CDT children once CDT wiring is implemented.
pub unsafe fn alias(
    root: *mut Capa,
    src: CapaIdx,
    dst: CapaIdx,
    start: usize,
    end: usize,
) -> Result<(), CapaError> {
    // r[op.root]
    let root_cnode = unsafe { as_cnode(root) }?;
    let src_ptr: *mut Capa = root_cnode.resolve_mut(src)?;
    let dst_ptr: *mut Capa = root_cnode.resolve_mut(dst)?;

    // r[op.dst]: dst must be a Null slot; check before deriving to avoid discarding the child.
    if unsafe { !matches!(*dst_ptr, Capa::Null) } {
        return Err(CapaError::CNodeSlotOccupied);
    }

    // r[op.src]
    let untyped = unsafe { as_untyped(src_ptr) }?;
    // r[op.alias.delegate]
    // TODO: pass CDT children once CDT wiring is implemented (r[untyped.alias.no-overlap-carved])
    let child = untyped.alias(start, end, core::iter::empty())?;

    unsafe { dst_ptr.write(Capa::Untyped(child, CdtNode::unlinked())) };
    Ok(())
}
