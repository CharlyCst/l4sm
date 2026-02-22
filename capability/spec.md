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
After revocation of an untyped capability, the corresponding range becomes available again in the parent: a new `alias` or `carve` covering that range may be issued.

### Untyped Memory

An untyped capability represents a contiguous range of physical memory. Tasks use untyped capabilities to delegate sub-ranges to other tasks or kernel objects, either exclusively (carve) or with shared access (alias).

#### Structure

r[untyped.structure]
```rust
pub struct UntypedCapa {
    start:     usize,       // physical start address (inclusive)
    end:       usize,       // physical end address (exclusive)
    watermark: usize,       // bytes allocated via the bump allocator (offset from start)
    kind:      UntypedKind, // how this capability was derived
}

pub enum UntypedKind {
    Aliased,  // derived via alias; range may overlap other aliased siblings
    Carved,   // derived via carve; range is exclusive among all siblings
}
```

r[untyped.kind.root]
Root untyped capabilities (created by the system at boot, not derived from another untyped) are of kind `Carved`.

r[untyped.kind.derived]
The `kind` of a derived capability records how it was created (via `alias` or `carve`). It governs what overlap constraints apply when checking new alias or carve requests against this capability's siblings.

#### Allocation mode

An untyped capability operates in one of two modes, determined implicitly from its state:

r[untyped.mode.delegation]
An untyped with `watermark == 0` and at least one CDT child is in **delegation mode**: its memory is managed by distributing sub-ranges to child capabilities via `alias` and `carve`.

r[untyped.mode.allocation]
An untyped with `watermark > 0` and no CDT children is in **allocation mode**: its memory is consumed linearly by the bump allocator to create kernel objects.

r[untyped.mode.fresh]
An untyped with `watermark == 0` and no CDT children is **fresh** and may enter either mode freely.

r[untyped.mode.exclusive]
An untyped must never have both `watermark > 0` and CDT children simultaneously. `alias` and `carve` must be rejected if `watermark > 0`. `allocate` must be rejected if the capability has any CDT children.

r[untyped.mode.switch]
After all CDT children are revoked, `watermark` is reset to 0, returning the capability to the fresh state. Conversely, there is no way to "un-allocate" kernel objects without revoking the capability entirely; the watermark only advances.

#### Children and ordering

r[untyped.children.ordered]
The direct children of an untyped capability in the CDT are kept ordered by `start` address. This invariant must be maintained on every alias and carve operation.

#### Alias

r[untyped.alias]
`alias(start, end)` derives a new child untyped capability of kind `Aliased` covering `[start, end)` from the current capability.

r[untyped.alias.bounds]
The range `[start, end)` must be entirely contained within the parent's `[start, end)`.

r[untyped.alias.no-overlap-carved]
The range `[start, end)` must not overlap any direct child of kind `Carved`. Overlapping an existing `Aliased` child is allowed.

r[untyped.alias.mode]
`alias` must be rejected if the capability is in allocation mode (`watermark > 0`).

#### Carve

r[untyped.carve]
`carve(start, end)` derives a new child untyped capability of kind `Carved` covering `[start, end)` from the current capability.

r[untyped.carve.bounds]
The range `[start, end)` must be entirely contained within the parent's `[start, end)`.

r[untyped.carve.no-overlap]
The range `[start, end)` must not overlap any direct child, whether `Aliased` or `Carved`.

r[untyped.carve.mode]
`carve` must be rejected if the capability is in allocation mode (`watermark > 0`).

#### Allocate

r[untyped.allocate]
`allocate(size, alignment)` reserves `size` bytes of memory from the capability's range using a bump allocator, returning the physical start address of the allocation. The returned address is naturally aligned to `2^alignment`.

r[untyped.allocate.mode]
`allocate` must be rejected if the capability has any CDT children.
