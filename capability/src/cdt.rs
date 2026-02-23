//! Capability Derivation Tree (CDT)

use crate::Capa;

// r[cdt.structure.node]
/// A node in the Capability Derivation Tree.
///
/// Embedded directly in each non-`Null` [`Capa`] variant; the CDT is encoded as a doubly-linked
/// list in in-order traversal (see `r[cdt.list.order]`).
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
