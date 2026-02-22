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
pub struct CdtNode {
    prev: *mut Capa,
    next: *mut Capa,
}

/// A capability, as stored in a CNode.
pub enum Capa {
    Null,
    CNode(CNodeCapa, CdtNode),
    Untyped(UntypedCapa, CdtNode),
}
