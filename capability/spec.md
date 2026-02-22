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

### CNode

A CNode (Capability Node) is a kernel object that stores an array of capability slots. CNodes are the building blocks of a CSpace: the hierarchical namespace used to name and look up capabilities.

#### Structure

r[cnode.structure]
```rust
pub struct CNodeCapa {
    slots:      u8,            // radix: the CNode holds 2^slots capability slots
    guard:      usize,         // guard value matched when entering this CNode
    guard_size: u8,            // number of bits in the guard
    address:    NonNull<Capa>, // backing array of 2^slots Capa values; uniquely owned
}
```

#### Invariants

r[cnode.invariant.guard-nonzero]
The guard must be non-zero and `guard_size` must be at least 1. This is a functional requirement: a zero guard would make slot 0 of this CNode unreachable (see `r[cspace.resolve.stop]`).

r[cnode.invariant.guard-fits]
The guard value must fit within `guard_size` bits: `guard < 2^guard_size`.

r[cnode.invariant.unique-owner]
A `CNodeCapa` uniquely owns its backing array. CNode capabilities cannot be copied or aliased; the backing array is freed when the capability is revoked.

#### Slot Access

r[cnode.get]
`get(&self, index: usize) -> Result<&Capa, CapaError>` returns a shared reference to the capability at `index`. The reference lifetime is tied to the shared borrow of `CNodeCapa`, preventing concurrent mutation.

r[cnode.get_mut]
`get_mut(&mut self, index: usize) -> Result<&mut Capa, CapaError>` returns an exclusive mutable reference to the capability at `index`. The reference lifetime is tied to the exclusive borrow of `CNodeCapa`.

r[cnode.bounds]
Both `get` and `get_mut` return `CNodeInvalidIndex` if `index >= 2^slots`.

#### Insert

r[cnode.insert]
`insert(&mut self, capa: Capa) -> Result<usize, CapaError>` performs a linear scan for the first slot containing `Capa::Null`, writes `capa` into it, and returns the slot index. Returns `CspaceOutOfSpace` if no free slot exists.

### CSpace

The CSpace is the capability address space of a task. It is a tree of CNodes rooted at a designated root CNode. Individual capabilities are named by a `CapaIdx`.

#### CapaIdx

r[cspace.capaidx]
```rust
pub struct CapaIdx(usize);
```

A `CapaIdx` is a `usize` encoding a path through the CNode tree. Bits are consumed from the most significant end. Each valid `CapaIdx` uniquely identifies one slot in the tree (see `r[cspace.resolve.uniqueness]`).

#### Resolution

r[cspace.resolve]
`resolve(root: &CNodeCapa, idx: CapaIdx) -> Result<&Capa, CapaError>` resolves a `CapaIdx` to a capability slot by walking the CNode tree starting from `root`. The walk is defined recursively: to resolve `idx` against a CNode `N`:

r[cspace.resolve.guard]
1. Check that the most significant `N.guard_size` bits of the remaining `idx` equal `N.guard`. If not, return `CNodeGuardMismatch`. Consume those bits.

r[cspace.resolve.index]
2. Use the next `N.slots` bits as the slot index `i`. Consume those bits.

r[cspace.resolve.stop]
3. If the remaining bits are all zero, return slot `i` of `N` as the resolved capability.

r[cspace.resolve.descend]
4. If the remaining bits are non-zero and slot `i` is `Capa::CNode(child, _)`, recurse with `child` as the new `N`.

r[cspace.resolve.error]
5. If the remaining bits are non-zero and slot `i` is not `Capa::CNode`, return `CNodeInvalidIndex`.

#### Uniqueness

r[cspace.resolve.uniqueness]
Every slot in the CNode tree has a unique `CapaIdx`. Any two distinct paths through the tree produce distinct bit patterns because:

- Each level consumes `guard_size + slots` bits.
- The guard bits consumed on descent are non-zero (`r[cnode.invariant.guard-nonzero]`), so the bit pattern for "descend into child, reach slot X" is always distinct from "stop at this CNode slot" (which leaves remaining bits as zero).

#### Errors

r[cspace.error.guard]
`CNodeGuardMismatch` is returned when the guard bits in the `CapaIdx` do not match the CNode's guard during resolution.

r[cspace.error.index]
`CNodeInvalidIndex` is returned when a slot index is out of bounds, or when the walk reaches a non-CNode slot with remaining bits still non-zero.

## Operations

This section documents the public L4sm API for capability derivation. These operations accept a raw pointer to CSpace memory and `CapaIdx` values, as L4sm does not own the capability memory.

### Shared preconditions

r[op.root]
`root: *mut Capa` must point to a valid `Capa::CNode`. It is the root of the CSpace used to resolve all `CapaIdx` arguments.

r[op.src]
`src: CapaIdx` must resolve (via `r[cspace.resolve]`) to a `Capa::Untyped`. Returns `InvalidCapaType` if the resolved slot has a different type.

r[op.dst]
`dst: CapaIdx` must resolve to a `Capa::Null` slot. Returns `SlotOccupied` if the slot is already occupied. This check is performed before the derivation operation so that no capability is created if the destination is unavailable. The new capability is written directly to that slot on success.


### carve

r[op.carve]
`carve(root, src, dst, start, end)` derives a new `Carved` untyped capability covering `[start, end)` from the untyped at `src` and writes it to the `Null` slot at `dst`.

r[op.carve.delegate]
The range and overlap checks are delegated to `UntypedCapa::carve` (see `r[untyped.carve]`, `r[untyped.carve.bounds]`, `r[untyped.carve.no-overlap]`, `r[untyped.carve.mode]`).

### alias

r[op.alias]
`alias(root, src, dst, start, end)` derives a new `Aliased` untyped capability covering `[start, end)` from the untyped at `src` and writes it to the `Null` slot at `dst`.

r[op.alias.delegate]
The range and overlap checks are delegated to `UntypedCapa::alias` (see `r[untyped.alias]`, `r[untyped.alias.bounds]`, `r[untyped.alias.no-overlap-carved]`, `r[untyped.alias.mode]`).
