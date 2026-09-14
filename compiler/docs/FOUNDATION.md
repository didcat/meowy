# Foundation values and ownership gates

The bootstrap resolves `@"core"`, `@"debug"`, `@"memory"`, `@"strings"` and `@"proof"` to
explicit module identities through ordinary lexical lookup. Local names do not
create intrinsics. Module, item and type aliases preserve the resolved identity;
unmodeled members of the partial memory/strings/proof modules remain B001.

## Proof revision metadata

`proof.revision` is the static `uint32` constant `1`. Module/member aliases preserve
its identity and width; importing the package has no initialization effects.
Immutable unannotated aliases need no runtime storage. Scalar uses and explicitly
annotated or mutable bindings can materialize the value normally.

```meowy
proof : @"proof"
debug : @"debug"
revision : proof.revision

<Items> : {
    count : proof.revision
    -> <uint8[count]>
}

items <Items> : [7]
debug.print(revision)
debug.print(items[1])
```

This prints `1` and `7`. Within required blocks, direct member reads and aliases
support ordinary integer arithmetic, comparisons and type queries with checked
widths and existing bootstrap budgets. The fixed revision is not a proof answer
and may determine a type extent. General extents outside an active required block
retain their existing syntax restrictions.

The [proof reference](../../docs/reference/stdlib/proof.md) defines a larger contract.
Queries, result construction, flag inspection and assertions remain B001; this
metadata slice does not implement or qualify executable proof analysis.

## Proof descriptor type aliases

The type namespace recognizes `proof.Always`, `proof.Never`,
`proof.Indeterminable`, `proof.Result` and `proof.Flags` as opaque static identities.
Local aliases and explicit file-module type exports preserve these identities and
privacy. They add no runtime storage and retain their names in generated API pages.

```meowy
proof : @"proof"
<Outcome> : <proof.Result>
<Guaranteed> : <proof.Always>
<Again> : <Outcome>
```

These declarations name types; they construct no proof result. An attempted
runtime binding, record field, list element or reference using a descriptor type
reports E223. No same-shaped record can provide a runtime descriptor value.

`proof.Result` names the reference's closed result union, but the bootstrap cannot
yet construct descriptor unions with adjacent type suffixes. Those constructors,
first-class descriptor type values, descriptor function signatures and `Bounds<T>`
remain B001. Active alternatives, retained observation origins and descriptor
inspection still require the later metadata evaluator.

## Nominal types

Foundation types now belong to the HIR type system. They remain distinct from
records with similar fields and from each other. Normal union construction,
retagging, storage and direct calls preserve the full nominal type.

| Type | Private LLVM payload | Size/alignment | Copy | Destruction |
| --- | --- | --- | --- | --- |
| memory.Allocator | ptr | 8/8 | Yes | No |
| memory.AllocationFailure | {i32, i64, i64} | 24/8 | Yes | No |
| strings.Owned | {ptr, ptr, i64} | 24/8 | No | Required; source storage is gated |

These are pinned bootstrap layouts, not public FFI layouts. The string layout
matches the [private descriptor](../../runtime/STRINGS.md); failure facts retain
cause, requested bytes and alignment. Padding is not initialized data. No source
record construction can forge an allocator or failure, and opaque fields are not
exposed by ordinary field projection.

Copyability is separate from destruction: an exclusive reference is move-only but
does not destroy its referent. Records/lists/unions derive destruction from their
owned constituents; references do not inherit their referent's destruction.
The checker rejects source storage requiring drops, and the backend rejects
unscheduled owning storage, results and expressions rather than silently copying
a resource payload. Constructor calls still require the later owning-HIR schedules.

Opaque foundation values have no [ordinary equality](../../docs/reference/types.md#ordinary-operator-domains).
This restriction applies to every constituent of full-shape record, list and
union equality, including empty lists and unions currently holding null. Comparing
references to their storage still compares addresses. Comparing an aggregate to a
compatible scalar still projects its primary. Direct foundation formatting remains
B001 until its library behavior is implemented.

## Static heap values

`memory.heap` now produces an ordinary copyable allocator handle. Bindings have
actual local storage: copying a handle preserves the allocator but creates a
separate binding cell. Mutable bindings, record fields, lists, nullable alternatives
and value-only direct function arguments/results reuse normal value lowering.
Taking a reference to a handle cell does not make that cell static; its original
scope and last-use rules still apply. Returning a reference to a local cell is
E303, and replacing a cell with a live conflicting borrow is E302.

```meowy
memory : @"memory"
debug : @"debug"

copy <memory.Allocator> : (handle <memory.Allocator>) {
    -> handle
}

first : memory.heap
second : copy(first)
debug.print(&first == &second)
```

This prints false because the cells are distinct. The
[heap-handles example](../examples/heap-handles.mwy) also exercises a borrowed list
element and replacement after its last use. Evaluating/copying the handle does
not allocate resource bytes.

The static heap is the only allocator-producing source path. Custom/arena
allocators and non-static allocator origins remain unavailable. Ordinary functions
now apply [allocator return bounds](ALLOCATOR_BOUNDS.md) to active borrow-carrying
inputs; returning the heap does not bypass that public contract. Immutable
records/unions and shared pointee snapshots preserve the constraints. Direct mutable
allocator bindings and fixed records also retain versions through branches and
restarts, including pure record-field writes. Bounded tagged/list/reference-bearing
records and emitted-alias mutation remain B001; existing unbounded static values
keep their normal storage behavior.

## Failure transport and remaining gates

AllocationFailure parameters, locals, references, aggregates and union alternatives
can now be checked and lowered without losing their identity or scalar facts.
Native probes inject failure arguments into source-lowered functions, copy through
a shared borrow and return both present and null alternatives. This validates
transport; it does not add a source error constructor or expose its private fields.
The existing private constructor separately tests actual allocation exhaustion.

AllocationFailure is Copy but [not descriptor-compatible](../../docs/reference/stdlib/errors.md#choose-inline-storage-or-explicit-erasure).
Its small layout must not grant implicit storage as the common error descriptor.
Source error construction, common error predicates/metadata APIs and erasure remain
unimplemented. The source `strings.copy` operation is still gated, as are
strings.Owned storage, views and automatic resource cleanup.

Next implement dynamic allocator/string-view origins and bounded initialized-state
and drop schedules from [OWNING_HIR.md](OWNING_HIR.md), retaining the explicit
[panic outcomes](PANIC_OUTCOMES.md). Prove normal, Leave, Restart and panic cleanup
before enabling owning constructors. Packages, general module graphs, generic
library APIs, source recovery and release qualification remain separate work.
