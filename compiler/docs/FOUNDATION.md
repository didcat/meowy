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
statements do not inherit a completed matcher body's control mark unless a derived
leave changes their availability. Forward leave continuations mark subsequent
statements and matcher arms until the target scope joins, including intervening
scopes, writes and pending query availability. Errors restore enclosing control;
independent statements after that join remain unmarked. Expression evaluation
boundaries also refresh continuation control, so later call operands and indexed
assignment operands mark temporary availability correctly. Final coercion errors
restore enclosing control, and operand-local joins do not taint later independent
temporaries. Validated restart sites retain bounded target, function-owner, source
span and proof-control evidence under their existing RestartId, separately from
forward continuation flags. Pending queries retain bounded active block identities
from their own function and link to restart targets in either registration order.
Descriptor copies keep the original query scope; recognition alone records nothing.
These associations do not change query control flags or evaluate outcomes. Backedge
propagation and termination dependence remain unimplemented.
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
Indirect-store graph operations separately retain exact target/RHS roots and a
bounded owner snapshot captured before RHS checking. Retargeting the reference
cell during the RHS does not change that snapshot. Canonical slot aliases share
owner IDs; reference-cell IDs are not substituted for pointees. Incomplete origins
stay explicit, and owner sets do not identify exact field/element write locations.
The target must complete before address capture and RHS entry; the RHS must
complete before the write effect. These links do not prove reachability or enable
proof evaluation. Existing conservative dependency marks remain separate.
Direct-function calls retain their callee and call-site IDs plus exact argument
roots, with dispatch receivers first. Ordered argument completion leads to an
opaque call-effect stage; a separate return edge permits normal continuation only
when the callee returns. Declared `never` results omit that edge. No purity,
termination or effect summary is inferred. Debug formatting and required/type-only
calls retain separate coverage boundaries.
List literals retain ordered element roots by source position, including scalar
elements checked later for contextual inference. Empty construction is explicit;
nonreturning elements do not publish a list result. Union candidate selection and
coercions remain unchanged. Custom effect-block elements capture exact grouped
roots after form recognition, then connect checked prefix/emission statements and
block results. Candidate probes allocate no roots and effects are checked once.
Failed checks restore the active point, scope/frame depths, owner and reach.
Construction links do not establish complete value provenance or proof outcomes.
Union-context lists also retain the final conversion decision for each exact
element root. Primary extraction and union conversion occur after that source
completes and before the next element or list construction. Earlier contextual
conversions are not repeated. Deferred checking still uses source position for
execution order; candidate probes remain free of graph roots. Direct or projected
`never` stops the sequence without suffix/construction edges, even if coercion
changes the final HIR type. Inputs, sequence links and endpoints are validated
together before publication and share the existing edge budget. Candidate
selection, HIR, ordinary diagnostics and required-evaluation gates are preserved.

List-construction reports now require an explicit checked producer record with
capacity, input count, owner, control and source span. Generic expression sequences
do not identify lists. Reports retain exact input roots in source order and optional
final contextual plans; ordinary per-input coercions keep their own reports.
Projection, conversion, construction and result visits are independent. Earlier
stages can survive a stopped input while later input records remain unobserved.
The retained normal flag describes the checked result type; a projected `never`
can still prevent construction after contextual coercion changes that type.

Validation checks complete roots, owners, parent/block relationships, unique source
slots, selectors, operation registration and exact original sequence/endpoint edges.
Input records and their two stage flags cost three shared payload entries each;
unused capacity adds no copies. The 65,536-input cap and shared work/effect limits
apply before publication. Repeated visits merge flags without another payload copy;
conflicts or exhaustion return no partial collection. These records infer no
element values, allocation effects, ownership authority, reachability or proof
answers. Required evaluation, reads, methods and borrows retain their own boundaries.

