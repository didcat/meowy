# meowy project status

Updated: 2026-09-23. This is the current project handoff; Git retains prior work.
[COMPILER.md](COMPILER.md) holds the implementation plan and
[compiler/STATUS.md](compiler/STATUS.md) the detailed compiler handoff.
Do not recreate STEP logs. The full documented v0.0.1 release remains incomplete.

## Documentation conventions

Write the project name as `meowy`. Reader examples follow
[the documentation conventions](docs/guide/documentation-style.md), including
linked standalone examples. Tests, internal tooling and generated source retain
independent layouts. Explicit compact-syntax demonstrations and exact identifiers,
protocol bytes, output text and reference fixtures remain preserved.

Documentation and 82 standalone examples retain the standardized readable layout.
The prior token/literal preservation audit is `/tmp/meowy-doc-style-audit.json`;
Git preserves its completed commit series. Compiler guides remain in `compiler/docs/`.

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
retain real storage locations for null and non-null values. All 203 dependency
groups and all ten compiler checks pass.

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

## Actual validation

- `python3 -B tools/verify.py --compiler`: all ten checks passed, including 1188
  library/903 native tests (2091 total), 20 Python harness tests, fmt, Clippy,
  build, links and catalog/schema checks. Conformance: 10 passed, 13 unsupported,
  0 failed in debug/release. Log: `/tmp/meowy-nullable-returned-record-views-gate.log`.
- All 203 dependency groups pass. Four new nullable-result groups cover null/non-
  null storage identity, copies/nested calls, stored/projected candidates, unknown
  locations, E303 lifetime rejection and heterogeneous-result exclusion. Accepted
  fixtures pass ordinary compilation/ownership; dependency marks remain seeded
  checker evidence.
- Flags/outcomes remain B001-gated. Returned carrier chains ending in borrowed
  records, broader result shapes, allocator-bound analysis, heterogeneous unions,
  precise joins, callee effect/data/control summaries and conditional-exit control
  remain open. Runtime sources, reference fixtures, dependencies and versions are
  unchanged; editor and separate runtime/sanitizer gates were not rerun.
  Full release qualification remains open.

## Area handoff

| Area | Current boundary |
| --- | --- |
| Compiler | Logical charges cover type-expression dispatch/materialization, required statements/blocks, scalar/record reads and constructed/copied record slots; query statement/annotation budgets are retained; descriptor outcomes and phase tracking stay open. |
| Documentation tooling | Constructed signatures and file graphs are checked; multi-file doc commands/indexes remain separate. |
| Editor integration | Pointer syntax previously passed Vim/Neovim; unchanged here. |
| Standard library | `proof` revision 1 specifies queries and static tests; implementation remains open. Net/HTTP foundations remain separate. |
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
   Fixed flag type queries are independent of answers and must remain admitted.
   Pending statement and annotation roots are integrated; descriptor construction
   and type inspection must charge the retained ledger when outcomes are admitted.
   Required-block descriptors and text/helper execution remain gated. The
   [compiler handoff](compiler/STATUS.md#executable-proof-plan) records the boundaries.

2. Broaden subtraction only after its remaining syntax/representation prerequisites
   are established. Do not push, bump versions or claim full release qualification.
