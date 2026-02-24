//! Work-In-Progress: Capabilities for L4sm
//!
//! L4sm is inspired by seL4's design.

use core::ptr::NonNull;

use thiserror::Error;

mod cdt;
mod cnode;
mod untyped;

pub(crate) use cdt::{direct_untyped_children, find_insert_after, CdtNode};
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

/// A capability index, represents an address in capability space (CSpace).
// r[cspace.capaidx]
#[repr(transparent)]
pub struct CapaIdx(usize);

/// A capability, as stored in a CNode.
// r[cdt.structure.embedded]
// r[cdt.structure.capa]
#[derive(Debug)]
pub enum Capa {
    // r[cdt.null.no-cdt-node]
    Null,
    CNode(CNodeCapa, CdtNode),
    Untyped(UntypedCapa, CdtNode),
}

impl Capa {
    /// Returns the Capability Derivation Tree (CDT) node for the capability.
    pub(crate) fn get_cdt(&self) -> Option<&CdtNode> {
        match self {
            Capa::Null => None,
            Capa::CNode(_, cdt_node) => Some(cdt_node),
            Capa::Untyped(_, cdt_node) => Some(cdt_node),
        }
    }

    /// Returns a mutable reference to the CDT node for the capability.
    pub(crate) fn get_cdt_mut(&mut self) -> Option<&mut CdtNode> {
        match self {
            Capa::Null => None,
            Capa::CNode(_, cdt_node) => Some(cdt_node),
            Capa::Untyped(_, cdt_node) => Some(cdt_node),
        }
    }
}

// ——————————————————————————— CSpace Operations ———————————————————————————— //

/// Extracts a `&mut CNodeCapa` from a raw `*mut Capa`.
///
/// # Safety
///
/// `ptr` must be a valid, non-aliased pointer to a `Capa` that outlives the returned reference.
unsafe fn as_cnode<'a>(mut ptr: NonNull<Capa>) -> Result<&'a mut CNodeCapa, CapaError> {
    match unsafe { ptr.as_mut() } {
        Capa::CNode(cnode, _) => Ok(cnode),
        _ => Err(CapaError::InvalidCapaType),
    }
}

/// Extracts a `&mut UntypedCapa` from a `NonNull<Capa>`.
///
/// # Safety
///
/// `ptr` must be a valid, non-aliased pointer to a `Capa` that outlives the returned reference.
unsafe fn as_untyped<'a>(mut ptr: NonNull<Capa>) -> Result<&'a mut UntypedCapa, CapaError> {
    match unsafe { ptr.as_mut() } {
        Capa::Untyped(untyped, _) => Ok(untyped),
        _ => Err(CapaError::InvalidCapaType),
    }
}

/// Derives an exclusive `Carved` child untyped capability covering `[start, end)` from the
/// untyped capability at `src`, and writes it to the `Null` slot at `dst`.
///
/// # Safety
///
/// `root` must be a valid pointer to a `Capa::CNode`. The caller is responsible for providing
/// CDT children once CDT wiring is implemented.
// r[op.carve]
pub unsafe fn carve(
    root: NonNull<Capa>,
    src: CapaIdx,
    dst: CapaIdx,
    start: usize,
    end: usize,
) -> Result<(), CapaError> {
    // r[op.root]
    let root_cnode = unsafe { as_cnode(root) }?;
    let src_nn = NonNull::from(root_cnode.resolve_mut(src)?);
    let dst_nn = NonNull::from(root_cnode.resolve_mut(dst)?);

    // r[op.dst]: dst must be a Null slot; check before deriving to avoid discarding the child.
    if !matches!(unsafe { dst_nn.as_ref() }, Capa::Null) {
        return Err(CapaError::CNodeSlotOccupied);
    }

    // r[op.src]
    let src_depth = unsafe { src_nn.as_ref().get_cdt().map(|n| n.depth) }
        .ok_or(CapaError::InvalidCapaType)?;
    // r[untyped.children.ordered]: find insertion point before mutably borrowing src.
    let insert_after = unsafe { find_insert_after(src_nn, start) };
    // r[op.carve.delegate]
    let children = unsafe { direct_untyped_children(src_nn) };
    let child = unsafe { as_untyped(src_nn) }?.carve(start, end, children)?;

    // r[cdt.derive.insert]: write child then link into the CDT.
    unsafe { dst_nn.as_ptr().write(Capa::Untyped(child, CdtNode::unlinked(src_depth + 1))) };
    unsafe { CdtNode::link(dst_nn, insert_after) };
    Ok(())
}

