# meowy project status

Updated: 2026-09-25. This is the current project handoff; Git retains prior work.
[COMPILER.md](COMPILER.md) holds the implementation plan and
[compiler/STATUS.md](compiler/STATUS.md) the detailed compiler handoff.
Do not recreate STEP logs. The full documented v0.0.1 release remains incomplete.

## Documentation conventions

Agent responses default to English unless the user requests another language,
as specified in `AGENTS.md`.

Write the project name as `meowy`. Reader examples follow
[the documentation conventions](docs/guide/documentation-style.md), including
linked standalone examples. Tests, internal tooling and generated source retain
independent layouts. Explicit compact-syntax demonstrations and exact identifiers,
protocol bytes, output text and reference fixtures remain preserved.

Documentation and 82 standalone examples retain the standardized readable layout.
The prior token/literal preservation audit is `/tmp/meowy-doc-style-audit.json`;
Git preserves its completed commit series. Compiler guides remain in `compiler/docs/`.

## Explicit ascription and bits-module migration

`value~<T>` now performs proven ascription; `value<T>` is a type predicate in every
expression position. Ascription consumes one bracketed target and still requires
prior proof (E208). Union aliases and computed targets work. Generic calls and
`value<>` type queries retain their syntax and existing bootstrap limits.

Integer bit operations now use lexically resolved `@"bits"` functions
`and/or/xor/not`; their old operator spellings are rejected. Integer widths,
required evaluation, budgets, borrows, matchers and boolean operators are preserved.
Documentation, examples and Vim/Neovim cover the new syntax.