List/string `size` operations retain exact receiver roots and distinct length
operations. List `add` snapshots the receiver and its length before checking the
item; capacity-success edges then lead to a new list result. Nonreturning receivers
skip arguments, and nonreturning items have no result edge. Original argument,
capacity and loan checks remain authoritative; the receiver is not mutated by `add`.
List indexing and list methods capture implicit shared-receiver loads during
checking. Receiver completion precedes the load; the load precedes the index/add
snapshot or size operation, with no direct bypass. Owned lists, explicit
dereferences and strings do not gain another load. Stopped receivers have no load,
and stopped index/item expressions have no bounds/capacity-success result. These
stages share the existing edge budget and preserve known/unknown lengths, source
owners, call-return conditions and diagnostics. They do not infer pointee storage,
grant loan authority or establish complete value provenance.
Indexed access retains its normal-result decision separately from position
completion. Uninhabited list elements keep their existing bootstrap rejection;
the separate result boundary is structural metadata, not new list-type support.
Shared element borrows retain parent/index roots and reborrow IDs. Parent metadata
distinguishes checked places, existing views and direct statement-owned temporaries.
Address and length capture precede index evaluation; bounds success precedes the
reference result. These stages retain source storage without copying the list,
granting new loan authority or extending temporary lifetimes.
Exclusive indexed borrows retain canonical storage, checked prefix fields and exact
field/index paths. Every containing list has a reservation and length-capture stage
before its index; only bounds success reaches the next address. Final acquisition
follows the completed path. A nonreturning index has no success edge, and its
borrow has no acquisition/result edge. These bounded metadata share the graph's
edge budget and preserve existing loan validation; they neither grant exclusive
authority nor complete restart propagation or proof evaluation.
Debug output retains exact roots for evaluated parts and distinguishes static text
segments. Each operand completes before its part is streamed; that output must
return before the next part begins. Panic initializes its pending message and
streams its prefix before evaluating message operands. Print newline and outer
panic publication follow full message completion. A nonreturning operand keeps
later checked roots without adding later output or completion edges; panic has no
normal result edge. Conditional output-return edges do not assert that I/O succeeds.
These are bounded ordering facts, not effect summaries or proof outcomes.
Ordinary grouped expressions now connect their entry to the exact checked child
and the child's normal result to the group result. These transparent links reuse
the bounded region ledger; a normal port does not prove that the child returns.
Nested calls and short-circuit branches retain their own effect/exit boundaries.
Grouping remains erased in HIR and adds no logical required-evaluation charges.
Formatting and other specialized group-flattening paths retain their own rules.
Ordinary scalar negation, boolean inversion and `bits.not` retain exact operand
roots, operation stages and scalar types before outer union coercion. Integer
negation reaches its result only through an overflow-success edge; floating
negation and the inversion operations use ordinary completion. A nonreturning
operand has no operation/result edge. Signed integer literals keep their direct
literal path, and required-only unary construction stays outside runtime entry
reports. Contextual typing, primary projection and logical charges stay unchanged.
Unary checking also captures primary extraction performed inside its value helper.
That projection occurs after source completion and before the unary operation,
with no direct bypass. Primaries already extracted by expected-value checking are
not repeated or inferred from an existing inner wrapper. Direct Never retains
entry only; invalid projected primaries keep their original rejection. Signed
literals and source-free required construction retain their separate paths.
Formatting retains each evaluated part's exact root and actual primary-projection
decision together; literal text retains no source point. A captured projection
occurs after its source and before that part's output, without a direct bypass.
Never primaries reach extraction but no output or suffix stage. Recursive string
flattening, once-only effects, reference/formattability checks, panic prefixes and
returned-I/O conditions remain unchanged. Scalar results already produced by
another operation are not projected again.
Explicit dereferences retain exact pointer roots, shared/exclusive mode and
pointer-before-load/result order. A nonreturning pointer has no load stage; a
`never` referent has no normal result. This records availability without copying
aggregate type shapes, identifying pointee storage from a reference cell, or
creating loan authority. HIR typing and existing move/lifetime checks remain
unchanged. Implicit dereferences retain separate source boundaries.
Explicit exclusive scalar reborrows retain exact parent roots, their existing
reborrow sites and exclusive mode. Parent evaluation precedes reborrow and result
availability, with no referent load. Stopped parents allocate no site and have no
operation/result edge. These links preserve existing parent suspension, transfers
and lifetime checks without granting authority. Direct shared `&*p` reborrows also
retain exact parents, existing sites and distinct parent/result modes. Shared and
exclusive parents preserve scalar, aggregate and reference-cell pointees; bounded
comparison work retains no aggregate shape copies. Stopped parents retain the
requested result mode without a parent mode, site or result edge.
Implicit exclusive-to-shared conversion retains a distinct checked raw-expression
root and the existing shared reborrow site. Raw call/borrow results precede
conversion and cannot bypass its result stage. Shared-reference expectations keep
their caller-facing roots; nested groups forward already-shared results without
duplicating sites. Stopped contexts allocate no site and retain entry links only.
These links reuse bounded reborrow validation and the shared edge budget, preserving
existing typing, parent suspension and lifetime checks. Expected primary extraction
and other coercions use the separate stages described below.
Ordinary runtime fields retain exact receiver roots, resolved field indices and
the decision to insert a shared-reference load. Receiver completion precedes that
load, when present, then field selection and result availability. Explicit receiver
dereferences keep their own stages, and call-return conditions are not bypassed.
The selected field and receiver field count are recorded before narrowing, with
bounded type validation and no retained aggregate type copies. A `never` field has
no result edge; a `never` receiver keeps its existing field-lookup error. Required
fields and resolved static/intrinsic symbols retain their separate paths. These
links use the shared edge budget without proving full field provenance or changing
loan authority.
Ordinary local and runtime-field reads now retain distinct raw-source roots for
narrowing. The checker records whether that invocation inserted a conversion;
unchanged values forward from source completion, while changed values pass through
a conversion stage before the caller result. Direct `never` sources have no result
edge; conversion to `never` has a conversion stage but no result edge. Field loads
and call-return conditions remain before narrowing, and outer expected conversions
remain after it. Mutable observations, guards, type/loan checks and required reads
are preserved. Projected-borrow traversal retains its separate builder. These
bounded links record ordering without granting narrowing or ownership permission.
Raw ordinary local reads retain their resolved local and canonical emitted-slot
storage IDs at the exact source point. Entry reaches the storage-read stage, then
normal completion before narrowing and outer expected conversion. A `never` local
has no normal-result edge. Reference-valued locals retain their own cell identity;
explicit dereference loads remain separate. These bounded read stages preserve
initialization, ownership and control checks without replaying initializers or
inferring pointee reads. Invalid identities and exhausted shared edge budgets
publish no read operation. Runtime file-module values reuse those bounded read
stages at their existing expression roots, preserving module identity before field
or primary consumers. Aliases share the module storage without replaying startup.
Required primary folding and static symbols retain their separate paths; runtime
module value reads inside functions keep their B001 gate. Module alias declarations
and required scalar inputs inside functions retain their existing behavior.
Ordinary scalar literals and resolved constants retain exact construction roots,
scalar kinds and numeric widths. Entry reaches construction and then result
availability before expected conversion. Signed integer literals keep their direct
path; groups and unchanged wrappers do not duplicate leaves. Metadata retains no
text payloads. Core/static aliases follow lexical resolution, and ordinary bindings
remain storage reads. Required evaluation and shared probe constructors gain no
runtime leaf stages; formatting text keeps its existing absent source entries.
Invalid shapes and exhausted shared budgets publish no scalar leaf operation.
Resolved static Heap values also retain exact roots, nominal Allocator identity
and handle/result stages before expected conversion, call arguments or temporary
materialization. Module aliases preserve resolution; ordinary handle bindings
remain local reads. Required evaluation and hints gain no runtime handle stages.
The operation represents availability of the static handle, not resource allocation.
Nominal-type validation and shared-budget failures publish no partial operation;
existing borrow lifetimes, conflicts and unsupported member/equality rules remain.

Emission composition now retains the exact staging local and its checked field
count before local types are transferred, without another type copy. The field count
leaves room for the primary within the existing target limit. Per-target
Value/Primary/Field decisions still come from checked HIR. Direct emissions retain
no composition local; named aliases keep their canonical slot storage, and Never
inputs create no emission record. This is a capture prerequisite: typed emission
target and statement-result observations remain pending.