/// Derives a shared `Aliased` child untyped capability covering `[start, end)` from the
/// untyped capability at `src`, and inserts it into the `Null` slot at `dst`.
///
/// # Safety
///
/// `root` must be a valid pointer to a `Capa::CNode`. The caller is responsible for providing
/// CDT children once CDT wiring is implemented.
// r[op.alias]
pub unsafe fn alias(
    root: NonNull<Capa>,
    src: CapaIdx,
    dst: CapaIdx,
    start: usize,
    end: usize,
) -> Result<(), CapaError> {
    // r[op.root]
    let root_cnode = unsafe { as_cnode(root) }?;
    let src_nn = NonNull::from(root_cnode.resolve_mut(src)?);
    let dst_nn = NonNull::from(root_cnode.resolve_mut(dst)?);

    // r[op.dst]: dst must be a Null slot; check before deriving to avoid discarding the child.
    if !matches!(unsafe { dst_nn.as_ref() }, Capa::Null) {
        return Err(CapaError::CNodeSlotOccupied);
    }

    // r[op.src]
    let src_depth = unsafe { src_nn.as_ref().get_cdt().map(|n| n.depth) }
        .ok_or(CapaError::InvalidCapaType)?;
    // r[untyped.children.ordered]: find insertion point before mutably borrowing src.
    let insert_after = unsafe { find_insert_after(src_nn, start) };
    // r[op.alias.delegate]
    let children = unsafe { direct_untyped_children(src_nn) };
    let child = unsafe { as_untyped(src_nn) }?.alias(start, end, children)?;

    // r[cdt.derive.insert]: write child then link into the CDT.
    unsafe { dst_nn.as_ptr().write(Capa::Untyped(child, CdtNode::unlinked(src_depth + 1))) };
    unsafe { CdtNode::link(dst_nn, insert_after) };
    Ok(())
}

/// Revokes the capability at `target`, deleting its entire CDT subtree.
///
/// All descendants of `target` are unlinked and set to `Capa::Null`. `target` itself is kept.
/// If `target` is an `Untyped` capability, its watermark is reset to 0 after the walk
/// (`r[untyped.mode.switch]`).
///
/// # Safety
///
/// `root` must be a valid pointer to a `Capa::CNode`. All CDT pointers reachable from
/// `target` must be valid.
// r[cdt.revoke.subtree]
pub unsafe fn revoke(root: NonNull<Capa>, target: CapaIdx) -> Result<(), CapaError> {
    // r[op.root]
    let root_cnode = unsafe { as_cnode(root) }?;
    let target_nn = NonNull::from(root_cnode.resolve_mut(target)?);

    let target_depth = unsafe { target_nn.as_ref().get_cdt().map(|n| n.depth) }
        .ok_or(CapaError::InvalidCapaType)?;

    // r[cdt.revoke.walk]: walk forward from target.next, deleting while M.depth > target.depth.
    let mut cursor_opt = unsafe { target_nn.as_ref().get_cdt() }
        .expect("target must have a CdtNode")
        .next;

    while let Some(cursor_nn) = cursor_opt {
        // Read next and depth BEFORE calling unlink: unlink zeroes cursor's prev and next
        // fields, so reading them afterwards would yield None and terminate the walk early.
        let (cursor_depth, cursor_next) = {
            let node = unsafe { cursor_nn.as_ref().get_cdt() }
                .expect("cursor must have a CdtNode");
            (node.depth, node.next)
        };

        if cursor_depth <= target_depth {
            break;
        }

        unsafe { CdtNode::unlink(cursor_nn) };
        unsafe { cursor_nn.as_ptr().write(Capa::Null) };
        cursor_opt = cursor_next;
    }

    // r[untyped.mode.switch]: reset watermark after all CDT children have been revoked.
    if let Capa::Untyped(ut, _) = unsafe { target_nn.as_ptr().as_mut().unwrap() } {
        ut.reset_watermark();
    }

    Ok(())
}
