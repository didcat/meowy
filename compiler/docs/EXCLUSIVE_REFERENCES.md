# Exclusive-reference implementation design

This documents the implemented first scalar slice of the existing
[memory rules](../../docs/reference/memory.md),
[reference conversions](../../docs/reference/types.md#inference-and-assignment), and
[ownership diagnostics](../../docs/reference/diagnostic-codes.md#ownership-borrows-and-storage).
Bounded access records, guarded ancestry, lifecycle events, forward availability,
exclusive permissions and scalar indirect stores are integrated. Native execution
and exact-code tests cover the matrix below; the wider language remains incomplete.

## First supported slice

Support exclusive references to initialized, mutable ordinary locals containing
booleans, integers or floats. Support local reference bindings, moves between
bindings, replacement of mutable reference bindings, scalar dereference reads and
writes, and shared/exclusive reborrows of those scalars. Include conditional paths,
short-circuit evaluation, nested blocks and named `leave`.
An immutable reference binding can mutate its referent through `&!T`; changing the
reference value itself still requires a mutable binding.
[Bare scalar-reference returns](REFERENCE_RETURNS.md) preserve guarded input authority. Direct scalar-reference
arguments with primitive results are supported by the
[function argument contract](EXCLUSIVE_FUNCTIONS.md).

Keep exclusive-bearing records, unions, lists, reference cells, widened record storage,
temporary owners, dispatch blocks, wider result contracts and captures B001
until their transfer and lifetime proofs exist. Reject inferred forms as well as
explicit annotations. Initial support excludes exclusive pointer equality and
exclusive-valued carrier results. Anonymous scalar-reference blocks now transfer
existing guarded authority and consume emitted holders. Ordinary scalar results computed through a reference remain
supported. [Scalar record fields](EXCLUSIVE_FIELDS.md) now use mutable-path and
canonical-region checks. [Scalar indexed storage](EXCLUSIVE_ELEMENTS.md) uses
owned authority and collection reservations for elements and field leaves across
local, projected, nested and emitted owners.

These exclusions also apply to authority derived from an exclusive loan when the
type contains only shared references. For example, `id(&*p)` returning a shared
reference, `{ -> view : &*p }` and `s : &*p; cell : &s` need parent-authority propagation that
the current shared call/carrier/reference-cell paths do not supply. Keep those
boundary crossings B001 initially. Check guarded authority facts as well as types;
a `has_exclusive(Type)` test alone is insufficient. Flat shared-local copies of a
scalar reborrow are included and must preserve their child loan's identity.

For the first implementation, a body combining exclusive values with a resolved
restart remains B001. This deliberately includes iteration-local exclusive values:
a static borrow-site ID alone does not distinguish a fresh acquisition from an
earlier execution. Other bodies retain all existing shared-reference restart
support. Relax this gate only after forward availability and backward authority
demand both solve bounded loop fixed points.

## Implementation map

| Owner | Responsibility |
| --- | --- |
| `hir.rs` | Shared/exclusive types, exhaustive Copy classification, explicit Store statement |
| `check/references.rs`, `check/statements.rs`, `check/expressions.rs` | Scalar roots, mutability, reborrows, expected shared conversion and store typing |
| `borrow/mutable.rs`, `check/blocks.rs` | Bounded capability scan and dispatch-receiver metadata |
| `borrow/`, `borrow_value.rs`, `borrow_contract.rs` | Origin lifetime, component traversal and guarded local-version facts |
| `loans/access.rs`, `loans/values.rs`, `loans/control.rs` | Physical accesses, taking intent, captured pointers and returning stores |
| `loans/authority.rs`, `loans/transitive.rs` | Mode-bearing LoanIds, copy identity, guarded parents and opacity |
| `loans/storage.rs`, `loans/init.rs` | Scoped cells and demand-driven forward availability |
| `loans/permissions.rs`, `loans/solve.rs` | Bounded ancestor walks, parent suspension, boundary gates and live dependency conflicts |
| `backend.rs`, `backend/storage.rs` | Pointer loads, mode validation and once-captured scalar stores |

The existing CFG and budgets are shared by availability, liveness and permission
checks. Native tests are in `tests/native/exclusive_references.rs`; the runnable
example is [exclusive-references.mwy](../examples/exclusive-references.mwy).

## Storage, values and authority

Keep three identities distinct:

1. `Source` identifies physical storage/provenance and lifetime. Reuse canonical
   Local and Slot roots, original slot views, projections, Temporary statement
   ownership and symbolic Input paths. Source::Expired remains terminal and
   nonphysical. It can never grant permission through a newly initialized site.
2. Immutable value IDs describe snapshots and demand transfers. Existing local
   reads create fresh versions; those IDs cannot identify exclusive permission.
3. A bounded loan identity names access authority, with shared/exclusive mode and
   guarded actual sources. A value can hold guarded alternative loan IDs; a derived
   loan retains guarded alternative parents. Root acquisitions have no parent.
   Moving a reference transfers those identities; reading a pointer for dereference
   does not create a new authority. Never merge loans merely because origins match.

For example, a branch assigning `p` from either `&!a` or `&!b` must retain that
guarded choice when `&*p` creates a child. One arbitrary parent or an unguarded union
cannot prove which owner is suspended on each path. Charge every alternative and
guarded edge in the existing storage/work ledgers.

Actual origins authorize access only through the corresponding loan. Public bounds
restrict lifetime and retain their conservative dependencies; they never authorize
writing an unrelated argument or make two references interchangeable. Keep source
and bound roles distinct until the conflict query that needs each role. Graph values now retain both roles separately; their dependency iterator visits
both for conservative liveness, while access regions use only actual origins.

Scalar exclusive function inputs now carry mode-aware input loans. Symbolic Input
roots participate in permission overlap; caller entry checks validate disjointness
for exclusive arguments. The local pointer cell remains distinct from its referent.
Wider reference-result shapes remain gated. Do not attach LLVM `noalias` or other alias promises
merely because a reference has exclusive mode.

## Explicit accesses and last use

Ordinary HIR Local reads, stored-tag inspections, shared acquisitions and writes
now produce bounded CFG access records. Direct storage records keep canonical roots,
lexical views and component paths; indirect records keep exact pointer value IDs.
Records inherit CFG reach guards. Static predicates without a stored tag add no
tag read. Shared acquisitions now have graph-local LoanIds and guarded parent
alternatives; copies preserve identity independently of value versions. Call results
and restart bodies retain explicit opaque ancestry. Restart-erased source/tag
correlations may also leave explicit unresolved-region guards; these cannot authorize
access. Non-restarting accesses still require actual-origin coverage. Mode-bearing provenance now
enforces scalar exclusive permissions and parent suspension. Lifecycle events retain
taking intent, and forward availability handles source-level non-Copy references. Preserve source access evidence through future folding,
and distinguish accessing a reference cell from accessing its referent.

| Active loan | External read/shared acquisition | External write/exclusive acquisition | Access through that loan |
| --- | --- | --- | --- |
| Shared | Allowed | E302 | Read only |
| Exclusive | E302 | E302 | Read/write, subject to live descendants |

An acquisition itself is an access even if its result is unused. Its own newly
created authority does not conflict with that acquisition. A live child may use
the authority delegated along its parent chain; an unrelated loan to the same
address may not. Shared sibling reborrows can coexist. An exclusive child excludes
overlapping siblings and parent referent access; a shared child permits compatible
parent reads but suspends parent writes/exclusive acquisition until its last use.

Use backward demand to end loans after their last required access, including
derived and transferred values. Do not synthesize reads at branch/header merges.
Preserve guarded source activity and canonical overlap: whole-owner access overlaps
its fields; proven siblings may be disjoint; all views within the first indexed
collection retain the existing conservative overlap boundary.

## Moves and initialization

Forward guarded availability is implemented on the existing graph, independently
of backward reference liveness. It tracks ready, moved, uninitialized and ended
alternatives demanded by lifecycle events; exclusive references are non-Copy.
An unused variable is not thereby moved, and a live reference does not prove its
holder initialized. Every read, borrow or move requires availability on its actual
entered guard.

- A normally returning initializer creates the binding's available value. A
  consuming use of an exclusive local transfers its authority and marks that
  source moved at the evaluation point. Initialization, assignment and discarded
  value expressions consume non-Copy values. Consuming a moved value is E301.
  Grouping and same-type ascriptions preserve that consuming context; `(p)` or
  `p~<&!int32>` cannot bypass the move.
- `*p`, `&*p` and `&!*p` inspect `p` without moving it. A read of `*p` copies the
  scalar referent. Exclusive-to-shared conversion must create a shared reborrow;
  it is not a type relabel or pointer copy granting another exclusive owner.
- Reassigning a mutable moved binding is legal without reading its previous value.
  Evaluate the RHS first; record any moves/effects that finish there. Only a
  returning RHS installs the new destination. Leave or panic skips that store
  without undoing completed earlier moves.
- Join only actual continuing predecessor states, with their guards. A definitely
  moved use reports E301; an access with feasible initialized and unavailable
  paths reports E309. Preserve branch-local facts through short circuits and
  exact-target Leave queues. Do not restore pre-branch availability over an exit.
- Destroying or leaving a reference holder removes its value without destroying
  the scalar referent. No nontrivial destructor is introduced by this slice.
  Ending actual borrowed storage still invalidates all surviving views with E303.

Implemented authority-transfer policy: moving a parent handle while a child borrows
the referent transfers the same suspended parent authority to the destination.
The child retains its original owner lifetime and parent relationship; no lifetime
bound on the old handle cell is invented. The moved source is unavailable, and
the destination regains conflicting access only after child demand ends. This is
an implementation choice consistent with the existing owner-lifetime rules, not
a general rule for unsupported owned payloads. Scalar transfer is covered by native
execution; future containing shapes still need separate proof. An actual `&p` reference-cell borrow is a distinct later case.

## Indirect stores and scoped exits

Represent an indirect store separately from assigning the pointer binding. Evaluate
and capture the target pointer exactly once before the RHS. On a returning path,
keep the authorizing loan demanded through the final scalar store. A read through
that same loan on the RHS is legal; an unrelated owner write is not.

If the RHS leaves or panics, there is no final store and no artificial future
pointer use. Completed RHS moves, writes and outputs still occur. Use the same
returning-edge discipline as existing indexed-write reservations. Never reevaluate
the target or restore a stale aggregate after the RHS.

Scope/statement exits are now explicit lifecycle events before later owned-value
support. For this scalar-reference slice they invalidate storage and retire holder
state without generating runtime destruction. Later owned payloads require flags
for initialized parts, reverse-order cleanup on normal/Leave/restart/panic paths,
and task joins while parent storage lives. Connect those events to runtime
mark/close only after payload layouts and failure-batch handling are defined;
this design does not qualify generated cleanup or unwinding.

## Acceptance matrix for implementation

These are implemented scalar acceptance conditions, covered by source checks and
native regression groups in both profiles. Wider excluded shapes remain B001.

| Expected after implementation | Source |
| --- | --- |
| Accept immutable handle, mutable referent | `x := 1; p : &!x; *p = 2; v : *p; x = 3` |
| Accept move to a new holder | `x := 1; p : &!x; q : p; v : *q` |
| E301 on moved holder | `x := 1; p : &!x; q : p; v : *p` |
| E301 after discarded consuming use | `x := 1; p : &!x; p; v : *p` |
| Accept reinitialization | `a := 1; b := 2; p := &!a; q : p; v : *q; p = &!b; *p = 3` |
| E302 on direct owner read | `x := 1; p : &!x; v : x; w : *p` |
| E302 on direct owner write | `x := 1; p : &!x; x = 2; v : *p` |
| E302 on competing exclusive acquisition | `x := 1; p : &!x; q : &!x; v : *p; w : *q` |
| E302 even for unused acquisition | `x := 1; s : &x; p : &!x; v : *s` |
| Accept shared child then parent write | `x := 1; p : &!x; s : &*p; v : *s; *p = 2` |
| Accept compatible parent read | `x := 1; p : &!x; s : &*p; v : *p; w : *s` |
| E302 while shared child is needed | `x := 1; p : &!x; s : &*p; *p = 2; v : *s` |
| E302 while a shared child's copy is needed | `x := 1; p : &!x; s : &*p; t : s; *p = 2; v : *t` |
| Accept exclusive child then parent write | `x := 1; p : &!x; q : &!*p; *q = 2; v : *q; *p = 3` |
| E302 while exclusive child is needed | `x := 1; p : &!x; q : &!*p; v : *p; w : *q` |
| Accept authorized RHS read | `x := 1; p : &!x; *p = *p + 1` |
| E302 through the returning store | `x := 1; p : &!x; *p = { x = 2; -> 3 }` |
| Accept skipped final store | `'out { x := 1; p : &!x; *p = { x = 2; 'out.leave() } }` |
| E305 on immutable owner | `x : 1; p : &!x` |
| E303 after owner scope | `a := 1; p := &!a; { b := 2; p = &!b }; v : *p` |
| Accept parent authority transfer when proved | `x := 1; p : &!x; s : &*p; q : p; v : *s; *q = 2` |
| Accept captured target despite handle replacement | `a := 1; b := 2; p := &!a; *p = { p = &!b; -> 3 }; v : a; w : *p` |
| E301 through grouping | `x := 1; p : &!x; q : (p); v : *p` |
| E301 through ascription | `x := 1; p : &!x; q : p~<&!int32>; v : *p` |

Use scalar function parameters for unknown branch conditions. The first program
reports E309; reinitializing `p` after the move inside that branch must accept:

```meowy
f <null> : (flag <boolean>) {
    a := 1; b := 2; p := &!a
    | flag | { q : p; v : *q }
    v : *p
}
```

The following also reports E309: the early Leave carries moved state, while
normal completion initializes a new value. Moving and reinitializing before an
unconditional Leave must accept.

```meowy
f <null> : (flag <boolean>) {
    a := 1; b := 2; p := &!a
    'out {
        | flag | { q : p; v : *q; 'out.leave() }
        p = &!b
    }
    v : *p
}
```

Short-circuit criteria must include E309 after `flag && { q : p; -> *q > 0 }` when `p` is
then read, and acceptance after `false && { q : p; -> *q > 0 }` because that RHS never moves
it. Complementary guards must allow a move under `flag` and use of the original
holder only under `!flag`. A child created after conditional owner replacement
must suspend exactly the guarded parent alternatives, preserving safe writes to
the other owner.

Retain B001 for exclusive loops, wider signatures/result shapes, inferred carriers and
reference cells. Once those forms are implemented, moving a non-Copy referent
through a borrow must use E304; do not claim that diagnostic is proved while the
containing shape is still unsupported. Shared scalar indirect writes report E305; wider indirect stores remain B001.

## Validation and delivery order

Design checkpoint, 2026-09-07: ran 43 standalone `check --json` probes using the
existing pinned compiler at `target/x86_64-unknown-linux-gnu/debug/meowy`.
Thirty-eight exclusive cases and one shared indirect-write case returned B001;
shared controls returned acceptance, E302, E303 and E305 as expected. This verifies
current gates and source parsing only, not the planned exclusive behavior. The
disposable probe sources/results are in `/tmp/meowy-exclusive-probes/` for this
session. No production source or reference conformance fixture changed.

Current implementation: seventeen native groups cover accepted scalar execution,
exact move/permission/lifetime/mutability diagnostics, guarded choices, target capture,
Leave and panic. Parent walks execute a 16-loan chain and reject a 512-loan chain
with B001 rather than publishing partial proofs. Existing shared-reference tests
remain enabled; reference fixtures are unchanged.

Next, define generated payload/diagnostic layouts and scope cleanup. Wider exclusive
shapes remain separate: collection references need parent-derived permissions,
carriers need non-Copy initialization and destruction, and restarts need dynamic
acquisition equivalence.

Reuse existing node, value, origin, liveness and shared-work limits. Authority
registries, parent edges, availability alternatives, access events and clone/merge
work all count toward those logical budgets. Charge parent walks and require
acyclic derivation; exhaustion remains B001 and cannot publish partial facts.
Do not add a dynamic epoch counter or silently erase availability on reset.