All 12 checks in `python3 -B tools/verify.py --compiler --editor both` passed,
including 1447 library/910 native tests. Log:
`/tmp/meowy-explicit-types-bits-gate.log`. The
[compiler handoff](compiler/STATUS.md#explicit-ascription-and-bits-module-migration)
lists the reviewed commits and remaining limits. Restart/proof work remains next.

## Dispatch receiver sigil migration

`$` now denotes the nearest dispatch receiver in the compiler, documentation,
examples and Vim/Neovim highlighting. Ordinary nested blocks retain it; nested
dispatches introduce their own. `self` is an ordinary name with no implicit alias.
Ownership, mutability and lifetime behavior are unchanged.

All 12 checks in `python3 -B tools/verify.py --compiler --editor both` passed:
1432 library/906 native tests, Vim/Neovim, lint, formatting, conformance, links and
schemas. Log: `/tmp/meowy-dollar-receiver-gate.log`. Native receiver tests run in
debug/release. The full composition project retains its pre-existing bootstrap
manifest/module-composition limitations. Restart/proof work remains the next task.

## Editor task and dispatch highlighting

Vim/Neovim recognize `%group` in operand positions, including group joins and
configuration methods, while binary remainder and ordinary borrows keep their
highlighting. Task-group editor slice: `27ad3e9`.

Simple/qualified callables in `.(function)` dispatch now receive function
highlighting, including multiline forms. Dispatches with extra arguments, such as
`"".(print, "")`, now recognize the callable before the comma; argument names
remain ordinary identifiers, with no argument-count limit. Ordinary parenthesized values, field
reads, comments and strings retain their previous groups.

`python3 -B tools/verify.py --editor both` passed all six checks for each slice;
logs: `/tmp/meowy-task-group-editor.log`, `/tmp/meowy-dispatch-highlight.log`,
`/tmp/meowy-dispatch-arguments-highlight.log`.
Those editor-only slices did not change compiler/runtime implementation.

## Task-group sigil documentation

Task groups now use `%name` in language/API references, tutorials and the documented
task program. `%name` identifies a group in its own namespace; `&name` still borrows
an ordinary value, and binary `%` remains remainder. Joining consumes an owned
handle/group; borrowing data for a task does not grant a borrowed join.

`python3 -B tools/verify.py` passed all four default checks; log:
`/tmp/meowy-task-group-sigil-docs.log`. Staged whitespace checks passed. This is a
documentation-only slice (`4b81c5d`); it did not compile or execute meowy programs.
The editor follow-up is recorded above. Task-group syntax remains unimplemented,
and the active compiler implementation handoff is unchanged.

## Scope-exit action documentation

The [deferred-action reference](docs/reference/values-and-blocks.md#deferred-actions)
specifies `<- expression` and `'scope <- expression`: delayed reads, reached
registrations only, LIFO cleanup
interleaved with owner release, per-iteration restart cleanup and static validation
of every exit path. Emitting an owner needed by an action is invalid. Module-level
actions run at initialization exit; entry-level actions run at entry exit. Neither
form installs a module-shutdown callback. Task joins precede deferred actions.

Labeled and unqualified registrations share the target's sequence. Names resolve
at registration; inner locals cannot survive by implicit snapshot or lifetime
extension. An inner restart can add repeated registrations to an outer target;
restarting that target runs and clears its sequence. Actions cannot register into
scopes outside their own execution boundary. Syntax, memory, diagnostics, style
and compiler planning now include these rules.

Labeled-action validation: `python3 -B tools/verify.py` passed all four default
checks; log: `/tmp/meowy-labeled-cleanup-docs.log`. Staged whitespace checks passed.
No compiler/runtime/editor execution was performed for this documentation slice.

Documentation slices: `ca4b2a4` specifies the core syntax/value/ownership contract;
the integration slice links module/task lifetimes, diagnostics, style and the
implementation plan. Default verification passed for the core contract; log:
`/tmp/meowy-deferred-docs-core.log`. Final integration validation passed all four
default checks with `python3 -B tools/verify.py`; log:
`/tmp/meowy-deferred-docs-final.log`. Staged whitespace checks passed for both
slices. No meowy source was compiled or executed by these checks.

Compiler parsing, registration, exit-path analysis and lowering remain unimplemented.
The ordered implementation slices and required behavioral evidence are recorded in
`COMPILER.md`. Compiler/runtime/editor execution and release qualification are not
claimed. Existing proof implementation priorities below remain unchanged.

## Current milestone

The logical ledger now charges type materialization, evaluated required statements
and blocks, selected integer/boolean evaluation, and scalar/record projection
paths. Required type literals, aliases and annotations now charge source nodes
before normalization, retaining duplicate inputs and work before errors. Type
literals, reads, queries and subtraction also charge expression steps;
grouping and synthetic wrappers add no logical construction costs. Required record
construction and recursive copies now charge aggregate slots; composition transfers
charged slots without duplication. Retained record reads charge without replaying
initializer work. Ordinary aliases and data annotations now have constructor
roots; nested extents share their budgets and computed operands restore input
mode afterward. Function definition/forward/export signatures now have roots;
body locals reuse checked parameter types without replaying annotations. Standalone
ordinary extents retain isolated roots and input gates. Ascription/type-test
targets and ordinary type-identity bindings now charge execution separately from
lookup, preserving ordinary extent restrictions and query operand isolation.
Field hints now restrict symbol resolution to name/import chains, preventing
computed-type bases from spending operand work or leaving sticky budget failures.
Nested roots retain original spans
and sticky failures; grouping, form checks and skipped branches spend no evaluation
steps. Pending queries retain shared argument/root budgets. Remaining accounting domains
and proof evaluation stay incomplete.

The partial proof package now exposes typed revision metadata and opaque descriptor
type aliases through local bindings and file facades. Type-only copy queries now
retain pending metadata and fixed Result signatures until ordinary typing and
ownership checks finish, then fail B001 before code generation. Evaluated results,
observations and assertions remain unimplemented.

Bounded `!<U>` subtraction now constructs normalized concrete type sets in annotations,
aliases and required expressions. Static queries, imported values and bare type blocks
retain identity, original failures and work. Removing all members produces `never`;
both operands are still evaluated. Subtraction does not validate nullable runtime data.

Commits: `bc3030c` (parser), `ae68e3b` (evaluator), `e5579d3` (integration).
The [guide](compiler/docs/COMPUTED_TYPES.md#type-subtraction) covers syntax, budgets
and remaining limits. Its example prints `7` in debug/release. The full compiler gate
passes. Mixed adjacent union suffixes, literal subtypes and broad bases remain gated.

Required type equality, including bare type blocks, remains supported with scoped
inference and scalar-context checks. No runtime type storage is created.

Named immutable `core.Type` exports continue to retain concrete payloads through
explicit facades. Private names, separate namespaces and module startup stay intact.
The compiler entry guide is [compiler/README.md](compiler/README.md); detailed guides
live in `compiler/docs/`.

Net/HTTP/TLS still needs broader generic-type/I/O/task foundations. Full v0.0.1
release qualification remains incomplete.

The first executable proof series is now planned: bounded type-only copy queries,
static descriptors and assertions. Phase/dependency tracking and logical root
accounting must precede query execution. Partial module/revision metadata and
direct required reads pass focused checker/native tests and the full compiler gate.
Proof evaluation remains unimplemented. Opaque descriptor type aliases now preserve nominal identities
and reject runtime storage; the final compiler gate passes.
Pending query copies retain source origins without runtime storage. Calls now
charge argument construction and retain their shared outer logical budget through
root completion, including tail work and failures. Remaining descriptor execution,
text/helper admission and phase/dependency work still gate query outcomes.

## Proof dependency evidence

Initializer evidence preserves proof-dependency marks through scalar operations,
selected conditions, tails and record projections; required reads reject marked
inputs with E225. Structural HIR inspection now also retains skipped operands,
block successors and call arguments. Ordinary binding copies preserve those marks
without turning them into constants. Marked guards provide neither fixed outcomes
nor boolean correlations to base typing/ownership checks. Seeded tests retain E302
under marked false conditions, including short-circuited expressions.

`f0288d6` adds structural inspection and binding/constant isolation; the guard slice
isolates boolean guards. All seven focused dependency groups and all ten compiler
checks pass. Source-level flags remain gated. Function result summaries,
aliased writes and control dependence after conditional exits remain prerequisites
to evaluated query outcomes.

Matcher bodies now retain lexical proof-control marks on ordinary bindings and
initializer evidence. Control and lexical scopes restore after errors; independent
following statements remain unmarked. Three new groups and all 990 library tests
pass. `4ab86be` records that slice. Pending queries now retain lexical control and
report E225 after ordinary typing/ownership, including controlled queries following
an independent pending query. Four focused query groups pass; the full compiler
gate passes all ten checks. Flags remain gated.

Direct mutable assignments now preserve marked RHS and lexical-control
dependencies for subsequent copies, guards and query availability. Four new groups
and all 998 library tests pass. Marks are conservative and monotone; independent
overwrites do not yet erase them. `aa89513` records the direct-write slice. Owned
field/list writes now include RHS/index/control dependencies using whole-owner
marks. Three focused path groups pass; all ten compiler checks pass. Alias
and indirect-store propagation and precise overwrite/join rules remain pending;
flags remain gated.

Named emitted bindings now preserve initializer/control dependencies. Emitted-slot
aliases share marks through their existing canonical storage root, including
later sibling aliases and completed record reads. Four focused groups pass;
all ten compiler checks pass. Borrowed-reference aliases and indirect stores
remain separate, and marks remain conservative. Source-level flags stay gated.

Immutable scalar-reference bindings now retain known single-owner links through
copies and reborrows. Later owner marks reach those reference reads without
replacing borrow/loan validation. Three new groups and all 1008 library tests pass;
`64dd1b0` records those links. Indirect stores now propagate marked RHS, target
and control dependencies to known owners after ordinary checks; marked unknown
origins report B001. Four focused store groups and all ten compiler checks pass. Mutable/aggregate/call-returned reference origins remain untracked and
source-level flags remain gated.

Reference links now use bounded owner sets with explicit completeness. Unknown
origins preserve known possibilities without being treated as proof evidence;
marked stores still gate incomplete origins. All 1014 library tests pass, including
new merge/snapshot and capacity regressions. `44a59fb` records this representation.
Mutable scalar-reference assignments now merge old/new owner sets, preserving
prior copies and incomplete origins. Four new retargeting groups pass, including
borrow/loan validation; all ten compiler checks pass. Marks/owner sets remain
conservative and source-level flags remain gated.

Named scalar-reference emissions now retain origins at their canonical slot root.
Sibling aliases merge possible owners/completeness and observe later retargets;
ordinary copies preserve prior origin snapshots. Four focused groups pass, with
borrow/loan checks for shared-reference fixtures and explicit preservation of the
exclusive-reference-carrier B001 gate. All ten compiler checks pass. Completed
record projections and function-returned origins remain separate; flags stay gated.

Immutable ordinary records now retain flat immutable scalar-reference field
origins through direct block initialization and record copies. Direct local-field
projections observe later pointee marks. Five focused groups pass, including
ordinary borrow validation and bounded metadata; all ten compiler checks pass.
Nested/coerced/composed and call-returned record origins remain incomplete.
Implementation: `0d45f39`; the foundation guide now documents this boundary.

Mutable ordinary record bindings now retain flat reference origins through
whole-record replacement. Old/new owners merge conservatively; earlier record
copies keep snapshots. Three new groups and all 1030 library tests pass, including
ordinary validation of conditional replacements. `d1d5f21` records that slice.
Direct mutable reference-field writes now merge origins while preserving earlier
copies and unrelated field metadata. Three new groups and all ten compiler checks
pass. Nested/indexed aggregates and returned records remain incomplete; flags
stay gated.

Record origin metadata now uses ordered field paths. Seeded nested-field lookups
preserve distinct owners and incomplete missing paths; all 13 focused record
groups and all ten compiler checks pass. This is a representation prerequisite:
nested source construction and subrecord writes are integrated below.

Record source-origin lookup is now separate from storage merging; all 1035
library tests pass. Nested path construction and writes are the next integration
slice. The supported source boundary is unchanged by this extraction. Nested source
lookup and seeded subrecord-copy lookup now also pass all 1035 library tests;
nested producers and write updates are now integrated. Origin traversal is bounded
to 32 levels and 256 visited record fields; scalar/subrecord writes merge selected
paths and preserve sibling metadata and prior copies. All five nested-source
groups and all ten compiler checks pass. Indexed/coerced and returned origins
remain incomplete; flags stay gated.

Record composition now snapshots reference origins on its existing temporary and
maps destination fields to source names/paths. Nested/conditional compositions
retain possible owners and completeness; later source writes do not alter the
snapshot. All five focused groups and all ten compiler checks pass. Indexed,
coerced and call-returned origins remain incomplete, and flags stay gated.

Nullable records with one record shape now retain origins through checked wrapping,
narrowing, copies and replacement, including nested nullable fields. Known null
contributes no owners; unknown sources and heterogeneous record unions remain
incomplete. All four focused groups and all ten compiler checks pass. Flags stay gated.

Shared element borrows now retain their container origin through reference-free
list/record views, copies, nested indices and scalar reborrows. Three new groups
and all 68 dependency groups pass, including ordinary ownership validation.
Returned/temporary and reference-bearing aggregate origins remain incomplete;
all ten compiler checks pass and flags stay gated.

Reference-free list/record views now retain owners in aggregate fields, including
nested/composed/nullable records and supported mutable record-view updates. Three
focused groups pass with ordinary ownership validation and snapshot checks.
Mutable list-view fields retain their existing B001 gate; returned and reference-
bearing aggregate origins remain incomplete. All ten compiler checks pass.

Temporary borrows now retain their explicit statement-owned storage IDs as
origins and preserve initializer/control marks. Three focused groups pass,
including unchanged E303 expiry. Direct temporary reads consult those marks;
all ten compiler checks pass. This does not resolve reference values copied
out of temporary carriers or extend their lifetimes. Flags stay gated.

Direct temporary carriers now snapshot their reference/record contents. Copies
recover external pointee origins through direct dereferences and empty-path
reborrows, without confusing them with the temporary cell's ID. Three focused
groups pass, including nested fields, unknown calls and unchanged E303 expiry.
All ten compiler checks pass; indirect carrier chains remain incomplete.
Implementation: `f4c2031`; the foundation guide documents the boundary.

Immutable one-level reference-cell aliases now retain their checked root/field
location. Direct/named cell reads recover stored pointee origins and preserve
prior value snapshots. Three focused groups pass, including ordinary ownership
validation and E302 cell protection. Mutable carrier aliases and unknown contents
stay incomplete; all ten compiler checks pass and proof flags stay gated.

Reference-cell metadata now uses bounded location sets and completeness rather
than a single location. All 1065 library tests pass, including capacity failure
and snapshot coverage. `393b939` records that representation. Mutable one-level
carrier retargets now merge possible cell locations while preserving earlier
carrier/value copies and incomplete alternatives. Four new source groups and all
eight cell groups and all ten compiler checks pass. Flags remain gated.

Reference-origin traversal now charges the analysis budget and rejects oversized
merged pointee sets explicitly. All 1070 library tests pass, including new budget
and capacity coverage. `ea02215` records that prerequisite. Bounded named/temporary
carrier chains now resolve one cell layer per dereference, preserve retarget/copy
snapshots and follow dependency marks without cycling. All five chain groups pass;
all ten compiler checks pass. Deeper field-stored carriers and returned origins
remain incomplete; flags stay gated.

Lexical named carrier emissions now register cell locations and share canonical
cell sets across sibling aliases and retargets. Ordinary copies keep snapshots.
Four focused groups pass; completed-record carrier-field lookup remains incomplete.
All ten compiler checks pass; proof flags stay gated.

Checked record source locations are now separate from pointee merging, preserving
known-null, unknown, direct-path and alternative-source distinctions. Both new
classification groups, all seven source-lookup groups and all ten compiler checks
pass. Record carrier-field snapshots and updates remain the next integration.

Record carrier-cell storage and location reads are in place. All 1084 library
tests pass, including seeded completed fields and inline named-field sources.
`aaa2a07` records the read-side prerequisite. Snapshot producers and matching
field/subrecord updates are now integrated, including nested/composed/nullable
records and prior copies. All seven record-cell groups and all ten compiler checks
pass. Returned origins and broader reference-bearing aggregates remain incomplete.

The scalar-reference return-candidate rule is now shared with origin analysis.
All 1089 library tests passed the extraction (`173d285`). Scalar-reference call
origins now retain compatible argument owners using that contract, with bounded
nested-call traversal and unknown-input completeness. Four call groups and all 108
dependency groups and all ten compiler checks pass. No function body is evaluated
or specialized by this analysis, and proof flags remain gated.

Shared returned views of lists/records without borrowed components now retain
compatible projected argument owners using the existing borrow-contract relation.
Three focused groups and all 111 dependency groups pass. Unknown inputs stay
incomplete; all ten compiler checks pass. Callee effect/data/control summaries
remain separate and proof flags stay gated. Concrete by-value record arguments
now retain nested named shared-reference fields using the same contract. Nullable
record wrappers and nested nullable fields reuse those snapshots; known null adds
no owners and unknown matching fields stay incomplete. All 16 call-origin groups
and all ten compiler checks pass. Bounded shared carrier chains now resolve
stored view owners through named, temporary and record-stored cells. Cell-layer
expansion is shared with ordinary dereferences (`020d736`). All 128 dependency
groups and all ten compiler checks pass. Borrowed-record references now retain
root/field locations through copies, retargets and record-stored views. Dependency
reads follow addressed record prefixes. All 132 dependency groups and all ten
compiler checks pass. Direct shared record-view calls now retain matching stored
reference owners and owned-field projection owners. Carrier-valued fields now
reuse bounded shared cell expansion (`0cda5fa`), including nested and retargeted
fields. Shared chains ending in concrete borrowed records now resolve those
records before matching stored and owned-field candidates. Nested borrowed-record
view fields now use a bounded type/location worklist, preserving unknown
alternatives. References to nullable single-record targets now reuse this
traversal, including known-null and nested views. All 153 dependency groups and
all ten compiler checks pass. Returned shared chains now retain compatible
argument cell locations through copies and nested calls, including matching inner
layers of deeper inputs. Unknown alternatives remain incomplete. All 162
dependency groups and all ten compiler checks pass. By-value record arguments
now contribute stored carriers through nested/nullable fields and copies. Direct
borrowed-record arguments now also contribute projected reference-cell locations
and stored carriers, reusing field projection from `a4686cb`. Shared input chains
now expand to borrowed-record locations before returned-cell matching. Nested
borrowed-view fields also contribute projected/stored cells through a shared
bounded worklist. Direct returned borrowed-record views now retain exact-compatible
argument locations through copies and nested calls; unknown contents remain
incomplete in later origin queries. By-value record containers now supply matching
stored views through nested/nullable fields, copies and composition. Shared chains
now supply exact-compatible inner locations for returned record views. Borrowed
record arguments also supply projected and stored view candidates through the
shared matcher (`136c9bf`). Returned views of nullable single-record targets now
retain real storage locations for null and non-null values. Returned shared
carrier chains ending in borrowed records now retain compatible intermediate
cells as well as projected/stored candidates. All 208 dependency groups and
all ten compiler checks pass. Direct record-valued calls now retain ordinary
shared-reference field origins, including nested fields, copies and nested calls,
using the extracted matcher (`7da170e`). Carrier-cell fields now retain locations
too, including record views, copies and nested calls. Explicit result-type and
depth prerequisites are `ee3a574` and `862eea2`. All 216 dependency groups and
all ten compiler checks pass.

Checked statements now retain bounded identities, function/block ownership,
same-function containment and explicit completion after successful checking.
Required reads and original pending queries retain these sites; copies preserve
query identity and required roots. All ten compiler checks pass, including 1478
library/910 native tests; `/tmp/meowy-checked-sites-gate.log`. The expression and
branch-point integration below extends these statement identities.
Proof outcomes remain gated. Statement-site prerequisite: `d8f7f64`.

Runtime expression checking now retains bounded point IDs with statement/block/
function ownership, same-function parents and completion state. Four focused groups
and all 1482 library tests pass; `/tmp/meowy-expression-points-lib.log`.
Existing continuation-budget diagnostic precedence is preserved.
Required reads and original queries now retain individual points, preserving
query-copy identity and logical roots. Four focused groups and all 1486 library
tests pass; `/tmp/meowy-required-points-lib.log`. Runtime branch-region capture
is implemented for matcher conditions/arms and both short-circuit alternatives.
All four focused branch groups and all ten compiler checks pass, including 1490
library/910 native tests; `/tmp/meowy-branch-points-gate.log`. The HIR source-link
integration below extends these points. Continuation/result transfers and restart
propagation remain incomplete.
Expression prerequisite: `10a45ec`; required/query integration: `b4a79de`.

HIR matchers and short-circuit expressions now retain explicit checked point IDs;
ordinary and synthetic binaries/branches retain no invented sources. Body facts
validate point kind, function, block and completion, linking erased query/read
regions without matching source spans. Clones preserve IDs; repeated checks remain
distinct. All ten compiler checks pass, including 1501 library/910 native tests;
`/tmp/meowy-hir-branch-sources-gate.log`. Matcher HIR: `1038bbc`; matcher fact links:
`f2dcfcf`; short-circuit HIR: `a66a363`. Explicit continuation/join/result transfers,
restart propagation and proof outcomes remain pending.

Checked point boundaries now return exact IDs alongside checked values, preserving
nested/error restoration and existing budgets. Both focused groups and all 1503
library tests pass; `/tmp/meowy-point-results-lib.log`. Explicit matcher decision/
normal-join edges are now retained with bounded, atomic source validation. Both
boolean successors remain explicit; normal ports do not imply runtime reachability.
Four focused groups and all 1507 library tests pass;
`/tmp/meowy-matcher-edges-lib.log`. Short-circuit edges now retain the true route
for && and false route for ||, explicit bypasses and normal joins. All ten compiler
checks pass, including 1510 library/910 native tests; `/tmp/meowy-branch-edges-gate.log`.
Point-result prerequisite: `e201bd7`; matcher edges: `1e30544`. Region-content links,
generic sequences and explicit exit/result transfers remain incomplete. Propagation
and outcomes stay gated.

Coerced/uncoerced expression APIs now return exact root IDs without adding points
or changing diagnostics/accounting. Both focused groups and all 1512 library tests
pass; `/tmp/meowy-expression-roots-lib.log`. Matcher-body statement identities are
now retained for erased and runtime bodies, without new lifetime scopes. Both
statement-point groups and all 1514 library tests pass;
`/tmp/meowy-statement-points-lib.log`. Matcher condition/body regions now link
to their exact checked contents; normal region exits depend on child normal exits.
The shared branch/content edge budget is bounded and atomic. Four focused groups
and all 1518 library tests pass; `/tmp/meowy-matcher-content-lib.log`. Short-circuit
condition/RHS regions now retain exact expression roots, including grouped and
skipped operands. All ten compiler checks pass, including 1521 library/910 native
tests; `/tmp/meowy-region-contents-gate.log`. Expression roots: `ba3debb`; matcher
body points: `ee6c2bc`; matcher contents: `e981b0b`. General sequencing, explicit
exit/result transfers and outcomes stay gated.

General statements now retain exact root IDs through their existing lifetime
boundary, and completed sites identify those roots. All 1521 existing library tests
and the new erased/runtime/failure root group pass;
`/tmp/meowy-general-statements-lib.log`. Core block sequences now retain exact
statement order, including erased uses and explicit forward-group barriers. Four
focused groups and all 1526 library tests pass; `/tmp/meowy-block-sequences-lib.log`.
Ordinary binary operands now retain exact direct roots and explicit normal-to-next
edges; composed roots remain unknown and short-circuit routing stays separate.
All ten compiler checks pass, including 1530 library/910 native tests;
`/tmp/meowy-sequences-gate.log`. Statement roots: `aaffd39`; core blocks: `4286bae`.
Other operand families, contextual list/effect blocks, explicit exits/results and
propagation/outcomes remain incomplete.

Scope operations now retain explicit exit edges from checked statement entries to
leave targets or target/RestartId ports. Ownership, spans and control marks are
preserved after ordinary validation; exit edges share the existing budget and do
not grant normal fallthrough. Four focused groups and all 1534 library tests pass;
`/tmp/meowy-scope-exits-lib.log`. HIR leaves now retain optional source IDs, and
restart metadata retains checked points through RestartId. Both provenance groups
and all 1536 library tests pass; `/tmp/meowy-exit-hir-lib.log`. Body facts now link
exact exit sources and validate target/function identity while retaining original
call spans and unknown synthetic sources. All ten compiler checks pass, including
1540 library/910 native tests; `/tmp/meowy-scope-exits-gate.log`. Exit edges:
`1f26948`; HIR provenance: `90a2065`. Block/statement/result endpoints and
propagation remain incomplete; proof outcomes stay gated.

Core blocks now retain entry, normal-completion and result ports, including target-
leave joins. Empty blocks have explicit completion paths; forward groups and
synthetic receiver setup remain opaque, and never blocks have no result edge.
Four focused groups and all 1544 library tests pass;
`/tmp/meowy-block-endpoints-lib.log`. Plain block expressions now use exact
producer/result links, and expression statements depend on their child normal
ports. All 1547 library tests pass; `/tmp/meowy-block-consumers-lib.log`.
Block prerequisite: `b45829a`; consumer links: `4f266e4`. Restart ports now connect
to validated block entries through separately marked backedges, sharing the edge
budget and publishing atomically with source exits. All ten compiler checks pass,
including 1550 library/910 native tests; `/tmp/meowy-block-results-gate.log`.
Remaining statement/value transfers and dependency propagation stay incomplete.

Matcher statement roots now connect to exact branch IDs in source order, with
independent decisions and conditional normal completion. Three focused groups and
all 1553 library tests pass; `/tmp/meowy-matcher-roots-lib.log`. Matcher slice:
`ed3baf3`. The storage operations below extend these roots; propagation stays gated.

The 4096-alias stress gate now accepts the shared control-flow B001 budget as well
as the origin-specific cap; the 2048 case still requires the loan-analysis budget.
Focused stress validation passed (`1f7abb7`). Ordinary binding operations now
retain exact value roots, canonical storage IDs and explicit effect ports. Four
focused groups and all 1557 library tests pass; `/tmp/meowy-binding-operations-lib.log`.
Direct assignments now retain write operations with exact RHS roots and canonical
storage IDs, preserving slot aliases, reference cells and control marks. All ten
compiler checks pass, including 1561 library/910 native tests;
`/tmp/meowy-storage-operations-gate.log`. Binding slice: `0e93b48`. Module roots
without exact producers remain unknown. Emission operations and remaining
store/address links are next; complete value-flow and proof outcomes stay gated.

Composed expression checking now retains exact outer root IDs without changing
partial-record shapes, scalar fallbacks or original errors. Both new root groups
and all 1563 library tests pass; `/tmp/meowy-composed-roots-lib.log`. Direct
primary/named/outer emissions now retain exact source, EmitId, slot and alias
identities. Never/static outputs do not invent runtime emissions. Four focused
groups and all 1567 library tests pass; `/tmp/meowy-direct-emissions-lib.log`.
Record fanout now retains ordered primary/field projections from the original
input, with validated staging cells and atomic target publication. All four fanout
groups and all 1571 library tests pass; `/tmp/meowy-emission-fanout-lib.log`.
Body facts now use exact point/target indices to validate EmitId, owner and slot
identity, preserving unknown synthetic sources. All ten compiler checks pass,
including 1574 library/910 native tests; `/tmp/meowy-emission-operations-gate.log`.
Composed roots: `e4374cc`; direct emissions: `6bb0f49`; fanout: `c6fd7e5`.
Field/indexed stores now retain canonical targets, exact field/index paths and
checked RHS roots. Address stages precede the RHS; each index follows its
containing-list reservation/length capture and reaches its next address only on
bounds success. The write follows normal RHS completion. Existing mutability,
type, bounds and loan validation remain authoritative. Exact index roots:
`9f2708c`; field operations: `e26a99c`. All ten compiler checks pass, including
1583 library/910 native tests; `/tmp/meowy-path-operations-gate.log`.
Indirect scalar stores now retain target/RHS roots, address capture and write
stages (`003c7d4`). Bounded pointee-owner snapshots are captured before RHS checking,
so reference-cell retargeting cannot alter the earlier snapshot. Unknown origins
remain incomplete, and projected writes retain owner-level granularity. All ten
compiler checks pass: 1590 library/910 native tests;
`/tmp/meowy-indirect-stores-gate.log`. Remaining operand links,
precise write locations, propagation and proof outcomes remain incomplete.
Direct-function calls now retain exact callee/site IDs and argument roots
(`fdeb312`), including dispatch receivers first. Ordered argument completion leads
to an opaque call effect. A distinct callee-return edge permits continuation;
`never` results omit it. All ten compiler checks pass: 1597 library/910 native tests;
`/tmp/meowy-call-order-gate.log`. Debug formatting, list methods and
required/type-only call paths remain separate; no effect summary or proof outcome
is inferred.
List indexing now retains exact receiver/position roots and an explicit value and
length snapshot before position evaluation. Bounds success leads to projection
and result availability; nonreturning operands do not gain result edges. The
receiver helper (`77a4a7e`) preserves shared-list dereference and existing HIR.
All ten compiler checks pass: 1603 library/910 native tests;
`/tmp/meowy-index-order-gate.log`. Element-borrow, list literal/method and
contextual builder paths remain separate.
Concrete/inferred list literals now retain element roots by source slot and reuse
ordered sequences and construction/result endpoints (`18f8e70`). Deferred scalar
checking does not reorder runtime edges. Union contexts retain ordinary/deferred
roots. Empty lists have a construction stage; nonreturning elements prevent result
publication. Custom effect blocks now separate recognition (`312eda3`) from exact
root capture (`00cc9fc`) and connect checked statement/body-result links. Candidate
probes allocate no points; error paths restore scopes, owner and active point.
All ten compiler checks pass: 1617 library/910 native tests;
`/tmp/meowy-custom-elements-gate.log`.
List/string `size` now retain exact receiver and operation roots (`1fd816d`). List
`add` retains receiver/item roots, list/length snapshots and capacity-success
result edges. Nonreturning operands have no result path; receiver storage is not
mutated. All ten compiler checks pass: 1624 library/910 native tests;
`/tmp/meowy-method-order-gate.log`.
Shared element borrows now retain exact parent/index roots, checked place paths,
view boundaries and direct temporary ownership. Address/length capture precedes
index evaluation; bounds success admits the resulting reference. Parent helpers:
`ff1373c`. All ten compiler checks pass: 1629 library/910 native tests;
`/tmp/meowy-element-order-gate.log`.
Exclusive indexed-borrow checking now exposes exact path/index roots without
changing HIR or replaying effects. Formatting and all 1631 library tests pass;
`/tmp/meowy-exclusive-roots-lib.log` (`f078e24`). Exclusive operations now retain
canonical storage and ordered reservation/length, bounds-success and acquisition
stages. Nonreturning indices do not acquire a result. All ten compiler checks pass:
1634 library/910 native tests; `/tmp/meowy-exclusive-order-gate.log`. Operation
integration: `3920c12`. The foundation guide documents the boundary. Debug output
formatting now exposes exact operand roots alongside flattened HIR parts,
preserving static text and primary projection. Formatting and
all 1637 library tests pass; `/tmp/meowy-format-roots-lib.log` (`ff58f82`). Output
metadata now interleaves operands and streamed parts, preserves panic prefixes and
only completes fully evaluated messages. All ten compiler checks pass: 1640
library/910 native tests; `/tmp/meowy-output-stages-gate.log`. Debug/release native
probes also confirm partial output on scope leave. Output integration: `99208a3`.
The foundation guide documents the boundary. Ordinary grouped expressions now
link their entry/result ports to exact checked children, reusing the bounded
region ledger. HIR, expected typing and logical grouping costs are preserved.
All ten compiler checks pass: 1644 library/910 native tests;
`/tmp/meowy-group-links-gate.log` (`a3a44e3`). The foundation guide documents the
boundary. Scalar unary checking now exposes exact operand roots while retaining
contextual inference and primary projection. Formatting and all 1647 library tests
pass; `/tmp/meowy-unary-roots-lib.log` (`7932a09`). Unary metadata now retains exact
scalar types and ordered operation/results, with checked integer negation and no
result edge for stopped operands. All ten compiler checks pass: 1650 library/910
native tests; `/tmp/meowy-unary-stages-gate.log` (`fdeb564`). The foundation guide
documents the boundary. Explicit dereference checking now exposes exact pointer
roots while preserving HIR, types and loan errors. Formatting and all 1653 library
tests pass; `/tmp/meowy-deref-roots-lib.log` (`89d255f`). Explicit dereference
metadata now preserves pointer-before-load/result order and reference mode without
copying aggregate shapes or inferring pointee storage. All ten compiler checks
pass: 1656 library/910 native tests; `/tmp/meowy-deref-stages-gate.log` (`15f1c74`).
The foundation guide documents the boundary. Exclusive scalar reborrows now
expose exact parent roots while preserving HIR, site allocation and loan errors.
Formatting and all 1659 library tests pass; `/tmp/meowy-reborrow-roots-lib.log`
(`03f5e54`). Exclusive reborrow metadata now retains parent/site/mode identities and
ordered result availability without a dereference load. Stopped parents allocate
no site. All ten compiler checks pass: 1662 library/910 native tests;
`/tmp/meowy-reborrow-stages-gate.log` (`a6e9d05`). The foundation guide documents the
boundary. Direct shared reborrow checking now exposes exact parent roots while
preserving original diagnostics, aggregate types and site allocation. Formatting
and all 1665 library tests pass; `/tmp/meowy-shared-reborrow-roots-lib.log`
(`5332455`). Direct shared metadata now retains parent/result modes and existing
sites with bounded aggregate comparisons and no shape copies. All ten compiler
checks pass: 1668 library/910 native tests;
`/tmp/meowy-shared-reborrow-stages-gate.log` (`4c6c381`). The foundation guide
documents the boundary. Projected parent selection now exposes exact expression,
indexed-borrow and reference roots alongside unchanged HIR and temporary identity.
The root prerequisite (`0a540a8`) now feeds checked projection plans. Helper-created
temporary storage is explicit (`3133214`); owned field reads, intermediate loads,
final addresses and reborrow sites are captured during checking (`ea6a625`).
Projection stage edges preserve that order and depend on exact parent completion;
stopped parents gain no result, and callee return conditions remain explicit.
All ten compiler checks pass: 1680 library/910 native tests;
`/tmp/meowy-borrow-projections-gate.log` (`7ca8a3a`). The foundation guide documents
the boundary. Ordinary shared place-borrow operations are next; broader propagation
and proof outcomes remain incomplete.

## Pending descriptor statement accounting

Pending descriptor flag type queries now resolve their fixed `boolean` signature
without exposing an answer or replaying a query. The checker accepts aliases and
grouping while retaining value/capture gates and ordinary typing/ownership errors.
`b71feef` implements this prerequisite; native coverage and the guide complete its
integration. All ten compiler checks pass, including 972 library/903 native tests.
Transitive proof data/control dependencies remain next.

Pending bindings and expression statements now share one construction root across
statement, call/read and annotation work. Query ledgers retain annotation charges
and failures. Copies preserve their original query without replaying arguments;
independent statements reset budgets. Required-block descriptor construction stays
gated, and skipped required branches do not prepare queries.

`84ba2bc` separates recognition from preparation; `835386a` integrates statement
roots and copy reads. Native origin/admission coverage and all ten compiler
checks pass. Descriptor outcomes and transitive phase/dependency tracking remain
unfinished; no descriptor payload is materialized by pending metadata.

Record-call projections now share bounded owned-field paths. Direct reference
reads and projected subrecord copies/compositions retain ordinary origins and
carrier snapshots. Shape-preserving nullable coercions now also retain these call
origins and carrier locations. Heterogeneous record shapes remain separate;
proof flags/outcomes stay gated.

Record path discovery now retains bounded shape selections for heterogeneous
unions and explicit unsupported alternatives. Existing positional storage excludes
these paths. Bounded shape-keyed snapshots retain distinct layouts, and local
narrowing reads select exact shapes. Storage prerequisite: `95b0086`.
Immutable bindings with immutable fields now capture root record-to-union widening,
known null and exact union copies. Whole-container dependency traversal follows
shaped origins/carriers, including later marks. Prerequisite: `a67560b`.
Nested immutable union-field construction and subrecord projections now preserve
shape selections and merge checked initializer alternatives. Composition temporaries
now retain immutable snapshots; destination fields map to source names and shape
paths. Unknown alternatives stay incomplete. Merge prerequisite: `0bba0cb`.
Immutable named emissions now retain shaped snapshots. Sibling aliases merge at
canonical slot roots, while ordinary copies retain their prior snapshots. Canonical
storage prerequisite: `7ab6a30`. Ordinary mutable bindings with immutable fields
now capture shapes and conservatively merge whole-value replacements, preserving
prior copies and unknown alternatives. Construction prerequisite: `c915eeb`.
Mutable named slots with immutable contents now capture shapes and merge retargets
at their canonical roots. Siblings share possible owners; earlier copies remain
independent. Completed mutable fields and descendants now capture final canonical
slots. Owned field/subrecord writes merge bounded prefixes, retaining sibling
metadata and prior copies. Prefix prerequisite: `7b491bc`. Statement-owned
temporaries now retain
shaped snapshots, including concrete projected reborrows, without extending
lifetimes. Read prerequisite: `dfb1e47`. Named shared record-union views retain owner locations,
and dereference reads merge exact snapshots through concrete prefixes. Location
prerequisite: `758c3fc`. All 319 dependency-filtered tests and all ten compiler checks
pass. Shared carrier chains ending at record unions now retain locations through
named/stored layers and conservative retargets. All 325 dependency-filtered tests
and all ten compiler checks pass. Returned carriers with at least two shared layers
now retain public-contract candidates for record/null-union terminals. Unmatched
union inputs remain incomplete. Terminal prerequisite: `d85ec70`. All 332 dependency-
filtered tests and all ten compiler checks pass. Direct returned shared union views
now retain exact owner locations through direct/deeper/stored inputs. All 338
focused tests and all ten compiler checks pass. Exact owned-union projections from
borrowed records now preserve concrete owner paths alongside stored candidates;
hidden exact/deeper shared candidates now use variant-qualified snapshots.
The location-reader prerequisite is `9969754`. All 350 focused tests and all ten
compiler checks pass. Concrete-record reference fields now resolve stored origins
through known shared view locations, including hidden union candidates. All 357
focused tests and all ten compiler checks pass. Stored carrier-field cells now
resolve through the same validated shared record views, retaining unknown owners
and contents. Validation prerequisite: `22a0e0c`. All 364 focused tests and all ten
compiler checks pass. Direct one-layer shared-union arguments now contribute hidden
candidates to concrete-record, union-view and carrier returns. Record-result slice:
`f5d001a`. All 370 focused tests and all ten compiler checks pass. Deeper direct
shared-union inputs now expand bounded cell layers before matching hidden candidates.
Unknown intermediates remain incomplete; exclusive edges remain unsupported.
All 374 focused tests and all ten compiler checks pass. Union inputs stored in
owned record arguments now retain candidates through nested concrete fields and
shared layers. All 379 focused tests and all ten compiler checks pass. Borrowed-record
union fields now read stored references before remaining shared-layer expansion;
nullable/unknown owners and fields remain conservative. Prerequisite: `40fc5e0`.
All 384 focused tests and all ten compiler checks pass. Hidden discovery now retains
borrowed-record continuation types, variant keys and shared-layer counts. Borrowed-record
continuations now resolve with cumulative depth across union-to-record transitions;
unknown contents retain their incomplete state. Bound prerequisite: `4c0b67b`.
All 392 focused tests and all ten compiler checks pass. Supported borrowed
record/null-union continuations now retain exact variant identities through the
same bounded traversal. All 397 focused tests and all ten compiler checks pass. By-value
union call reference origins now follow exact variant-qualified result types and
all compatible public inputs. Type lookup prerequisite: `b50f87a`. All 404 focused
tests and all ten compiler checks pass. By-value union carrier leaves now retain
public-contract cell locations independently from reference-origin completeness.
All 409 focused tests and all ten compiler checks pass. By-value union arguments now
retain shared-reference origins through exact variant snapshots, including nested
fields and inline wrappers. All 414 focused tests and all ten compiler checks pass.
Shared carrier argument leaves now expand exact cell snapshots to reference-free
terminal origins, preserving unknown intermediates. All 419 focused tests and
all ten compiler checks pass. Borrowed concrete/nullable-record contents now resolve
from variant-qualified cells with the enclosing structural depth preserved.
Traversal prerequisite: `1482d20`. All 425 focused tests and all ten compiler checks pass.
Borrowed heterogeneous-union terminal contents now resolve typed snapshots through
cumulative record/union transitions. Discovery prerequisite: `c9c5325`.
All 429 focused tests and all ten compiler checks pass. Direct/deeper/stored
borrowed-union origin arguments now retain variant-aware origins while preserving
caller depth. Prerequisite: `d600bb6`. All 435 focused tests and all ten compiler checks pass.
Returned cell locations from by-value union arguments now use exact expression
snapshots, preserving unknowns and typed continuations. Resolver prerequisite:
`1e97d1d`. All 440 focused tests and all ten compiler checks pass. Forward conditional-
leave continuation state now marks statement successors,
including writes and query availability, until the target scope joins.
All 447 focused tests and all ten compiler checks pass. Later expression operands
now refresh continuation control, including temporary owners and final coercion
rollback. Scope prerequisite: `bac55cf`. All 453 focused tests and all ten compiler checks pass.
Restart sites now retain bounded target/owner/span/control evidence under their
existing IDs. Pending queries now retain same-function active block IDs and link
to restart targets in either registration order; copies keep their original scope.
Checked runtime bodies now
retain bounded facts under block/function IDs, including nested-body links, bindings,
reads, writes, calls and exits (`87e7b20`). Successful required-input reads retain
storage-root IDs, source/root spans and lexical control in the same scope inventory.
Body facts now retain operand/branch parent links (`fe19008`) and canonical storage
IDs (`5099fd1`), including distinct short-circuit condition/RHS roles. All 22 focused
body groups and all ten compiler checks pass (1469 library/910 native tests); log:
`/tmp/meowy-body-relations-gate.log`. These relations do not establish execution
order or complete value flow. Erased query/input sites still
need explicit branch/continuation identities, and block/emission results need
consumer links before backedge/header propagation. Termination dependence remains incomplete.
Union-interior writes and proof outcomes stay gated.

## Actual validation

- Checked shared-borrow projection plans and stage edges passed all ten checks in
  `python3 -B tools/verify.py --compiler`: 1680 library/910 native tests.
  Conformance: 10 passed, 13 unsupported, 0 failed in debug/release.
  Log: `/tmp/meowy-borrow-projections-gate.log`. The graph remains partial;
  bounded dependency propagation and proof outcomes remain pending.
- `python3 -B tools/verify.py --compiler --editor both`: all 12 checks passed,
  including 1447 library/910 native tests (2357 total), 16 Python tooling and four
  compiler harness tests, Vim/Neovim, fmt, Clippy, build, links and catalog/schema
  checks. Conformance: 10 passed, 13 unsupported, 0 failed in debug/release.
  Log: `/tmp/meowy-explicit-types-bits-gate.log`.
- Explicit ascriptions retain E208 and ownership checks; predicates use comparison
  precedence in every expression position. Generic-call fallback, one-target
  ascription, union targets, bit-function widths/evaluation order, required budgets
  and contextual lists are covered. Native behavior runs in debug/release.
- Flags/outcomes remain B001-gated. Shape-changing wrappers, broader result shapes,
  allocator-bound analysis, heterogeneous unions, precise joins, callee effect/data/control
  summaries and conditional-exit control remain open. Runtime sources, reference
  fixtures, dependencies and versions are unchanged; separate runtime/sanitizer
  gates were not rerun. Full release qualification remains open.

## Area handoff

| Area | Current boundary |
| --- | --- |
| Compiler | Logical charges cover type-expression dispatch/materialization, required statements/blocks, scalar/record reads and constructed/copied record slots; query statement/annotation budgets are retained; descriptor outcomes and phase tracking stay open. |
| Documentation tooling | Constructed signatures and file graphs are checked; multi-file doc commands/indexes remain separate. |
| Editor integration | Explicit ascriptions, type predicates, dispatch receivers/callables and task-group sigils pass Vim/Neovim checks. |
| Standard library | `bits.and/or/xor/not` execute with existing integer widths and required-evaluation rules. Other bits APIs remain gated. Proof outcomes and Net/HTTP foundations remain separate. |
| Runtime and release | File-site runtime helpers passed native probes; platform/distribution qualification remains open. |

## Proof package reference

The [proof reference](docs/reference/stdlib/proof.md) specifies revision 1: opaque
compile-time results, value/range queries, type capabilities, immediate ownership
probes, snapshot rules, a canonical bounded analysis and strict phase separation.
`assert` requires Always; `expect<S>` tests any exact result, including Indeterminable.
The testing section covers compile-only contracts and negative diagnostics, alongside
a qualification matrix. E223/E224/E225 are registered in the diagnostic catalog.

Commits: `12d21df` (results/value APIs), `415f7c4` (ownership/canonical analysis),
`947c56b` (testing, work accounting, qualification and cross-links).
The docs, library index, compile-time rules, testing and memory APIs link the contract.
Default repository verification and whitespace checks pass. The query APIs, observation
parameter mode, proof-only descriptor unions and executable proof fixtures are not
implemented; no runtime/compile-time execution of proof examples was claimed.

## Release naming

[VERSIONING.md](VERSIONING.md) defines the project release policy: keep major/minor
at zero, use the patch field as a chosen release identifier, make releases without a
fixed cadence, and label pre-releases as uncertain. Numbers do not promise compatibility
or chronological order. The existing full-release target remains an implementation
milestone, and protocol/schema/contract revision rules remain independent.
README and COMPILER link this policy. No package version or release tag was changed.
All four default repository checks pass, including 16 tooling tests, local links,
catalog metadata and schemas. Log: `/tmp/meowy-versioning-docs.log`. Compiler/runtime
execution was not part of this documentation edit.

## Next steps

1. Add transitive proof data/control dependency tracking before enabling outcomes
   or flags, preserving E225 separation and ordinary typing/ownership checks.
   Expression/read/query points and explicit runtime branch regions now distinguish
   uses within a statement. HIR branches and body facts now retain validated
   source links. Branch decision/bypass/normal-join edges are now explicit.
   Region ports now link to exact checked contents; core blocks and ordinary binary
   operands retain explicit sequence edges. Leave/restart exits now preserve exact
   target ports and checked sources. Core block ports, plain block consumers and
   expression statements now connect; restart reentry is marked separately.
   Matcher roots, ordinary bindings and direct writes now retain explicit operation
   links. Direct and composed emissions now retain slot identities, source roots
   and projections. Field/indexed stores now retain address/index/RHS order,
   canonical paths and reservation/bounds-success stages. Indirect stores capture
   target/RHS roots and pre-RHS pointee origins, keeping incomplete origins and
   reference-cell identities distinct. Direct calls retain ordered argument roots,
   opaque effects and conditional return edges. List indices now retain receiver
   snapshots, ordered position roots and bounds-success stages. List literals retain
   source-ordered element roots and construction endpoints, including exact custom
   effect-block roots and body links after recognition. `size`/`add` retain receiver
   and item roots, snapshots and capacity-success stages. Shared element borrows
   now retain parent/index roots, address stages and temporary ownership. Exclusive
   paths retain canonical storage and ordered reservation/bounds/acquisition stages.
   Debug output retains exact formatting roots and interleaved output stages,
   including panic prefix, conditional output returns and stopped suffixes.
   Ordinary groups now link exact child entry/result ports through the shared
   region ledger, preserving HIR, expected typing and logical grouping charges.
   Scalar unary operations retain exact roots and result types, with checked
   integer negation and stopped-operand boundaries. Signed literals and required
   construction retain their own paths. Explicit dereferences now retain pointer
   roots and load/result order without inferring pointee storage or loan authority.
   Exclusive scalar reborrows retain parent/site/mode identities and ordered result
   availability without dereference loads. Direct shared reborrows now preserve
   requested result and actual parent modes with bounded aggregate comparisons.
   Projected shared borrows now connect captured materialization, owned-field,
   intermediate-load and final-address stages to reborrow/result availability.
   Next record ordinary shared place-borrow operations in the successful address
   branch of `compiler/src/check/references.rs::borrowed`. Preserve checked paths,
   canonical slot storage, mode and existing loan/lifetime rules without inventing
   operand or pointee reads. Validate focused tests and the compiler gate; exclusive
   place borrows, standalone temporaries, implicit conversions and remaining
   field/builder coverage stay separate.
   Required/type-only calls remain separate.
   Result availability is not complete value provenance.
   Other operand families and contextual
   list/effect blocks remain sequence-coverage gaps.
   Normal ports do not imply reachability. Preserve independent matcher arms,
   nested targets and unknown effects; do not
   infer runtime order from point IDs or source spans. Bounded backedge/header
   propagation remains unimplemented.
   Fixed flag type queries are independent of answers and must remain admitted.
   Pending statement and annotation roots are integrated; descriptor construction
   and type inspection must charge the retained ledger when outcomes are admitted.
   Required-block descriptors and text/helper execution remain gated. The
   [compiler handoff](compiler/STATUS.md#executable-proof-plan) records the boundaries.

2. Broaden subtraction only after its remaining syntax/representation prerequisites
   are established. Do not push, bump versions or claim full release qualification.
