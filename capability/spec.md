# Capability Specification

## Capabilitities

The list of existing capabilities is:

- Null
- CNode
- Untyped

### Capability Derivation Tree

The Capability Derivation Tree (CDT) tracks the parent-child relationships between capabilities. Its primary purpose is to support **revocation**: when a capability is revoked, all capabilities derived from it (its entire subtree) are atomically invalidated.

#### Structure

r[cdt.structure.embedded]
The CDT is not stored as a separate data structure. Instead, the derivation links are embedded directly in each non-`Null` `Capa` variant. A CNode slot is a `Capa` value; each non-`Null` variant carries both the capability data and a `CdtNode`:

r[cdt.structure.capa]
```rust
pub enum Capa {
    Null,
    CNode(CNodeCapa, CdtNode),
    Untyped(UntypedCapa, CdtNode),
}
```

r[cdt.structure.node]
```rust
pub struct CdtNode {
    prev: *mut Capa,  // pointer to the previous entry in the list
    next: *mut Capa,  // pointer to the next entry in the list
}
```

r[cdt.list.order]
The tree is encoded as a **doubly-linked list in in-order traversal**: all descendants of a node appear contiguously after it in the list, before the next non-descendant. This means:

- `next` points to the first child, or to the next sibling if there are no children.
- `prev` points to the parent, or to the previous sibling if this node is not a first child.
- An ancestor always appears **before** all of its descendants in the list.

Example layout for an untyped capability `U` with two derived children `A` and `B`, where `A` has one child `A1`:

```
... → [U] → [A] → [A1] → [B] → ...
```

#### Invariants

r[cdt.null.no-cdt-node]
1. A `Null` capability carries no `CdtNode` and is never part of the CDT.

r[cdt.invariant.order]
2. An ancestor always precedes all of its descendants in the list.

r[cdt.invariant.list-consistent]
3. The pointers form a consistent doubly-linked list: for any node `N`, `N.next.prev == N` and `N.prev.next == N`.

#### Derivation

r[cdt.derive.insert]
When a new capability is derived from an existing one (e.g. via copy, mint, or retype), the new `Capa` is inserted immediately after the source node (or after the last existing descendant of the source).

r[cdt.derive.rights]
The new node inherits or reduces the rights of the source; it cannot exceed them.

#### Revocation

r[cdt.revoke.subtree]
Revoking the capability at node `N` invalidates `N`'s entire subtree.

r[cdt.revoke.walk]
The kernel walks forward from `N.next`, deleting each `Capa` until it reaches one that is not a descendant of `N`. A `Capa` `M` is a descendant of `N` if and only if `M` appeared after `N` at insertion time (i.e. the CDT invariant is maintained).

r[cdt.revoke.memory]
After revocation, any memory that was backing the revoked kernel objects is returned to the parent untyped capability (its free index is reset), making it available for reuse via a new retype call.