An internal bounded inventory now enumerates stored edges from all 31 families.
It preserves duplicates, exact ports, conditional routes and explicit backedges;
it neither fills gaps nor treats inventory order as execution order. Actual family
totals must match retained counters within the existing 262,144-edge limit. Shared
work is charged for families, stored rows (including empty rows) and copied edges.
Failed counts or budgets leave graph ledgers unchanged and return no inventory.
Successful compilation audits these counts after ordinary checks, pending-query
gates and documentation validation. A separate bounded pass resolves completed
point anchors, block identities, emission source/target links and restart sites.
Edges must stay within one function owner. Indexed output/projection/conversion/
address/reservation ports validate their selectors against retained producers;
snapshot and panic-prefix ports require matching producer kinds. Operation ports
must be declared by their owning producer, using one bounded registry that retains
call point identities separately from call IDs. These checks leave ledgers and
routes unchanged. Normal ports need no reachable incoming edge.
A bounded forward index now owns the validated inventory and maps exact source
ports to original entry positions. Each entry appears once in either a forward
list or a separate backedge list; duplicates and conditional routes remain intact.
Port-key ordering is structural, not execution order. The existing edge limit
also bounds source keys and index slots, with shared work charged before growth.
Failed construction returns no partial index and leaves stored ledgers unchanged.
Missing source keys, including ports seen only as destinations, mean no outgoing
edge was indexed; they prove neither termination nor independence.
A bounded structural walk follows only forward buckets from one exact seed port.
It retains visited ports and original inventory positions for encountered forward
edges and backedges. Duplicate edges remain distinct, while each port is queued
once, so converging paths and cycles without a Backedge marker cannot loop the walk.
Backedges are reported without following their targets; absent source buckets are
reported separately. A source with only backedges is a backedge boundary, not a
missing source. Both conditional paths are inspected, and Checked/Returned labels
remain uninterpreted. Visit order is not execution order or runtime reachability.
The edge limit bounds scanned positions, and inventory length + 1 bounds visited
and pending ports. Shared work is charged before growth, including empty walks;
failure returns no partial report and leaves the index reusable.
Successful compilation now collects independent reports for the exact HIR program
and function BlockEntry ports after existing gates. The collection retains one
validated index alongside reports keyed by owner, so edge positions remain
resolvable. Program owner 0 and each function's assigned owner are checked against
retained block metadata; duplicate, missing or mismatched identities are rejected.
Unused and recursive functions are inspected independently of call reachability;
calls do not traverse callee bodies. HIR function ordering does not assign owners.
The collection admits at most 65,536 entries and 917,504 retained report items
(visited ports, forward/backedge positions and missing-source boundaries). Each
walk checks the remaining item allowance before growth. Entry selection, index
construction and walks share the work ledger, including empty programs. Failure
returns no partial collection and preserves stored graph metadata.
The validated operation-owner registry now stays with the index. A separate effect
map describes Operation ports and observed output stages from the entry walks,
keyed by exact producer point and paired with its owner. Direct Bind/Write effects preserve the local ID,
canonical storage ID, optional RHS root and captured control mark. Reference-cell
writes remain distinct from pointee writes, and emitted-slot aliases retain their
canonical storage. Missing RHS metadata remains unknown; it is not synthesized.
Local-read effects now preserve exact local/canonical storage IDs and the producer's
normal/control flags. Reports retain the transferred program's local count after
bounded validation of read storage and stable alias roots; they do not copy local
types or values. Effect capture validates that bound, the operation-owner registry,
checked point/span, canonical mapping and original read-edge shape. Repeated ports
produce one record; failed identity or work checks return no partial collection.
A reference-cell read describes the cell, not its pointee. Never reads lack normal
completion; stopped predecessors keep later read effects absent. Required-only
inputs retain InputUse metadata without runtime read records. These structural
facts grant no copy/borrow authority or complete value provenance.
Explicit dereferences now retain a separate Deref effect with the exact pointer
root, shared/exclusive mode and captured normal/control flags. Capture validates
the completed producer and pointer points, parent/block/owner agreement, source
span, operation registry and original pointer/load/result edges. A stopped pointer
has no load effect; a shared `never` referent has a load without normal completion.
Reference-valued referents remain distinct from reads of their pointer cells.
Repeated visits produce one record, and shared work/effect limits are checked before
returning the collection. No aggregate type shape, pointee storage or value is copied
or inferred. Implicit field/list loads and reborrows retain their separate boundaries;
existing ownership/lifetime diagnostics and proof gates are unchanged.
Runtime field reads now retain a Field effect with the exact receiver root, resolved
field index, implicit shared-load choice and captured normal/control flags. The
producer's field count bounds the selected index after HIR transfer. Capture validates
completed producer/receiver points, parent/block/owner agreement, source span,
operation registration and exact receiver/load/field/result edges. Repeated visits
produce one record; failed identity or work/effect limits return no partial collection.
Explicit dereferences remain separate effects, and reference-valued fields do not
imply another pointee load. Never fields omit normal completion; stopped predecessors
keep later field operations outside the report. Required/static fields and projected
borrows keep their existing paths. These records describe the raw field read before
narrowing, without copying aggregate types or inferring storage/value provenance,
borrow authority, call termination or proof outcomes.
Indexed-read reports retain exact receiver/position roots, capacity, optional
initialized length, implicit-load/control flags and separate position/result
completion decisions. Load, snapshot and read observations are aggregated separately
at the producer point, so a stopped position can retain earlier loads/snapshots
without a terminal read. Stopped receivers retain no index effect. Capture validates
both completed roots, parent/block/owner agreement, source span, capacity/length,
stage selectors and exact load/snapshot/Checked/read/result edges. Only terminal
reads require a registered Operation owner. Repeated stages set their existing flags;
shared work/effect limits and conflicting metadata fail without partial publication.
The payload has fixed size and copies no type shapes or runtime values. Element
borrows, explicit dereferences and list-method stages remain separate. Encountered
read ports retain conditional bounds success; they do not prove bounds, runtime
reachability, precise storage, borrow authority or a proof result.
List/string size and list add now retain typed Method reports with exact receiver
roots, checked kinds, implicit-load/control flags and add item/length/capacity
metadata. Observed loads, snapshots and terminal operations remain independent:
stopped add items can retain earlier loads/snapshots, while stopped receivers produce
no method effect. Size operations have no add snapshot and string size has no
implicit list load. Capture validates complete roots, parent/block/owner agreement,
source span, kind-specific metadata and exact edges, including add's Checked capacity
route. Only terminal stages require registered Operation owners. Duplicate stages
set existing flags; conflicting metadata and shared work/effect limits publish no
partial collection. These fixed-size reports copy no types or values and do not
infer capacity success, storage, borrow authority, call termination or proof outcomes.
Existing receiver-first checking, argument/error precedence and add's new-list result
are preserved; add does not implicitly mutate its receiver. Explicit dereferences,
indexed reads and element borrows retain their separate operations.
Unary reports retain exact operand roots, checked operator kinds, compact scalar
types/widths and primary/control flags before outer coercion. Compatible boolean,
integer and floating descriptors are selected without copying or walking aggregate
types; malformed types and unsupported widths are rejected. Projection, operation
and result observations remain independent. Integer negation retains its Checked
result edge, so an operation observation does not imply a result observation or
successful arithmetic. Capture validates completed roots, parent/block/owner
agreement, source span, exact edges and registered operation/result owners.
Repeated stages update existing flags; conflicting metadata and shared work/effect
limits return no partial collection. Required construction can retain unary metadata
outside runtime entry walks; those points produce no runtime effect. Stopped operands
and signed-literal leaves keep their separate paths, and expected primary projections
are not repeated. These structural reports evaluate no values or proof outcomes.
Binary metadata now retains compact Never/scalar/other operand and result classes
without copying aggregate shapes. Scalar binary reports preserve exact roots,
canonical operator symbols, type widths, primary/normal plans and control flags.
Capture validates operator/type domains, completed points, parent/block/owner
agreement, spans and both edge ledgers: the binary edges retain projections and
operation/results, while the sequence retains the exact left-to-right link and
ordered operand IDs. Checked integer results cannot bypass their success edge.
Left/right projection, operation and result observations remain independent; a
projected Never can retain its projection while stopping later stages. Operation
and result observations require registered owners. Duplicate observations update
existing flags, and shared work/effect limits or metadata conflicts publish no
partial collection. Nonscalar comparisons remain supported with Unknown effects;
their shapes and transfers are not inferred. Short-circuit and required-only paths
remain separate. No values, arithmetic success or proof outcomes are evaluated.
Scalar-leaf reports retain compact Null/Bool/Int/Float/String kinds, numeric widths,
control flags and independent construction/result observations. Capture checks
complete expression roots, owner/span identity, supported widths, exact construction/
result edges and registered operation owners. Duplicate observations update existing
flags; conflicting metadata and shared work/effect limits publish no partial
collection. Signed literals retain their direct roots before outer coercion, and
resolved static constants gain leaf reports at runtime uses. Static alias declarations,
ordinary storage reads, required evaluation and formatting text retain their separate
paths. Stopped successors gain no observations. No literal values or text payloads are
copied, and result observations do not establish singleton domains or proof outcomes.
Static heap reports retain the nominal Allocator type, control flags and independent
handle/result observations. Capture validates completed expression roots, owner/span
identity, both exact edges and registered operation owners. Duplicate visits merge
flags; conflicting metadata and shared work/effect limits publish no partial
collection. Module aliases preserve their resolved identities, while ordinary handle
copies remain storage reads. Reports retain the original leaf before conversions,
call arguments and temporary consumers; required evaluation, hints and stopped
successors add no observations. A static handle report describes availability only;
it does not allocate storage, extend a lifetime or grant borrow/proof authority.
Existing nominal, lifetime, conflict, member and equality diagnostics remain unchanged.
Narrowing reports retain exact raw-source roots, changed/normal/control flags and
independent conversion/result observations. Capture validates complete expression
points, matching spans, parent/block/owner agreement and exact original edges.
Unchanged values retain only a forwarding result, without an invented operation or
operation-owner requirement. Actual conversions require registered operation owners;
conversion to Never may retain its operation without a result, while direct Never
sources retain neither observation. Duplicate visits merge flags, and conflicting
metadata or shared work/effect limits publish no partial collection. Required reads,
projected borrows and outer expected conversions remain separate. These records
replay no receiver effects, copy no types and infer no narrowed value, same-value
identity, successful execution or proof outcome. Mutable guards, invalidation,
ascription and borrow diagnostics retain their ordinary checking rules.
Coercion reports retain exact raw sources, Forward/Convert/Stopped decisions,
primary/control flags and independent projection/conversion/result observations.
Capture validates completed roots, matching spans, parent/block/owner agreement,
supported expression/short-circuit source kinds, selectors and exact original edges.
Forwarding adds no operation; direct stops add no observations, while a projected
Never can retain its projection without a conversion or result. Only conversion
operation/result visits require registered operation owners; projections remain
independent. Duplicate visits merge flags, and conflicts or shared work/effect limits
publish no partial collection. Composed fallbacks and expected contexts preserve
inner call/branch identities. Shared reborrows, unchanged shared forwarding,
required evaluation and uncaptured helpers remain separate. No target types or
values are copied, and these flags infer no transfer, borrow authority or proof outcome.
Predicate/ascription reports retain exact operand roots, checked operation kinds,
normal/control flags and independent operation/result observations, including erased
no-op ascriptions. Capture validates complete expression points, root spans,
parent/block/owner agreement, supported operand kinds, exact original edges and
registered operation owners. Operand spans remain distinct from the complete suffix
expression. Stopped operands gain no observations; a stopped predecessor excludes
later operations from the same entry walk.
Duplicate visits merge flags; conflicting metadata or shared work/effect limits
publish no partial collection. Computed target reads, required evaluation, type
queries and pending proof descriptors remain separate from runtime inputs. Reports
copy no target types or values and infer no predicate truth, refined value, borrow
authority or proof outcome. Operand/target error precedence and E208 remain unchanged.
Ordinary place-borrow reports retain the exact local/field path, canonical storage,
checked shared/scalar-exclusive mode and control flag. Independent flags record each
root/field-address visit, reference acquisition and result visit. An address visit
alone records neither acquisition nor result. Retained per-field record counts
validate indices after locals move into the checked program, without copying type
shapes. Capture checks completed point/owner/span identities, parent/block agreement,
local bounds, canonical alias storage, selectors, registered operation ownership and
the exact address/acquisition/result edges. Paths have at most 256 fields; copied
indices and address flags share the payload budget described below. Repeated visits
merge flags without another copy; conflicts or exhausted limits publish no partial
collection. Stopped predecessors exclude later borrows from that entry walk.
Reference-cell storage remains distinct from pointee storage. Indexed borrows,
reborrows, projected references and temporary borrows retain separate producers;
shared indexing can still report its ordinary root borrow. Observations grant no
new loan authority, longer lifetime, referent value or proof outcome.
Standalone temporary-borrow reports retain exact initializer roots, cell local/
statement identities, control and independent acquisition/result observations.
Validation checks completed point/owner/span identities, initializer parent/block/
site agreement, local bounds, temporary registration and the completed owning
statement's root. Original initializer/acquisition/result edges and registered
operation owners must agree. Stopped initializers have no cell or observation;
stopped predecessors exclude later temporary borrows from the same entry walk.
Records have fixed size and copy no initializer values or type shapes. Repeated
visits merge flags; conflicting metadata or shared work/effect exhaustion publishes
no partial collection. Reference-valued initializers retain their distinct cell
identity. Projected/indexed temporary materialization and reborrows stay separate.
These observations preserve full-statement lifetimes and grant no longer lifetime,
pointee identity, loan authority or proof outcome.
Reborrow reports retain exact parent roots, existing ReborrowIds, checked parent/
result modes, control and independent acquisition/result observations. Explicit
shared/scalar-exclusive reborrows and implicit exclusive-to-shared conversions use
the same producer. Validation checks completed point/parent/owner/block/span
identities, mode compatibility, site bounds, registered operation ownership and
original evaluation/acquisition/result edges. Site IDs remain unchanged when
other producers allocate intervening sites; parent calls retain their return
conditions. Stopped parents have no site or observations, while a shared Never
referent can be reborrowed without loading it. Records have fixed size and copy
no referent shapes or values. Duplicate visits merge flags; conflicts or shared
work/effect exhaustion publish no partial collection. Unchanged shared forwarding,
indexed/projected borrows and temporary materialization stay separate. These
observations describe child-loan creation without granting authority, extending
lifetimes or evaluating proofs; ordinary move/conflict/permission errors remain intact.
Shared element-borrow reports retain exact parent/index roots, source kinds, existing
sites, capacity/optional initialized length, control and independent address,
acquisition and result observations. Owned sources preserve bounded field indices,
field counts and canonical storage after locals transfer; temporary sources retain
their existing local/statement registration, while view storage stays opaque.
Validation checks completed point/owner/parent/block/span identities, source bounds,
temporary statement roots, selectors and exact original edges. Completing accesses
require registered operation ownership; partial addresses validate without a terminal
operation. A stopped parent produces no observation. A stopped index can retain the
earlier address without acquisition or result. The Checked bounds condition remains
explicit and does not establish success. Owned path copies share the payload budget
below; views and temporaries copy only fixed-size metadata. Duplicate visits reuse
records; conflicts and exhausted work/effect/payload limits publish no partial
collection. These observations grant no new loan authority, source value, longer
lifetime or proof outcome. Exclusive indexed paths and projected borrows remain separate.
Exclusive indexed-borrow reports retain the exact source place/canonical storage,
ordered field/index steps, bounded field counts and each index's original length
input and checked completion. These facts are captured while types and HIR remain
available, without extending the generic PathStep representation or copying types.
Independent flags record root/field addresses, per-index reservations, final
acquisition and result visits. Validation checks point/owner/parent/block/span
identity, source and capacity bounds, distinct index roots, exact ledger edges,
selectors and operation registration. Reservations precede index evaluation and
Checked edges retain bounds conditions. A stopped index excludes later visits and
terminal results; later disconnected ledger entries remain intact. Observations
infer no runtime reachability or bounds success. Prefix plus path length is capped
at 256; copied metadata and flags share the payload allowance below. Duplicates
reuse records, and identity conflicts or exhausted shared budgets publish no partial
collection. Ordinary field permissions, reservation conflicts, moves, lifetimes
and capability gates remain authoritative; reports grant no new loan authority or
proof outcome. Projected references retain their separate producer.

