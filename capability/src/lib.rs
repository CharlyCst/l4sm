//! Work-In-Progress: Capabilities for L4sm
//!
//! L4sm is inspired by seL4's design.

use thiserror::Error;

mod cnode;
mod untyped;

use cnode::CNodeCapa;
use untyped::UntypedCapa;

// ——————————————————————————————— Constants ———————————————————————————————— //

/// Size of a frame as a power of two.
///
/// Note: on Arm the frame size is configurable. 4kiB is the minimal size, but we might want to
/// make this configurable in the future.
const FRAME_SIZE_EXPONENT: u8 = 12;

// ————————————————————————————————— Errors ————————————————————————————————— //

/// Capability operation error.
#[derive(Error, Debug)]
pub enum CapaError {
    // CSpace
    #[error("invalid cnode index")]
    CNodeInvalidIndex,
    #[error("cspace is full")]
    CspaceOutOfSpace,

    // Untyped Memory
    #[error("untyped memory capabilities is already split")]
    UntypedAlreadySplit,
    #[error("untyped memory has already allocated objects")]
    UntypedAlreadyInUse,
    #[error("untyped memory can't be split further thant its current size")]
    UntypedCantSplitFurther,
    #[error("untyped memory doest have enough free space")]
    UntypedOutOfSpace,
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
