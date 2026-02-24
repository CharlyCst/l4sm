//! Capability Derivation Tree (CDT)

use crate::untyped::UntypedCapa;
use crate::Capa;

use core::ptr::NonNull;

// r[cdt.structure.node]
/// A node in the Capability Derivation Tree.
///
/// Embedded directly in each non-`Null` [`Capa`] variant; the CDT is encoded as a doubly-linked
/// list in in-order traversal (see `r[cdt.list.order]`).
#[derive(Debug)]
pub struct CdtNode {
    pub(crate) prev:  Option<NonNull<Capa>>,
    pub(crate) next:  Option<NonNull<Capa>>,
    // r[cdt.invariant.depth]
    pub(crate) depth: usize,
}

impl CdtNode {
    /// Creates a new CDT node with null pointers, not yet linked into the tree.
    pub(crate) fn unlinked(depth: usize) -> Self {
        Self {
            prev: None,
            next: None,
            depth,
        }
    }

    // r[cdt.derive.insert], r[cdt.invariant.list-consistent]
    /// Wires `new_ptr` into the CDT list immediately after `insert_after`.
    ///
    /// # Safety
    ///
    /// - `new_ptr` must point to a valid non-Null `Capa` whose `CdtNode` is currently unlinked.
    /// - `insert_after` must point to a valid non-Null `Capa` already linked in the CDT.
    /// - Neither pointer may alias any other pointer being dereferenced by the caller.
    pub(crate) unsafe fn link(new_ptr: NonNull<Capa>, insert_after: NonNull<Capa>) {
        // Read insert_after's current successor BEFORE mutating insert_after.next.
        // If we read it after, we would observe the new_ptr we just wrote, not the original
        // successor, causing new_ptr.next to point back to itself.
        let old_next: Option<NonNull<Capa>> = unsafe {
            (*insert_after.as_ptr())
                .get_cdt()
                .expect("insert_after must have a CdtNode")
                .next
        };

        // Wire new node: prev → insert_after, next → old_next.
        {
            let new_node = unsafe {
                (*new_ptr.as_ptr())
                    .get_cdt_mut()
                    .expect("new_ptr must have a CdtNode")
            };
            new_node.prev = Some(insert_after);
            new_node.next = old_next;
        }

        // Wire insert_after's next forward to new_ptr.
        {
            let after_node = unsafe {
                (*insert_after.as_ptr())
                    .get_cdt_mut()
                    .expect("insert_after must have a CdtNode")
            };
            after_node.next = Some(new_ptr);
        }

        // Wire old_next's prev back to new_ptr.
        if let Some(old_next_nn) = old_next {
            let old_next_node = unsafe {
                (*old_next_nn.as_ptr())
                    .get_cdt_mut()
                    .expect("old_next must have a CdtNode")
            };
            old_next_node.prev = Some(new_ptr);
        }
    }

    // r[cdt.invariant.list-consistent]
    /// Removes the node at `ptr` from the CDT list, connecting its neighbors directly.
    ///
    /// After this call the node's `prev` and `next` fields are `None`.
    ///
    /// # Safety
    ///
    /// - `ptr` must point to a valid non-Null `Capa` currently linked in the CDT.
    /// - All neighbors reachable via `prev`/`next` must be valid and unaliased.
    pub(crate) unsafe fn unlink(ptr: NonNull<Capa>) {
        // Read prev and next BEFORE any mutation. `unlink` zeroes both fields on `ptr`
        // and overwrites the neighbor pointers; reading after would yield stale or None values.
        let (prev_opt, next_opt) = {
            let node = unsafe {
                (*ptr.as_ptr())
                    .get_cdt()
                    .expect("ptr must have a CdtNode")
            };
            (node.prev, node.next)
        };

        // Wire predecessor's next to skip over ptr.
        if let Some(prev_nn) = prev_opt {
            unsafe {
                (*prev_nn.as_ptr())
                    .get_cdt_mut()
                    .expect("prev must have a CdtNode")
                    .next = next_opt;
            }
        }

        // Wire successor's prev to skip over ptr.
        if let Some(next_nn) = next_opt {
            unsafe {
                (*next_nn.as_ptr())
                    .get_cdt_mut()
                    .expect("next must have a CdtNode")
                    .prev = prev_opt;
            }
        }

        // Zero ptr's own pointers.
        {
            let node = unsafe {
                (*ptr.as_ptr())
                    .get_cdt_mut()
                    .expect("ptr must have a CdtNode")
            };
            node.prev = None;
            node.next = None;
        }
    }
}

/// Walks the CDT from `parent_ptr` and returns the node after which a new child with
/// start address `new_start` should be inserted, maintaining sibling ordering by start address
/// (`r[untyped.children.ordered]`).
///
/// Returns `parent_ptr` itself if no children exist yet or all existing children precede
/// `new_start`.
///
/// # Safety
///
/// `parent_ptr` must be a valid pointer to a non-Null `Capa`.
pub(crate) unsafe fn find_insert_after(parent: NonNull<Capa>, new_start: usize) -> NonNull<Capa> {
    let parent_depth = unsafe { parent.as_ref().get_cdt() }
        .expect("parent must have a CdtNode")
        .depth;

    let mut cursor = parent;

    loop {
        let next_opt = unsafe { cursor.as_ref().get_cdt() }
            .expect("cursor must have a CdtNode")
            .next;

        let next_nn = match next_opt {
            None => break,
            Some(nn) => nn,
        };

        let next_depth = unsafe { next_nn.as_ref().get_cdt() }
            .expect("next must have a CdtNode")
            .depth;

        if next_depth <= parent_depth {
            break; // Left the parent's subtree.
        }

        // At a direct child: check whether the new node should be inserted before it.
        if next_depth == parent_depth + 1 {
            let next_start = match unsafe { next_nn.as_ref() } {
                Capa::Untyped(ut, _) => ut.start(),
                _ => 0,
            };
            if next_start >= new_start {
                break; // New node belongs before this sibling.
            }
        }

        cursor = next_nn;
    }

    cursor
}

/// Returns an iterator over the `UntypedCapa` data of each direct Untyped child of `parent_ptr`.
///
/// Used to pass existing siblings to `UntypedCapa::carve` / `UntypedCapa::alias` for overlap
/// checking.
///
/// # Safety
///
/// `parent_ptr` must be a valid pointer to a non-Null `Capa`. The CDT must not be mutated
/// while the returned iterator is live.
pub(crate) unsafe fn direct_untyped_children<'a>(
    parent: NonNull<Capa>,
) -> impl Iterator<Item = &'a UntypedCapa> {
    let parent_depth = unsafe { parent.as_ref().get_cdt() }
        .expect("parent must have CdtNode")
        .depth;

    let mut cursor_opt: Option<NonNull<Capa>> = unsafe { parent.as_ref().get_cdt() }
        .expect("parent must have CdtNode")
        .next;

    core::iter::from_fn(move || loop {
        let cursor_nn = cursor_opt?;
        let node = unsafe { cursor_nn.as_ref().get_cdt() }?;
        let depth = node.depth;

        if depth <= parent_depth {
            cursor_opt = None;
            return None;
        }

        let next = node.next;
        cursor_opt = next;

        if depth == parent_depth + 1 {
            if let Capa::Untyped(ut, _) = unsafe { cursor_nn.as_ref() } {
                return Some(ut);
            }
            // Non-Untyped direct child: skip.
        }
        // Grandchild or deeper: advance without yielding.
    })
}