Projected-borrow reports retain exact parent roots, ordered materialization/field/
load/address steps, existing reborrow sites, parent modes and control. The stored
mode describes the final parent reference; the resulting borrow remains shared.
Field/address counts are captured while concrete types are available. Temporary
steps retain their original local/statement identities after locals transfer.
Independent flags record each projection and owned-field narrowing conversion,
final acquisition and result. Validation checks completed point/owner/parent/block/
span identities, temporary registration and statement roots, field bounds, step
order, selectors, operation registration and exact original edges. Narrowing after
a reference load remains forbidden. Stopped parents retain no site or observation;
stopped predecessors exclude later borrows from the same entry walk. Calls retain
their original conditional return boundaries. Paths contain at most 256 steps;
each copied descriptor and its two observation flags consume three shared payload
entries. Duplicate visits merge flags without another copy; conflicts and exhausted
work/effect/payload budgets publish no partial collection. Ordinary places, elements,
exclusive indexed paths and direct/implicit reborrows remain separate. These records
copy no values or type shapes and infer no pointee storage, narrowed truth, loan
authority, longer lifetime, runtime reachability or proof outcome.

Owned field/index writes now retain a Path effect with local/canonical storage,
ordered Field/Index steps, exact index roots/capacities/spans, RHS and control.
Paths reuse captured metadata without evaluating indices or replaying address/RHS
effects. Each copied path is bounded by 256 steps. Path steps, field indices and
address flags for ordinary borrows, field indices and counts for element borrows,
exclusive prefix/step/count/access metadata and stage flags, direct-call argument
roots, projected-borrow steps and observation flags, and observed output-part records
share a limit of 262,144 copied entries.
Lookup and copy work are charged before allocation;
duplicate operation ports copy a path once. Empty paths and owner mismatches are
rejected. Aliases retain their shared canonical storage and distinct local IDs.
Stopped address or RHS evaluation leaves the write outside the effect map when
its Operation port is not encountered. Index values and bounds success remain
unknown; the metadata does not establish precise overwrite or alias independence.
Indirect effects now preserve captured target/RHS point IDs, control and pre-RHS
origin snapshots. Roots remain canonical pointee owners rather than reference-cell
IDs or precise field/index locations. Later RHS retargeting does not change an
earlier snapshot. Incomplete and empty origin sets retain their completeness flag;
they do not establish precise writes or independence. Each snapshot is capped at
256 roots and the collection at 262,144 copied roots, separately from the shared
path/argument/output allowance. Copy work is charged before allocation, including
empty snapshots, and
duplicate operation ports copy origins once. Owner mismatches and exhausted budgets
return no partial collection and preserve reports and conservative marks.
Direct calls retain a Call effect with exact call/callee IDs, ordered argument
roots, captured control and the declared return boundary. A bounded lookup maps
operation points to registered CallIds. Capture validates the independently
registered callee entry/body and checked argument owners/parents, plus the original
argument-order and conditional Returned edges. Function HIR need not remain in the
checker. Callee effects stay opaque; capture never traverses bodies or changes edges.
Argument copies obey per-call and shared payload bounds and are charged once for
repeated operation ports. Stopped argument evaluation excludes the call and later
arguments from the caller's report; unused/recursive bodies retain independent
entry reports. Return metadata does not prove that a call returns or terminates.
Print/panic effects now aggregate the prefix, projection, streamed-part and terminal
ports actually encountered by each structural walk. Records retain panic/control
flags, checked total/stopped metadata and exact input roots or literal None markers.
Projection and output flags are separate: a Never primary can reach projection
without producing output. Prefixes and earlier parts survive a stopped operand even
when no terminal Operation port exists. A terminal print newline or panic publication
still requires the matching operation-owner registry.
Source owner/point, dynamic-parent, selector and stop checks precede aggregation.
Conflicting headers or part inputs fail closed, and repeated visits copy each part
once under the shared payload/work bounds. No operand is replayed and no edge is
added. These records describe observed structural stages, not successful I/O or
runtime reachability; Returned edges remain conditional.
Other encountered producers retain Unknown effects. Neither Unknown nor Call
establishes purity or independence. Stored
operations beyond a stopped RHS are not added unless their port is encountered.
Duplicate ports produce one effect entry. Registry and producer owner mismatches
are rejected, and lookup never rescans every producer for each port. At most
262,144 effect entries are retained in addition to the bounded walk reports.
Shared work charges cover entry/port visits and lookups before map growth; failure
returns no partial effect map or report collection. Existing graph metadata and
conservative dependency marks remain unchanged.
A bounded call graph now groups those typed call sites by caller and callee owner,
retaining exact operation points and independent body entries. All entries remain,
including unused functions and an empty program. Missing-source ports and backedge
positions retain their exact identities beside each entry; they do not become call
edges or establish complete effects. The graph caps entries at 65,536, call sites
at 262,144 and copied boundaries at 917,504, with shared work charged before growth.
Invalid ownership, identities or exhausted bounds return no partial graph.
An iterative component pass validates adjacency/site membership and partitions
owners into deterministic groups. It traverses the call graph and its reverse with
bounded explicit worklists, avoiding recursion proportional to graph depth. Groups
and their owners are sorted; multiple owners or a self-loop mark a recursive group.
An independent three-owner reachability oracle and a deep-chain regression check
the partition. Restart backedges remain separate. These groups prepare later
analysis; they do not execute or enter callees, summarize effects, establish
termination/purity, propagate backedges or enable proof results.
Component condensation now retains internal call sites separately from grouped
cross-component sites, using the same canonical component IDs. Partition and site
membership are validated before allocation; each original call appears exactly
once. Original body entries and missing/backedge evidence remain available in the
retained call graph. Condensation and ordering cap owners/groups at 65,536 and
sites at 262,144, with shared work charged before growth.
A bounded iterative ready set orders analysis of callees before callers. Distinct
target groups count as dependencies; parallel call sites are preserved without
inflating dependency counts. The smallest ready component ID resolves ties.
Invalid targets, duplicate sites or a remaining cross-group cycle return no partial
order. Internal recursion stays within its group. This order prepares later
analysis; it neither schedules runtime calls nor supplies complete effect summaries.
Successful ordinary function Bind definitions now retain one entry-to-normal
statement edge after signature/body checking, using the exact registered function
ID. Statement identity and the independent function-body owner are validated;
shared work and edge limits precede publication. Repeated identical registration
is idempotent, while conflicting identities or exhausted budgets publish no new
endpoint. Definitions do not traverse or execute their bodies: an unused `never`
body does not stop declaration completion, while a call returning `never` still
lacks normal continuation. Program-entry store walks now cross these definitions.
Module function definitions and typed re-exports retain the same completion edge
after all signature, scope, duplicate-export and declaration checks succeed.
The export helper returns the exact declared or resolved function ID after module
registration, including definitions that allocate nested functions and aliases
of imported functions. The existing completed-function validator preserves the
statement/module owner and independent body identity, with the same atomic work
and edge limits. Export completion does not enter the body, imply a call returns,
or change module initialization order. Failed exports publish no completion edge;
public-signature, type, duplicate and capability diagnostics remain unchanged.
Explicit `<Name> : ...` type-alias declarations, including exported aliases,
retain bounded statement-completion edges after required construction and all
declaration/export checks succeed. The checked statement identity is sufficient;
no type payload, runtime type storage or new type ID is retained for the edge.
Computed aliases keep their original logical charges, input restrictions, error
order and root restoration. Aliases inside required type blocks still use the
required evaluator directly and add no runtime statement endpoints; existing
required expression/read metadata is preserved. Failed construction or declaration
publishes no alias endpoint. Completion never bypasses an earlier stopped path.
Immutable, unannotated bindings of resolved function items and the current
foundation modules/callable items also retain bounded statement-completion edges.
Classification uses the resolved value, which is declared unchanged; publication
follows successful declaration, preserving duplicate-name and lookup errors.
Function aliases accept registered slots whose bodies are still being checked,
including self/forward aliases. They never traverse the target body or imply that
a later call returns. Foundation classification includes module aliases,
print/panic, string-copy, copy-query and bit-operation identities; it does not
admit execution of unsupported calls or proof queries.
Resolved type values and foundation type identities now share that bounded
classification. Ordinary annotated meta bindings publish only after `meta_binding`
and declaration succeed. Named meta exports publish only after export validation
and module registration. The original payload is preserved without additional type
copies, runtime fields or storage. Type-query operands remain unevaluated, and
shadowed `Type` names retain ordinary lexical meaning. Required-only helpers and
their logical costs remain separate from statement completion; E220 and ordinary
errors precede publication. Graph exhaustion still reports B001, and proof outcome
gates are unchanged.
Immutable, unannotated aliases of resolved static constants also publish bounded
completion after declaration. `proof.revision` and its aliases retain the existing
`uint32` payload; classification follows the resolved value, never the name.
Annotated and mutable copies still use ordinary runtime storage and scalar-read
operations, including their width checks. Shadowed record fields remain ordinary
data. Required-only static producers and aliases keep their evaluator paths and
logical costs without gaining runtime statement endpoints. This adds no constant
evaluation, payload copy or proof outcome; failed declarations publish no endpoint.
Scoped-control aliases now capture the resolved target and owner for bounded
completion validation. The original value retains its leave/restart flag unchanged.
After successful declaration, a charged active-frame lookup requires the target to
belong to the current owner before publishing Entry-to-Normal. Alias creation adds
no scope exit, restart site or backedge and does not change reachability. Actual
calls retain their existing argument, function/lifetime and control checks and
their separate exit/restart edges. Inactive or foreign metadata and exhausted
budgets publish no endpoint; ordinary lookup/declaration errors remain first.
File-module aliases now retain the exact backing LocalId in their completion
marker. Bounded lookup validates both local storage and exports registration after
successful declaration, preserving the original type payload without another
shape copy. Alias creation adds no module-body execution or extra storage; global
module identity may be aliased within a function while the statement retains its
own function owner. Privacy, required-read eligibility, annotated/mutable alias
gates and runtime module-capture restrictions remain unchanged. Required-only
evaluation keeps its existing path; alias creation does not initialize a module.
Synthetic module-initializer bindings now retain the exact expression point
returned by `module_value_point`, after module and documentation checks succeed.
The helper exposes the existing checked root without adding a point or evaluating
the body again; expected-value wrappers and prior module/docs state are preserved.
Storage operations validate the root's parent, owner, block and completion, then
connect statement entry to input evaluation and input normal completion to storage
and continuation. The existing three-edge/work bounds apply atomically.
Program-entry reports can now collect initializer effects in module order. An
emission does not complete initialization: tail effects precede materialization,
and stopped/Never inputs have no path to the store or later modules. No empty-HIR
bypass or unconditional completion is added; errors publish no outer storage
operation and original privacy/export/initialization checks remain authoritative.
Forward-function groups retain the actual reserved FunctionIds and a
checked point/site anchored at the first source signature. IDs are captured before
nested body checking can allocate more functions. The group finishes only after
every definition succeeds; completed body/owner identities, reservation order and
bounded work are validated before one Entry-to-Normal edge is published. Statement
site context is restored after checking, retaining an empty lifetime wrapper when
needed. Function bodies are independent; unused `never` functions do not prevent
group completion, while actual Never calls still lack continuation.
Block and ordinary/composed dispatch sequences use the checked group point. Real
None prefixes remain explicit and are never skipped. Named immutable exported
forward definitions at module top level fulfill the same reservations. Their public
signature comes from the explicit forward header; an omitted definition result uses
that signature, while explicit annotations must match. Ordinary exports still require
their existing annotation. Scope, collision and budget checks remain authoritative.
Exports publish only after every definition and documentation check succeeds;
non-top-level forward exports remain gated. Documentation links resolve to exact
peer definition locations in either order, preserving public/private visibility.
Ordinary record and subrecord copies retain runtime storage operations and checked
initializer inputs. Required-evaluation record aliases use temporary scratch values
with no runtime locals or statement points; their enclosing declaration completes
only after checking succeeds. Successful reads retain the original input/storage
identity and whole-ancestor eligibility/error evidence. Scratch aliases do not
invent runtime reads, and neither kind of copy bypasses a stopped predecessor.
Focused tests pin scope, budgets and proof-derived input rejection; source cases
exercise inline storage independence, imported staging, errors and panic ordering.
Pending query bindings, copies and discarded query forms now publish bounded
statement completion after annotation and name checks. The checker retains exact
query IDs and distinguishes creation from copying; point, owner, site and closed
or currently active budget-root identities are validated before publication.
Aliases retain their original query origin and root, and cross-function capture
still fails E223. Completion introduces no runtime query edge or descriptor storage,
does not bypass stopped predecessors and grants no proof outcome. Metadata identity,
work or edge exhaustion publishes no endpoint; final proof evaluation remains gated.
Heap handles keep their separate runtime value path. There is no general bypass
for empty HIR statements. Completion edges are structural connectivity, not
evidence that a runtime path is reached.
The report is internal structural metadata, with no proof/data propagation,
restart-header analysis or termination inference. Proof outcomes remain gated.
Type predicates and explicit ascriptions retain exact operand roots and distinct
operation/result stages, including no-op ascriptions whose HIR wrapper is erased.
Operand completion precedes result availability; never operands retain entry only.
Target construction keeps its original compile-time order and error precedence,
including target errors after a never operand. Required target reads are not
runtime operand links. Stage publication follows existing ascription acceptance
checks and retains neither target descriptors nor evaluated predicate answers.
These bounded links preserve boolean predicates, E208, ownership and proof gates;
they introduce no runtime cast or additional type-shape copies.
Ordinary dispatch blocks retain exact receiver roots and the existing `$` local
and body identities. Entry reaches receiver evaluation through the block prefix;
receiver completion precedes initialization of that local and the immediate
checked body successor. An empty body completes after initialization. Synthetic
prefix markers remain explicit. A successfully checked forward group is the next
source point; unknown entries are not skipped. Stopped receivers have no
initialization/result link, and stopped bodies have no result link. These bounded
stages preserve
expected typing, nested receiver scope, permission/lifetime checks and call-return
conditions without inventing source statements.
Grouped composition links exact checked child roots, and plain partial blocks
connect through existing body endpoints while allowing outer fields to be supplied
elsewhere. Composed dispatch retains its separate partial-record checking path
and now captures receiver/local/body identities for the same bounded prefix stages.
Partial mode, expected slots and caller-specific error order remain unchanged;
the ordinary caller's early exclusive-receiver gate is not imposed here.

