# Allocator return lifetime bounds

Ordinary functions returning allocator values now apply the conservative
[public lifetime contract](../../docs/reference/memory.md#lifetimes). A returned handle
is bounded by every active borrow-carrying input, even when the body simply returns
memory.heap. Copies, supported rebindings and further calls retain those bounds. The former
blanket signature gate is removed for the supported shapes below.

The actual allocator-producing intrinsic is still the static heap. This change
models public lifetime constraints on that value; it does not add arena/custom
allocator constructors or origins for dynamically owned allocator contexts. Allocator
parameters have no physical context-origin facts in this static-only slice; dynamic
factories will require explicit symbolic context sources in the input/call model.

## Propagation and consumption

The borrow engine's existing State stores these constraints in `bounds`, separate
from physical reference `origins`. Call substitution attaches active input origins
and inherited bounds to allocator components of the result. Input evaluation order
is unchanged, and full argument validation occurs only after every argument returns.
An argument Leave skips entry; an expired argument at actual entry reports E303 even
when the function returns only a scalar.

Allocator parameters are available throughout their synchronous invocation. The
caller substitutes its actual input constraints into the public return contract;
callee-local borrows still require proof in every body, including uncalled bodies.
No shared physical loan is invented for the returned handle. A lifetime bound
requires its source storage to remain alive; it does not by itself prevent replacing
a scalar in that still-live storage. Actual references retain their normal access
restrictions. Typed loan bundles preserve that distinction through copies, block
results, branches and shared pointee transfers.

```meowy
memory : @"memory"

select <memory.Allocator> : (context <&int32>) {
    -> memory.heap
}

context := 7
handle : select(&context)
context = 8
copy : handle
```

Both uses of the returned allocator occur while context's cell remains alive.
Returning `select(&local)` from local's own block, or consuming a handle from
`select(&1)` in a later statement, reports E303. The
[allocator-bounds example](../examples/allocator-bounds.mwy) executes the supported
case in debug/release without changing the runtime allocator representation.

## Supported shapes and explicit limits

| Operation                                                                                                                 | Current boundary                                                                                                                                             |
| ------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Immutable allocator bindings, copying and forwarding                                                                      | Retain all active input bounds                                                                                                                               |
| Immutable record fields and closed unions                                                                                 | Retain component paths and variant guards; scalar projections and proven null alternatives do not retain unrelated allocator bounds                          |
| Shared borrows/reborrows of allocator cells and carriers                                                                  | Preserve contained bounds beneath the reference; direct dereference drops only the cell access, while function results also retain their public input bounds |
| Direct mutable memory.Allocator bindings and assignment                                                                   | Retain guarded bound versions through branches, Leave and Restart; overwrite-before-read can discard expired bounds                                          |
| Mutable fixed allocator records and tagged values                                                                         | Whole replacement and pure field paths preserve per-component bounds, shared-reference origins and current variant activity; no lists or exclusive members   |
| Bounded mutable lists and allocator-only emitted aliases                                                                  | Remain B001; fixed reference-bearing aliases support identical/member/whole-union backing when Restart resets their slots or is independent                  |
| Field assignment in fixed allocator records                                                                               | Replace only the selected subtree after RHS completion, retaining sibling effects and constraints                                                            |
| Indexed assignment and list construction/append                                                                           | Reject active bounds before discarding their facts; existing unbounded static heap lists remain supported                                                    |
| Calls returning allocator lists, including behind references                                                              | B001 when input constraints need to be attached to unmodeled element paths                                                                                   |
| Restart headers for direct handles, tagged allocator values/records and shared references containing allocator components | Preserve bound paths and variant activity; reference components still require physical origin coverage                                                       |

## Mutable versions and restart

Direct mutable allocator bindings reuse the reference version/branch machinery.
The RHS finishes before assignment commits; earlier writes survive a later Leave,
while an unfinished outer store does not occur. Copies made before replacement
keep their own constraints. Replacing a handle with memory.heap clears its current
bounds without repairing an older copy or reading the overwritten value. A live
reference to the handle cell still prohibits replacement with E302.

Restart headers use the existing bounded canonical replay and exact predecessor
snapshots. Allocator components may have an empty bound set, so their CFG versions
retain explicit empty entries. Header shape distinguishes those optional bound
paths from physical reference paths, which still require origin coverage. Empty
bounds cannot stand in for a missing reference proof.

Ancestor sources survive an inner restart. Ended Local, Temporary and emitted Slot
sources become terminal Expired identities; reinitializing the same source site
does not revive an older handle. Overwriting the handle or pointer before reading
its expired contents is allowed. An actual expired read reports E303. Header
widening remains conservative and does not prove arbitrary relationships between
iteration counters and prior assignments. The
[mutable-allocators example](../examples/mutable-allocators.mwy) exercises the supported
loop and empty/bounded transitions in debug/release.

## Fixed record mutation

Mutable records containing allocators now participate in the same version and
restart analysis when their entire shape is free of lists, exclusive references and
non-Copy constituents. Nested records, closed unions and shared references with
fixed supported referents are supported. The same machinery supports
[reference-only mutable carriers](OWNERSHIP.md#mutable-borrowed-carriers).
This does not enable bounded mutable emitted aliases during record construction.
A record returned from a function can carry public input bounds into a mutable
binding, and ordinary field writes can add them after construction.

Whole assignment replaces the current record state. A pure field path replaces
only that subtree, using the state after the RHS finishes. Earlier sibling writes
or whole-record replacement inside the RHS survive the final field store. Leave
preserves completed writes and skips an unfinished outer field store. Unchanged
CFG components keep their previous IDs; they are not eagerly reread or overwritten.

A field read selects its component before checking lifetime. An expired allocator
field therefore does not prevent reading an independent scalar/allocator sibling
or repairing the expired field. Reading the whole record, or passing a reference
to the whole carrier at call entry, still requires all relevant bounds to be live.
Old copies retain their own constraints after the current record is repaired.
The [allocator-records example](../examples/allocator-records.mwy) demonstrates this
selective read and repair in debug/release.

Record headers may have no physical origins when every reference path is inactive.
A header containing an active physical reference still requires origin coverage;
allocator lifetime bounds cannot satisfy it. Restart expiry and overwrite-before-read apply per component;
clearing one field never clears an expired sibling.

Tag-only inspection remains distinct from consuming allocator contents. A scalar
projection from a temporary carrier can leave its block without carrying an
unselected allocator field. Retaining the whole emitted carrier still validates
all active bounds. A call through a reference to a carrier validates its contained
constraints at entry; borrowing a cell does not hide an expired allocator bound.

All bound expansion uses the existing part, path, fact and shared work budgets.
Input-by-result fanout beyond those limits reports B001 without publishing partial
call facts. No list capacity is expanded into per-element state. These B001 cases
are bootstrap boundaries, not successful language-conformance rejections.

## Tagged mutation

Mutable nullable allocators and fixed records containing closed unions preserve
current variant activity through whole assignment, pure field writes and restart.
Frontend read sites retain their own tag snapshots. At a type-test observation,
the origin pass relates those tags to the current storage version, conditioned on
parent activity, and passes the same proof to the loan CFG. It never reuses the
final binding tag map as proof for an earlier mutable value.

Assignment invalidates refinements for the written place; disjoint field facts
survive. Predicates observed before assignment cannot narrow the replacement.
The RHS finishes before a new version commits, including its sibling writes and
Leave effects. Restart canonicalization erases prior iteration correlations while
preserving possible variants and terminal expired source identities.

A tag-only observation does not consume an allocator payload. An expired allocator
can therefore be inspected and replaced with null. A proven null branch may read
its value; an active expired allocator read still reports E303. Copies retain their
own variants and bounds. A reference to the mutable cell still prevents replacement
with E302. The [tagged-allocators example](../examples/tagged-allocators.mwy) demonstrates
inspection and repair after the original public input bound has expired.

## Shared-reference allocator carriers

Fixed mutable allocator records and closed unions can contain shared references,
including nullable and nested reference components. Whole replacement versions both
physical origins and allocator bounds. Mutable reference fields support direct and
nested writes on completed fixed records. Constructors preserve initial borrowed
emitted-name snapshots; identical/member backing and whole union-view writes
synchronize them. Restart supports
reset or independent slots; written outer result owners enclosing an inner Restart
remain gated for published results. Discarded reference-bearing aliases retain local
versions without a result transfer; union-view addresses/field paths and allocator-only
alias bounds stay gated. See [reference fields](OWNERSHIP.md#mutable-reference-fields).

Replacing a carrier releases only loans no longer demanded by its current value.
Old copies retain their own sources. Reading a reference field keeps its pointee
loan; copying an allocator field retains lifetime bounds without freezing the
bound source's contents. The [allocator-carriers example](../examples/allocator-carriers.mwy)
replaces a reference to x with one to y, then changes x while retaining an allocator
bounded by x's lifetime.

Field stores preserve post-RHS sibling references and their loan IDs. Tag tests
observe current nullable reference activity without reading expired payloads.
Restart can enter with a null reference and later carry a populated variant; only
active physical paths require origins. Expired iteration sources remain terminal
and cannot be revived by the next iteration's local initialization. Shared cell
borrows and transitive call-input validation retain their existing checks.

Fixed carrier eligibility also admits reference-only values and nullable references.
Lists at any depth, exclusive carriers and broader emitted-alias writes remain
separate work. No runtime layout or ABI changes.

## Evidence and next work

Checker tests cover transitive/all-input bounds, E303 escapes and temporary expiry,
shared carrier/reborrow paths, inactive nullable results, scalar projection,
guarded mutation, Leave, restart expiry, required reference coverage, remaining
carrier gates and fanout exhaustion. Native tests verify full execution,
non-freezing lifetime bounds, skipped call entry and distinct E303/B001 diagnostics
in debug/release. Existing reference and restart regressions remain required.

Next represent bounded lists, surviving emitted results/union-view addresses and
dynamic string-view origins in these same passes. Keep owning construction gated until the
[initialized-state/drop schedules](OWNING_HIR.md) prove normal, Leave, Restart and
panic exits. Runtime task cancellation, native unwinding and release qualification
remain separate work.
