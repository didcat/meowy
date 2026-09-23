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

## Deferred proof query checking

The checker recognizes type-only `proof.can_copy<T>()` through ordinary module and
member aliases. It retains the checked type argument, call origin, owner, target
and revision as pending metadata. No Always/Never/Indeterminable answer is formed.
Every program containing a pending query still fails with B001 before code generation.

Immutable local bindings can copy pending metadata without runtime storage or
creating another query. An explicit `proof.Result` annotation and a direct type
alias such as `<Result> : result<>` retain the fixed declared result type.
Annotations claiming a narrower alternative fail E207. Runtime storage, formatting,
truthiness, mutable bindings, captures and runtime-typed exports fail E223.
Static descriptor metadata exports remain B001 until their implementation exists.

Type queries on a pending binding's `always`, `never` and `indeterminable` fields
resolve to `boolean` without evaluating the answer. For example,
`<Flag> : result.always<>` declares an ordinary boolean type alias; that type may
also be used as another query's type argument. Copies and grouped bindings keep
these signatures. The type query charges dispatch and type materialization,
without replaying the original observation. Direct flag values and queries on
inline observation calls remain unavailable. Every pending observation still
reaches the B001 evaluation gate after ordinary checks; this does not enable
proof-derived scalar values or complete transitive dependency tracking.