Dispatch reports now retain independent receiver-initialization and result visits,
with the exact input point, receiver local, body, checked body completion and control.
Receiver completion is captured separately before HIR transfer. Stopped receivers
produce no dispatch observation; stopped bodies can retain initialization alone.
The body keeps its own effects, and conditional receiver calls keep their return edges.
An observation does not prove reachability or copy/move authority.

Validation checks complete points, owners, parent/block relationships, the body's
synthetic receiver Bind, local/dispatch registration and the leading None slot.
Immediate and final statement sites must match their checked identities. Exact
dispatch and body endpoint edges preserve empty bodies and opaque successors;
initialization never skips an unknown entry to reach a later statement.
Each record has fixed-size metadata and shares work/map limits with other effects.
Receiver values and body statements are not copied. Duplicate visits merge flags;
conflicts or exhaustion publish no partial collection. Ordinary/composed checking,
receiver immutability, loans, lifetimes and proof-outcome gates remain unchanged.

Record-context equality retains both exact composed operand roots for ordered
sequencing, including scalar-primary comparisons. Existing hinting, contextual
widths, projections and once-only effects are preserved.
The composed fallback retains its exact nested source and classifies forwarding,
new conversion and stopped input before coercion changes the HIR type. Forwarding
links source completion directly to the result; conversions use a distinct
operation stage. Stopped inputs retain entry only, even when existing coercion
gives them a non-Never result type. Existing inner wrappers are not mistaken for
new conversions. These bounded links retain no aggregate type copies and do not
replay acceptance; the final type does not prove that a stopped operand returns.
Non-required expected-value contexts retain a caller root and a distinct raw
source, preserving inner branch/call identities and their result conditions.
Captured expected-value decisions connect forwarding, optional primary extraction
and conversion without bypassing raw effects. Direct Never retains entry only;
a Never primary reaches projection but has no result link, even when coercion
changes the final HIR type. Existing inner wrappers are not reclassified as new
work. Shared reborrows and unchanged shared forwarding retain their own paths.
Required checking and source-free expected-value helpers gain no runtime stages;
logical budgets, typing, E207 and loan authority remain unchanged.
Ordinary non-short-circuit binaries retain exact operand roots and actual primary
projection decisions. Entry reaches left evaluation, its projection precedes right
evaluation, and right projection precedes the operation. Binary stages and the
adjusted operand sequence publish atomically; no parallel link skips projection.
Integer arithmetic requires a checked-success edge for overflow/divisor checks;
floating arithmetic, bit functions and comparisons use ordinary result edges.
Stopped operands or projections suppress later stages. Short-circuit graphs keep
their existing branches. Required-only value construction adds no runtime nodes,
and required AST checking retains its prior sequence-only metadata. These stages
retain no aggregate type copies and do not evaluate proof outcomes.
Projected shared borrows retain bounded plans captured during checking: exact
parent roots, new temporary local/statement identities, owned field reads,
intermediate reference loads, final field-address paths and existing reborrow
sites/modes. Temporary materialization follows source evaluation only when the
parent helper creates that storage; existing reference values are not materialized
again. Stage edges follow the checked order into reborrow and result availability.
The final address path does not load its referent. Stopped parents have only an
entry link, and unknown call effects retain their own return conditions. These
links use the shared edge budget without inventing source points, reconstructing
order from spans, extending lifetimes or granting loan authority.
Owned field steps also retain the actual narrowing decision. A captured conversion
follows that field selection before further projection, an intermediate load or
reborrow. Nested narrowed fields retain separate conversion stages, and unchanged
fields gain none. Fields selected after an intermediate load preserve their
existing behavior without narrowing. Conversion edges share the graph budget while
path-step counts retain their existing limits. Guard requirements, stopped parents,
temporary materialization, site/mode identities and loan checks remain unchanged.
Ordinary place borrows retain the checked local/field path, canonical emitted-slot
storage and shared/exclusive mode. Root and field-address stages lead to reference
creation without an evaluated operand or pointee read. Field/type validation uses
the source local's declared type, retaining that identity when aliases share a
canonical slot. Shared reference-cell borrows retain the cell's identity. Exclusive
scalar targets preserve root/final-field permissions, record-shape restrictions and
identical emitted backing requirements. Wider exclusive shapes remain gated;
alias bookkeeping and loan/lifetime checks are unchanged.
Standalone temporary borrows retain exact initializer roots and existing local/
statement identities. Initializer completion precedes cell materialization and
reference availability; `never` inputs allocate no cell and gain no result edge.
A reference-valued initializer is stored in a distinct reference cell. These links
preserve the original statement lifetime and do not duplicate projected or indexed
parent staging. Type comparisons are charged; publication uses the shared edge budget.
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
locations or matching fields remain incomplete. Shared input chains ending in
borrowed records expand to those locations before returned-cell matching, including
nullable targets. Temporary chains retain their ordinary statement lifetimes.
Nested borrowed-view fields also participate in returned-cell matching through a
bounded type/location worklist. Shared visit and depth limits apply across all
descendants, even when their locations are unknown or empty. Direct shared results
pointing to concrete records containing references retain locations from all
exact-compatible shared arguments, including copies and nested calls. By-value
record containers also contribute matching stored views through nested/nullable
fields, copies and composition; null containers contribute no locations. Unknown
matching fields stay incomplete. Shared input chains can supply exact-compatible
inner record-view locations through bounded expansion, including chains stored in
record fields. Unknown intermediate cells remain incomplete and temporary chains
retain ordinary statement lifetimes. Borrowed-record inputs also contribute
projected subrecord locations and nested stored views through the bounded
location matcher, including mixed and unknown candidates. Known record
locations do not make unknown contained-reference origins complete. Returned
shared record views may target a nullable single-record shape as well: a null
value still has its real storage location but contributes no contained reference
owners. Returned shared carrier chains may end in borrowed records as well.
Compatible intermediate carrier locations are retained before terminal expansion,
including carriers stored in borrowed records. Unknown layers remain incomplete.
Direct record-valued calls retain ordinary shared-reference field origins through
public argument matching, including nested named fields, copies and nested calls.
All compatible arguments remain candidates; unknown inputs keep snapshots
incomplete. Carrier-cell fields also retain locations through copied/nested results
and later origin queries, including borrowed-record views. Direct carrier-field
access on call expressions is supported. Direct ordinary reference projections and
projected subrecord copies/compositions retain the same origins and carrier
snapshots through bounded owned-field paths. Source queries share nested-call
depth without replaying calls. Shape-preserving nullable record wrapping and
narrowing retain these call origins, including projected subrecords and carrier
fields. Field and coercion traversal share the bounded path budget. Known null
contributes no owners; unknown arguments remain incomplete. Wrappers that change
record shape and heterogeneous result layouts remain separate. Internal path
discovery retains bounded record-shape selections at heterogeneous union boundaries
and marks unsupported borrowed alternatives. Positional origin storage excludes
these qualified paths. Separate bounded snapshots own shape selections and retain
ordinary origins and carrier locations without crossing shapes. Local narrowing
reads select exact keys; absent entries stay incomplete. Ordinary bindings
capture root record-to-union widening and exact union copies.
Known null contributes no owners, while unknown sources remain incomplete. Copies
preserve their prior snapshots. Whole-container and addressed-prefix dependency
traversal follows shaped origins and carrier locations, including later pointee
marks; marked guards still cannot narrow ordinary types. Nested immutable
union-field construction captures checked initializer alternatives, including
conditional and nullable fields. Subrecord copies/projections preserve field and
shape offsets through outer narrowing. Alternative owners are combined; unknown
sources keep snapshots incomplete. Composition temporaries retain immutable
snapshots and map destination field names to source indices and shape offsets.
Mixed direct/composed alternatives share the same bounded merge; later source
replacement does not change prior copies. Capture does not replay initializers.
Named emissions capture shaped snapshots before slot registration.
Sibling aliases merge possible owners and carrier locations at the canonical slot
root; missing alternatives remain incomplete. Lexical reads and later dependency
marks use the shared root, while ordinary copies keep prior snapshots.
Ordinary mutable bindings retain old and new owners across
whole-value replacement, including nested union containers and carriers. RHS
snapshots are built before storage changes, so self-assignment reads prior metadata
and failed merges preserve it. Earlier copies retain their snapshots. Null or
independent overwrites do not erase earlier owners; unknown alternatives remain
incomplete. Mutable named slots also merge whole-slot retargets at the canonical
root, preserving sibling visibility and earlier copies. Completed mutable fields
and mutable descendants capture final canonical slot snapshots, retaining lexical
updates. Owned concrete-record field/subrecord writes merge qualified prefixes and
preserve sibling metadata and prior copies. Writes through union interiors retain
their concrete-storage gate. Statement-owned temporary values retain shaped
snapshots on their existing storage IDs. Direct dereferences and reborrows recover
those snapshots through bounded concrete field paths, keeping external pointee
owners distinct from temporary storage. Copies retain their contents after source
replacement; null/unknown distinctions and E303 expiry remain unchanged. Named
shared views of record/null unions retain bounded owner locations through aliases,
stored fields and retargets. Dereference reads merge exact shaped snapshots across
those locations, preserving prior copies, null contents and unknown alternatives.
Concrete record prefixes are checked before applying field indices; prefixes that
cross unselected heterogeneous shapes remain incomplete. E302/E303 ownership checks
are unchanged. Shared carrier chains ending at record/null unions retain locations
through bounded cell-layer expansion, including named/stored chains, retargets and
prior copies. Unknown layers remain incomplete; the new classification does not
cross exclusive edges. Returned carriers with at least two shared-reference layers
match record/null-union terminals through the public borrow contract, including
direct/deeper inputs and supported stored or projected record fields. All compatible
candidates remain possible; unknown candidates and untraversed unmatched union
inputs keep results incomplete. Nested-call depth and analysis budgets remain shared,
and matching does not replay calls or inspect private bodies. Direct returned
shared record/null-union views also retain exact owner locations through direct,
deeper and stored shared inputs. Exact owned union projections from borrowed
records retain concrete field paths alongside compatible stored candidates.
Owned record/null unions also contribute exact/deeper shared-reference candidates
through variant-qualified stored snapshots. Direct and deeper shared-union arguments use
bounded cell expansion before the same discovery for record-view, union-view and
carrier results. Unknown intermediates or contents remain incomplete; exclusive
edges remain unsupported. Union views stored in owned record arguments use concrete
field paths before the same shared-layer expansion, including nested fields and
unknown contents. Borrowed-record union fields resolve their concrete field address,
read the stored view, then expand remaining shared layers through the same matcher.
Nullable and unknown owners retain their completeness state. Typed borrowed-record
continuations resolve variant-qualified snapshots, expand shared layers and traverse
concrete fields with cumulative depth across union-to-record transitions and the
shared work ledger. Supported borrowed record/null-union continuations use the
same traversal, preserving exact variant identities through differing layouts.
Unknown contents remain incomplete; unsupported members and exclusive edges retain
their gates. Different layouts never become ordinary
field paths. Unsupported borrowed contents and owned projections inside variants
remain incomplete. Shared concrete-record field reads and copies resolve stored
origins through all known returned locations, including hidden union candidates.
Concrete owner prefixes and leaf types are checked; unknown alternatives remain
incomplete. Nested/nullable records retain their paths and null contents. Stored
shared carrier fields also recover their cell locations through these views,
including deeper carriers and stored record/union views. Unknown
owners or contents stay incomplete. By-value union calls retain variant-specific
origins for supported shared reference leaves with reference-free pointees, matched
against every compatible public input. Copies and nested result paths keep those
origins; callee bodies and runtime variants are not selected. Supported shared
carrier-cell result leaves also retain public-contract locations, including deeper
carriers and stored record/union views. Origin and cell completeness stay independent;
unknown and unsupported inputs remain incomplete. Owned by-value record/null-union
arguments contribute compatible shared-reference origins through exact variant-key
snapshots, including nested owned fields and inline call wrappers. Null contents
contribute no origins; no private runtime variant is inferred. Shared carrier leaves
also expand stored cell snapshots to reference-free terminal origins, preserving
unknown layers and compatible record projections. Borrowed concrete-record and
nullable-record terminal contents also use bounded location-based traversal,
including stored references and owned-field projections. Structural depth includes
the enclosing variant path. Supported borrowed heterogeneous record/null-union
terminals resolve exact location-backed snapshots through cumulative record/union
transitions, preserving null and unknown contents. Borrowed primary values and
unsupported members remain incomplete. Direct, deeper and record-stored borrowed
union origin arguments use the same variant-aware traversal, preserving caller
depth when fetching input locations. By-value union arguments also contribute
returned cell locations through expression-backed variant snapshots, including
record/union views, deeper carriers and typed continuations. Unknown candidates
remain incomplete; concrete prefixes never replace variant-qualified paths.
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
join rules, function summaries, restart backedges and termination dependence
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
