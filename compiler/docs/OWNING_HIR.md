# Owning HIR and cleanup schedules

This is an implementation design, not an enabled compiler feature. The
HIR describes nominal resource layouts but emits no automatic resource destructor. The [private bridge](../../runtime/GENERATED_CLEANUP.md)
proves relocation and destruction through generated LLVM callbacks; it does not
yet connect source ownership to cleanup. This design guides the ownership
stage of [COMPILER.md](../../COMPILER.md#the-pipeline).

## First resource

Use the documented `strings.copy(text, allocator)` constructor and
`strings.Owned` from the [text library](../../docs/reference/stdlib/text-and-data.md#owned-strings-and-formatting).
Its result is `strings.Owned` or `memory.AllocationFailure`. Begin with the static
`memory.heap` handle, immutable UTF-8 storage and `owned.view()`. This needs no
generic container, builder, task or user-defined destructor. Allocation failure
must remain a typed result; substituting null, a panic or a successful empty
string would change the contract.

Resolve `strings`, `memory`, their types and operations through ordinary module
bindings, preserving aliases and shadowing. Establish foundational module item
identity and the concrete failure representation before admitting this source.
Do not add a resource-testing keyword, a special spelling or a hidden allocator.
The standalone foundational module slice need not claim package/manifest support.

`Exclusive` is non-Copy but owns no resource to destroy. `String` is Copy and
currently represents static literal views. Neither is a substitute for this
resource. Keep copying, destruction and borrowed-origin classification separate:
`!is_copy()` alone is not a drop predicate, and `has_reference()` alone cannot
describe an owning string's allocator or a string view's buffer lifetime.

The first payload has allocation address, initialized byte length and the
allocator/release information required by the chosen runtime representation.
Keep its layout private. Moving transfers these fields without copying text or
allocating; dropping releases the allocation once. Define zero-length ownership
and failed-construction behavior before implementing callbacks. No byte of
uninitialized storage or padding can establish an initialized owner.

## Existing seams

| File                                            | Current behavior                                                       | Required extension                                                           |
| ----------------------------------------------- | ---------------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| [hir.rs](../src/hir.rs)                         | Typed tree with local, statement, block, emission and call identities  | Concrete resource type, ownership operations and explicit storage/exit plan  |
| [check.rs](../src/check.rs)                     | Resolves/types HIR, then checks origins and loans                      | Require a complete ownership plan before returning a buildable program       |
| [borrow/value.rs](../src/borrow/value.rs)       | Literal strings carry no dynamic origin                                | Track string-view buffer origins through copies, aggregates, calls and exits |
| [loans.rs](../src/loans.rs)                     | Availability and last-use graph for reference authority                | Validate resource moves, replacement and destruction against live views      |
| [backend.rs](../src/backend.rs)                 | Erases statement boundaries; copies values; branches directly on exits | Consume explicit cleanup/transfer operations and preserve statement cleanup  |
| [backend/storage.rs](../src/backend/storage.rs) | Maps local/result aliases to backing storage                           | Map each alias to its single ownership cell, including discarded emissions   |

Reuse HIR identities, resolved control targets and guarded flow machinery. Extend
the storage/control representation described in COMPILER.md; do not reconstruct
ownership by scanning emitted LLVM or repurpose loan-demand events as drops.
The resulting plan is compiler-private, not a release artifact schema.

## Places and initialized state

Every destructible cell has a stable identity, type, source span and owning
region. Regions are function activations, block iterations and full statements.
Parameters belong to the callee activation; an expression result remains in its
statement until transferred. An emission belongs to its target block immediately,
even when its alias is declared in an inner scope.

Keep compile-time availability separate from runtime cleanup state. A cell can
be absent, reserved for construction, live, or moved/released. Only live storage
is armed. Guarded joins retain all reachable incoming possibilities; reading
requires initialization on every reachable path. A known move reports E301;
ordinary possibly uninitialized storage reports E309. Missing emission rules
retain their existing diagnostics. Missing proof or a capability/budget boundary
reports B001, never a successful language rejection.

Use runtime flags/tokens where initialization varies by branch. Null defaults
and zeroed LLVM storage do not arm resource payloads. A union's active tag and
payload initialization must agree before publishing either. For structural
records, track initialized primary/fields; for lists, track only initialized
elements. Never allocate analysis state proportional to list capacity. Wider
partial aggregates remain B001 until their own transfer/drop rules are proved.

| Event               | Required state transition                                                                                                |
| ------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| Constructor         | Reserve cleanup and payload storage before acquiring a resource; commit and arm only after success                       |
| Constructor failure | Release acquired partial resources; leave the owner unarmed; produce the documented failure alternative                  |
| Move                | Require an available source and no conflicting loan; transfer actual payload and arm destination before disarming source |
| Borrow/view         | Retain origin and lifetime evidence without transferring cleanup ownership                                               |
| Replacement         | Evaluate RHS once into an owned temporary; after it returns, release the current destination, then transfer RHS          |
| Drop                | Release only live state once, preserving the enclosing exit reason and panic                                             |

Reserve destination cleanup immediately before commitment, after effectful RHS
evaluation. Keep the RHS in an armed temporary while destination preparation is
pending. An early Leave/Restart in the RHS skips the unfinished store but retains
all completed earlier writes and moves. For self-replacement, the source may
already be moved; the destination's old live flag determines whether it is dropped.
The target address/index evaluation order remains the existing HIR order.

`strings.copy` consumes source bytes during the call and retains only the
allocator, as allowed by the [intrinsic lifetime contract](../../docs/reference/memory.md#lifetimes).
`owned.view()` creates a Copy string bounded by the owner. Last-use checking must
reject destruction, replacement or movement while that view remains required.
Returning a view of a local reports E303. Literal views remain static. General
user wrappers retain the documented conservative input bounds.

## Exits and result retention

Use the [cleanup order](../../docs/reference/memory.md#cleanup) and
[named-scope rules](../../docs/reference/values-and-blocks.md#named-scopes-and-cleanup).
Keep one ordered cleanup region for an active block iteration and separate
statement regions for remaining temporaries. Reserve in actual initialization
order, not local ID, field order or CFG traversal order.

| Exit                    | Result cells                                                                             | Other live cells                                                          |
| ----------------------- | ---------------------------------------------------------------------------------------- | ------------------------------------------------------------------------- |
| Full statement          | Values already transferred remain with their owners                                      | Release remaining temporaries in reverse initialization order             |
| Normal block completion | Transfer initialized emissions to protected result storage before publication            | Release block locals in reverse initialization order                      |
| Leave target            | Retain the target's initialized result; discard incomplete results of intervening scopes | Unwind inner scopes through the target, preserving Leave as the cause     |
| Restart target          | Release target and inner results; surviving ancestor results remain owned                | Release target/inner locals and temporaries, then enter a fresh iteration |
| Panic                   | Release partial results rather than publish them                                         | Capture the panic while evidence lives, then unwind all exited regions    |

Retaining an emission is not publishing it: callers cannot consume a block result
until local cleanup succeeds. Result storage needs an armed obligation in the
receiving context before the emitting region is unwound. Retain components in
their actual initialization order so later aggregate destruction reverses that
order. Do not move an emitted payload while a live borrow depends on its cell;
prove the move legal or keep stable backing with a separately verified ownership
handoff. Heap-view stability alone does not authorize borrowing the moved cell.

An emission discarded by the final result shape still evaluates and consumes its
operand. Keep its initialized owner as target-owned partial storage until the
target exit; the current backend's skipped result store is insufficient. A
SlotAlias names this same cell and must not create a second drop obligation.

Restart unwinds before resetting result flags or jumping to the header. Each
iteration gets fresh runtime tokens; reusing the same HIR IDs must not revive
old tokens or origins. A Leave during argument evaluation releases already
evaluated, untransferred arguments; callee ownership begins only when the call
is actually entered. Returned ownership is protected across callee cleanup.

## Bridge constraints and bounded first lowering

The current Stack orders destruction by reservation, arms only the top reserved
entry, and retains disarmed entries until unwind. Its `owned_transfer` requires
a reserved top destination. The destination slot must therefore be reserved after
all effectful argument/RHS evaluation, while a live source token still protects
the result. Merely pre-reserving every local in function-entry order is wrong.

Separate block and statement regions permit transfer into an ancestor without
putting its cleanup entry inside an inner region's unwind suffix. On completion,
transfer retained result components into protected receiving storage, then unwind
the original region, skipping only successfully transferred entries. On restart,
unwind the complete iteration, including its partial result. Do not unwind locals
and emissions as independent batches on failure: their initialization order can
interleave. Any region split must preserve that interleaving.

A maximum number of simultaneously live owners is not a capacity proof for the
current Stack. Repeated replacement of an ancestor owner can accumulate disarmed
entries even with one live payload. The first implementation must prove a finite
reservation bound between unwinds for every region, counting moves, failed
reservations retained for cleanup and result handoffs. A restart cycle that adds
entries to a surviving ancestor fails B001 until a separately tested reclamation
scheme exists. Increasing an arbitrary fixed capacity is not a solution.

The initial executable slice may support immutable owners, finite moves and
branching, and restarts whose owner regions are wholly unwound each iteration.
Reject owner replacement or ancestor-directed transfers when no finite bound is
proved. Check capacity arithmetic and every bridge status. Cleanup and Owned
status enums differ. Never ignore a failure after resource acquisition or turn
cleanup metadata exhaustion into the language's allocation failure value.

## Panic and enablement

[Scalar panic outcomes](PANIC_OUTCOMES.md) now preserve streamed diagnostics and
propagate a caller-owned snapshot through explicit failure exits. Native probes
use the returned snapshot for cleanup; ordinary source has no owning drop schedules.
Adding only normal/Leave/Restart drops would still omit those panic cleanup edges.
Keep resource construction unavailable to ordinary source until all reachable
recoverable failures with live owners use an owning outcome and cleanup edges.
This includes failures inside called scalar functions, not just the constructor
or explicit `debug.panic` expressions.

Explicit outcomes can implement a bounded synchronous call graph before DWARF
landing pads. They must preserve original diagnostics and evaluation order, run
cleanup through the original panic cause, and propagate across each call boundary.
Do not use complete-cause `owned_release` while unwinding an active panic. A drop
panic follows the existing fatal P008 path. Task suspension, cancellation, child
settlement and pinned unwinding remain separate gates; no task source is enabled
by a successful synchronous owner test.

## Observable schedule cases

The following traces are acceptance criteria, not currently passing tests. Letters
identify successful resource acquisitions; `drop(x)` records an actual release.

| Execution                                                | Required observation                                                                     |
| -------------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| Initialize local A, emit B, initialize local C; complete | drop(C), drop(A); B remains live until the receiving owner releases it                   |
| Same initialization; Leave this block                    | Same drops and retained B, with Leave as the cleanup cause                               |
| Same initialization; Restart this block                  | drop(C), drop(B), drop(A); the next iteration starts with no initialized result          |
| Initialize A; constructor for B fails                    | A remains live; no drop(B); allocation failure is available for matching                 |
| Move A into B; exit                                      | One release through B; no release through the moved source A                             |
| Evaluate argument A; later argument leaves before call   | Drop untransferred A at the exited statement boundary; callee never starts               |
| Emit B into a result that an outer Leave abandons        | Release B as an initialized partial result, despite the absence of a final result field  |
| Initialize A; called scalar function panics              | Preserve its diagnostic, drop(A) under panic, then propagate the failure                 |
| Repeatedly replace an ancestor owner across Restart      | B001 until reservation reuse has a proven bound; no eventual runtime frame-full surprise |

## Implementation and acceptance order

1. Implemented prerequisite: [owning panic outcomes](PANIC_OUTCOMES.md) and explicit
   propagation through scalar failure/call lowering. Preserve its debug/release
   diagnostic/effect tests and generated cleanup/original-cause P008 probes while
   adding actual owning-HIR cleanup edges.
2. Implemented prerequisites: [foundational identities](FOUNDATION.md) and the
   [private string ABI](../../runtime/STRINGS.md) with static heap, typed native
   allocation failure, deterministic failure tests and allocation/release counting.
   [Nominal failure transport and static heap values](FOUNDATION.md) are now lowerable;
   [allocator-return bounds](ALLOCATOR_BOUNDS.md) now cover immutable values, direct
   mutable handles/fixed records, field writes, restart headers and shared snapshots.
   Failure construction and tagged/list/emitted-alias bound carriers remain pending.
   Keep source construction gated until ownership and exit acceptance passes.
3. Add resource/view origins and an explicit bounded storage/cleanup plan at the
   HIR checking boundary. Cover live/moved/conditional states, actual initialization
   order, alias identity, discarded emissions and reservation bounds. Reject live
   view invalidation with E302/E303 and moves through borrows with E304.
4. Lower planned transfers and normal/Leave/Restart/panic exits through the existing
   bridge. Prove one release per successful acquisition; none for failed or moved
   source cells; no publication before cleanup. Cover nested argument exits,
   alternating union branches, repeated fresh iterations and rejected accumulating
   ancestor transfers. A later replacement slice must prove bounded entry reuse.
5. Enable the coherent source path and add ordinary meowy execution/rejection
   cases. Test module aliases/shadowing, failure results, moves, retained emissions,
   temporary views and calls. Run the compiler gate and combined repository gate
   for this compiler/runtime integration. Keep unsupported conformance and host
   execution distinct from full release qualification.

The owner schedules above remain a design. Scalar panic propagation is implemented
and independently tested; it does not validate automatic resource destruction.
Owning source storage remains gated; no syntax, dependency or reference fixture was added.