Initializer evidence carries an internal dependency mark through evaluated scalar
operations, selected block conditions and record copies/projections. Required type
construction rejects marked reads with E225 while fixed type queries remain
answer-independent. Seeded checker tests validate this boundary; source programs
cannot produce marked flag values yet. A structural HIR walk additionally retains dependencies in skipped operands,
block successors, call arguments and indices. Ordinary binding copies preserve
these marks even when selected initializer evaluation skips the marked source.
Marked expressions do not supply scalar constants or correlated boolean guards
to base checking. Both branch successors remain possible, so marked conditions
cannot hide loan conflicts. Ordinary constants and guard correlations are unchanged.
Matcher bodies carry lexical proof-control marks into ordinary bindings and their
initializer evidence, including nested ordinary conditions. Enclosing control and
lexical scopes restore after success or error. Pending observation calls retain
those control marks and report E225 after ordinary type/ownership validation,
even if an earlier independent query still awaits evaluation. Descriptor copies
create no observations; fixed flag type queries remain permitted. Later independent
statements do not inherit a completed matcher body's control mark.
Direct local assignments and owned field/list-element writes retain dependencies
from the RHS, evaluated indices and lexical control. These marks conservatively
cover the whole destination owner and survive later independent overwrites.
Failed type/mutability checks do not mark a destination. Emitted-slot aliases share
marks through their canonical storage root. Ordinary scalar-reference bindings retain
bounded owner sets through copies/reborrows and observe later owner marks. Mutable
retargeting conservatively merges old/new owners; earlier copies retain their
origin snapshots. Sets have at most 256 owners and use the analysis work budget.
Unknown origins preserve known possibilities and explicitly remain incomplete.
Indirect stores with marked RHS, target or lexical control mark all retained
owners after ordinary checks, or report B001 if origins are incomplete. This does
not replace borrow/loan validation.
Named scalar-reference emissions use the same bounded sets at their canonical
slot root. Sibling aliases share later retargets; ordinary copies keep snapshots.
This does not enable exclusive-reference carriers or mutable exclusive-reference
fields: existing capability gates remain.
Ordinary record bindings retain origins for scalar-reference fields in nested records initialized
by direct blocks or copied from another tracked record. Mutable whole-record
replacement, nested reference-field writes and subrecord replacements conservatively
merge old/new owners at the selected path. Previous copies retain their snapshots,
and unrelated sibling paths stay unchanged. Nested field/subrecord projections
preserve those owners, and later owner marks reach reads. Named record emissions
retain source snapshots. Traversal is capped at 32 levels and 256 visited record
fields and uses the existing analysis budget. Unknown field origins retain known
possibilities but remain incomplete. Ordinary type and ownership checks still apply.
Record composition retains the source temporary's origin snapshot and maps fields
by name, including when destination positions differ. Conditional named/composed
alternatives merge known owners and retain incompleteness from unknown sources.
Later source writes do not change a composed snapshot. Checked wrapping/narrowing
of one record shape with null retains the same field paths, including nested
nullable fields. Known null contributes no owners; unknown sources stay incomplete.
Unions with different record shapes do not share field-index metadata. Shared
list-element borrows retain the container owner through ordinary reference-free
list/record views, copied views, nested indices and scalar reborrows. Owner sets
remain whole-container and conservative. Reference-free views retain these owners
when stored in nested record fields, copied/composed or wrapped in nullable records.
Supported mutable record-view fields merge old/new owners while earlier copies
keep snapshots. Mutable list-view fields retain their existing bootstrap gate.
Temporary borrows retain their existing statement-owned storage IDs as origins,
including shared element borrows over temporary lists. Initializer/control marks
remain attached to that storage. These marks do not extend lifetimes or change
ordinary expiry errors. Direct temporary carriers snapshot their reference/record
contents. Copying those contents recovers external pointee origins, including
nested record fields and empty-path reborrows, without treating the temporary
cell as the pointee. One-level aliases to named reference cells retain bounded
sets of checked root/field locations, including nested fields and transparent
reborrows. Mutable carrier retargets conservatively merge old/new locations;
earlier carrier copies retain their sets and earlier value copies retain their
pointee snapshots. Unknown alternatives preserve known locations but remain
incomplete. Cell sets are capped at 256 locations and use the analysis budget.
Ordinary cell-borrow and expiry checks remain unchanged. Named and temporary
carrier chains resolve one stored cell layer per dereference, with at most 64
layers and 256 locations per expansion. Traversal uses the analysis budget and
reports B001 on exhaustion; dependency lookup visits each cell location once to
avoid cycles. Unknown alternatives retain known possibilities without becoming
complete. Lexical named carrier emissions retain cell sets at the emitted slot's
canonical root, so sibling aliases share retargets while ordinary copies retain
snapshots. Completed records retain carrier-cell locations by field path through
construction, copying, composition, nullable wrappers and direct/nested reads.
Field/subrecord writes merge selected cell sets while preserving sibling fields
and prior copies. The existing depth, field-count, cell-count and work limits apply;
unknown alternatives remain incomplete. References to records containing borrowed
fields also retain root/field locations through copies, retargets and record-stored
views. Dependency reads follow only addressed record prefixes, including nested
reference cells; unknown locations remain incomplete. Direct shared record-view
arguments now retain both matching owned-field projection owners and nested stored
shared-reference owners through the borrow contract. Matching ignores private body
choices and preserves unknown alternatives. Carrier-valued fields reuse bounded
shared cell expansion, including nested fields and retargeted cells. Shared chains
ending in concrete borrowed records expand their locations before matching both
stored and owned-field candidates. Nested borrowed-record view fields follow a
bounded type/location worklist with shared visit and depth limits. Unknown nested
locations remain incomplete and their types are still traversed for matching
descendants and capacity checks. Shared targets may also be a single record shape
plus null, including through chains and nested views. Known null contributes no
stored owners; unknown fields or locations remain incomplete and heterogeneous
record layouts stay separate. Calls covered by the scalar-reference
borrow contract retain compatible argument origins using exact pointee types and
reference modes. All matching arguments remain possible sources, regardless of a
private function-body choice. Unknown candidates keep results incomplete; nested
argument traversal is bounded and never replays calls. Ordinary return/ownership
checks still run. Returned shared carrier chains retain cell locations from all
compatible shared argument layers, including copies and nested calls. Deeper
inputs contribute compatible inner cells through bounded expansion. The terminal
view must have no borrowed components. Unknown arguments or intermediate cells
keep sets incomplete; type/call depth, capacity and work limits remain explicit.
By-value record arguments also contribute stored carrier candidates through
nested and nullable fields, including inline construction, copies and composition.
Known null contributes no cells; unknown matching fields keep results incomplete.
Direct borrowed-record arguments also contribute contract-projected reference-cell
locations and stored carriers through nested/nullable owned records. Unknown
locations or matching fields remain incomplete. Nested borrowed-view fields and
chains ending in borrowed-record arguments remain separate for returned cells.
Shared returns of supported
scalar/list/record views retain
origins through the general contract's
record-field/list-element projections when referenced inputs/results have no
borrowed components. Concrete by-value records also contribute shared-reference
fields, including nested named fields, when their primaries have no borrowed
components. A single record shape may be wrapped in a nullable union, including
nested named fields. Known null contributes no owners; unknown matching fields
keep results incomplete. Heterogeneous union layouts are not combined. Traversal
uses bounded field paths and the existing analysis budget. Bounded shared-reference
chains can also supply their stored view owners, including named, temporary and
record-stored carriers. The innermost shared view must have no borrowed components.
Each stored-cell layer preserves known locations and incomplete alternatives;
unknown cells or stored origins keep matching candidates incomplete. Type traversal
and cell expansion retain depth, capacity and analysis-work limits.
Whole-container owners are retained conservatively, including direct record/list
views and nested projections.
Unknown matching arguments keep results incomplete. Calls with other borrowed
aggregate shapes, unsupported borrowed-record contents and reference chains,
allocator-bound pointees or broader return shapes,
and heterogeneous record unions remain separate. Callee effects and data/control
summaries are not supplied by this origin mapping. Precise overwrite/
join rules, function summaries and control after conditional leave/restart also
remain prerequisites to admitting flags or evaluated answers.

