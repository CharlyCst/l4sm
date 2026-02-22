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
    // CSpace
    #[error("invalid cnode index")]
    CNodeInvalidIndex,
    #[error("guard bits do not match during CSpace resolution")]
    CNodeGuardMismatch,
    #[error("cspace is full")]
    CspaceOutOfSpace,

    // Untyped Memory
    #[error("untyped memory does not have enough free space")]
    UntypedOutOfSpace,
    #[error("proposed range is not within the parent's range")]
    UntypedOutOfBounds,
    #[error("proposed range overlaps a conflicting sibling")]
    UntypedOverlap,
    #[error("operation rejected due to implicit mode (watermark > 0)")]
    UntypedWrongMode,
}

// —————————————————————————————— Capabilities —————————————————————————————— //

/// A capability index, represents an address in capability space (CSpace).
#[repr(transparent)]
pub struct CapaIdx(usize);

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

/// A capability, as stored in a CNode.
#[derive(Debug)]
pub enum Capa {
    Null,
    CNode(CNodeCapa, CdtNode),
    Untyped(UntypedCapa, CdtNode),
}