Ordinary typing and ownership validation finish before the pending-evaluation gate,
including uncalled function bodies and runtime-skipped branches. Earlier query
copies do not replace the original diagnostic location. Type arguments discover
file imports normally; checking never executes their startup code.

Type-only calls charge one invocation step and construct their written type
argument in a required root at the call span. An existing outer root supplies its
remaining counters and original span. Ordinary extent restrictions remain active;
computed operands temporarily enter required-input mode. Arity checks precede
construction, and failed arguments do not allocate pending query metadata.

Each admitted query retains an index into its outer root's logical ledger. That
ledger is saved when the root exits, including work after the query and any sticky
budget failure. Queries in the same root share it; independent calls start fresh
ledgers. Copies reuse the query index without replaying argument construction.
Pending bindings and expression statements start a construction root at the
statement span, or share an existing outer root. Each charges its statement and
call or copy read; grouping is free. Explicit annotations construct their written
type in that same root, charging aliases and direct descriptor names alike.
For a new query, annotation work and budget failure remain in its retained
ledger. Independent statements start fresh budgets; copied query IDs still refer
to their original observations. Unannotated copies add no constructor work.
Malformed annotations keep their original errors and ordinary extent restrictions.
Required-block descriptor construction remains gated; skipped required branches
do not prepare queries, while ordinary checked bodies still validate their queries.
This retains accounting for future evaluation; no proof outcome is produced.

The parser retains up to 64 explicit type arguments using supported type syntax;
nested generic types and value arguments remain unavailable. The copy query accepts
exactly one type argument and no value arguments. The queue caps at 4096 calls;
copies add no entries. These are B001 bootstrap limits, not E220 logical accounting.
Query evaluation, scalar projections, assertions, place queries, required-block
query construction and general generic specialization remain unavailable.

## Logical required-evaluation accounting

Existing required type roots now carry an independent logical ledger. Each
materialized type node charges one evaluation step and one constructed type node,
including copies and repeated materialization. Nested computed-type and metatype
bindings share the outer root and its source span. Independent roots start fresh.
Skipped constructors contribute no type materialization charges.

The ledger enforces revision 1 limits of 1,000,000 steps, 65,536 type nodes
and 1,048,576 constructed aggregate slots.
Charges are atomic and checked for integer overflow. E220 identifies the root and
exhausted counter; a caught failure remains fatal for that root and prevents later
nested evaluation. Root state is cleared on completion or failure.

Evaluated required statements and blocks each add one logical step. Boolean
operators and value reads add one step per evaluated outer expression node.
Parentheses, form checks and skipped operands/bodies add no evaluation charges.
Delegated boolean blocks are charged at block execution, so wrapper paths do not
count them again. Repeated boolean reads are separate steps; retained bootstrap
initializer visits are not copied into the logical ledger.

Required integer evaluation charges literal/name/outer-field reads and arithmetic
nodes in the shared evaluator. Block arithmetic charges its own operators while
delegating leaves and block bodies, avoiding duplicate charges. Unary minus and
an immediately following integer literal each cost one step; the signed-minimum
literal rule is preserved. Type-query operands, eligibility walks and runtime
folding add no integer evaluation charges. Field hints resolve metadata only
through name/import chains, so computed type bases and nested queries do not
execute while inspecting an operand. Unsupported operand hints retain B001
without consuming constructor work or poisoning the enclosing logical budget.
Extents inside an existing required root use the same evaluator and charges.

Scalar and record field reads charge every evaluated projection ancestor and named
root. Record name reads charge one step, including reads of materialized scratch
copies. Available immutable inputs do not replay initializer work in the logical
ledger; retained ancestor errors and bootstrap work checks still apply. Record
copy type construction retains its separate step/type charges. Lookup, grouping,
type-query operands and skipped copies add no record-read charges.

Required records charge one slot per initialized field and unit primary, including
nested records. Materialized copies charge all their slots recursively; repeated
copies charge again. Reading an available record or scalar field allocates no
slots. Type-only list extents do not construct list elements.

Named emissions charge after their value is ready; implicit unit primaries charge
at completion. Composition transfers the slots already charged by its constructed
or copied source into the flattened result, without charging them twice. Added
fields still charge normally. Skipped construction allocates nothing, failed roots
retain consumed prefixes, and independent roots reset the slot counter.

This remains partial accounting. Required list values, other type-expression
dispatch, text, source-helper depth, deferred proof analysis and descriptor
construction remain unimplemented.
Pending queries retain their argument/root budgets as described above.
The lower existing bootstrap limits still fail with B001 first; their counters are
not treated as language work. Logical-limit boundaries are tested internally,
not claimed as source-level E220 qualification. Proof evaluation remains gated.

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
