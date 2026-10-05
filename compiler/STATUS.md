# Compiler handoff and work tracker

Updated: 2026-10-05. Narrowed field-source lookup passes compiler and documentation gates.
Proof evaluation remains unimplemented. Full v0.0.1 is incomplete.
[../STATUS.md](../STATUS.md) tracks the project; [../COMPILER.md](../COMPILER.md)
records the plan. Keep this handoff current; Git holds history. Do not recreate STEP logs.

## Numeric shadowing implementation

Active user request: implement numeric value bindings and update every affected
language, compiler and editor document. Exact numeric spellings resolve lexically;
`1`, `01`, `0x1` and `1.0` remain distinct. Declared values keep ordinary types,
mutability, lifetime and scope rules. Unbound numbers retain literal defaults.
The explicit escape is `@"core".literal(0x1)`, replacing the proposed numbered
core members. Its resolved intrinsic identity accepts numeric syntax without
looking up the operand; aliases retain that identity. Signs remain operators.

Dependency-ordered commit plan:

1. Parse numeric value declarations, parameters and members with focused parser
   regressions; preserve numeric token validation and type-name grammar.
2. Resolve numeric reads and signed-literal boundaries, then numeric storage,
   writes and borrows in a separate commit, each with checker/native regressions.
3. Integrate required evaluation and contextual list inference, preserving
   fixed binding types and existing budget/eligibility gates.
4. Add the core literal intrinsic, aliases, numeric argument validation and
   contextual typing with runtime/required regressions.
5. Publish the contract and required source conformance cases; refresh reviewed
   coverage hashes and retain all existing capability exceptions.
6. Update editor highlighting, teaching/compiler docs and project handoff; run
   the full compiler/editor verification gate and inspect each staged slice.

Investigation: numeric tokens currently become Int/Float AST leaves and bypass
lookup. Existing checker symbol resolution, literal typing and intrinsic-call
expansion are the integration points. Parser support is complete: all 34 parser tests pass
(`/tmp/meowy-numeric-parser.log`). Numeric AST leaves retain spelling until lookup.
Unrelated `docs/programs/hey/` stays excluded. The unfinished direct field-source
Forward coercion work below remains the next existing compiler milestone.

Parser slice: `ccebefc`. Numeric reads now resolve existing lexical bindings before
literal construction; functions/module aliases and fixed binding widths retain
ordinary rules. Focused checker cases and debug/release native output pass
(`/tmp/meowy-numeric-reads.log`). Numeric member access uses grouping, e.g.
`(1).field`, because existing malformed-number token rules remain unchanged.

Read resolution: `ca66e6d`. Numeric storage now preserves mutable writes, shared
and exclusive borrows, field/index paths and narrowing. Focused checker rejection
cases and both native groups pass in debug/release
(`/tmp/meowy-numeric-storage.log`).

Storage slice: `2a6a88a`. Required integer/boolean evaluation now resolves numeric
bindings before scalar validation and preserves signed shortcuts only for unbound
literals. Focused accepted/rejected required cases pass
(`/tmp/meowy-numeric-required.log`), including widths, extents and runtime inputs.

Required scalar slice: `36c4347`. Required records, composition, type-value
aliases/equality and field roots now use the same numeric lookup, including
required-read tracking. All three focused numeric groups pass
(`/tmp/meowy-numeric-identities.log`).

Required identity slice: `f8ee8f2`. Numeric list elements now preserve typed
inference, candidate probes, isolated probe environments and element storage.
All four numeric checker groups pass (`/tmp/meowy-numeric-lists.log`).

Stack adapter: `abad72b`. Optional numeric rewrites are boxed so deep existing
candidate probes stay within the default stack. The linked signature change
required the helper and all 14 caller files in one buildable commit.
Numeric AST payloads now retain an explicit literal marker alongside spelling;
source tokens remain lookup-enabled. All 2547 library tests pass with a fresh
TMPDIR (`/tmp/meowy-numeric-payload-clean.log`). One prior run encountered an
existing test-directory collision after the aborted stack run; the focused
recheck and full clean-directory run both passed.

Literal representation: `ad796f5`. core.literal now resolves through the Core
module and ordinary immutable aliases; it accepts one numeric token or its direct
unary negation, marks the operand intrinsic and preserves contextual widths.
Arity/syntax/range rejections and three debug/release native groups pass
(`/tmp/meowy-core-literal.log`).

Core literal slice: `183e41a`. Required integer construction, comparisons, block
operands and type queries now preserve escaped numeric identity across repeated
checking. Both focused literal groups pass (`/tmp/meowy-literal-required.log`).

Required literal slice: `a10c027`. Literal calls now retain contextual list
inference and union selection, pure-probe intrinsic aliases, and ordinary list
extent eligibility. All three literal groups pass (`/tmp/meowy-literal-lists.log`).

Literal list slice: `439b435`. Numeric pending copies retain one query identity,
and non-data type queries no longer fall back to the numeric default type.
All six numeric groups pass (`/tmp/meowy-numeric-query.log`); proof outcomes
remain gated exactly as for identifier bindings.

Query boundary slice: `31c1cb9`. Checked-documentation value paths now follow
the source parser, retaining numeric spelling, grouping and field boundaries.
All 15 documentation groups pass (`/tmp/meowy-numeric-doc-links.log`).

Checked-doc links: `64d585c`. Syntax and core references now specify exact-spelling
lookup, fixed binding types and the literal syntax escape. Two new required source
cases pass; conformance is 326 passed, 19 unchanged gaps, zero failures in both
profiles (`/tmp/meowy-numeric-source-core.log`). Reviewed syntax/core inventory
hashes and evidence are refreshed; all four documentation checks pass
(`/tmp/meowy-numeric-contract-docs.log`). Proof obligations are unchanged.

Core contract: `a0cfb29`. Types, values and collections now explain fixed numeric
binding types and storage/index lookup. The numeric storage source case passes;
conformance is 327 passed, 19 unchanged gaps, zero failures in both profiles
(`/tmp/meowy-numeric-source-storage.log`). Corresponding coverage hashes reviewed.

Storage contract: `3b3ce22`. Required evaluation and diagnostic references now
cover numeric lookup and syntax-only literal construction. Required execution
and fixed-type rejection cases pass: 329 required passes, 19 unchanged gaps and
zero failures in both profiles (`/tmp/meowy-numeric-source-required.log`).
Coverage hashes reviewed. All six documentation/editor checks also pass
(`/tmp/meowy-numeric-editor.log`); editor changes remain a separate slice.

Required contract: the preceding commit. Numeric declaration and callable heads now
receive appropriate Vim/Neovim highlighting; ordinary uses retain lexical numeric
colors. All six checks passed (`/tmp/meowy-numeric-editor.log`).

Editor slice: `2ceded0`. Numeric checked links now have a required executable
source fixture and an updated documentation reference. All 330 required cases
pass with 19 unchanged gaps (`/tmp/meowy-numeric-source-links.log`).

Checked-link contract: `610bfec`. Numeric scalar and function exports now have
required multi-file facade coverage; module semantics explicitly retain numeric
names. Conformance is 331 passed, 19 unchanged gaps, zero failures in both profiles
(`/tmp/meowy-numeric-source-modules.log`). The module coverage hash is reviewed.

Module contract: `ce4b0ab`. Numeric duplicate/write/borrow/arity rejection sources
pass: 335 required passes, 19 unchanged gaps, zero failures in both profiles
(`/tmp/meowy-numeric-source-errors.log`). The reference anchor correction was
validated by regenerated coverage and all six doc/editor checks
(`/tmp/meowy-numeric-final-docs.log`).

The final audit found required boolean form validation missing literal expansion.
The focused fix preserves E222 for numeric operands even in skipped arms; all
four literal groups pass (`/tmp/meowy-literal-boolean-forms.log`).

Boolean form fix: `8772860`. Five required rejection sources cover duplicate
numeric names, immutable writes, borrow conflicts, intrinsic arity and skipped
numeric boolean operands. Conformance is 336 passed, 19 unchanged gaps and zero
failures in both profiles (`/tmp/meowy-numeric-source-errors-final.log`).
All 343 previous cases, 375 source assets, capability pins and proof obligations
are preserved (`/tmp/meowy-numeric-preservation.log`).

The root introduction, first tour and design notes now teach numeric spelling
lookup and literal escape. Their local links passed the six documentation/editor
checks in `/tmp/meowy-numeric-final-docs.log`.

Final gate passed formatting and all-target Clippy but found an existing extent
boundary regression: eager literal probing changed B001 to E201 for an unresolved
call in an ordinary extent. Literal identity probing now defers failed lookup to
the original context. The gate failure remains in `/tmp/meowy-numeric-final-gate.log`.

Five literal groups and all six existing extent-root groups now pass
(`/tmp/meowy-literal-lookup.log`). Literal probing uses existing non-evaluating
symbol hints; unknown calls retain their original context diagnostics.

The compiler numeric-name guide and foundation, required-evaluation, module and
implementation-plan documentation are complete. Their local links and all
six doc/editor checks pass (`/tmp/meowy-numeric-final-docs.log`). The root tracker
records compiler, documentation and editor scope separately from release status.

All 12 final compiler/editor checks pass (`/tmp/meowy-numeric-final-gate-2.log`):
2555 library tests, 918 native tests, 62 Python groups and 336 required conformance
passes with 19 unchanged gaps. Strict mode exits 1 only for those gaps
(`/tmp/meowy-numeric-strict.log`). Additional Vim/Neovim coverage explicitly checks
binary, digit-separated and signed-exponent variable declarations; all six editor
and documentation checks pass (`/tmp/meowy-numeric-editor-final.log`).

Next step: replace this active plan with the completed handoff and commit ledger.

## LLVM 23 and Rust 1.99 host qualification

The bootstrap now pins LLVM/Clang/LLD/ar 23.1.1 and Rust 1.99.0. Target lookup passes
Triple directly for LLVM 23; two redundant closure borrows satisfy Rust 1.99 Clippy.
The runtime runner requires matching Clang and rejects the former version before
building. Cargo's Rust compatibility floor remains 1.98.1.

The isolated `16c06d9` baseline at `/tmp/meowy-llvm23-check` passes all ten compiler
checks: 2249 library/914 native tests, 217 required conformance passes, 19 unchanged
gaps and no failures (`/tmp/meowy-llvm23-compiler.log`). All 15 runtime harness groups
pass; native debug/release and ASan/UBSan/LSan, including the expired-fiber-local
probe, pass (`/tmp/meowy-llvm23-runtime.log`). LeakSanitizer required execution
outside ptrace; no sanitizer was disabled. Owning diagnostic sizes remain identical.

This qualifies the host bootstrap, not a bundled distribution or minimum OS/libc.
Reverting the toolchain pins and API adaptation restores the previous selection;
no host packages changed. Toolchain code is committed as `712afe6`; active compiler,
runtime and implementation-plan documentation now uses the same pins. Historical
Clang 22 size measurements remain labeled; Clang 23 reproduces their current column.
Emission changes were excluded from baseline qualification; their final validation
is recorded below.
All four final documentation checks pass (`/tmp/meowy-llvm23-docs.log`).

## Documentation coverage and conformance audit

The initial coverage strengthening series is complete. Compiler implementation,
all 23 original cases/sources and the reference contracts are unchanged. Source
coverage grew from 23 to 64 cases: 45 required passes, 19 pinned capability gaps,
zero failures in debug/release. Six gaps are newly recorded coverage (five proof
outcomes and direct-call list extents), not regressions introduced by compiler edits.

The runner requires every case by default. Exceptions pin the exact B001 blocker;
changed blockers, unexpected unsupported cases and newly passing exceptions fail.
Promotion removes the exception after both profiles satisfy the reference outcome.
Malformed/mixed diagnostics, crashes, profile disagreement and incorrect runtime
bytes fail independently of capability status. `--strict` rejects all known gaps.

[Coverage inventory](../docs/conformance/COVERAGE.md) maps all 37 reference documents
to catalog cases, classified internal test evidence and explicit remaining scope.
Reviewed content hashes force reference-change review; missing evidence and stale
reports fail the default gate. All 33 proof obligations are tracked (31 acceptance
rows plus runtime-noninterference and release-matrix requirements), with required
observations checked for drift. A mapped or blocked fixture does not qualify an
entire obligation. Most proof outcomes and broad library/runtime areas remain gaps.

| Reviewable slice | Commit |
| --- | --- |
| Default-required harness and pinned blockers | `3fb74df` |
| Receiver/predicate/ascription contracts | `c29ce36` |
| Bits widths/order and direct-extent gap | `991b356` |
| Ownership and ordered stores | `a7b06c0` |
| Blocks, restarts and guard invalidation | `871fb1b` |
| Proof staging and diagnostic precedence | `e4e18af` |
| Proof outcome obligations and blockers | `c4bbb13` |
| Required evaluation and collection boundaries | `d3b4387` |
| Reference inventory and drift checks | `7303a58` |
| Complete proof-obligation map | `cf0cfaa` |
| Adversarial catalog validation | `3976524` |

All ten compiler checks pass: 1890 library/913 native tests, 28 tooling and 11
compiler-harness groups, formatting, Clippy, build, metadata/coverage and conformance.
Log: `/tmp/meowy-conformance-coverage-gate.log`. Strict execution correctly exits 1
for the 19 known gaps, with no failed cases; `/tmp/meowy-conformance-strict.log`.
The reference/original-fixture byte-preservation audit also passed. Final default
checks pass in `/tmp/meowy-conformance-final-docs.log`. No outstanding
test failures remain. Root/compiler AGENTS now require source coverage, reviewed
traceability and promotion of passing exceptions. This is not release qualification.

### Current companion-file conformance slices

The companion-file series is complete. Catalog version 2 adds declared `.mwy`
companions under each entry source's directory; version-1 single-file catalogs
remain readable. Shared checks reject missing, escaping, noncanonical, duplicate,
symlinked and out-of-bundle declarations. Empty companion modules remain valid.
The orphan audit includes every entry/companion. Only declared files are copied,
byte for byte, after input preflight into a fresh temporary directory per case.
Nested relative layout is preserved; failures are reported per case and staging is
cleaned before the next case. No compiler implementation behavior was changed.

Ten required module cases cover diamond initialization, nested importer resolution,
record/list exports, borrowed returns, typed function/type facades, namespace
separation, private value/type errors, dependency errors, re-export cycles, missing
imports and signature mismatch. Companion links are visible in the coverage report.
The previous 64 case records/source bytes, 19 capability pins and all reference
contracts are unchanged. The catalog now has 74 cases: 55 required passes,
19 pinned gaps and zero failures in debug/release.

| Reviewable slice | Commit |
| --- | --- |
| Companion declaration/path validation | `2afe88b` |
| Isolated staging and version-2 runner | `a22e003` |
| Diamond initialization | `46e9ff1` |
| Nested imports and borrowed returns | `0a1f087` |
| Typed re-exports and namespaces | `5f4f9d2` |
| Private value/type diagnostics | `f8276b9` |
| Dependency errors and re-export cycles | `8db5622` |
| Missing imports and signature mismatch | `c6cf01e` |

All ten compiler checks pass: 1890 library/913 native tests, 29 tooling and 22
harness groups, formatting, Clippy, build, catalog/coverage and conformance.
Log: `/tmp/meowy-companion-conformance-gate.log`. Strict mode correctly exits 1
for the 19 unchanged gaps; `/tmp/meowy-companion-strict.log`. The preservation
audit passed. Final default checks also pass; `/tmp/meowy-companion-final-docs.log`.
No outstanding test failures remain. Compiler AGENTS now documents
companion staging and the distinction between missing assets and missing imports.
Package policy, symlink fixtures, public FFI and full release qualification remain open.

### Completed runtime-panic conformance series

All planned runtime-panic work is complete: schema, runner, adversarial tests,
source fixtures, coverage maintenance and final gates. Catalog version 3 preserves
v1/v2 compatibility and adds accepted run expectations with exact stdout, a
documented panic code and exit 1. Checking must succeed without diagnostics first.
Ordinary successful runs still require exit zero and empty stderr.

The runner recognizes one complete UTF-8 bootstrap panic record with a byte-site
suffix and optional source path. Wrong code/profile/output/exit, malformed ranges,
mixed diagnostics, compiler faults, signals, timeouts and runtime capability errors
fail. Human wording and source-location values are not golden strings. Multiline
panic/cleanup records and unsupported panic families remain explicit qualification
gaps, not silently accepted results.

Fourteen new required cases cover ordered addition, unsigned subtraction, signed
multiplication/negation, zero divisors, signed-minimum division, initialized-length
bounds, negative/wide indices, stopped nested writes, capacity checks after item
effects, explicit panic aliases and module initialization stopping later modules
and entry code. All 74 prior case records, entry/companion bytes, reference contracts,
compiler implementation files and capability exceptions are unchanged.

| Reviewable slice | Commit |
| --- | --- |
| Version-3 panic expectation schema | `e927e30` |
| Strict runtime result recognition | `4ba13f4` |
| Overflow and operand effect order | `360fbea` |
| Division/remainder boundaries | `cac274c` |
| Collection bounds/capacity and stopped effects | `8760c69` |
| Explicit and module-initialization panics | `6f9d3cd` |

All ten compiler checks pass: 1890 library/913 native tests, 32 tooling and 30
harness groups, formatting, Clippy, build, metadata/coverage and execution.
Conformance: 88 cases, 69 required passes, 19 unchanged pinned gaps and zero
failures in debug/release; `/tmp/meowy-runtime-panic-conformance-gate.log`.
Strict mode correctly exits 1 for those gaps; `/tmp/meowy-runtime-panic-strict.log`.
Preservation audit: `/tmp/meowy-panic-preservation.log`. Final default checks also
pass; `/tmp/meowy-runtime-panic-final-docs.log`. No outstanding test failures remain. Compiler AGENTS and the coverage inventory describe the new requirements.

The requested testing-strengthening work is complete. Remaining language/library,
multiline panic, task/cleanup and proof qualification gaps stay visible in the
coverage inventory; they are not claimed as finished implementation. The existing
ordinary function-definition completion repair is the next compiler item below,
with these conformance/traceability gates required for further changes.

## Current ordinary function-definition completion repair

The dependency-ordered implementation slices are complete:

1. Return exact successful FunctionIds, including nested declarations (`b2dd92c`).
2. Add bounded ordinary-definition endpoints and structural regressions (`8509cc9`).
3. Source conformance, documentation and final compiler validation: complete.

`edges/declarations.rs` captures one Entry(statement) -> Normal(statement) edge only
from the successful ordinary function Bind branch in `statements.rs`. It validates
the current statement owner/completion, exact registered FunctionId and independent
body owner before publication. Shared work/edge capacity is bounded, duplicate
registration is idempotent, and invalid metadata or exhausted budgets adds no edge.
Function bodies are never connected to declaration execution. An unused Never body
does not block the enclosing sequence; an actual Never call still lacks continuation.
This slice covered ordinary definitions; alias/export extensions are recorded below.
Forward-group completion is covered by the current slice below.

All 1898 library tests and all-target Clippy pass;
`/tmp/meowy-function-endpoints-lib.log`, `/tmp/meowy-function-endpoints-clippy.log`.
Six endpoint groups cover leading/interleaved/nested owners, Never boundaries,
ordinary errors, invalid identities, preserved opaque forms and exact work/edge caps.
The indirect-store regressions now inspect real program-entry reports after ordinary
definitions, keeping calls Unknown. New source fixtures pin non-execution of unused
Never definitions and error checking in unused bodies. All ten compiler checks pass:
1898 library/913 native tests, 32 tooling and 30 harness groups, formatting, Clippy,
build, metadata/coverage and conformance (71 passed/19 pinned gaps/0 failed in both
profiles); `/tmp/meowy-function-declarations-gate.log`. Strict mode correctly rejects
the unchanged gaps; `/tmp/meowy-function-declarations-strict.log`. All previous 88
source cases/assets and reference contracts are preserved. Final default checks pass;
`/tmp/meowy-function-declarations-docs.log`. No test failures remain.

Immutable aliases and exported functions now have form-specific completion metadata,
described below. Forward groups now have their own completion path below.

## Current immutable identity-binding completion

The dependency-ordered series is complete:

| Reviewable slice | Commit |
| --- | --- |
| Share bounded endpoint publication without changing definition validation | `29db896` |
| Classify and connect selected immutable identities with focused regressions | `6387133` |
| Pin self/forward aliases, lexical shadowing and duplicate-name conformance | `cabfb71` |

`edges/identities.rs` classifies resolved Function items, the current foundation
modules and print/panic/string-copy/copy-query/bit-operation identities. The original
Value is declared unchanged. Statement Entry-to-Normal publication follows successful
immutable, unannotated declaration, so lookup/duplicate-name errors stay first.
Function slots may be registered but incomplete during self/forward checking;
completed slots must retain the exact ID. Function bodies are never traversed by
an alias endpoint. Work and aggregate edge limits precede publication; replay is
idempotent and failures add no endpoint. Calls retain their effects and return gates.

Record/pending forms remain excluded. Type-valued/static/control/file-module
bindings and meta exports are covered below. Heap handles retain their runtime value path;
forward groups have their own completion path below. Required
roots, ownership, ordinary errors and unsupported-call/proof gates remain unchanged.
Six structural groups cover resolution, self/forward slots, exclusions, errors,
identity validation and exact publication budgets. The four source cases exercise
observable language behavior independently of those structural tests.

All ten compiler checks pass: 1904 library, 913 native, 32 tooling and 30 harness
groups, formatting, Clippy, build, metadata/coverage and conformance;
`/tmp/meowy-identity-bindings-gate.log`. Conformance has 94 cases: 75 passes,
19 unchanged pinned gaps and zero failures in debug/release. Strict mode exits 1
only for those gaps; `/tmp/meowy-identity-bindings-strict.log`. All previous 90 case
records, 107 source assets, 37 reference files and capability exceptions are
unchanged; `/tmp/meowy-identity-preservation.log`. No outstanding test failures
remain. All four final documentation checks pass;
`/tmp/meowy-identity-bindings-docs.log`. AGENTS already requires source coverage,
classified evidence and exact capability exceptions; no further rule changes are
needed for this slice.

Exported definitions and re-exports are now covered by the completed slice below.
Required-only evaluation, callee summaries and proof outcomes remain separate.

## Current exported function completion

The dependency-ordered series is complete:

| Reviewable slice | Commit |
| --- | --- |
| Return exact successful exported FunctionIds without graph changes | `6fce856` |
| Connect successful export statements with focused graph regressions | `447c0e4` |
| Pin module initialization, chained re-exports and export errors | `3506f03` |

`export_function` returns the exact declared/resolved ID after signature, scope,
duplicate-export, declaration and module registration checks. Nested function
allocation cannot change the returned identity; re-exports preserve imported IDs.
Unhandled forms return None without gaining completion. Successful function export
statements reuse `function_declaration_endpoint`, validating statement/module owner,
completed function/body identity and atomic work/edge budgets. Bodies remain
independent; unused Never bodies do not block declaration completion and Never
calls still lack continuation. Export errors publish no endpoint.

Three identity groups and five graph groups cover definitions, nested allocation,
chained/imported aliases, independent module sequences, errors, unhandled forms,
Never calls and exact publication limits. Forward groups, explicit type aliases
and meta value exports have subsequent completion slices below. Required
roots, export errors/capability gates,
module initialization, callee effects and proof outcomes are unchanged.

All ten compiler checks pass: 1912 library/913 native tests, 32 tooling and 30
harness groups, formatting, Clippy, build, metadata/coverage and conformance;
`/tmp/meowy-function-exports-gate.log`. The 97 source cases report 78 passes,
19 unchanged pinned gaps and zero failures in debug/release. Three new cases pin
startup around exported definitions/re-exports, an unused Never function, duplicate
exports and missing public signatures; the existing signature-mismatch case stays
required. Strict mode correctly exits 1 only for known gaps;
`/tmp/meowy-function-exports-strict.log`. All 94 prior case records, 111 source assets,
37 reference files and capability exceptions are preserved;
`/tmp/meowy-function-exports-preservation.log`. All four final documentation checks
pass; `/tmp/meowy-function-exports-docs.log`.
No outstanding compiler/test failures remain. AGENTS already covers the source
conformance/evidence requirements; no rule change is needed.

Explicit type-alias completion is now covered below. Other erased forms, forward
groups, callee summaries and proof propagation/outcomes remain separate.

## Explicit type-alias completion

Ordinary TypeAlias statements connect only after required construction and
successful declaration/export checks (`133ba70`). Required-only helper aliases
add no runtime statement endpoints. Source coverage (`90d6263`) pins computed/scoped
aliases, separate namespaces, module type exports, duplicate names and required
tail failures. These checks remain in the full gate below; the
[foundation guide](docs/FOUNDATION.md) records completion and accounting boundaries.

## Type-valued binding completion

Resolved type/foundation-type bindings (`3692a56`) and meta exports (`1c4d254`)
connect only after required checking and successful declaration/export validation.
Payloads, query non-execution and required-only helpers remain unchanged. Source
coverage (`d43aab9`) pins imported meta values, Type shadowing and relevant errors.
The full gate below retains these checks; the foundation guide records the bounds.

## Immutable static-binding completion

Resolved immutable Static aliases connect after declaration (`c2f7d7b`) without
changing payload widths, required-only evaluation or typed/mutable runtime storage.
Source coverage (`46d450e`) pins revision aliases, shadowing, required extents and
rejections. The full gate below retains these checks; the foundation guide records
the metadata/evaluation boundary.

## Scoped-control alias completion

Control aliases connect after bounded active-target/owner validation (`58fd3cf`)
without performing leave/restart at declaration. Actual calls retain their separate
argument/lifetime/control checks. Source coverage (`8d47955`) pins alias creation,
real exits/restarts and rejections. The full gate below retains these checks; the
foundation guide records the metadata/control-transfer boundary.

## File-module alias completion

File-module aliases connect after bounded local/exports registry validation
(`1143ce5`), preserving exact IDs, caller ownership, privacy and required-read rules
without replaying module bodies. Source coverage (`43a4308`) pins initialization,
function-local aliases and privacy. The initializer-input extension below replaces
the prior input=None boundary; other synthetic prefixes remain separate.

## Synthetic module-initializer inputs

Exact module-expression roots (`514324d`) feed bounded synthetic storage operations
(`97209a5`), preserving parent/owner/completion checks and module/docs restoration.
Program-entry reports follow initializer bodies before stores; stopped/Never inputs
cannot reach materialization or later modules. Source coverage (`6ea543a`, `90791b1`)
pins tail order and failed partial results. The full gate below retains these checks.

## Current forward-group completion

Ordinary groups retain exact reservation IDs, checked first-signature points/sites,
and bounded completion only after every definition succeeds (`82595c3`, `a687419`).
Block and ordinary/composed dispatch sequences connect those points (`4940924`);
reordered recursion, unused Never functions and rejection coverage is in `6791859`.
Bodies keep independent owners; completion grants no callee purity or reachability.
Other None prefixes remain explicit. Export support below completes the documented
module-level definition forms without changing those graph boundaries.

## Exported forward definitions

Module-level exported definitions reuse reserved IDs and publish only after the
whole group succeeds (`054c9b4`, `9ee8cb6`). Public signatures come from the forward
header; explicit definition annotations must match. Peer documentation links use
exact definition locations and preserve privacy (`1f995e4`). Required source
coverage (`a8f8988`, `25aa9e0`) pins recursion, re-exports, initialization and failures.
The compiler gate below retains those checks. Non-top-level forward exports remain
B001-gated; ordinary export annotation rules and independent body owners are intact.

## Record alias boundaries and coverage

Ordinary record/subrecord copies retain distinct storage and checked initializer
operations. Required scratch records introduce no runtime identities and complete
through their enclosing declaration. Graph/read/budget regressions (`f8d7aed`) and
source coverage (`48298eb`, `e5a5f39`) preserve exact reads, scope, imported staging,
failures and stopped paths. No production change was needed; the compiler gate
below retains this coverage.

## Pending proof-statement completion

Checked pending statements retain exact query IDs (`4506888`) and publish bounded
completion after query/point/site/owner/root validation (`8a019dc`). Copies retain
origins without querying again; same-owner nested scopes and open construction roots
work, while E223 capture restrictions and final B001 proof gates remain. Source
rejections (`532c3a5`) pin fixed signatures and storage/capture boundaries. The gate
below retains these checks; completion adds no runtime query execution or outcome.

## Direct-call effect metadata

Bounded point-to-CallId lookup (`16c700a`) and typed call effects (`ab9a03a`) retain
exact callee, argument/control and conditional-return metadata after HIR transfer.
Callees are validated through independent entries; calls do not enter their bodies.
Source coverage (`8c2cf12`) pins order, stopped arguments, arity and borrow rules.
The compiler gate below preserves these checks and opaque callee effects.

## Call graph and recursion groups

Bounded caller/callee adjacency (`84324c6`) retains exact sites and original
missing/backedge boundaries. Iterative components (`d64de6c`) preserve independent
owners, unused entries and self/mutual recursion; source coverage (`2983acc`) pins
execution and original body errors. The component partition passed an exhaustive
512-graph oracle. The compiler gate below preserves this work without purity,
termination, runtime reachability or proof claims.

## Component condensation and analysis order

Condensed adjacency preserves exact internal/cross-group sites (`37be1e7`), and
bounded iterative ordering places callees before callers (`d28ce65`, `37ef903`).
Partition/site validation, deterministic ties, cycle rejection, exhaustive small
graphs and deep-chain limits are covered. Source checks (`a0635f6`) preserve runtime
order. Original body/missing/backedge records and unknown effects remain explicit;
analysis order does not schedule runtime calls or provide complete summaries.

## Print and panic effect stages

Bounded capture (`0ad7f11`) and typed aggregation (`c018728`, `cae9858`) retain
prefix/projection/output/terminal stages independently, including partial formatting.
Exact roots, literal markers, owners, stops and terminal registry checks remain;
repeated visits copy each part once. Source coverage (`44e3073`) and the gate below
preserve streaming, panic and checked tails. Observed stages are not successful-I/O,
runtime reachability or complete effect-summary claims.

## Canonical local-read effects

The dependency-ordered series is complete:

| Reviewable slice | Commit |
| --- | --- |
| Retain validated local/storage bounds after HIR transfer | `4f51efb` |
| Capture typed canonical read effects | `cb389e2` |
| Pin slot aliases, reference cells and required-input limits | `04ba6d8` |

Reports retain the transferred program's local count after validating every stored
read's local/storage bounds and stable alias root. No local types or values are
copied. For encountered read operations, typed records preserve exact local and
canonical storage IDs plus normal/control flags. Point kind/completion/span,
owner/operation registry, retained bounds, alias mapping and exact read-edge shape
are checked before returning effects. Shared work and effect limits remain atomic.

Reference-cell reads stay distinct from explicit pointee loads, captured below.
Never reads retain no normal edge; later reads beyond stopped predecessors
stay outside the report. Required-only inputs keep their separate InputUse records
and gain no runtime read effects. This metadata grants no copy/borrow authority,
value provenance, purity, termination or proof result.

Eight new regression groups cover HIR transfer, canonical aliases, controls,
parameter owners, reference cells, Never/stopped/required-only boundaries, malformed
bounds/identity/edge metadata, duplicate visits and exact work limits. The seeded
control test is separate from complementary emissions, preserving the existing
E205 validation. Three required source cases cover slot reads across writes,
reference-cell copies/retargeting with a required extent, and rejection of a runtime-
dependent type extent.

All ten compiler checks pass: 2021 library/914 native tests, 32 tooling and 30 harness
groups, formatting, Clippy, build, coverage and conformance;
`/tmp/meowy-read-effects-gate.log`. The 151 cases report 132 required passes,
19 unchanged pinned gaps and zero failures in debug/release. Strict mode exits 1
only for the same known gaps (`/tmp/meowy-read-effects-strict.log`). All four final
documentation checks pass (`/tmp/meowy-read-effects-docs.log`); no selected check
has an outstanding failure. Preservation against `a067729` confirms all 148 prior cases,
180 source assets, 37 reference files and exact capability exceptions are unchanged
(`/tmp/meowy-read-effects-preservation.log`). AGENTS already covers this evidence
workflow; no update is needed. Unrelated `docs/programs/hey/` remains untouched.

## Explicit dereference-load effects

Retained result decisions (`e9aa13a`) and typed Deref records (`f3f4e4c`) preserve
exact pointer roots, shared/exclusive mode and normal/control flags after bounded
point/owner/edge validation. Reference-cell reads stay separate; stopped pointers
have no load effect and shared Never referents have no normal result. Exclusive
Never references retain B001. Boundary tests (`f6044ab`) and source cases (`cfc7d62`)
remain covered by the current gates below. These records copy no aggregate types,
pointee storage or values and grant no borrow authority or proof outcome.

## Bounded field-read effects

Checked field counts (`f834ed9`) and typed Field records (`e0b6732`) preserve exact
receiver/index/load/result boundaries before narrowing. Explicit dereferences,
reference-valued fields, projected borrows and required/static reads retain their
separate identities. Boundary tests (`deb2b53`) and source cases (`e984699`) remain
covered by the current gates below. No type shapes, storage/value provenance,
borrow authority, call termination or proof outcomes are inferred.

## Bounded indexed-read effects

Separate result decisions (`b29e9a0`), stage validation (`03f4681`) and typed partial
reports (`fcb6b88`) retain exact roots, optional lengths/capacities and independent
load/snapshot/read observations. Stopped positions retain only earlier stages;
terminal reads require registered owners and preserve conditional bounds edges.
Boundary tests (`b04783a`) and source cases (`776f00a`) remain covered by the current
gates below. No types/values, bounds proof, storage or borrow authority are inferred.

## Collection method effects

Stage validation (`0412249`) and typed partial reports (`c9d4361`) retain list/string
size and list-add kinds, exact roots and independent load/snapshot/terminal flags.
Stopped items retain earlier stages, while stopped receivers produce no method effect.
Boundary tests (`d83f647`) and source cases (`a1f0875`) remain covered by the current
gates below. Receiver/error ordering and add's new-list result are preserved without
inferring capacity success, values, storage, borrow authority or proof outcomes.

## Bounded unary effects

Stage validation (`97cec7c`) and typed observations (`77976fa`) retain exact operands,
compact scalar descriptors and independent projection/operation/result flags.
Integer negation preserves its Checked result edge, while required, stopped and
signed-literal paths remain separate. Boundary tests (`ec2d6c6`) and source cases
(`9dbc4ae`) remain covered by the current gates below. Metadata conflicts and shared
work/effect limits publish no partial collection. No values or proof outcomes are
evaluated; operation/result observations do not establish successful execution.

## Bounded binary effects

Compact type capture (`5503576`), stage validation (`3ae26e3`) and typed observations
(`98babec`) preserve scalar operand/result kinds, partial projections and checked
results across both operation and sequence ledgers. Boundary tests (`cb5a88d`) and
source cases (`37726ee`) remain covered by the current gates below. Nonscalar
comparisons stay opaque, and short-circuit/required paths remain separate. Shared
limits or conflicts publish no partial collection. No values or proof outcomes are
inferred from operation/result observations.

## Bounded scalar-leaf effects

Validated reports (`3be764d`) retain scalar kinds, numeric widths, control and
independent construction/result observations. Signed roots, runtime constant uses,
ordinary storage reads and required/stopped paths retain their distinctions.
Boundary tests (`28ec4c2`) and source cases (`6b1d6c1`) remain covered by the current
gates below. Duplicate/conflict handling and shared limits publish no partial
collection. Literal values are not copied or replayed, and result observations
establish no singleton domains or proof outcomes.

## Bounded heap handle effects

Validated reports (`4800e0d`) retain nominal Allocator identity, control and
independent handle/result observations. Module aliases, ordinary reads, contextual
conversions, temporary consumers and required/hint/stopped paths remain distinct.
Boundary tests (`f8fca7b`) and source cases (`89d7d1f`) remain covered by the current
gates below. Duplicate/conflict handling and shared limits publish no partial
collection. Static handle availability implies no allocation, extended lifetime,
borrow authority or proof outcome.

## Bounded narrowing observations

Validated reports (`c6e092c`) retain exact raw sources, changed/normal/control flags
and independent conversion/result observations. Unchanged forwarding, direct Never
and conversion-to-Never remain distinct. Boundary tests (`a86b02f`) and source cases
(`ef17f36`) remain covered by the current gates below. Duplicate/conflict handling
and shared limits publish no partial collection. No receiver work is replayed or
narrowed values, same-value identities, borrow authority or proof outcomes inferred.

## Bounded coercion observations

Validated reports (`912e62d`) retain exact raw sources, Forward/Convert/Stopped
kinds and independent projection/conversion/result observations. Forwarding,
direct/projected Never and shared-reference exclusions remain distinct. Boundary
tests (`16d183f`) and source cases (`00fb650`) remain covered by the current gates
below. Duplicate/conflict handling and shared limits publish no partial collection.
No target values, transfers, borrow authority or proof outcomes are inferred.

## Bounded predicate and ascription observations

The dependency-ordered implementation series is complete:

| Reviewable slice | Commit |
| --- | --- |
| Validate and retain predicate/ascription stages | `a031d11` |
| Cover exclusions, diagnostic precedence and exact budgets | `da43cb7` |
| Pin operands, computed targets, stops and guarded results | `c68e7f9` |

Reports retain exact operand roots, checked Predicate/Ascription kinds, normal/control
flags and independent operation/result observations, including erased no-op ascriptions.
Capture validates completed expression points, root spans, parent/block/owner
agreement, supported operand kinds, exact original edges and registered operation
owners. Operand spans remain distinct from the complete suffix expression. Stopped
operands gain no observations; a stopped predecessor excludes later operations from
the same entry walk. Duplicate visits merge flags;
conflicts and shared work/effect limits publish no partial collection or metadata changes.

Computed target reads, required evaluation, type queries and pending proof descriptors
remain separate from runtime operand links. Reports copy no target types or values
and infer no predicate truth, refined values, borrow authority or proof outcomes. Ordinary
operand/target error precedence, E208 and the final B001 proof gate are preserved.

Nine new internal groups cover predicates/ascriptions, erased wrappers, nested branches,
function owners/control, mutable guards, independent visits, computed targets and
stopped/required/query exclusions. One hundred corrupt source/edge cases, six merge
conflicts and exact shared limits preserve atomic failure
(`/tmp/meowy-typed-effects-limits.log`). Four required source cases pass in debug/release:
operand execution, nested/guarded operations, computed targets, both stopped suffixes
and target checking after a stop (`/tmp/meowy-typed-effects-sources.log`). Structural
reports remain separate from observable language conformance and proof evaluation.

All ten compiler checks pass: 2134 library/914 native tests, 32 tooling and 30 harness
groups, formatting, Clippy, build, coverage and conformance;
`/tmp/meowy-typed-effects-gate.log`. The 192 cases report 173 required passes,
19 unchanged pinned gaps and zero failures in debug/release. Strict mode exits 1
only for those gaps (`/tmp/meowy-typed-effects-strict.log`); all four final documentation
checks pass (`/tmp/meowy-typed-effects-docs.log`). No selected check has an outstanding failure.
Preservation against `1e3306a` confirms all 188 prior cases, 220 source assets,
37 reference files, proof obligations, reference hashes and capability exceptions
unchanged (`/tmp/meowy-typed-effects-preservation.log`). Unrelated `docs/programs/hey/`
is preserved. No compiler capability or reference contract changed.

## Bounded ordinary place-borrow observations

The dependency-ordered implementation series is complete:

| Reviewable slice | Commit |
| --- | --- |
| Retain bounded per-field record counts | `ac590ac` |
| Validate identities, selectors and stored edges | `97bda15` |
| Report independent address/acquisition/result observations | `6de8c96` |
| Cover stops, separate producers, conflicts and limits | `f4ebd60` |
| Pin execution, overlap and field lifetimes | `44e8b39` |

Reports retain exact local/field paths, canonical alias storage, checked shared/
scalar-exclusive modes, control and independent per-address/acquisition/result
flags. Precharged field counts survive local transfer without copying type shapes.
Validation checks completed point/owner/span identities, parent/block agreement,
local bounds, canonical storage, field bounds, selectors, operation registration
and exact original edges. Reference-cell addresses remain distinct from pointees.
An address-only visit grants no acquisition or result observation.

Paths retain at most 256 fields. Copied indices and address flags share the
262,144-entry payload limit with write paths, call arguments and output parts.
Duplicate visits merge flags without another copy; conflicts or exhausted work/
effect/payload budgets publish no partial collection. Stopped predecessors exclude
later borrows from the same entry walk. Shared indexing may retain an ordinary
root borrow while element acquisition stays with its separate producer.

Fourteen new internal groups cover field-count capture, shared/exclusive places,
reference cells, aliases, owners/control, stopped paths, other producers and original
mutability/conflict/lifetime/capability diagnostics. Ninety malformed identity/path/
edge cases, eight merge conflicts, exact shared budgets and the maximum path are
covered (`/tmp/meowy-place-counts.log`, `/tmp/meowy-place-boundaries.log`). Four
required source cases pass in debug/release: shared/exclusive places, cells, aliases,
stops, overlapping fields and local-field escape (`/tmp/meowy-place-sources.log`).
Structural observations remain distinct from source conformance and proof evaluation.

All ten compiler checks pass: 2148 library/914 native tests, 32 tooling and 30 harness
groups, formatting, Clippy, build, coverage and conformance (`/tmp/meowy-place-gate.log`).
The 196 cases report 177 required passes, 19 pinned gaps and zero failures in both
profiles. Strict mode exits 1 only for the same 19 gaps (`/tmp/meowy-place-strict.log`);
all four final documentation checks pass (`/tmp/meowy-place-docs.log`). No selected
check has an outstanding failure.
Preservation against `bef3b76` confirms all 192 prior cases,
224 source assets, 37 reference files, proof obligations, reference hashes and
capability exceptions unchanged (`/tmp/meowy-place-preservation.log`). Unrelated
`docs/programs/hey/` is preserved. No compiler capability or reference contract changed.

## Bounded standalone temporary-borrow observations

The dependency-ordered implementation series is complete:

| Reviewable slice | Commit |
| --- | --- |
| Validate cell registration and checked statement identities | `7b28c38` |
| Report independent acquisition/result observations | `80d6d78` |
| Cover stopped inputs, lifetimes, conflicts and exact limits | `eea5989` |
| Pin initialization order, stops and temporary escapes | `fd9c16d` |

Reports retain exact initializer roots, temporary local/statement cells, control
and independent acquisition/result flags. Validation checks complete point/owner/
span identities, initializer parent/block/site agreement, local bounds, temporary
registration, completed owning statement roots, operation membership and exact
original edges. Locals may already have transferred into the program. Stopped
initializers have no cell or observations; stopped predecessors exclude later
borrows from the same entry walk. No completion is inferred from successful checking.

Records have fixed size and copy no source values or type shapes. Duplicate visits
merge flags without another record; conflicts or shared work/effect exhaustion
publish no partial collection. Reference-valued initializers retain distinct cells,
never inferred pointee identities. Projected/indexed temporary materialization and
reborrows remain separate; full-statement lifetimes and E303 stay authoritative.

Eleven new internal groups cover scalar/record/list/call and short-circuit inputs,
reference cells, nested statement lifetimes, owners/control, stops and other producers.
Eighty malformed identity/site/edge cases, six record conflicts and exact shared
limits preserve atomic failure. A 1-byte/65,536-byte comparison confirms fixed
report cost (`/tmp/meowy-temporary-boundaries.log`). Four required source cases
pass in debug/release: once-only call initialization, immediate values/cells,
short-circuit inputs, stopped initializers and cell/record lifetime escapes
(`/tmp/meowy-temporary-sources.log`). Structural reports remain distinct from
observable language conformance and proof evaluation.

All ten compiler checks pass: 2159 library/914 native tests, 32 tooling and 30 harness
groups, formatting, Clippy, build, coverage and conformance
(`/tmp/meowy-temporary-gate.log`). The 200 cases report 181 required passes,
19 pinned gaps and zero failures in debug/release. Strict mode exits 1 only for those
gaps (`/tmp/meowy-temporary-strict.log`); all four final documentation checks pass
(`/tmp/meowy-temporary-docs.log`). No selected check has an outstanding failure.
Preservation against `0a28423` confirms
all 196 prior cases, 228 source assets, 37 reference files, proof obligations,
reference hashes and capability exceptions unchanged
(`/tmp/meowy-temporary-preservation.log`). Unrelated `docs/programs/hey/` is preserved.
No compiler capability or reference contract changed.

## Bounded reborrow observations

The dependency-ordered implementation series is complete:

| Reviewable slice | Commit |
| --- | --- |
| Validate parent modes, retained sites and exact edges | `6702ace` |
| Report independent acquisition/result observations | `133ed62` |
| Cover stops, permissions, site gaps and shared budgets | `ad414a1` |
| Pin execution, stopped parents and loan permissions | `3314092` |

Reports retain exact parent roots, existing ReborrowIds, checked parent/result modes,
control and independent acquisition/result flags. Explicit shared/scalar-exclusive
and implicit shared paths use the same producer. Validation checks completed point/
parent/owner/block/span identities, mode compatibility, site bounds, operation
membership and exact original edges. Parent call return conditions stay intact;
intervening projected-borrow IDs are not renumbered. Stopped parents have no site or
observations, while shared Never referents retain reborrow results without loads.

Records have fixed size and copy no referent values or type shapes. Duplicate visits
merge flags; conflicts or shared work/effect exhaustion publish no partial collection.
Reference-cell reads, unchanged shared forwarding, ordinary/indexed/projected borrows
and temporary materialization remain separate. Observations preserve existing loan
creation and lifetimes, grant no new authority and infer no proof outcomes.

Eleven new internal groups cover supported modes, nested sites, owners/control,
reference cells, conditional calls, stops, shared Never referents, other producers
and ordinary diagnostics. One hundred twenty malformed identity/mode/site/edge cases,
seven record conflicts and exact shared budgets preserve atomic failure. A 1-field/
128-field comparison confirms fixed report cost (`/tmp/meowy-reborrow-boundaries.log`).
Four required source cases pass in debug/release: explicit/implicit execution,
reference cells, returned parents, stopped modes, live-child conflicts and forbidden
upgrades (`/tmp/meowy-reborrow-sources.log`). Structural reports remain distinct from
observable language conformance and proof evaluation.

All ten compiler checks pass: 2170 library/914 native tests, 32 tooling and 30 harness
groups, formatting, Clippy, build, coverage and conformance (`/tmp/meowy-reborrow-gate.log`).
The 204 cases report 185 required passes, 19 pinned gaps and zero failures in
debug/release. Strict mode exits 1 only for those gaps (`/tmp/meowy-reborrow-strict.log`);
all four final documentation checks pass (`/tmp/meowy-reborrow-docs.log`). No selected
check has an outstanding failure. Preservation against `d2058e6` confirms all 200 prior cases,
232 source assets, 37 reference files, proof obligations, reference hashes and
capability exceptions unchanged (`/tmp/meowy-reborrow-preservation.log`). Unrelated
`docs/programs/hey/` is preserved. No compiler capability or reference contract changed.

## Bounded shared element-borrow observations

The dependency-ordered implementation series is complete:

| Reviewable slice | Commit |
| --- | --- |
| Retain bounded source paths and canonical storage | `d013921` |
| Validate sources, access and partial stages | `e1fa743` |
| Report independent address/acquisition/result observations | `8644137` |
| Cover source identities, aliases and loan boundaries | `c5abe07` |
| Bound source payloads, duplicates and merge conflicts | `9d696e8` |
| Pin receiver order, stopped inputs and dynamic bounds | `3fa8ab0` |
| Pin loan conflicts and temporary escapes | `658a01c` |
| Satisfy the test-helper visibility lint | `16cd21e` |

Reports retain exact parent/index roots, Source/Access metadata, retained sites,
capacity/optional initialized length, control and independent address/acquisition/
result flags. Owned source paths retain precharged field counts and canonical alias
storage after local transfer; temporary sources preserve cell/statement registration.
View storage remains opaque. No source values or type shapes are copied or inferred.

Validation checks completed point/owner/parent/block/span identities, source bounds,
canonical storage, temporary statement roots, access limits, selectors and exact
original edges. Completing accesses require registered operation ownership; partial
addresses require its absence. Stopped parents produce no observation; stopped
positions can retain earlier addresses without acquisition/result. Checked bounds
edges remain explicit, without granting success or new loan authority.

Each owned source path has at most 256 fields. Copied indices and field counts share
the 262,144-entry payload limit with existing path/call/output metadata; view and
temporary payloads have fixed size. Duplicate visits reuse records. Conflicts and
exhausted shared work/effect/payload limits publish no partial collection.

Seventeen new internal groups cover source capture, nested paths, owned/view/temporary
receivers, aliases, owners/control, stopped inputs, other producers and ordinary
receiver/index/loan/lifetime diagnostics. Coverage includes 152 corrupt stage/source
cases, canonical-alias faults, 14 record conflicts, exact shared limits and maximum
paths (`/tmp/meowy-element-sources-capture.log`, `/tmp/meowy-element-limits.log`).
Five required source cases pass in debug/release: receiver/index order, source kinds,
stopped inputs, dynamic initialized-length bounds, E302 conflicts and E303 temporary
escapes (`/tmp/meowy-element-execution.log`, `/tmp/meowy-element-rejections.log`).
Structural reports remain distinct from source conformance and proof evaluation.

All ten compiler checks pass: 2187 library/914 native tests, 32 tooling and 30 harness
groups, formatting, Clippy, build, coverage and conformance (`/tmp/meowy-element-gate.log`).
The 209 cases report 190 required passes, 19 pinned gaps and zero failures in both
profiles. Strict mode exits 1 only for those gaps (`/tmp/meowy-element-strict.log`);
all four final documentation checks pass (`/tmp/meowy-element-docs.log`). No selected
check has an outstanding failure.
Preservation against `087ee8e` confirms all 204 prior cases, 236 source assets,
37 reference files, proof obligations, reference hashes and capability exceptions
unchanged (`/tmp/meowy-element-preservation.log`). Unrelated `docs/programs/hey/`
is preserved. No compiler capability or reference contract changed.

## Bounded exclusive indexed-borrow observations

The dependency-ordered implementation series is complete:

| Reviewable slice | Commit |
| --- | --- |
| Retain bounded prefix/per-field counts | `bf99d6f` |
| Capture original index lengths and completion decisions | `29abc4b` |
| Validate path stages and stopped frontiers | `b5cdaef` |
| Report addresses, reservations, acquisition and results | `e8debce` |
| Cover boundaries, snapshots, conflicts and exact limits | `720e062` |
| Pin nested order, stops and dynamic initialized-length bounds | `c7abffc` |
| Pin field permissions, reservation conflicts and lifetimes | `3d0a9bc` |

Reports retain exact place/canonical storage, ordered PathSteps, prefix/per-field
counts, per-index original length inputs and checked normal flags, overall completion
and control. Facts are captured before local/HIR transfer with bounded work and no
type copies. An exclusive-only plan preserves the generic PathStep representation.
Independent flags retain each address/reservation visit and final acquisition/result.

Validation checks complete point/owner/parent/block/span identity, canonical storage,
field/capacity/length bounds, distinct index roots, exact original ledger edges and
operation membership. Reservations precede index evaluation; Checked bounds edges
remain explicit. Selectors after the first stopped index are rejected. Later
unreachable ledger entries remain intact and gain no observations. Missing operations
or successful checking never imply normal completion or runtime reachability.

Prefix plus path length is capped at 256. Copied prefixes, steps, field counts,
index facts and stage flags share the existing 262,144-entry payload allowance.
Duplicate visits reuse records; identity conflicts and exhausted shared work/effect/
payload limits publish no partial collection. Ordinary permissions, reservation
conflicts, moves, lifetimes and capability gates remain authoritative.

Sixteen new internal groups cover source capture, known/unknown lengths, retained
snapshots, nested paths, aliases, owners/control, stopped indices/fields and other
producers. Coverage includes 140 corrupt metadata cases, malformed plans, canonical
alias faults, 14 record conflicts, exact budgets and combined maximum paths
(`/tmp/meowy-exclusive-counts.log`, `/tmp/meowy-exclusive-access.log`,
`/tmp/meowy-exclusive-boundaries.log`). Six required source cases pass in debug/release:
nested order, scalar mutation, aliases, stops, initialized-length bounds, E305 field
permissions, E302 reservation conflicts and E303 local escapes
(`/tmp/meowy-exclusive-execution.log`, `/tmp/meowy-exclusive-rejections.log`).
Structural reports remain distinct from source conformance and proof evaluation.

All ten compiler checks pass: 2203 library/914 native tests, 32 tooling and 30 harness
groups, formatting, Clippy, build, coverage and conformance (`/tmp/meowy-exclusive-gate.log`).
The 215 cases report 196 required passes, 19 pinned gaps and zero failures in both
profiles. Strict mode exits 1 only for those gaps (`/tmp/meowy-exclusive-strict.log`);
all four final documentation checks pass (`/tmp/meowy-exclusive-docs.log`). No selected
check has an outstanding failure.
Preservation against `95a1334` confirms all 209 prior cases, 241 source assets,
37 reference files, proof obligations, reference hashes and capability exceptions
unchanged (`/tmp/meowy-exclusive-preservation.log`). Unrelated `docs/programs/hey/`
is preserved. No compiler capability or reference contract changed.

## Bounded projected-borrow observations

Reports retain exact parent roots, materialization/field/load/address steps, owned
narrowing decisions, reborrow sites, parent modes and control. Field/address counts
come from the concrete checked records before their type context disappears. The
stored mode describes the final parent reference; the produced borrow stays shared.
Independent flags record each projection and narrowing conversion, acquisition and
result. Stopped parents have no site or observation; calls retain conditional returns.

Validation checks complete point/owner/parent/block/span identities, temporary local/
statement registration and roots, exact field bounds, step order, selectors, operation
registration and original edges. Paths retain the 256-step cap; descriptors and their
two flags cost three shared payload entries per step. Duplicates merge flags without
another copy. Invalid identities or exhausted work/effect/payload budgets publish no
partial collection. Ordinary places, shared elements, exclusive indexed paths and
direct/implicit reborrows remain separate; reference cells do not identify pointee
storage. No observation grants borrow authority, extends lifetimes, evaluates a
predicate or proves runtime reachability. `queries::finish` remains B001-gated.

| Reviewable slice | Commit |
| --- | --- |
| Exact field counts and capture bounds | `dc0777e` |
| Source, temporary and stage validation | `d7735b0` |
| Independent projection/conversion/acquisition/result reports | `73be259` |
| Temporary identities, stops and ordinary error boundaries | `f821bad` |
| Shared budgets, maximum paths and atomic conflicts | `c0b741c` |
| Reference-chain, narrowing, order and stopped-parent execution | `39423eb` |
| Owner-write, temporary-lifetime and invalidated-guard rejections | `c6ae594` |

All ten compiler checks pass (`/tmp/meowy-projection-gate.log`): 2216 library,
914 native and 32 tooling/30 harness tests, formatting, all-target Clippy, schemas,
links, coverage and source execution. Thirteen new internal groups cover counts,
128 corrupt stage cases, 36 corrupt temporary cases, 12 merge conflicts, exact
shared limits and independent observations. Six new required source cases pass
debug/release, including exact output, stopped parents/P006 and E302/E303/E208.
The catalog has 221 cases: 202 required passes, 19 unchanged pinned gaps, zero failures.
Strict mode exits 1 solely for the same 19 gaps, with zero failures
(`/tmp/meowy-projection-strict.log`). All four final documentation checks pass
(`/tmp/meowy-projection-docs.log`).

The audit against `ef3fa41` preserves all 215 prior cases, 247 source assets,
37 reference files/hashes, capability pins and 33 proof obligations
(`/tmp/meowy-projection-preservation.log`). The coverage map distinguishes structural
report evidence from source behavior. Full language/release qualification remains
incomplete. Unrelated `docs/programs/hey/` remains untouched.

## Bounded list-construction observations

Explicit producer records now distinguish list construction from generic expression
sequences. They retain capacity, source count, contextual identity, checked completion,
owner/control/span and ordered input roots. Reports preserve optional final contextual
primary/Forward/Convert/Stopped plans; ordinary per-input coercions remain separate.
Projection, conversion, construction and result visits are independent. Stopped inputs
retain earlier stages and disconnected suffix records without construction or results.
The normal flag describes the checked type, so a projected Never can stop construction
even when contextual coercion gives the final HIR a non-Never type.

Validation checks complete roots, exact owners/parents/blocks, unique source slots,
selectors, operation registration and original sequence/endpoint edges. Inputs retain
the 65,536-item cap; descriptors and their two flags cost three shared payload entries
per input. Unused capacity costs no copies. Duplicate visits merge flags. Invalid
metadata, conflicts or exhausted work/effect/payload limits publish no partial map.
No value, allocation, reachability, storage provenance, loan authority or proof answer
is inferred. Required evaluation, list reads/methods/borrows and generic sequences
remain separate. `queries::finish` stays B001-gated.

| Reviewable slice | Commit |
| --- | --- |
| Explicit list identity and checked facts | `3450dfd` |
| Root, source-slot and stage validation | `7344c0f` |
| Independent construction and contextual-stage reports | `6ff637d` |
| Owners, control, calls and malformed contextual plans | `520b8d2` |
| Shared limits, maximum inputs and atomic conflicts | `e90b966` |
| Source order, contextual conversion and stopped suffixes | `9b32d9f` |
| Capacity, inference and scalar-range diagnostics | `13f8aaa` |

The report integration includes six existing regression files whose exact shared
payload totals or Unknown expectations had to change with the new report family.
That dependency required an 11-file slice to keep the intermediate suite passing.

All ten compiler checks pass (`/tmp/meowy-list-construction-gate.log`): 2231 library,
914 native and 32 tooling/30 harness tests, formatting, all-target Clippy, schemas,
links, coverage and source execution. Fifteen new internal groups include 99 corrupt
metadata cases, ten merge conflicts, independent visits, exact shared limits and a
seeded maximum-input boundary. Seven new required source cases pin exact output/P006
and E103/E207/E216 in debug/release. The catalog has 228 cases: 209 required passes,
19 unchanged pinned gaps and zero failures. Strict mode exits 1 only for those gaps
(`/tmp/meowy-list-construction-strict.log`). All four final documentation checks pass
(`/tmp/meowy-list-construction-docs.log`). No outstanding test failures remain.

Preservation against `5e3dff1` passes for all 221 prior cases, 253 source assets,
37 reference files/hashes, capability pins and 33 proof obligations
(`/tmp/meowy-list-preservation.log`). The coverage map and foundation guide distinguish
structural reports from source behavior. Full language/release qualification remains
incomplete. Unrelated `docs/programs/hey/` is preserved.

## Bounded dispatch observations

Dispatch reports retain exact receiver input/local/body identities, checked body
completion and control, with independent initialization/result visits. Receiver
completion is captured separately before HIR transfer. Stopped receivers produce no
observation; stopped bodies retain only initialization. Conditional calls keep their
return edges, and nested/ordinary/composed dispatch preserve their existing checking.
Body effects remain independent; reports copy neither receiver values nor statements.

Validation checks complete points, owners, parents/blocks, the synthetic receiver
Bind and its storage/source/link identity, receiver/dispatch registration, the leading
None slot, immediate/final statement sites and exact dispatch/body endpoint edges.
An opaque successor does not connect initialization to a later statement. Metadata
has fixed size and shares work/map limits; duplicate visits merge flags. Conflicts
and exhausted budgets publish no partial collection. These observations infer no
reachability, values, pointee storage, ownership/borrow authority or proof answers.
`queries::finish` remains B001-gated.

| Reviewable slice | Commit |
| --- | --- |
| Separate checked receiver/body completion | `833ab1a` |
| Receiver binding and stage validation | `68334bb` |
| Independent initialization/result reports | `73511c7` |
| Opaque successors, receiver identities and source boundaries | `7768051` |
| Shared work/map limits and atomic conflicts | `a6fae62` |
| Receiver order, composition and stopped-stage execution | `9f5dcc5` |
| Lifetimes, loans and composed-result diagnostics | `6d1ddbc` |

All ten compiler checks pass (`/tmp/meowy-dispatch-reports-gate.log`): 2245 library,
914 native and 32 tooling/30 harness tests, formatting, all-target Clippy, schemas,
links, coverage and source execution. Fourteen new internal groups cover 88 malformed
identity cases, seven merge conflicts, an opaque-successor walk, independent visits,
exact shared limits and fixed report costs for 1/65,536-byte receiver values. Eight
new required source cases pin nested/empty/composed order, forward declarations,
scoped leave, stopped receivers/bodies/P006 and E303/E302/E305/E204 in debug/release.
The catalog has 236 cases: 217 required passes, 19 unchanged pinned gaps, zero failures.
Strict mode exits 1 only for those gaps (`/tmp/meowy-dispatch-reports-strict.log`).
All four final documentation checks pass (`/tmp/meowy-dispatch-reports-docs.log`).
No outstanding test failures remain.

Preservation against `bf9a160` passes for all 228 prior cases, 260 source assets,
37 reference files/hashes, capability pins and 33 proof obligations
(`/tmp/meowy-dispatch-preservation.log`). The coverage map and foundation guide
separate structural evidence from source behavior. Full language/release qualification
remains incomplete. Unrelated `docs/programs/hey/` is preserved.

## Bounded emission observations

Emission reports retain exact input/EmitId/target identities, Value/Primary/Field
projections, composition local/count, field names, canonical alias storage and
control. Every target initialization and statement completion is independent.
An emission is not a scope exit or publication of the whole block: tail work and
failures still follow. Stopped inputs and disconnected later statements gain no
observation; required/type-only emissions remain separate.

Validation uses the emission-source registry rather than invented Operation ports.
It checks completed roots, bounded same-site matcher ancestry, lexical target scopes,
composition bounds, unique slots, aliases and exact stored edges. Descriptors and
visit flags cost two shared payload entries per target, plus UTF-8 field-name bytes.
Caps of 65,536 targets and 262,144 bytes of names apply before copying, with shared
work/map/payload limits. Duplicate visits merge flags without another copy. Invalid
metadata, conflicts or exhaustion publish no partial collection. No values,
reachability, storage provenance, ownership authority or proof outcomes are inferred.

| Reviewable slice | Commit |
| --- | --- |
| Composition local and field-count prerequisite | `b54c7d6` |
| Sources, targets, aliases and exact edge validation | `298e75b` |
| Inherited matcher lifetime-site validation | `de88cc4` |
| Independent target and statement-completion reports | `2261568` |
| Composition, lexical ancestry and canonical alias boundaries | `7c7e172` |
| Shared payload/work limits and atomic merges | `e28d8ea` |
| Order, composition, aliases and stopped-source execution | `acb7fe0` |
| Composition, scope, type and lifetime errors | `36cead6` |
| Final lint repair and compiler gate | `ff256ab` |

Report integration needed an 18-file slice after split review: 14 existing exact
shared-budget regressions in 13 files had to count the new emission copies when the
hook became active. Either side alone would break their exact pass/fail boundaries.
Additional validation, boundary and source scenarios remained separate commits.

All ten compiler checks pass (`/tmp/meowy-emission-reports-gate.log`) on LLVM 23.1.1
and Rust 1.99.0: 2262 library/914 native tests, 32 tooling/30 harness groups, formatting,
all-target Clippy, schemas, links, coverage and source execution. Thirteen new report
and validation groups cover 141 seeded identity/site/ancestry faults, 15 merge
conflicts, independent visits, shared exact limits and bounded field-name copies.
Eight new required sources pin normal output/P006 and E205/E207/E201/E303. Conformance
has 244 cases: 225 required passes, 19 unchanged pinned gaps and zero failures in both
profiles. Strict mode exits 1 only for those gaps (`/tmp/meowy-emission-reports-strict.log`).
All four final documentation checks pass (`/tmp/meowy-emission-reports-docs.log`).
No outstanding test failures remain.

The audit against `16c06d9` preserves all 236 prior cases, 268 source assets,
37 reference files/hashes, capability pins and 33 proof obligations
(`/tmp/meowy-emission-preservation.log`). Coverage and the foundation guide distinguish
structural metadata from source execution. Proof outcomes and full language/release
qualification remain incomplete. Unrelated `docs/programs/hey/` is preserved.

## Bounded non-scalar equality observations

Checked record/list/shared-reference/union equality now has bounded Binary reports.
Ordinary `binary_plan_values` establishes complete type identity and recursive
eligibility before recording its successful equality decision. Compact operand
classes retain field/member counts, list capacities and reference modes without
copying or walking types. Equal summaries alone never admit a source comparison.
The eligibility flag does not record an equality result or arithmetic success.

Reports preserve exact ordered operand roots, projections, owner/control flags and
independent operation/result visits. Existing sequence/operation edges and registered
owners remain authoritative. A stopped input can retain its projection but cannot
introduce later stages. Fixed-size records add no variable payload copies; operand
producers keep their own costs. Shared work/map limits, duplicate visits and metadata
conflicts preserve atomic publication. Unsupported nominal types remain Other;
exclusive-reference comparisons retain their B001 ownership gate and Unknown reports.
Values, addresses, borrow authority, reachability and proof outcomes are not inferred.

| Reviewable slice | Commit |
| --- | --- |
| Capture checked equality and bounded categories | `3db93eb` |
| Admit supported equality reports | `308fcbc` |
| Validate categories, owners and complete type compatibility | `240cec7` |
| Bound shared work/maps and atomic merges | `c685bf3` |
| Pin whole-shape results, reference addresses and operand order | `4735fb1` |
| Pin incompatible types and recursive opaque-member rejections | `ccebb5d` |

Twelve new internal groups cover capture, stopped stages, matching-summary type
mismatches, 32 corrupt signatures, fixed costs across capacities and atomic limits.
All ten compiler checks pass on LLVM 23.1.1/Rust 1.99.0: 2274 library/914 native tests,
32 tooling/30 compiler-harness groups, formatting, Clippy, build and conformance
(`/tmp/meowy-equality-reports-gate.log`). The eight new required source cases pass
in debug/release. The catalog has 252 cases: 233 required passes, 19 unchanged pinned
gaps and zero failures. Strict mode exits 1 only for those gaps
(`/tmp/meowy-equality-reports-strict.log`). All four final documentation checks pass
(`/tmp/meowy-equality-reports-docs.log`).

All 244 prior case records, 276 source assets, 37 reference contracts/hashes,
capability pins and 33 proof obligations are unchanged
(`/tmp/meowy-equality-preservation.log`). Classified coverage and the foundation
guide distinguish metadata tests from observable source behavior. The native runtime
and toolchain sources did not change in this series. Preserve `docs/programs/hey/`.

## Stopped operand context repair

Binary checking no longer supplies Never as a value-construction context. Each
operand hint is filtered after scalar-primary selection, so a usable RHS hint or
enclosing arithmetic width can still guide literals. The RHS uses the checked left
type when available, otherwise the selected context or independent inference.
Boolean operators retain their boolean context. A known Never left call bypasses
record composition, preserving its stopped type and graph edges instead of coercing
it into a normal operand. Full-record equality and shared composition stay unchanged.

This repairs the E207 exposed by a leaving left block compared with an emitting
RHS block, as well as widths hidden by nested Never hints. Later operands are still
checked for names, annotations, duplicate declarations and operator errors. Runtime
order, stopped stages, ordinary type compatibility and required evaluation remain
intact. No reference contract, ownership gate or proof outcome changed.

| Reviewable slice | Commit |
| --- | --- |
| Repair context selection with checker and native regressions | `418e681` |
| Require constructor/order, width and unreachable-error source cases | `e6621f4` |

Four checker groups and one native group pass
(`/tmp/meowy-stopped-context-focused.log`). Four new required cases pass independently
in debug/release (`/tmp/meowy-stopped-binary-conformance.log`): list/record and boolean
stop order, wide/nested arithmetic, missing-name E201 and explicit-width E216.
The original reproduction now reaches its expected P006 with ordered output
(`/tmp/meowy-stopped-context-repro.log`). All ten compiler checks pass on LLVM 23.1.1
and Rust 1.99.0: 2278 library/915 native tests, 32 tooling/30 compiler-harness groups,
formatting, Clippy, build and conformance (`/tmp/meowy-stopped-context-gate.log`).
Conformance has 237 required passes, 19 unchanged pinned gaps and zero failures in
debug/release. Strict mode exits 1 only for those gaps
(`/tmp/meowy-stopped-context-strict.log`). All four final documentation checks pass
(`/tmp/meowy-stopped-context-docs.log`).

All 252 prior case records, 284 source assets, 37 reference contracts/hashes,
capability pins and 33 proof obligations are unchanged
(`/tmp/meowy-stopped-context-preservation.log`). The catalog has 256 cases, with
237 required and the same 19 pinned B001 gaps. Classified coverage closes the
specific Never/RHS-constructor gap; broader contextual builders and composition
transfers remain separately bounded. Unrelated `docs/programs/hey/` is preserved.

## Bounded ordinary block completion and result observations

`Reports.blocks` now retains exact BlockId keys independently of PointId operation
effects. Each record has its checked owner, parent, source span and shallow
completion/result shape, with separate normal/result visit flags. Empty program
and function roots stay independent; partial records keep their actual checked
field counts. The capture cap is 65,536 bodies/counts, with oversized or unsupported
shapes retained as Other without walking or copying types.

Validation checks root ownership, complete expression parents, registered enclosing
bodies, source spans, all statement sites, sequence adjacency and exact endpoint
order. Unknown slots stay barriers. Grouped contextual list bodies may have wider
parent spans. The synthetic owner-0 module-startup root preserves per-file statement
spans outside the entry-file span; function and nested-body containment remains
required. A Never block may have a structural normal visit, but no checked result
edge. Dispatch keeps its own reports; required-only bodies add no runtime reports.

Block and operation records share the 262,144-entry map limit and existing Flow work
budget. Entry/call-graph bounds remain separate. Duplicate visits merge fixed flags;
conflicts, malformed metadata or exhausted limits publish no partial collection.
Emission-slot initialization and result availability stay separate. No slot values,
reachability, ownership authority, restart propagation or proof outcomes are inferred.

| Reviewable slice | Commit |
| --- | --- |
| Capture bounded checked body spans/completion | `1816109` |
| Validate ordinary block ports and endpoint identities | `0d5b5de` |
| Collect independent normal/result observations | `e5191fc` |
| Cover corrupt identities and unknown sequence barriers | `84e8d09` |
| Bound shared maps/work and atomic merges | `16a4e99` |
| Pin block results, outer leaves and tail-effect order | `fb8b386` |
| Pin missing and duplicate emitted fields | `b8b80a0` |
| Require registered enclosing identities | `e00f349` |
| Preserve module-root source spans and original replay spans | `8d53ba2` |

Nineteen new internal groups cover capture, result shapes, independent roots/stages,
corrupt metadata, grouped source spans, unknown slots, exact budgets and atomic
merges. The module regression also pins wrapper/module spans and result reports.
All ten compiler checks pass on LLVM 23.1.1/Rust 1.99.0: 2297 library/915 native tests,
32 tooling/30 compiler-harness groups, formatting, Clippy, build and conformance
(`/tmp/meowy-block-reports-gate.log`). Six required source cases
pass independently in debug/release (`/tmp/meowy-block-source-fixtures.log`), covering
nested partial composition, outer leaves, unused Never definitions, tail panic,
missing fields (E204) and duplicate emissions (E205). Full conformance has 243
required passes, 19 unchanged pinned gaps and zero failures in debug/release.
Strict mode exits 1 only for those gaps (`/tmp/meowy-block-reports-strict.log`). All
four final documentation checks pass (`/tmp/meowy-block-reports-docs.log`).

All 256 prior case records, 288 source assets, 37 reference contracts/hashes,
capability pins and 33 proof obligations are unchanged
(`/tmp/meowy-block-preservation.log`). Classified coverage distinguishes structural
reports from observable source behavior. The catalog has 262 cases: 243 required
and the same 19 pinned gaps. Unrelated `docs/programs/hey/` is preserved.

## Bounded block result-slot source links

Checked layouts now retain an explicit primary plus ordered field names, mutability
and shallow shapes before HIR transfer. Each layout has caps of 65,536 slots and
65,536 name bytes; persistent totals cap slots and name bytes at 262,144 each.
Preflight bounds precede copies, successful replay charges no new persistent payload,
and failed capture publishes no metadata/counters. Stopped/unknown layouts remain
distinct. Endpoint validation checks exact names/order/mutability/shape agreement.

`Reports.results` associates observed BlockResult slots with exact initialized
emission target identities: EmitId, statement PointId and target index. Composition
projections remain recoverable. Joins use owner, BlockId and exact optional field
name; ordinary consumers use validated block-expression endpoints. Program/function
roots acquire no caller link. Target visits stay independent of statement completion.

These candidates record observed initialization history. Discarded paths may supply
candidates with a value shape unlike the final scalar slot; valid targets absent
from the checked layout create no slot. Mutable slots or mutable-alias histories,
aggregate/union slots and unsupported layouts remain unknown. An empty candidate
set does not establish null, completeness, a compatible value domain or a selected
runtime contributor. Ownership and proof-outcome gates are unchanged.

Operation, block and result maps share the existing map cap. Scratch source-index
rows/candidates, result-slot entries and copied candidates use the remaining
operation payload budget. Index names are borrowed; shared work charges validation,
lookups and copies. Duplicates retain one descriptor. Late failure leaves previous
maps, capture counters and remaining payload unchanged.

| Reviewable slice | Commit |
| --- | --- |
| Capture bounded checked layouts and persistent copy totals | `0592307` |
| Preserve remaining operation payload budget | `3395b59` |
| Validate result-slot identities before reporting | `716f535` |
| Join conservative emission candidates and consumers | `16cdf31` |
| Cover independent visits, composition and discarded paths | `3a30543` |
| Bound combined maps/payload/work and atomic publication | `cda38c4` |
| Pin defaults, branches, composition and stopping order | `cb60d46` |
| Pin duplicate-slot and mixed-mutability errors | `c41d71c` |

Sixteen new internal groups cover capture, shallow validation, budget carry-over,
source identity, independent visits, mutable discarded aliases, exact limits and
atomic failures. All ten compiler checks pass on LLVM 23.1.1/Rust 1.99.0: 2313
library/915 native tests, 32 tooling/30 compiler-harness groups, formatting, Clippy,
build and conformance (`/tmp/meowy-result-slot-gate.log`). Six new required fixtures
pass both profiles (`/tmp/meowy-result-slot-source-fixtures.log`), covering nullable
defaults, branch choices, aliases, composition/outer-target order, stopping after
initialization, duplicate slots (E205) and mixed mutability (E206).
Conformance has 268 cases: 249 required passes, 19 unchanged pinned gaps and zero
failures in debug/release. Strict mode exits 1 only for those gaps
(`/tmp/meowy-result-slot-strict.log`). All four final documentation checks pass
(`/tmp/meowy-result-slot-docs.log`).

All 262 prior case records, 294 source assets, 37 reference contracts/hashes,
capability pins and 33 proof obligations are unchanged
(`/tmp/meowy-result-slot-preservation.log`). Coverage and the guide distinguish
structural source history from observable source behavior. Unrelated
`docs/programs/hey/` is preserved.

## Direct block result-slot consumers

`Reports.consumers` indexes validated block-result consumer PointIds to exact
BlockIds/owners. Root/function results remain separate; unknown layouts remain
explicit. `Reports.slot_uses` maps extraction ports to `(block, slot)` identities,
without copying or resolving source candidates. Direct owned field Operation ports
use checked field index+1, preserving HIR field order, field count and availability.
A field's normal flag is checked result availability, not an observed Normal visit.

Observed unary/binary primary Projection ports use slot0 and retain the binary
step. Full producer headers, identities, edges and every observed stage are
validated. Operation/result flags alone cannot create a projection link, and a
left projection may exist before a stopped RHS with no operation or result.
Owning effects keep flags; result sources keep multiple/unknown candidate state.
Reference loads, calls, primary-extracting/converting coercions, ascriptions and
unclassified regions remain opaque as forwarding sources. Wrapper and immutable-read
forwarding are described below. Ordinary scalar-context constructor errors are unchanged.

The operation/block/result maps, reverse index and extraction links share the map
cap. Fixed-size consumer records consume no variable payload. Shared work charges
validators and subsequent body/field lookups. Duplicate visits retain one link per
port. Late failure preserves prior maps, candidate histories and payload counters.
Seeded Never-field availability is structural evidence only; ordinary selection
from a Never block still reports E201. No value, lifetime, caller-return or proof
inference is added.

| Reviewable slice | Commit |
| --- | --- |
| Index exact validated result consumers | `ec42f09` |
| Link direct owned-field Operation ports | `e07bd3d` |
| Link observed unary/binary primary Projection ports | `2fba3c2` |
| Cover field identities, availability and unknown sources | `120c657` |
| Bound shared maps/work and preserve partial observations | `2ac60c2` |
| Pin field order, tails, owners and E201 | `06b7c8b` |
| Pin primary order, stopped RHS and E222 | `f0dba6d` |

Seventeen new internal groups cover reverse identities, 18 field corruptions,
independent visits, seeded Never availability, 26 primary header/stage faults,
late failures and exact map/work limits (`/tmp/meowy-consumer-boundaries.log`).
All ten compiler checks pass on LLVM 23.1.1/Rust 1.99.0: 2330 library/915 native tests,
32 tooling/30 compiler-harness groups, formatting, Clippy, build and conformance
(`/tmp/meowy-consumer-gate.log`). Six required fixtures pass debug/release with the
rebuilt compiler (`/tmp/meowy-result-consumer-source-final.log`). Conformance has
274 cases: 255 required passes, 19 unchanged pinned gaps and zero failures.
Strict mode exits 1 only for those gaps (`/tmp/meowy-consumer-strict.log`). All four
final documentation checks pass (`/tmp/meowy-consumer-docs.log`).

All 268 prior case records, 300 source assets, 37 reference contracts/hashes,
capability pins and 33 proof obligations are unchanged
(`/tmp/meowy-consumer-preservation.log`). Coverage and the guide distinguish
structural links from observable field/projection behavior. Unrelated
`docs/programs/hey/` is preserved.

## Bounded grouped-expression forwarding

Ordinary and composed AST groups now register exact checked input, owner, block and
span identities alongside their unchanged two-edge regions. Required-only checking
keeps its old edges without a marker. Shared-reference, statement/control and
specialized group-flattening paths acquire no group classification. Capture has a
65,536-entry cap; identity/replay/map/work checks precede edge publication so failures
publish neither a partial marker nor new edges.

Consumer resolution follows only those explicit identities. It validates completed
Expr wrappers, Expr/And/Or children, parent/owner/block/span agreement and the exact
ordered edges. Direct-anchor/group overlap, malformed metadata and cycles fail.
Unmarked terminals remain unknown. Scratch is bounded to 65,536 visited groups;
shared work covers every lookup, comparison and insertion. Reversing seeded wrapper
IDs preserves the answer, so traversal does not depend on numerical allocation order.

Resolved anchors reuse existing field and unary/binary primary slot links. Original
extraction ports, independent flags and candidate/unknown state remain unchanged.
Stored Normal edges never establish completion, and no Entry-to-Normal shortcut is
added. A left projection can remain observed before a stopped RHS without operation
or result observations. Calls, ascriptions, shared-reference forwarding and
unclassified regions remain outside these links. Later Forward and coercion-owned
consumer support is described below; no value or proof is inferred.

| Reviewable slice | Commit |
| --- | --- |
| Capture explicit AST-group forwarding identities | `eb7e6d0` |
| Resolve bounded checked chains to existing consumers | `b1311c1` |
| Cover corrupt identities, cycles and opaque regions | `a790e3e` |
| Bound traversal, reordered IDs and atomic failures | `d1f839d` |
| Pin grouped execution and stopping order | `4817fba` |
| Pin grouped E201/E222 rejection behavior | `5da70d0` |

Eighteen new internal groups cover capture, repeated spans, required exclusion,
owners, stopped children, 28 corruption variants, cycles, logical terminals and
exact hop/map/work limits. Grouped-focused checks pass
(`/tmp/meowy-group-forwarding-tests.log`, `/tmp/meowy-group-boundaries.log`). All ten
compiler checks pass on LLVM 23.1.1/Rust 1.99.0: 2348 library/915 native tests,
32 tooling/30 compiler-harness groups, formatting, Clippy, build and conformance
(`/tmp/meowy-group-gate.log`). Six required fixtures pass rebuilt-compiler debug/release
(`/tmp/meowy-group-consumer-source-final.log`). Conformance has 280 cases: 261
required passes, 19 unchanged pinned gaps and zero failures. Strict mode exits 1
only for those gaps (`/tmp/meowy-group-strict.log`). All four final documentation
checks pass (`/tmp/meowy-group-docs.log`).

All 274 prior case records, 306 source assets, 37 reference contracts/hashes,
capability pins and 33 proof obligations are unchanged
(`/tmp/meowy-group-preservation.log`). Source execution and structural forwarding
remain separate evidence. Unrelated `docs/programs/hey/` is preserved.

## Observed Forward coercion consumers

Consumer resolution now traverses mixed explicit-group and observed Forward chains.
The coercion helper requires Forward, no primary extraction and an observed result;
reported owner/input/kind/control must match the checked producer. Existing stage
validation checks completed roots, parents, owners, equal spans and exact edges.
Conversions, projected Forward, Stopped, reborrows and missing observations remain
opaque. Groups retain their existing capture rules and contained-span validation.

Mixed traversal shares one 65,536-hop scratch bound and the existing Flow budget.
Group/coercion/direct-anchor overlap is invalid even without a coercion observation.
Cycles and corrupt late metadata fail without changing reports, captured registries
or remaining payload. No values, candidate copies, loan authority, caller provenance
or proof answers are introduced. Real typed initializer roots now resolve through
five alternating wrappers in both entry and function owners. Local-read joins are
qualified separately by the consumer pass below.

| Reviewable slice | Commit |
| --- | --- |
| Qualify observed Forward wrappers and resolve mixed chains | `97032a4` |
| Cover mixed identity conflicts, cycles and resource limits | `c2552a2` |
| Pin typed initializer order, owners and stopped execution | `31f9912` |
| Pin typed slot E207 and width E216 rejections | `4523ee7` |

Ten new internal groups cover eligibility, exact edges, 24 corruption variants,
classification overlap, cycles, missing results, logical terminals and exact
hop/work bounds. Focused checks pass (`/tmp/meowy-forward-consumers.log`,
`/tmp/meowy-forward-qualifier.log`, `/tmp/meowy-forward-mixed-tests.log`). All six
new required cases pass rebuilt-compiler debug/release
(`/tmp/meowy-forward-source-final.log`). All ten compiler checks pass on
LLVM 23.1.1/Rust 1.99.0: 2358 library/915 native tests, 32 tooling/30 compiler-harness
groups, formatting, Clippy, build and conformance
(`/tmp/meowy-forward-gate.log`). Catalog/coverage checks pass at 286 cases:
267 required passes, 19 unchanged pins and zero failures in debug/release. Strict
mode exits 1 only for those gaps (`/tmp/meowy-forward-strict.log`). All four final
documentation checks pass (`/tmp/meowy-forward-docs.log`).

All 280 prior case records, 312 source assets, 37 reference files/reviewed hashes,
capability pins and 33 proof obligations are unchanged
(`/tmp/meowy-forward-preservation.log`). Source execution and internal structural
qualification remain separate evidence. Unrelated `docs/programs/hey/` is preserved.

## Observed unchanged narrowing consumers

Unchanged narrowing wrappers now join the bounded group/Forward resolver. The
helper requires unchanged checked type handling, normal availability and an observed
result, with exact reported owner/input/changed/normal/control agreement. An
operation visit on an unchanged wrapper is invalid. Existing stage validation
rechecks complete Expr children, equal spans, parents, owners, blocks and edges.
Changed, nonnormal and unobserved wrappers remain opaque.

Group/Forward/narrowing/direct-anchor classification overlaps fail even without
observations. Mixed chains share the existing 65,536-hop scratch and Flow bounds.
Failures preserve reports, capture registries and payload. Real typed local reads traverse six
wrappers to Read; initializer traversal is handled separately below. Field wrappers
stop at Field and preserve existing extraction links. Direct narrowing-to-block
chains retain seeded structural evidence; field-value joins remain separate.

| Reviewable slice | Commit |
| --- | --- |
| Qualify unchanged wrappers and integrate bounded traversal | `a8442b3` |
| Cover identity conflicts, cycles and shared work limits | `554cbfb` |
| Pin read order, mutable snapshots, stopping and E208 | `9a251dd` |

Ten new internal groups cover real read/field boundaries, owner/control metadata,
19 corruption variants, overlap, cycles and exact hop/work limits. Focused checks
pass (`/tmp/meowy-narrow-consumers.log`, `/tmp/meowy-narrow-mixed.log`,
`/tmp/meowy-narrow-qualifier-limits.log`). Four required source cases pass rebuilt
compiler debug/release (`/tmp/meowy-narrow-source-final.log`). Catalog/coverage
checks pass (`/tmp/meowy-narrow-metadata.log`). All ten compiler checks pass on
LLVM 23.1.1/Rust 1.99.0: 2368 library/915 native tests, 32 tooling/30 compiler-harness
groups, formatting, Clippy, build and conformance (`/tmp/meowy-narrow-gate.log`).
The 290-case catalog has 271 required passes, 19 unchanged pins and zero failures
in debug/release. Strict mode exits 1 only for those gaps
(`/tmp/meowy-narrow-strict.log`). All four final documentation checks pass
(`/tmp/meowy-narrow-docs.log`).

All 286 prior case records, 318 source assets, 37 reference files/reviewed hashes,
19 capability pins and 33 proof obligations are unchanged
(`/tmp/meowy-narrow-preservation.log`). Source execution remains distinct from
structural eligibility. Unrelated `docs/programs/hey/` is preserved.

## Bounded immutable-local eligibility

Entry reports now retain eligible LocalIds directly from borrowed `Program.locals`
after HIR transfer. Primitive values (including Null/Never), records, lists and
unions qualify only when recursively immutable and reference-free. All Foundation
variants remain opaque. Mutable bindings and nested mutable fields are excluded;
field flags are inspected independently of the existing `proofs.fields` registry.

The iterative type walk charges inspected nodes, fields and pending children, with
a 65,536-entry scratch cap. A deeper chain using one pending slot remains valid;
shared Flow work bounds its traversal. Local count, binding count and every binding
ID must agree, including unread locals. Eligible IDs append after existing slot links
under their combined MAX_EDGES map cap. No type/candidate copies or payload charges
are added. Late identity/work/capacity failures publish no partial set.

Parameters and emitted aliases may be type-eligible. Eligibility grants no ordinary
initializer, owner/lifetime authority, completion, value or proof result. Eligibility
alone grants no read or field-value join; qualified read forwarding is described below.

| Reviewable slice | Commit |
| --- | --- |
| Retain bounded type eligibility after HIR transfer | `5af4c18` |
| Cover identities, shared capacity and iterative type bounds | `407c6d7` |
| Pin immutable copies, mutable snapshots and shared-write rejection | `5e7e3a5` |

Seven new eligibility groups pass (`/tmp/meowy-local-eligibility-limits.log`), along
with existing entry-report tests (`/tmp/meowy-local-entries.log`) and library lint
(`/tmp/meowy-local-lint.log`). Three required source cases pass rebuilt-compiler
debug/release (`/tmp/meowy-local-source-final.log`). Metadata/coverage checks pass
at 293 cases: 274 required and 19 unchanged pins (`/tmp/meowy-local-metadata.log`).
All ten compiler checks pass on LLVM 23.1.1/Rust 1.99.0: 2375 library/915 native
tests, 32 tooling/30 compiler-harness groups, formatting, Clippy, build and conformance
(`/tmp/meowy-local-gate.log`). All 274 required cases pass debug/release, with
19 unchanged pins and zero failures. Strict mode exits 1 only for those gaps
(`/tmp/meowy-local-strict.log`). All four final documentation checks pass
(`/tmp/meowy-local-docs.log`).

All 290 prior case records, 322 source assets, 37 reference files/reviewed hashes,
19 capability pins and 33 proof obligations are unchanged
(`/tmp/meowy-local-preservation.log`). Source behavior and structural eligibility
remain separate evidence. Unrelated `docs/programs/hey/` is preserved.

## Exact observed Bind qualification

The Bind qualifier validates observed Storage metadata during initializer indexing.
Raw Storage reports retain their existing representation.
The qualifier requires exact reported/checked owner, kind, local, canonical storage,
input and control headers; complete statement/site identities; registered Operation
ownership; local/binding bounds; checked containing-body ownership; and exact
ordered producer edges. Initializer roots retain their checked parent/block/site,
owner, kind and contained span. Writes gain no Bind qualification.

Matcher inner statements share lifetime sites, so a bounded charged parent walk
reaches the exact site root. Site-span checks do not impose parent-span nesting on
matcher conditions. Synthetic module roots preserve empty outer-body spans and
equal initializer/statement spans. A containing body's later stop does not invalidate
an earlier observed Bind. No new normal-completion claim is introduced.

A qualified absent initializer is `Some(Binding { input: None, .. })`, distinct from
an unobserved/non-Bind result. It requires exactly Operation-to-Normal; removing an
input while retaining stale edges fails. Work, identity and cycle failures preserve
all reports, captured registries and payload. Qualification alone adds no map or
read join; eligibility and alias/parameter admission belong to the index below.

| Reviewable slice | Commit |
| --- | --- |
| Qualify observed Bind identities and validate entry reports | `7ec1583` |
| Cover corrupt identities, unknown inputs, cycles and work | `970c6a3` |
| Pin initializer order, stopping and E207 rejection | `6d910f8` |

All seven new Bind groups pass (`/tmp/meowy-bind-limits.log`), including 35 corrupt
identity variants and exact/one-short work. Existing entry/module/Storage regressions
and library Clippy pass (`/tmp/meowy-bind-entries.log`, `/tmp/meowy-bind-modules.log`,
`/tmp/meowy-bind-effects.log`, `/tmp/meowy-bind-lint.log`). Three required source cases
pass rebuilt-compiler debug/release (`/tmp/meowy-bind-source-final.log`). Metadata
checks pass at 296 cases, 277 required and 19 unchanged pins
(`/tmp/meowy-bind-metadata.log`). All ten compiler checks pass on LLVM 23.1.1/Rust
1.99.0: 2382 library/915 native tests, 32 tooling/30 compiler-harness groups,
formatting, Clippy, build and conformance (`/tmp/meowy-bind-gate.log`). All 277
required cases pass debug/release, with 19 unchanged pins and zero failures.
Strict mode exits 1 only for those gaps (`/tmp/meowy-bind-strict.log`). All four
final documentation checks pass (`/tmp/meowy-bind-docs.log`).

All 293 prior case records, 325 source assets, 37 reference files/reviewed hashes,
19 capability pins and 33 proof obligations are unchanged
(`/tmp/meowy-bind-preservation.log`). Source behavior and structural qualification
remain separate evidence. Unrelated `docs/programs/hey/` is preserved.

## Bounded ordinary initializer index

Entry reports now index admitted ordinary Bind identities by LocalId, retaining
statement, owner and optional outer input root. The builder qualifies every observed
Storage header before admission, replacing the earlier validation-only pass. Eligible
immutable locals must be their own canonical storage and have no alias, parameter,
receiver or temporary-cell classification. Writes and missing observations add no
entry; malformed excluded Storage still fails validation.

Parameters come from borrowed Program functions with checked FunctionId owners and
entry/body identities, independent of vector order. Duplicate/out-of-range parameters,
conflicting function contexts and exhausted bounded scratch/work fail. The retained
index shares remaining capacity with effects, blocks, results, consumers, slot uses
and eligibility; it copies no types/candidates and leaves variable payload unchanged.
Rebuilds ignore the old index size because they return a fresh replacement.

A second statement for an admitted LocalId is a conflict even if owner/input agrees.
Missing roots remain explicit indexed unknowns. Conditional declarations retain
structural history without proving execution or selecting a value. Exact outer
roots remain unresolved in the index and owners stay independent. The consumer
pass below now handles qualified local reads.
Late failures publish no partial map or report mutation.

| Reviewable slice | Commit |
| --- | --- |
| Validate bounded function/parameter exclusion context | `d3b22ec` |
| Index qualified ordinary initializer identities | `869e5c7` |
| Cover conflicts, unknown roots, exclusions and shared limits | `a433f00` |
| Pin source order, special sources and stopped initializers | `2d460d2` |

Eleven new internal groups pass: three parameter groups, two admission/root groups
and six boundary groups (`/tmp/meowy-initializer-parameters.log`,
`/tmp/meowy-init-limits.log`). Related initializer, entry-report and Bind regressions
plus library Clippy pass (`/tmp/meowy-init-integration.log`, `/tmp/meowy-init-entries.log`,
`/tmp/meowy-init-bindings.log`, `/tmp/meowy-init-lint.log`). Three required source
cases pass rebuilt-compiler debug/release (`/tmp/meowy-init-source-final.log`).
Catalog/coverage checks pass at 299 cases, 280 required and 19 unchanged pins
(`/tmp/meowy-init-metadata.log`). All ten compiler checks pass on LLVM 23.1.1/Rust
1.99.0: 2393 library/915 native tests, 32 tooling/30 compiler-harness groups,
formatting, Clippy, build and conformance (`/tmp/meowy-init-gate.log`). All 280
required cases pass debug/release, with 19 unchanged pins and zero failures.
Strict mode exits 1 only for those gaps (`/tmp/meowy-init-strict.log`). All four
final documentation checks pass (`/tmp/meowy-init-docs.log`).

All 296 prior case records, 328 source assets, 37 reference files/reviewed hashes,
19 capability pins and 33 proof obligations are unchanged
(`/tmp/meowy-init-preservation.log`). Source behavior and structural indexing
remain separate evidence. Unrelated `docs/programs/hey/` is preserved.

## Validated local-read initializer forwarding

Eligibility and initializer collection now precede slot-use extraction. Extraction
reserves both retained collections; eligibility rebuilds reserve the existing index.
Each of these collectors excludes its own prior output from the aggregate cap. Rebuilds
preserve report maps and variable payload, and extraction runs once.

Observed canonical immutable Read operations now resolve through revalidated
same-owner initializer identities. The helper compares complete Read metadata against
`read_effect`, checks normal availability, eligibility and ordinary storage, then
revalidates the indexed statement through `binding_effect`. Missing observations,
indexes or roots remain unknown. Corrupt Read/Bind/index metadata fails atomically.
Normal is checked availability, not an observed Normal visit or runtime completion.

Read-to-initializer lookup can cross statements, blocks and spans within one owner;
wrapper parent/containment rules apply only to their own edges. Reads join the
exclusive group/Forward/narrowing/anchor classifications and share the existing
65,536-hop scratch and Flow limits. Consistent mixed initializer cycles fail.
Owned field and observed binary-primary consumers preserve original ports and slot
identities through immutable copy chains. Unary projections performed by expected
coercions now retain their own consumer ports below; no additional projection is inferred. Special storage,
field values, calls, mutable/reference inputs and caller provenance stay opaque.

| Reviewable slice | Commit |
| --- | --- |
| Reserve initializer maps before consumer extraction | `f83ba59` |
| Consolidate existing local-read expectations without behavior changes | `64339d7` |
| Forward immutable reads to checked initializer roots | `1308c62` |
| Bound mixed read chains, conflicts and missing evidence | `b2d0033` |
| Pin source order, barriers, panic and shared-write rejection | `8e5c5d1` |

Eight new regression groups plus consolidated consumer coverage exercise shared
budgets, real owner/copy chains, 17 identity corruptions, producer overlap, mixed
cycles, missing evidence and exact hop/work limits. Focused checks and library lint
pass (`/tmp/meowy-read-consumers.log`, `/tmp/meowy-read-qualifier.log`,
`/tmp/meowy-read-mixed.log`, `/tmp/meowy-read-lint.log`). All ten compiler checks pass
on LLVM 23.1.1/Rust 1.99.0: 2402 library/915 native tests, 32 tooling/30 harness
groups, formatting, Clippy, build and conformance (`/tmp/meowy-read-gate.log`).
Four required cases pass rebuilt-compiler debug/release
(`/tmp/meowy-read-source-final.log`). The catalog has 303 cases: 284 required passes,
19 unchanged pins and zero failures in debug/release. Strict mode exits 1 only for
those gaps (`/tmp/meowy-read-strict.log`). All four final documentation checks pass
(`/tmp/meowy-read-docs.log`).

All 299 prior case records, 331 source assets, 37 reference files/reviewed hashes,
19 capability pins and 33 proof obligations are unchanged
(`/tmp/meowy-read-preservation.log`). Structural links do not evaluate candidate
values or proof outcomes. Unrelated `docs/programs/hey/` is preserved.

## Coercion-owned primary consumers

Observed coercion Projection ports now map step0 to the original result's slot0.
The scalar primary-input collector revalidates each observed Projection, Operation
and Normal stage through `coercion_effect_stage`, checking owner, input, kind,
primary/control metadata and exact source edges. Only an observed projection yields
an input; later stages cannot imply one. Existing slot resolution preserves original
ports, field order, owner boundaries, unknown candidates and shared map/work limits.

Forward and Convert consumers now cover local unary and typed/nullable scalar
contexts through checked initializer chains. A Convert projection alone requires
no operation registry entry; observed Operation/Normal stages require the exact
registered owner. Seeded Stopped projection-only evidence retains a link without
operation/result observations or a runtime completion claim. Primary-extracting
coercions remain opaque to value forwarding. No new map, collection order or
variable payload is introduced; late invalid stages publish no partial links.

| Reviewable slice | Commit |
| --- | --- |
| Link observed coercion primary ports to result slots | `5001917` |
| Cover stopped/registration/stage and shared-resource boundaries | `0399df9` |
| Pin scalar copy order, stopped RHS and width rejection | `9748410` |

Seven new groups cover source/owner identity, three-stage selection, opaque sources,
seeded Stopped, Convert registration, 18 late corruptions, duplicate visits and exact
map/work limits. Focused checks and library Clippy pass
(`/tmp/meowy-coercion-consumers.log`, `/tmp/meowy-coercion-effects.log`,
`/tmp/meowy-coercion-limits.log`, `/tmp/meowy-coercion-lint.log`). Three required
source cases pass rebuilt-compiler debug/release (`/tmp/meowy-coercion-source-final.log`).
Metadata checks pass at 306 cases, 287 required and 19 unchanged pins
(`/tmp/meowy-coercion-metadata.log`). All ten compiler checks pass on LLVM 23.1.1/Rust
1.99.0: 2409 library/915 native tests, 32 tooling/30 harness groups, formatting, Clippy,
build and conformance (`/tmp/meowy-coercion-gate.log`). All 287 required cases pass
debug/release, with 19 unchanged pins and zero failures. Strict mode exits 1 only for
those gaps (`/tmp/meowy-coercion-strict.log`). All four final documentation checks pass
(`/tmp/meowy-coercion-docs.log`).

All 303 prior case records, 335 source assets, 37 reference files/reviewed hashes,
19 capability pins and 33 proof obligations are unchanged
(`/tmp/meowy-coercion-preservation.log`). Structural projection-only evidence remains
distinct from observable source execution. Unrelated `docs/programs/hey/` is preserved.

## Output-owned primary consumers

Formatting primary projections now retain their original sparse part indices in
slot-use links. Literal/scalar gaps are preserved, more than two parts can link,
and each observed projection resolves through the existing qualified source path
to slot zero. Parameters, mutable/reference storage, calls and unknown sources stay
opaque. No projection is inferred from an output or terminal observation.

One whole-producer replay per output compares exact stored prefix/input/projection/
output/terminal edges, preserving Next/Returned routes and the stopped boundary.
Every stored child is checked, including suffix descriptors after a stopped part;
no suffix edges or result visits are invented. Capture remains the authority for
unique source roots and formatting types. Replay checks descriptor/edge agreement
without PointId ordering or another seen set/input vector.

Sparse report qualification checks owner/panic/control/total/stopped headers, exact
part inputs, nonempty independent observation flags and valid selectors. Only an
observed terminal requires its registered operation owner. The collector streams
observed projections into the existing Uses scratch map through shared insertion,
with bounded Flow/map capacity and unchanged payload. Late failures publish no
partial links. Seeded stopped-record anchors remain structural evidence, separate
from runtime completion or I/O success.

| Reviewable slice | Commit |
| --- | --- |
| Extract shared bounded slot insertion | `9aaca5d` |
| Replay exact whole-output edge sequences | `0c5c184` |
| Qualify sparse observed output reports | `1bf878c` |
| Stream original primary-part ports to result slots | `7ac15f3` |
| Cover observed stages, corruption and shared limits | `4509d0e` |
| Pin print/panic, stopped prefixes and checked tails | `8e1269d` |

Eleven new groups cover complete edges, sparse reports, source/owner links, stopped
stages, 21 edge faults, 14 report faults, 18 late consumer faults and exact resources.
Focused checks and library Clippy pass (`/tmp/meowy-output-edges.log`,
`/tmp/meowy-output-reports.log`, `/tmp/meowy-output-consumers.log`,
`/tmp/meowy-output-limits.log`, `/tmp/meowy-output-lint.log`). All ten compiler
checks pass on LLVM 23.1.1/Rust 1.99.0: 2420 library/915 native tests, 32 tooling/30
harness groups, formatting, Clippy, build and conformance (`/tmp/meowy-output-gate.log`).
Four required cases pass rebuilt-compiler debug/release
(`/tmp/meowy-output-source-final.log`). The catalog has 310 cases: 291 required
passes, 19 unchanged pins and zero failures in debug/release. Strict mode exits 1
only for those gaps (`/tmp/meowy-output-strict.log`). All four final documentation
checks pass (`/tmp/meowy-output-docs.log`).

All 306 prior case records, 338 source assets, 37 reference files/reviewed hashes,
19 capability pins and 33 proof obligations are unchanged
(`/tmp/meowy-output-preservation.log`). Unrelated `docs/programs/hey/` is preserved.

## Contextual-list primary consumers

Whole-list validation is now separate from indexed stage selection. Replay preserves
exact sequence/endpoints, unique source roots, full child/suffix identities, capacity,
context and operation registration. The existing effect collector retains its
per-stage wrapper; consumer report qualification replays once per list and then
uses constant-work selectors, avoiding another whole scan for every observed flag.

Full report qualification compares capacity/context/normal/control and every input
root/plan, including unobserved suffixes. Projection/conversion/construction/result
flags remain independent. Construction requires normal metadata and no stopped input;
a projected Stopped input may retain normal metadata. Producers with construction
edges require their registered owner even for projection-only reports. No weaker
output/coercion registration rule is substituted.

The consumer streams only observed projected inputs into the existing Uses map,
retaining original Projection indices and source slot zero. An immutable record
fitting multiple contexts can retain list-owned projection until a later selector
chooses the list context. Real-source tests prove sparse indices 0/2/3 through
same-owner local copies. Ordinary typed lists retain coercion-owned ports;
parameters, changed narrowing and unknown sources remain opaque. Seeded stopped
anchors remain structural evidence, not runtime completion. Work/map failures
publish no partial links, and candidate/input payload is neither copied nor charged again.

| Reviewable slice | Commit |
| --- | --- |
| Separate whole-producer validation from stage selection | `6694ada` |
| Qualify complete observed list reports | `d5e966b` |
| Link contextual-list primary inputs to result slots | `c214833` |
| Cover stopped plans, corruption and shared resources | `5d908ad` |
| Pin late-selector source order, panic and width rejection | `0c48b3e` |

Ten new groups cover indexed selection, independent report stages, a genuine source
join, 12 report faults, sparse plans, stopped suffixes, 13 late faults and exact
resources. Focused checks and library Clippy pass (`/tmp/meowy-list-selection.log`,
`/tmp/meowy-list-reports.log`, `/tmp/meowy-list-consumers.log`,
`/tmp/meowy-list-limits.log`, `/tmp/meowy-list-consumer-lint.log`). Three required
source cases pass rebuilt-compiler debug/release (`/tmp/meowy-list-source-final.log`).
Metadata checks pass at 313 cases, 294 required and 19 unchanged pins
(`/tmp/meowy-list-metadata.log`). All ten compiler checks pass on LLVM 23.1.1/Rust
1.99.0: 2430 library/915 native tests, 32 tooling/30 harness groups, formatting, Clippy,
build and conformance (`/tmp/meowy-list-gate.log`). All 294 required cases pass
debug/release, with 19 unchanged pins and zero failures. Strict mode exits 1 only
for those gaps (`/tmp/meowy-list-strict.log`). All four final documentation checks
pass (`/tmp/meowy-list-docs.log`).

All 310 prior case records, 342 source assets, 37 reference files/reviewed hashes,
19 capability pins and 33 proof obligations are unchanged
(`/tmp/meowy-list-preservation.log`). Unrelated `docs/programs/hey/` is preserved.

## Composed-emission source-slot consumers

Composed emissions now link initialized `Emission(EmitId)` ports to the original
source block: primary slot zero and named field slot `index + 1`. Whole-report
qualification is shared with result-source indexing and preserves checked statement,
input, composition, target, alias, registry and edge identities. Each composition
resolves its source once, then validates the complete source count and ordered names,
including unobserved target suffixes. Streaming shares existing map/work limits and
adds no candidate copies or report payload.

Initialization flags remain independent of statement results and destination
BlockResult. Partial expected records, discarded destination fields and later stopped
tails retain source identity. Direct Value emissions are not primary extraction;
empty records still have slot zero. Unknown sources and mutable/aggregate candidate
values preserve their boundaries. Empty/multiple source histories are unchanged.
These links do not infer values, runtime reachability, lifetime authority or proof results.

| Reviewable slice | Commit |
| --- | --- |
| Share complete emission report qualification | `2a2de97` |
| Link initialized composition targets to source slots | `bcf8eb0` |
| Cover partial observations, corruption and resources | `2ec24cf` |
| Pin source order, stopped tails and duplicate-field rejection | `b13d8fc` |

Thirteen new library groups cover qualification, source mapping, independent owners,
empty records, partial flags, opaque/Unknown sources, late identity faults and exact
resources. Depth/width work comparison checks that source resolution runs once per
composition. Focused tests and Clippy pass (`/tmp/meowy-emission-consumers.log`,
`/tmp/meowy-emission-candidates.log`, `/tmp/meowy-emission-limits.log`,
`/tmp/meowy-emission-test-lint.log`). Three required source cases pass fresh-compiler
debug/release (`/tmp/meowy-emission-source-final.log`).

All ten compiler checks pass on LLVM 23.1.1/Rust 1.99.0: 2443 library/915 native tests,
32 tooling/30 harness groups, formatting, Clippy, build, metadata and conformance
(`/tmp/meowy-emission-gate.log`). All 297 required cases pass debug/release, with
19 unchanged pinned gaps and zero failures. Strict mode exits 1 only for those gaps
(`/tmp/meowy-emission-strict.log`). All four final documentation checks pass
(`/tmp/meowy-emission-docs.log`).

All 313 prior case records, 345 source assets, 37 reference contracts/reviewed hashes,
19 capability pins and 33 proof obligations are unchanged
(`/tmp/meowy-emission-preservation.log`). Unrelated `docs/programs/hey/` is preserved.
No compiler test failures remain. Proof evaluation and full release qualification
remain incomplete.

## Unchanged explicit-ascription consumers

Checked ascriptions now retain the actual `refinement.rs::coercion` changed decision
without copying types or changing HIR/edge behavior. Typed reports qualify exact
input, owner, kind, changed, normal and control identities. Only unchanged normal
Ascription results with an observed result forward their input; operation/result
visits remain independent. Predicates, widening/changed forms, stopped inputs and
missing results stay opaque.

Mixed resolution shares the existing conflict/cycle/hop/work bounds. Direct,
grouped and immutable-copy field, primary and composed-emission consumers retain
original slots and independent owners. Unknown values, multiple candidate histories,
reference loads, calls and field-result values retain their boundaries. Links do
not infer runtime reachability, selected values, loan authority or proof outcomes.

| Reviewable slice | Commit |
| --- | --- |
| Retain actual checked ascription decisions | `beaade7` |
| Carry and qualify typed result reports | `af3641a` |
| Resolve mixed consumers through unchanged ascriptions | `406d35f` |
| Cover conflicts, cycles, late failures and exact resources | `09a5498` |
| Pin source behavior and classified coverage | `d4d105b` |

Thirteen new library groups cover capture, report qualification and consumer
boundaries. All six capture, 12 typed-report and eight ascription-consumer groups
pass; the wider matching consumer run passed 82 groups before the five additional
boundary groups. Logs: `/tmp/meowy-ascription-capture.log`,
`/tmp/meowy-ascription-reports.log`, `/tmp/meowy-ascription-consumers.log`,
`/tmp/meowy-ascription-limits.log`. All-target Clippy passes
(`/tmp/meowy-ascription-lint.log`). Three required source cases pass fresh-compiler
debug/release (`/tmp/meowy-ascription-source.log`). All four source-slice documentation
checks pass (`/tmp/meowy-ascription-source-docs.log`).

All ten compiler checks pass: 2456 library/915 native tests, 32 tooling/30 harness
groups, formatting, Clippy, build, metadata and conformance
(`/tmp/meowy-ascription-gate.log`). Conformance has 319 cases: 300 required passes,
19 unchanged pinned gaps and zero failures in debug/release. Strict mode exits 1
only for those gaps (`/tmp/meowy-ascription-strict.log`). All four final documentation
checks pass (`/tmp/meowy-ascription-docs.log`). No test failures remain. The
preservation audit confirms all 316 prior case records, 385 reference/source assets,
37 reviewed hashes, capability pins and proof obligations are unchanged
(`/tmp/meowy-ascription-preservation.log`). Unrelated `docs/programs/hey/` is preserved.
Proof evaluation and full release qualification remain incomplete.

## Independent field result observations

Field reports now retain independent Operation and Normal observations alongside
checked normal availability. Both stages reuse bounded checked-header/edge
validation, including exact receiver, field index, load choice, owner, control,
point ancestry and operation registration. Never fields retain operations without
results; shared-load projection-only visits remain separate. Duplicate visits merge
flags; conflicts and shared work/map exhaustion publish no partial effect map.
No variable payload or type shapes are copied.

Field consumers qualify the complete header and flags but create a source-slot link
only for an observed Operation. A result-only row creates no operation link. Fields
remain terminal in mixed source resolution: observing a field result does not
forward its receiver as the extracted value. Existing Unknown/candidate histories,
borrow/lifetime rules, required/static reads and proof gates remain unchanged.

| Reviewable slice | Commit |
| --- | --- |
| Separate checked field validation | `b395515` |
| Retain independent operation/result flags and qualify consumers | `582a7f2` |
| Cover sparse observations, identities and exact limits | `da2b050` |
| Pin source behavior and classified evidence | `b9031c2` |

All 114 matching library tests pass before/after the validation extraction
(`/tmp/meowy-field-observations-before.log`, `/tmp/meowy-field-observations-validation.log`).
All 120 matching tests pass with six new groups and expanded corruption/consumer
coverage (`/tmp/meowy-field-observations-limits.log`). All-target Clippy passes
(`/tmp/meowy-field-observations-limits-lint.log`). Three required source cases pass
fresh-compiler debug/release (`/tmp/meowy-field-observations-source.log`). All four
source-slice documentation checks pass (`/tmp/meowy-field-observations-source-docs.log`).

All ten compiler checks pass: 2462 library/915 native tests, 32 tooling/30 harness
groups, formatting, Clippy, build, metadata and conformance
(`/tmp/meowy-field-observations-gate.log`). All 303 required cases pass debug/release,
with 19 unchanged pinned gaps and zero failures. Strict mode exits 1 only for those
gaps (`/tmp/meowy-field-observations-strict.log`). All four final documentation
checks pass (`/tmp/meowy-field-observations-docs.log`). No test failures remain.
The preservation audit confirms all 319 prior case records, 351 source assets,
37 references/reviewed hashes, capability pins and proof obligations are unchanged
(`/tmp/meowy-field-observations-preservation.log`).
Unrelated `docs/programs/hey/` is preserved. Proof evaluation and full release
qualification remain incomplete.

## Bounded result-candidate input qualification

Candidate-input reports now retain exact emission input PointIds and checked
Value/Primary/Field projections by original block/slot/candidate position. Each
fixed-size descriptor keeps the EmitId, statement, target and owner. Qualification
revalidates result headers, immutable scalar-slot identity, emission metadata and
observed target initialization without choosing a candidate or inferring its type
from the completed slot. Discarded-path source shapes remain unchanged.

The existing result map keeps empty and Unknown histories; they produce no input
descriptors. Multiple candidates retain separate positions. Statement-result and
block-normal flags remain independent. Per-pass block/emission validation caches
charge shared payload and avoid repeated wide-producer scans. Retained descriptors
reserve remaining combined map capacity, and shared work bounds all lookups. Late
failures publish no partial map or remaining-budget update. Source traversal,
field-value forwarding and proof outcomes remain separate; projected source-slot
qualification is recorded below.

| Reviewable slice | Commit |
| --- | --- |
| Share checked result-header qualification | `98140f0` |
| Qualify exact candidate-input identities | `4475569` |
| Collect bounded descriptors in entry reports | `71b1d6a` |
| Cover discarded/partial sources and exact resource limits | `2f23847` |
| Pin source behavior and classified evidence | `3afaa03` |

All 67 matching result tests pass before/after header extraction
(`/tmp/meowy-candidate-inputs-before.log`, `/tmp/meowy-candidate-inputs-validation.log`).
All 2467 library tests pass after collection integration
(`/tmp/meowy-candidate-inputs-library.log`). All 11 candidate-input groups and
all-target Clippy pass after boundary coverage (`/tmp/meowy-candidate-inputs-boundaries.log`,
`/tmp/meowy-candidate-inputs-boundaries-lint.log`). Three required source cases pass
fresh-compiler debug/release (`/tmp/meowy-candidate-inputs-source.log`), and all four
source-slice documentation checks pass (`/tmp/meowy-candidate-inputs-source-docs.log`).

All ten compiler checks pass: 2473 library/915 native tests, 32 tooling/30 harness
groups, formatting, Clippy, build, metadata and conformance
(`/tmp/meowy-candidate-inputs-gate.log`). Conformance has 325 cases: 306 required
passes, 19 unchanged pinned gaps and zero failures in debug/release. Strict mode
exits 1 only for those gaps (`/tmp/meowy-candidate-inputs-strict.log`). All four final
documentation checks pass (`/tmp/meowy-candidate-inputs-docs.log`). No test failures
remain. The preservation audit confirms all 322 prior cases, 354 source assets,
37 reference contracts/reviewed hashes,
capability pins and proof obligations are unchanged
(`/tmp/meowy-candidate-inputs-preservation.log`). Unrelated `docs/programs/hey/` is
preserved. Full proof evaluation and release qualification remain incomplete.

## Composed candidate source-slot qualification

Candidate descriptors now retain an optional qualified source Slot for Primary/Field
projections. Existing Emission-link owners and indices must match the checked
candidate and producer; direct Value links are rejected and missing links stay
unknown. Shared `emission_source_block` qualification resolves the original source
and validates its complete count/name layout, including unobserved suffixes.

A per-pass cache resolves each composed source once and charges remaining payload.
The optional Slot adds no map entries or candidate copies. Exact combined map,
scratch and work boundaries remain enforced; late failures publish no partial map.
Original positions, independent target/result flags, discarded paths and source
Unknown/empty/multiple histories are preserved. These associations do not select
values, establish runtime reachability, forward fields or enable proof outcomes.

| Reviewable slice | Commit |
| --- | --- |
| Share composed-emission source qualification | `c489bd1` |
| Retain qualified candidate source slots | `74671ed` |
| Cover sparse/corrupt sources and exact cache/work limits | `590cb3e` |
| Pin source behavior and classified coverage | `e04ad8e` |

All ten emission-consumer groups pass before/after extraction
(`/tmp/meowy-candidate-slots-before.log`, `/tmp/meowy-candidate-slots-validation.log`).
All 2476 library tests pass after implementation (`/tmp/meowy-candidate-slots-library.log`).
Ten new source-slot groups plus the existing candidate regressions pass in the
27-group focused run (`/tmp/meowy-candidate-slots-boundaries.log`); all-target Clippy
passes (`/tmp/meowy-candidate-slots-boundaries-lint.log`). Three required source cases
pass fresh-compiler debug/release (`/tmp/meowy-candidate-slots-source.log`). All four
source-slice documentation checks pass (`/tmp/meowy-candidate-slots-source-docs.log`).

All ten compiler checks pass: 2483 library/915 native tests, 32 tooling/30 harness
groups, formatting, Clippy, build, metadata and conformance
(`/tmp/meowy-candidate-slots-gate.log`). Conformance has 328 cases: 309 required
passes, 19 unchanged pinned gaps and zero failures in debug/release. Strict mode
exits 1 only for those gaps (`/tmp/meowy-candidate-slots-strict.log`). All four final
documentation checks pass (`/tmp/meowy-candidate-slots-docs.log`). No test failures
remain. The preservation audit confirms all 325 prior
cases, 357 source assets, 37 reference contracts/reviewed hashes, capability pins
and proof obligations are unchanged (`/tmp/meowy-candidate-slots-preservation.log`).
Unrelated `docs/programs/hey/` is preserved. Proof evaluation and full release
qualification remain incomplete.

## Bounded candidate-source traversal

Stored candidate inputs now form a borrowed graph view after complete position,
owner and descriptor qualification. Missing, extra or changed rows fail; empty
candidate lists still require immutable scalar slots. No type shapes, runtime
values or candidate vectors are copied or selected.

The iterative engine follows qualified source Slots and retains each original
candidate position. Direct points, unresolved projections, Unknown/empty histories,
completed shared sources and active cycles have distinct visits. Entry reports
retain one forest over known owner/Slot roots with shared visited state, avoiding
repeated expansion of source chains. Layouts without known slots gain no roots.
Aggregate output/visited/pending storage is bounded before growth, with shared work
charges and remaining-payload accounting. Failures publish no partial walk.

| Reviewable slice | Commit |
| --- | --- |
| Qualify stored candidate graph descriptors | `322d4ed` |
| Add iterative traversal and the entry-report forest | `b3a6516` |
| Cover sharing/cycles, deep chains and exact limits | `60fd5e0` |
| Pin source behavior and classified evidence | `01baf4a` |

All four graph-qualification groups pass (`/tmp/meowy-candidate-walk-qualification.log`).
All 2489 library tests pass after integration (`/tmp/meowy-candidate-walk-library.log`).
All nine walk groups and all-target Clippy pass after boundary coverage
(`/tmp/meowy-candidate-walk-boundaries.log`, `/tmp/meowy-candidate-walk-boundaries-lint.log`).
Cycles, malformed engine graphs and the 2048-slot chain are seeded structural
fixtures; checked-source sharing/discarded-input tests are separate. Three required
source cases pass fresh-compiler debug/release (`/tmp/meowy-candidate-walk-source.log`).
All four source-slice documentation checks pass (`/tmp/meowy-candidate-walk-source-docs.log`).

All ten compiler checks pass: 2496 library/915 native tests, 32 tooling/30 harness
groups, formatting, Clippy, build, metadata and conformance
(`/tmp/meowy-candidate-walk-gate.log`). Conformance has 331 cases: 312 required
passes, 19 unchanged pinned gaps and zero failures in debug/release. Strict mode
exits 1 only for those gaps (`/tmp/meowy-candidate-walk-strict.log`). All four final
documentation checks pass (`/tmp/meowy-candidate-walk-docs.log`). No test failures
remain. The preservation audit confirms 328 prior cases,
360 source assets, 37 reference contracts/reviewed hashes, capability pins and
proof obligations are unchanged (`/tmp/meowy-candidate-walk-preservation.log`).
The editor follow-up is separately committed as `d124403`; `docs/programs/hey/`
remains excluded. Proof evaluation and full release qualification remain incomplete.

## Observed field-result source links

Observed owned field results now retain Normal(PointId)-to-source-Slot associations.
Qualification requires checked normal availability, independent operation/result
visits and a matching requalified Operation link. Missing links, sparse visits,
Never fields, shared loads and unresolved receivers remain opaque. Nested aggregate
reads can retain an Unknown slot without forwarding the extracted field value.

Collection indexes existing candidate-forest Root markers once, only when the first
eligible link needs them. Owner, slot bounds and duplicate roots are checked; no
walk is rerun per field. Root scratch consumes remaining payload and associations
reserve remaining combined map capacity. Shared work bounds validation/lookups;
late failures publish no partial map or remaining-budget update. Unknown, empty and
multiple histories remain intact, with no candidate selection or lifetime authority.

| Reviewable slice | Commit |
| --- | --- |
| Qualify observed field-result source slots | `aeefeed` |
| Collect bounded links to candidate forest roots | `03e46c3` |
| Cover root/header/link identities and exact limits | `1ff75ba` |
| Pin source behavior and classified evidence | `9373325` |

All five qualification groups pass (`/tmp/meowy-field-result-links-qualification.log`).
All 2504 library tests pass after collection (`/tmp/meowy-field-result-links-library.log`).
All 14 matching field-result groups and all-target Clippy pass after boundary
coverage (`/tmp/meowy-field-result-links-boundaries.log`,
`/tmp/meowy-field-result-links-boundaries-lint.log`). Three required source cases pass
fresh-compiler debug/release (`/tmp/meowy-field-result-links-source.log`). All four
source-slice documentation checks pass (`/tmp/meowy-field-result-links-source-docs.log`).

All ten compiler checks pass: 2508 library/915 native tests, 32 tooling/30 harness
groups, formatting, Clippy, build, metadata and conformance
(`/tmp/meowy-field-result-links-gate.log`). Conformance has 334 cases: 315 required
passes, 19 unchanged pinned gaps and zero failures in debug/release. Strict mode
exits 1 only for those gaps (`/tmp/meowy-field-result-links-strict.log`). All four final
documentation checks pass (`/tmp/meowy-field-result-links-docs.log`). No test failures
remain. The preservation audit confirms all 331 prior
cases, 363 source assets, 37 reference contracts/reviewed hashes, capability
pins and proof obligations are unchanged (`/tmp/meowy-field-result-links-preservation.log`).
Only unrelated `docs/programs/hey/` remains excluded. Proof evaluation and full
release qualification remain incomplete.

## Field-result lookup through unchanged narrowing

Stored Normal-port field associations now have bounded read qualification against
the checked producer, original Operation link and forest root. A borrowed per-pass
context caches validated fields and roots with shared payload/work charges. Missing
associations remain opaque and corrupt retained identities fail atomically.

Direct-input lookup follows observed, normal, unchanged narrowing, with explicit
group forwarding described below. It preserves point/body/owner identities and
rejects producer conflicts and cycles. The 65,536-hop scratch cap is separate from
cache payload; all lookups share work limits.
Changed/unobserved narrowing, coercions, ascriptions, reads, calls and reference
loads remain explicit boundaries. The original record-consumer resolver is unchanged.

A separate position-keyed direct-source map retains each Value candidate's original
point and optional terminal field/source Slot. Collection follows field-result
reporting, reserves remaining combined map capacity and preserves existing candidate
inputs and forest visits. Unknown/empty/multiple histories stay intact. These links
feed the separate expanded forest below and infer no selected value, runtime
reachability, lifetime authority or proof outcome.

| Reviewable slice | Commit |
| --- | --- |
| Qualify cached stored field-result lookups | `1cd353b` |
| Resolve sources through unchanged narrowing | `444a66a` |
| Collect optional direct-candidate sources | `324fc89` |
| Cover conflicts, cycles, late faults and exact limits | `f4e12ff` |
| Pin source behavior and classified coverage | `5f602e9` |

All three lookup groups and both initial resolver groups pass
(`/tmp/meowy-field-narrowing-lookup.log`, `/tmp/meowy-field-narrowing-resolver.log`).
All 2515 library tests pass after integration (`/tmp/meowy-field-narrowing-library.log`).
All five narrowing groups, five direct-source groups and all-target Clippy pass after
boundary coverage (`/tmp/meowy-field-narrowing-boundaries.log`,
`/tmp/meowy-field-narrowing-direct-limits.log`, `/tmp/meowy-field-narrowing-boundaries-lint.log`).
The cycle is seeded structural evidence. Three required source cases pass fresh-compiler
debug/release (`/tmp/meowy-field-narrowing-source.log`); all four source-slice
documentation checks pass (`/tmp/meowy-field-narrowing-source-docs.log`).

All ten compiler checks pass: 2521 library/915 native tests, 32 tooling/30 harness
groups, formatting, Clippy, build, metadata and conformance
(`/tmp/meowy-field-narrowing-gate.log`). Conformance has 337 cases: 318 required
passes, 19 unchanged pinned gaps and zero failures in debug/release. Strict mode
exits 1 only for those gaps (`/tmp/meowy-field-narrowing-strict.log`). All four final
documentation checks pass (`/tmp/meowy-field-narrowing-docs.log`). No test failures
remain. The preservation audit confirms 334 prior cases,
366 source assets, 37 reference contracts/reviewed hashes, capability pins and
proof obligations are unchanged (`/tmp/meowy-field-narrowing-preservation.log`).
Unrelated `docs/programs/hey/` remains excluded. Proof evaluation and full release
qualification remain incomplete.

## Traversal through qualified direct field sources

Entry reports now retain a separate expanded forest after field-result and
direct-source collection. Its borrowed graph view requalifies every Value descriptor
against the original candidate, point, owner and narrowing/field-result evidence,
rejecting missing, extra or changed rows with shared per-pass caches.

Explicit Field visits retain original candidate positions and input descriptors,
terminal fields and source slots. The original Value projections and qualification
forest remain unchanged. Absent optional sources stay terminal. Composed and field
edges share owner checks, active-cycle/completed-sharing state and bounded
pending/visited/output storage and work. Unknown, empty, multiple and discarded
histories remain distinct; no selected value, execution or proof outcome is inferred.

| Reviewable slice | Commit |
| --- | --- |
| Qualify retained direct-source graph descriptors | `2f043ea` |
| Traverse explicit direct field edges | `7972426` |
| Collect a separate expanded forest | `4104d99` |
| Cover mixed cycles and exact resource limits | `cfe9512` |
| Pin source behavior and classified coverage | `cc43868` |

Three qualifier groups, twelve original/expanded traversal groups and four seeded
boundary groups pass (`/tmp/meowy-expanded-qualifier.log`,
`/tmp/meowy-expanded-engine.log`, `/tmp/meowy-expanded-boundaries.log`). All 2529
library tests and all-target Clippy pass after integration, followed by both final
focused integration groups (`/tmp/meowy-expanded-library.log`,
`/tmp/meowy-expanded-integration-lint.log`, `/tmp/meowy-expanded-integration.log`).
The cycles and 2048-slot chains are seeded structural evidence. Three required
source cases pass fresh-compiler debug/release (`/tmp/meowy-expanded-source.log`);
all four source-slice documentation checks pass (`/tmp/meowy-expanded-source-docs.log`).

All ten compiler checks pass: 2533 library/915 native tests, 32 tooling/30 harness
groups, formatting, Clippy, build, metadata and conformance
(`/tmp/meowy-expanded-gate.log`). Conformance has 340 cases: 321 required passes,
19 unchanged pinned gaps and zero failures in debug/release. Strict mode exits 1
only for those gaps (`/tmp/meowy-expanded-strict.log`). All four final documentation
checks pass (`/tmp/meowy-expanded-docs.log`). No test failures remain.
The preservation audit confirms 337 prior cases,
369 source assets, 37 reference contracts/reviewed hashes, capability pins and proof
obligations are unchanged (`/tmp/meowy-expanded-preservation.log`). Unrelated
`docs/programs/hey/` remains excluded. Proof evaluation and full language/release
qualification remain incomplete.

## Direct field sources through explicit groups

Direct field-source resolution now traverses explicitly registered groups mixed
with observed unchanged narrowing. A shared helper preserves the record-consumer
child/span/body/owner and exact region-edge checks and work charge. Both callers
retain producer, point, body and cycle validation; record consumers still stop at
Field results. Unmarked regions and stored Normal edges cannot invent a source
association or completion.

Collection, retained-descriptor qualification and the expanded forest preserve
original candidate positions and points, terminal fields and source slots. The
original candidate forest remains unchanged. Groups and narrowing share bounded
hop/cycle state and work; field/root caches share payload across repeated lookups.
Coercions, ascriptions, initializer reads, calls, reference loads and changed or
unobserved narrowing remain opaque. No value, reachability or proof outcome is
selected or inferred.

| Reviewable slice | Commit |
| --- | --- |
| Share explicit group child qualification | `39032ab` |
| Resolve grouped direct field sources | `d75b744` |
| Reject corrupt and cyclic grouped routes | `fa55889` |
| Bound grouped lookup and report resources | `1790a65` |
| Pin source behavior and classified coverage | `b2005f1` |

All 34 grouped-consumer tests pass before and after extraction
(`/tmp/meowy-group-fields-before.log`, `/tmp/meowy-group-fields-extract.log`). The
three initial resolver groups pass (`/tmp/meowy-group-fields-resolver.log`), and
all 2536 library tests and all-target Clippy pass after integration
(`/tmp/meowy-group-fields-library.log`, `/tmp/meowy-group-fields-lint.log`).
Both corruption/cycle groups, all seven grouped-source groups and all ten direct-report
groups pass (`/tmp/meowy-group-fields-faults.log`, `/tmp/meowy-group-fields-limits.log`,
`/tmp/meowy-group-fields-reports.log`). Seeded cycle legs qualify individually before
the resolver rejects the revisit; collection may reject altered candidate ancestry
earlier. Those cycles and registry limits are structural evidence only.
Three required source cases pass fresh-compiler debug/release
(`/tmp/meowy-group-fields-source.log`); all four source-slice documentation checks
pass (`/tmp/meowy-group-fields-source-docs.log`).

All ten compiler checks pass: 2540 library/915 native tests, 32 tooling/30 harness
groups, formatting, Clippy, build, metadata and conformance
(`/tmp/meowy-group-fields-gate.log`). Conformance has 343 cases: 324 required passes,
19 unchanged pinned gaps and zero failures in debug/release. Strict mode exits 1
only for those gaps (`/tmp/meowy-group-fields-strict.log`). All four final
documentation checks pass (`/tmp/meowy-group-fields-docs.log`). No test failures
remain. The preservation audit confirms 340 prior cases, 372 source assets,
37 reference contracts/reviewed hashes, capability pins
and proof obligations are unchanged (`/tmp/meowy-group-fields-preservation.log`).
Unrelated `docs/programs/hey/` remains excluded. Proof evaluation and full
language/release qualification remain incomplete.

### Next: direct field sources through observed Forward coercions

1. In `consumers/field_results/lookup/narrowing.rs`, reuse
   `forward_coercion_input` from `effects/coercions.rs` to follow observed,
   non-projecting Forward coercions mixed with groups and unchanged narrowing.
   Keep collection and stored-descriptor qualification on the same resolver;
   preserve original Value points, terminal fields and the original forest.
   Include focused typed/grouped candidate and expanded-forest regressions.
2. In a separate boundary slice, cover absent result observations, Convert,
   primary-extracting and Stopped coercions, corrupt headers/routes, producer
   conflicts, mixed cycles, owners and exact shared work/hop/cache limits.
   Keep ascriptions, initializer reads, calls and reference loads opaque.
3. Add required typed field-source cases and classified evidence while preserving
   reference contracts, earlier fixtures and pinned capability gaps.
4. Run compiler, strict and final documentation gates and update this handoff.

Do not make Field results transparent in `consumers/grouped.rs` or select candidate
values. Aggregate provenance, precise branch/overwrite joins, function returns,
restart propagation, E225 enforcement and proof outcomes remain later work.
Shared-reference loads gain no loan authority.

## Documentation conventions and layout

[README.md](README.md), `AGENTS.md` and this handoff stay at the compiler root;
the 16 detailed guides live in `docs/`. Cargo and links use this layout.
[Documentation conventions](../docs/guide/documentation-style.md) require lowercase
`meowy` and readable example spacing. Tests, tooling and generated source keep their
own layouts. Intentional compact demonstrations and reference fixtures are preserved.

All 82 reformatted standalone examples retain executable tokens, literal contents,
ordinary comments and statement newlines. The nested documentation example also
retains its code tokens/attributes/output and passed `doc check --run-examples`.
Generated API-page branding is lowercase and covered by the renderer regression.
Git preserves that documentation series; the root STATUS links its preservation audit.

## Dispatch receiver sigil migration

`$` is the sole implicit dispatch receiver. Nested dispatches introduce a new `$`;
ordinary nested blocks retain the enclosing receiver. It cannot be declared or
rebound. `self` is an ordinary name, including inside dispatches; legacy implicit
uses fail E201. Receiver ownership, mutability and lifetime rules are unchanged.

Implementation: `39d9bd1`, `213cfe4`; fixture batches: `eeb3b73`, `7af7755`,
`4deba7b`; contract/guides: `280a9e5`, `f9c6fae`. Vim/Neovim now highlight receiver
fields, borrows, type queries and interpolation while preserving literal dollars.
All 12 compiler/editor checks pass, including 1432 library/906 native tests;
log: `/tmp/meowy-dollar-receiver-gate.log`. Native receiver execution covers debug
and release. Default documentation checks pass (`/tmp/meowy-dollar-docs.log`).
The full composition project still hits existing manifest/module-composition gates;
no full-language example execution or release qualification is claimed.
The restart/proof implementation handoff below remains the next compiler task.

## Explicit ascription and bits-module migration

The migration is complete. `value~<T>` consumes one bracketed ascription target
and retains proof-before-use (E208) and ownership checks. Named union aliases and
existing computed targets work. `value<T>` is always a type predicate at comparison
precedence; `value<>` and generic-call parsing retain their syntax. The expression
parser no longer carries matcher/ascription or bitwise-pipe mode flags. Generic
binders and value arguments retain their existing bootstrap capability gates.

`@"bits"` provides lexically resolved `and/or/xor/not`, including aliases and
dispatch, through the existing integer HIR and required evaluator. Exact widths,
evaluation order, budgets, E225 separation and contextual list inference are
preserved. Binary integer `&`, `|`, `^` and prefix `~` are no longer source operators.
Borrows, matchers, boolean operators, capabilities and remainder keep their meanings.
Other documented bits APIs remain gated. Documentation, examples and Vim/Neovim
now teach and highlight the explicit syntax.

The dependency-ordered plan separated syntax admission, bits runtime/required
integration, bounded fixture migration, uniform predicates/operator retirement,
and parser cleanup. Reviewed commits:

| Slice | Commit |
| --- | --- |
| Vim explicit ascription highlighting | `f58c1ce` |
| Lexically resolved bit functions | `e669908` |
| Ascription and predicate contract | `c7045e7` |
| Bits-module documentation | `92d9f61` |
| Required integer evidence for bit functions | `5b224dd` |
| Explicit proven ascription parser and regressions | `020568c` |
| Ascription teaching guides | `e889dad` |
| Contextual list inference for bit functions | `540cff9` |
| Documented ascription examples | `70e02e6` |
| Checker ascription fixtures | `129cdd6` |
| Ownership ascription fixtures | `b93f443` |
| Loan ascription fixtures | `6209ee5` |
| Native scalar/type ascription fixtures | `e91293e` |
| Native loan/slot ascription fixtures | `128fbd7` |
| Native restart/header ascriptions | `dad0bf9` |
| Native reference-return ascriptions | `5451e23` |
| Integer checker bit-function fixtures | `c1a12b4` |
| Native integer bit-function fixtures | `5ce0eb3` |
| Context-independent type predicates | `5d07043` |
| Retired bitwise operator syntax | `a85adf1` |

The final cleanup commit removes obsolete parser mode parameters and records this
handoff. All 12 checks in `python3 -B tools/verify.py --compiler --editor both`
passed: 1447 library/910 native tests, Vim/Neovim, lint, formatting, build, tooling,
links and schemas. Conformance: 10 passed, 13 unsupported, 0 failed in debug/release.
Log: `/tmp/meowy-explicit-types-bits-gate.log`. Parser cleanup also received an
independent read-only review. No outstanding failures remain. The unrelated user
asset deletion is preserved. Restart/proof implementation below remains next;
this migration does not enable proof outcomes or qualify the full release.

## Executable proof plan

The next milestone is bounded type-only `proof.can_copy<T>()`, with opaque static
results, direct flags, `assert` and `expect<S>`. Begin with concrete types already
represented by the bootstrap; defer value observations, place probes, generic
analysis, bounds, composition helpers and static descriptor exports. This is a
partial package milestone, not revision 1 qualification. Only module/revision
metadata and descriptor type aliases are implemented so far. Pending copy-query metadata is retained, but no evaluated result or observation
outcome is constructed. The reference remains authoritative.

### Current indirect-store effect slices

The dependency-ordered implementation series is complete:

1. Bounded pre-RHS indirect-store snapshots and core tests (`7110adb`).
2. Origin/owner/stopped-input and atomic exhaustion coverage (`8983112`).
3. Compiler gate and documentation handoff: complete.

`edges/forward/effects.rs` retains Indirect effects for encountered stores, preserving
target/RHS IDs, control and captured pre-RHS Origins. Copying never consults later
pointee state. Roots remain canonical pointee owners, not reference cells or precise
paths; completeness stays explicit, including empty snapshots. Registry/producer
owners are checked, and duplicate ports copy once. Calls remain Unknown.

MAX_ROOTS (256) bounds each snapshot and MAX_EDGES (262,144) bounds total copied
roots, separately from path steps and report items. Lookup and clone work, including
empty clones, are charged before allocation. Failed collection publishes no partial
effects and preserves reports, graph counts and conservative marks. No precise
write, independence, reachability, propagation or proof outcome is inferred.

All 66 initially selected effect-related tests and ten selected indirect-effect
tests pass (nine snapshot groups plus one existing borrow regression);
`/tmp/meowy-indirect-effects-focused.log`, `/tmp/meowy-indirect-effects-boundaries.log`.
All-target Clippy passed for implementation; `/tmp/meowy-indirect-effects-clippy.log`.
Coverage includes RHS retargeting, indexed/reborrow/aliased pointees, owners/control,
incomplete/empty snapshots, duplicate copying, per-snapshot/aggregate storage caps
and early/mid/late work failure with exact-budget success. Ordinary E305 for an
unconditionally exiting pointer expression remains intact. All ten compiler checks
pass: 1890 library/913 native tests, formatting, Clippy, build, tooling and conformance
(10 passed/13 unsupported/0 failed in debug/release);
`/tmp/meowy-indirect-effects-gate.log`. No outstanding test failures remain.
The foundation guide documents snapshot scope and the remaining graph boundary.
All four default checks pass, including 1208 local links in 110 Markdown files;
`/tmp/meowy-indirect-effects-docs.log`.

Declaration/identity aliases, module-initializer inputs and ordinary forward-group
completion are implemented; the current handoff above records validation and the
remaining export gap. Preserve independent bodies, explicit None prefixes, unknown
effects and proof gates.

### Proof dependency implementation slices

1. Preserve a proof-dependency mark in initializer evidence, including scalar
   copies, arithmetic, evaluated predicates, block conditions and record projections.
   Validate with seeded checker evidence while actual flag projections stay gated.
2. Reject marked evidence at the shared required-input boundary with E225,
   preserving existing source errors, budgets and fixed type signatures.
3. Add source/base-graph dependency propagation across skipped successors, calls
   and mutable state; prevent proof-derived facts from narrowing ordinary checking.
4. Complete E225 enforcement at type formation and observation availability, retaining fixed
   signature queries, then integrate native coverage before enabling outcomes.

Current investigation: `Input::add` combines evaluated scalar and record evidence;
boolean field projection reconstructs evidence and must explicitly preserve the
mark. This is only an evidence prerequisite, not complete phase analysis: selected
initializer traversal cannot replace base-graph analysis of both successors.
The evidence field and five seeded checker groups pass: scalar copies/arithmetic,
selected scalar conditions/tails, nested record projections, original failures and
independent ordinary inputs. Fixed query signatures/revision remain unmarked.
All 977 library tests pass (`/tmp/meowy-proof-dependencies-library.log`);
formatting and whitespace checks pass. No source-level flag is admitted.
Evidence slice committed as `781ce60`. The shared `Work::input` boundary now
rejects marked evidence with E225 after existing budget/source-error checks.
Three focused checker groups pass, covering scalar copies, boolean selection,
record/leaf reads and computed query arguments, fixed signatures, root restoration
and diagnostic precedence. All ten compiler checks pass, including 980 library
and 903 native tests (`/tmp/meowy-proof-dependencies-gate.log`).
The first slice necessarily updates all
`Input` struct literals together to remain buildable; these small constructor edits
span more than eight files and cannot be committed separately from the new field.

### Current base-fact isolation slices

1. Add a checked-HIR dependency walk over both expression operands and block
   successors; retain local dependency marks separately from scalar constants.
   Integrate immutable binding propagation and prevent marked constant folding.
2. Make marked boolean guards opaque to base facts, preserving both successors
   and preventing correlations from granting typing/ownership acceptance. Add
   seeded checker/ownership regressions, then run the complete compiler gate.

Investigation: `constant` follows ordinary local constants and short-circuits;
`guard` additionally reuses local boolean identities and decomposes logical
operators. Initializer evidence alone cannot protect either path. The HIR walk
must include call arguments, indices and both block successors without executing
anything. `f0288d6` adds the HIR walk and binding/constant integration. Three focused groups and
all 983 library tests pass (`/tmp/meowy-proof-base-library.log`).
Opaque guard integration and complete-block borrow/loan checks pass: marked
conditions retain E302 and cannot justify complementary emissions (E205).
Ordinary guards remain unchanged. The prior evidence test now checks block results
and copies using supported branch bindings and unconditional emissions; it passes.
All ten compiler checks pass after the test adaptation: 987 library/903 native
tests (`/tmp/meowy-proof-base-gate.log`). No outstanding failures remain. Binding evidence merges the structural mark even when initializer
evaluation short-circuits before the derived operand.
Lexical control and query availability are implemented below. Function result
summaries, mutable writes and nonlexical control remain prerequisites; flags stay gated.

### Scoped control-dependency slices

1. Track lexical proof-controlled matcher bodies, restore enclosing control on
   success/error, and mark ordinary bindings plus initializer evidence created
   there. Prevent those bindings from supplying ordinary constants. Test nested
   branches, independent following statements and E225 required reads.
2. Retain control-dependent query availability in pending metadata and report
   E225 after ordinary type/ownership checks, including when an earlier independent
   query remains B001-gated. Test copies, nesting, spans and diagnostic precedence;
   run the complete compiler gate and update both handoffs.

Investigation: matcher bodies currently restore only reach/scope on success.
Pending queries are finalized only after borrow/loan validation, but only the first
query is inspected. Lexical control must not leak into later independent statements.
The lexical control field and binding propagation pass all three new checker
groups and all 990 library tests (`/tmp/meowy-proof-control-library.log`). Matcher
control and lexical scopes restore on errors; unrelated following bindings remain
unmarked. Compiler-created emission locals remain separate from source bindings.
Binding/control slice committed as `4ab86be`. Query metadata now retains lexical
control and the final gate inspects controlled queries before unsupported outcomes,
after ordinary validation. Four focused query groups pass: nested/skipped control,
original spans, descriptor copies, later independent queries, earlier B001 queries
and ordinary E207/E302 precedence. The complete compiler gate passes all ten
checks, including 994 library/903 native tests (`/tmp/meowy-proof-control-gate.log`).
Control after conditional leave/restart, function summaries and mutable writes
remain separate work; these slices do not enable source-level proof flags.

### Mutable dependency slices

1. Preserve dependency marks on direct mutable local writes from marked RHS values
   or lexical proof control; keep ordinary type/mutability checks first. Test later
   copies/guards/query availability and unrelated locals.
2. Extend owned field/list path writes to include RHS, evaluated index and lexical
   control dependencies. Test nested paths, ordinary failures and independent
   storage; run the full compiler gate and update both handoffs.

Investigation: `stmt_inner` clears ordinary refinements with `forget` after scalar
writes; `write_path` uses `forget_field` after checking indices and RHS. Neither
records dependency writes. These bounded slices use conservative whole-owner,
monotone marks: a later independent overwrite does not yet erase a dependency.
Alias/indirect-store propagation, precise path overwrite/join rules, function
summaries and conditional-exit control remain prerequisites to enabling flags.
No source-level proof flag becomes available in this series.
Direct local write propagation passes four new checker groups and all 998 library
tests (`/tmp/meowy-proof-writes-library.log`). Invalid immutable/type-mismatched
writes retain E305/E207 and leave target marks unchanged. Committed as `aa89513`.
Owned path writes now merge RHS/index/control dependencies after validation;
three focused path groups pass, covering RHS/index/control propagation,
subsequent query availability, unrelated owners and preserved E201/E207/E305
errors. All ten compiler checks pass, including 1001 library/903 native tests
(`/tmp/meowy-proof-writes-gate.log`).

### Emitted-slot dependency slice

Preserve marked initializers/control on named emissions, canonicalize dependency
writes through existing `Alias::root`, and consult that root for alias reads.
Include sibling-alias and completed-record regressions plus ordinary validation
failures. Run the full compiler gate and commit this storage-identity slice.

Investigation: emitted names bypass ordinary Bind mark propagation. Their alias
records already share a root keyed by target block and field; copying references
is a different relation and must not be inferred from that root. Existing marks
remain conservative and monotone. Borrowed aliases/indirect stores, precise
writes/joins, function summaries and nonlexical control remain unfinished.
Four focused emitted-alias groups pass. The library run found one older test
counting only ordinary binding marks; its assertion now includes named emission
bindings. All ten compiler checks pass, including 1005 library/903 native tests
(`/tmp/meowy-proof-aliases-gate.log`).

### Scalar-reference dependency slices

1. Retain single-owner links for immutable scalar-reference bindings, direct
   borrows and reborrows; consult current owner marks on reference reads. Cover
   copies, independent owners and explicit unsupported origin shapes.
2. Use known owner links for indirect scalar stores after ordinary permission/type
   checks. Include RHS/control/target dependencies, preserve error precedence,
   and run the full compiler gate before committing the store integration.

The checker precedes borrow/loan replay, so this bounded metadata must not replace
ownership analysis. Mutable reference retargeting, joins, aggregate/call-returned
reference origins and wider alias graphs stay untracked; flags remain gated.
Known owners retain the existing conservative whole-owner, monotone marks.
Three new reference groups and all 1008 library tests pass
(`/tmp/meowy-proof-pointees-library.log`). Immutable scalar reference copies and
reborrows observe later owner marks. Committed as `64dd1b0`. Indirect stores now
mark known owners after ordinary checks; dependency-bearing stores with unknown
origins report B001 explicitly. Four focused store groups pass, including
borrow/loan validation, reborrows, indexed targets, unknown-origin gating and
ordinary E305/E207 precedence. All ten compiler checks pass, including 1012
library/903 native tests (`/tmp/meowy-proof-references-gate.log`).

### Mutable-reference origin slices

1. Replace single-owner reference links with bounded owner sets and explicit
   completeness. Preserve current immutable-reference admission and indirect-store
   gating; unknown origins are not proof evidence. Validate multiple retained
   owners, snapshots, and origin limits alongside the existing reference tests.
2. Track mutable scalar-reference bindings and conservatively union old/new owners
   on assignments. Preserve incomplete origins across updates, propagate store
   marks to every retained owner, and test retargeting/copies/branches/errors.
   Run the full compiler gate and commit each validated slice.

Retargeting uses monotone owner sets rather than claiming precise overwrite or
branch joins. Unknown call/aggregate origins retain an incomplete marker; dependent
stores cannot silently write only a known subset. Source-level flags stay gated.
The bounded representation passes all 1014 library tests, including new merge/
snapshot and capacity regressions (`/tmp/meowy-proof-origin-sets-library.log`).
Owners are capped at 256 and merging spends the existing analysis budget; failed
updates retain prior metadata. Committed as `44a59fb`. Mutable bindings now
record origins and assignments conservatively merge prior/new sets; missing or
incomplete prior origins cannot become complete through a conditional update.
Four focused retargeting groups pass, including borrow/loan validation for
conditional stores, retained prior copies, incomplete-source gates and failed
assignment metadata preservation. All 34 dependency groups and all ten compiler
checks pass (`/tmp/meowy-proof-retargets-gate.log`); no outstanding failures remain.

### Emitted reference-origin slice

Track scalar-reference origins on named emissions, canonicalize origin reads/writes
through the emitted slot root, and merge sibling emission origins with existing
bounds/completeness rules. Keep ordinary reference copies as snapshots. Validate
initializers, alias retargets, later sibling reads and incomplete-source gates;
run the complete compiler gate and commit this shared-storage slice.

This slice covers lexical emitted names and shared storage identity; completed
record projections and function-returned origins remain separate. Four focused
groups pass. Shared-reference fixtures pass borrow/loan validation; exclusive-slot
stores retain the existing B001 carrier gate, and mutable exclusive-reference
fields remain gated. Incomplete origins still reject dependent stores. Marks and
owner sets remain conservative and source-level flags stay gated. All ten compiler
checks pass (`/tmp/meowy-proof-reference-slots-gate.log`); no outstanding failures
remain.

### Completed record reference slices

1. Retain bounded field-origin snapshots for immutable ordinary record bindings
   whose fields hold immutable scalar references. Use existing emitted slot roots
   for direct block initializers and clone snapshots for ordinary record copies.
   Resolve direct local field projections and propagate later owner marks; test
   accepted borrow/loan behavior, field identity and incomplete-source boundaries.
2. Document the admitted flat-record boundary, refresh the root handoff, and run
   the final compiler gate before committing the validated implementation; validate
   the guide integration separately without repeating successful runtime checks.

Mutable record bindings/fields, nested aggregate projection, record coercion,
composition and function-returned origins remain incomplete. Unknown origins are
not proof evidence and dependent stores still gate incomplete sets. Flags stay gated.
Five focused groups pass: direct projections, copies/later owner marks, ordinary
borrow validation, incomplete-source boundaries and the 256-field metadata limit.
Failed capacity checks retain prior metadata. All ten compiler checks pass
(`/tmp/meowy-proof-record-origins-gate.log`). `0d45f39` records the implementation;
the foundation guide describes the flat immutable boundary. All four default
documentation checks pass (`/tmp/meowy-proof-record-origins-docs.log`).

### Mutable flat-record origin slices

1. Track flat reference origins for mutable record bindings and merge field origins
   on whole-record assignments, preserving earlier copies, completeness, bounds
   and ordinary assignment errors. Keep mutable reference fields outside this slice.
2. Track mutable scalar-reference fields and update their metadata on direct field
   writes. Retain conservative old/new owners, test incomplete sources and copies,
   then run the full compiler gate and document the expanded boundary.

Nested/indexed aggregate references, coercion/composition, returned records and
precise overwrite/branch joins remain unfinished. Field metadata is analysis-only;
ordinary type and ownership checking remain authoritative. Flags stay gated.
Three new whole-record groups and all 1030 library tests pass
(`/tmp/meowy-proof-record-writes-library.log`). Conditional replacements also pass
ordinary borrow/loan validation. Committed as `d1d5f21`. Direct mutable
scalar-reference fields now retain initial origins and merge updates after ordinary
validation. Three new field-write groups and all 11 record dependency groups pass,
including ordinary compilation of conditional updates, copied snapshots, independent
fields and incomplete sources. All ten compiler checks pass
(`/tmp/meowy-proof-record-mutations-gate.log`); no outstanding failures remain.

### Nested record path prerequisite

Replace scalar field-index metadata keys with ordered field paths and resolve
checked field chains against those paths. Preserve flat construction, copies,
updates, bounds and incomplete-source behavior. Validate seeded nested paths and
run the full compiler gate before committing this representation slice.

This is a prerequisite only: source-level nested record construction/writes remain
untracked. The next slice must populate bounded nested paths and update or merge
entire subrecord paths before admitting those forms. Flags stay gated.
Two new seeded path groups and all 13 record dependency groups pass. Flat record
construction/writes retain single-component paths. All ten compiler checks pass
(`/tmp/meowy-proof-record-paths-gate.log`); no outstanding failures remain.

### Nested record implementation slices

1. Extract record source-origin lookup from storage merging without changing flat
   behavior. Validate the existing library suite and commit the prerequisite.
2. Generalize source lookup to nested paths/subrecord copies and merge sibling
   source origins while preserving completeness; validate before producer admission.
3. Collect bounded nested paths, retain nested named-emission snapshots, and merge
   scalar/subrecord writes at the matching prefix. Include nested copies/projections,
   mutable updates, ordinary errors and bounds, then run the full compiler gate.

Indexed aggregates, coercion/composition, returned records and precise overwrite/
branch joins stay incomplete. No source-level proof flag is enabled by this series.
The source lookup extraction passes all 1035 library tests
(`/tmp/meowy-record-source-library.log`), committed as `8585305`. Nested source
lookup and subrecord snapshot lookup pass all 1035 library tests
(`/tmp/meowy-nested-source-library.log`). Source-level nested construction remains
next, together with matching write updates. Source lookup committed as `76a54b5`.
Nested producers, named-emission snapshots and prefix updates are implemented.
Focused checks pass for construction, copies, subrecords, conditional replacements,
unknown sources, depth limits and ordinary errors. All ten compiler checks pass
(`/tmp/meowy-proof-nested-records-gate.log`); no outstanding failures remain.

### Record composition origin slice

Snapshot origins on the existing composition temporary and retain successful
composition sources per target block. Resolve destination field names back to
source field paths, merging named/composed alternatives with existing origin
bounds/completeness. Test reordered fields, nested records, conditional composition,
copy snapshots and unknown sources, then run the full compiler gate and commit.

Inspection: record acceptance uses exact alternatives; general union coercion
metadata needs separate work. Composition is an immediate origin gap because its
temporary is not an ordinary binding and composed fields have no named aliases.
Indexed/coerced/call-returned origins and precise joins remain unfinished. Flags
stay gated. No ownership or runtime layout rules change in this slice.
All five composition groups and all ten compiler checks pass, including ordinary
compilation of composed records and conditional mixed completeness
(`/tmp/meowy-proof-composition-gate.log`). No outstanding failures remain.
Composition snapshots and name-based lookup reuse existing origin limits.

### Nullable record origin slice

Track origins through checked wrapping/narrowing of one record shape with null.
Use the same field paths for that shape; known null contributes no owners while
unknown sources stay incomplete. Cover copies, mutable replacement, nested nullable
fields and rejection of mixed record shapes, then run the full compiler gate.

This slice does not interpret arbitrary union layouts or infer runtime tags.
Ordinary typing/ownership and proof-derived guard isolation remain unchanged.
Indexed/call-returned origins, heterogeneous record unions and precise joins stay
unfinished; flags remain gated.
Four focused nullable-record groups pass, including ordinary compilation of wrapping,
narrowing, nested nullable fields and replacement. Known null retains empty complete
origin sets; unknown and heterogeneous sources remain incomplete. All ten compiler
checks pass (`/tmp/meowy-proof-nullable-records-gate.log`); no outstanding failures
remain.

### Indexed borrow origin slice

Unwrap shared element-borrow expressions to their checked container origin and
retain ordinary reference bindings to reference-free lists/records. Preserve
whole-owner identity through view copies, nested indices and reborrowed scalar
fields. Test ordinary borrow validation, later marks, independent owners, indexed
control dependencies and incomplete returned/reference-bearing aggregate origins;
run the full compiler gate and commit this bounded slice.

This does not admit reference-bearing list elements or resolve call/temporary
origins. Existing lifetime/ownership checks remain authoritative. Flags stay gated.
Three indexed groups and all 68 dependency groups pass, including ordinary
compilation/ownership validation, nested list/record-element borrows, late owner
marks, index dependencies and incomplete returned views. All ten compiler checks
pass (`/tmp/meowy-proof-indexed-origins-gate.log`); no outstanding failures remain.

### Aggregate-stored view origin slice

Share the supported reference-pointee predicate between ordinary bindings and
record field collection/updates. Retain reference-free list/record view owners
through aggregate storage, nested/composed/nullable copies and mutable field
replacement. Test ordinary ownership validation, prior snapshots and incomplete
returned/reference-bearing origins; run the complete compiler gate and commit.

This expands origin metadata only. References to reference-bearing aggregates,
unknown returned/temporary origins and heterogeneous record unions remain
incomplete. Owner sets stay conservative and source-level flags remain gated.
All three focused view-field groups pass, including ordinary compilation of
nested/composed/nullable stored views and mutable record-view updates. Mutable
list-view fields retain their existing B001 gate. All ten compiler checks pass
(`/tmp/meowy-proof-stored-views-gate.log`); no outstanding failures remain.

### Temporary storage origin slice

Reuse existing temporary storage IDs as borrow origins and propagate initializer/
lexical-control marks onto those owners. Cover distinct temporary identities,
shared element borrows over temporaries, later marks and ordinary lifetime errors.
Run the full compiler gate and commit this bounded storage-identity slice.

This does not infer pointees of references copied out of temporary carriers or
extend statement lifetimes. Returned/reference-bearing origins, heterogeneous
record unions and precise phase joins remain unfinished; flags stay gated.
Three focused groups pass, including distinct IDs, initializer/control marks and
unchanged E303 expiry. Direct temporary-expression dependency lookup now consults
the retained owner mark too. All ten compiler checks pass
(`/tmp/meowy-proof-temporary-origins-gate.log`); no outstanding failures remain.

### Direct temporary-carrier slices

1. Snapshot reference/record contents when temporary storage is created, then
   resolve direct dereferences of that temporary through the content snapshot.
   Keep storage IDs distinct from pointee IDs; test copied references, nested
   record fields, unknown calls and existing expiry errors. Run the full gate.
2. Document the direct-carrier boundary and refresh the handoff separately.

Indirect carrier chains and returned/reference-bearing aggregate origins stay
incomplete. This metadata must not extend statement lifetimes. Flags stay gated.
Content snapshots and direct dereference lookup are implemented. Focused tests
identified empty-path reborrow wrappers; these now resolve to the same temporary
storage ID. Nonempty projections/indirect carrier chains remain incomplete.
All three focused carrier groups pass, including ordinary compilation and E303
expiry. All ten compiler checks pass (`/tmp/meowy-proof-temporary-carriers-gate.log`);
no outstanding failures remain. Implementation committed as `f4c2031`; the guide
now describes the direct-carrier boundary. All four default documentation checks
pass (`/tmp/meowy-proof-temporary-carriers-docs.log`).

### Named reference-cell slice

Retain checked cell locations for immutable one-level reference-to-reference
bindings and their copies/empty-path reborrows. Resolve direct or named cell reads
through the current reference/record-field origin metadata, preserving pointee
identity, snapshots and ordinary borrow/expiry errors. Test local and nested-field
cells, mutable pointee-reference updates, unknown contents and unsupported mutable
carrier aliases. Run the full compiler gate and commit the bounded integration.

Mutable carrier aliases, deeper carrier chains, returned origins and broader
reference-bearing aggregates stay incomplete. No lifetime or permission rule changes;
flags remain gated.
Three focused cell groups pass, including ordinary compilation, nested field cells,
prior snapshots, incomplete contents and E302 protection. A stored-view cell case
passes too. All ten compiler checks pass (`/tmp/meowy-proof-reference-cells-gate.log`);
no outstanding failures remain.

### Mutable reference-cell slices

1. Replace single cell locations with bounded location sets and completeness;
   preserve immutable aliases and merge dereferenced pointee origins without
   confusing unknown cell locations with an empty known set. Validate and commit.
2. Track mutable carrier bindings and conservatively merge retargeted locations,
   preserving prior copies, incomplete alternatives, ordinary errors and bounds.
   Run the complete compiler gate and update the guide/handoffs.

Only one-level cells containing supported references are in scope. Deeper carrier
chains, returned origins, heterogeneous record unions and precise joins remain
unfinished; flags stay gated.
Cell sets/completeness pass all 1065 library tests, including capacity failure
and snapshot regressions (`/tmp/meowy-proof-cell-sets-library.log`). Mutable
binding/retarget integration after `393b939` passes four new source groups and
all eight cell groups, including ordinary compilation of conditional updates.
All ten compiler checks pass (`/tmp/meowy-proof-cell-retargets-gate.log`);
no outstanding failures remain.

### Bounded carrier-chain slices

1. Make reference-cell/origin traversal fallible and charge wrapper/cell expansion
   to the existing analysis budget; reject oversized pointee unions before storing
   them. Preserve unknown-source incompleteness and validate/commit this prerequisite.
2. Resolve bounded deeper cell dereferences and admit matching carrier types,
   preserving snapshot/completeness and ordinary ownership checks. Add depth,
   fan-out and source-chain regressions, then run the complete compiler gate.

Deeper field-stored carriers, returned origins and precise joins remain separate.
Resource failures remain B001, never proof outcomes; flags stay gated.
Fallible traversal and explicit budget/merged-pointee capacity regressions pass
all 1070 library tests (`/tmp/meowy-proof-origin-traversal-library.log`). Bounded
deeper-chain integration follows `ea02215`. Iterative dereference expansion,
carrier-type admission, temporary content snapshots and cycle-safe dependency
lookup pass all five chain groups, including depth/fan-out bounds and ordinary
E302/E303 preservation. All ten compiler checks pass
(`/tmp/meowy-proof-carrier-chains-gate.log`); no outstanding failures remain.

### Emitted carrier-cell prerequisite

Register carrier contents on named emissions and canonicalize cell metadata through
`Alias::root`, including sibling emission merges and subsequent retargets. Preserve
ordinary carrier-copy snapshots, bounds and unknown alternatives. Test named/deep
emitted carrier reads and sibling aliases, then run the full compiler gate.

Completed-record carrier-field lookup still needs its own field-location metadata;
this slice covers lexical emitted names only. Returned/reference-bearing origins,
heterogeneous unions and precise joins remain unfinished; flags stay gated.
All four focused emitted-cell groups pass. Named/deep aliases and retargets pass
ordinary compilation; snapshots and canonical sibling locations remain distinct.
Unknown contents/completed-record carriers retain incomplete metadata. All ten
compiler checks pass (`/tmp/meowy-proof-emitted-cells-gate.log`).

### Record carrier-field source prerequisite

Separate checked record source locations from pointee-origin merging. Preserve
explicit unknown, known-null, direct-path and alternative-source distinctions,
including existing budget charging. Validate the old origins and new location
classification, then run the full compiler gate and commit this prerequisite.

The next slice will use these locations for bounded record carrier-cell snapshots
and matching field/subrecord updates. Completed-record carrier fields remain
incomplete until that storage and update integration is present. Flags stay gated.
All 30 existing record dependency groups pass after extraction. Explicit source
classification regressions now cover direct storage/alternatives, known null and
unknown calls without replaying calls. Both new groups and all seven source-lookup
groups and all ten compiler checks pass (`/tmp/meowy-proof-record-locations-gate.log`);
no outstanding failures remain.

### Record carrier-cell integration slices

1. Add bounded record-cell storage, source/location reads and carrier path
   classification, preserving ordinary pointee paths. Validate seeded field reads
   and inline named-field sources before committing this read-side prerequisite.
2. Populate snapshots on record construction/copies and merge field/subrecord
   updates through the existing hooks. Include composition, nullable records,
   snapshots, limits and ordinary errors; run the complete compiler gate.

Do not treat missing record-cell metadata as complete. Returned/reference-bearing
origins, heterogeneous unions and precise phase joins remain unfinished; flags stay gated.
The read-side storage/classification slice passes all 1084 library tests
(`/tmp/meowy-proof-record-cell-reads-library.log`), including seeded field cells,
inline named-field sources and separation from pointee paths. Producer/write
integration follows `aaa2a07`. Record cell snapshot producers, prefix merges,
scalar carrier-field writes and composition registration are now wired together;
three focused source groups and all six record-cell groups pass. Capacity and
depth failure preservation is covered. All ten compiler checks pass
(`/tmp/meowy-proof-record-cells-gate.log`); no outstanding failures remain.

### Scalar-reference return origin slices

1. Extract the existing borrow-contract return-candidate rule without changing
   ownership behavior; validate mode/type matching and commit the prerequisite.
2. Resolve scalar-reference call origins from those compatible checked arguments,
   with bounded nested-call traversal and completeness preservation. Test multiple
   candidates, exclusive modes, unknown inputs and ordinary error precedence;
   run the full compiler gate and update the guide/handoffs.

No private function-body inference or proof-answer evaluation is added. Broader
return shapes and function data/control summaries remain separate; flags stay gated.
The shared return-candidate predicate and mode/type regression pass all 1089
library tests (`/tmp/meowy-return-candidates-library.log`), committed as `173d285`.
Four call-origin groups and all 108 dependency groups pass. Existing unknown-input
fixtures now use untracked block-produced references rather than known direct
arguments. All ten compiler checks pass (`/tmp/meowy-proof-return-origins-gate.log`);
no outstanding failures remain.

### Shared aggregate-view return origins

Extend call-origin tracking to the bounded general borrow-contract subset whose
shared-reference inputs/results have no borrowed components. Reuse the existing
`borrow_contract::projections` relation for record fields/list elements, retaining
whole-container owners and all compatible candidates. Test direct/nested views,
projected returns, mixed completeness and preserved ordinary checks, then run the
full compiler gate and commit.

Concrete by-value record arguments now contribute nested named shared-reference
fields through the current slice above. Other borrowed aggregates,
reference-bearing/allocator-bound pointees, returned
carriers/records and function effect/data/control summaries remain separate.
No private body is evaluated or used to narrow candidates; flags stay gated.
Three focused return-view groups and all 111 dependency groups pass. Direct record
views and nested-list projections also pass. All ten compiler checks pass
(`/tmp/meowy-proof-return-views-gate.log`); no outstanding failures remain.

### Prerequisites and current integration

Completed dependency-ordered signature slices:

1. `b71feef`: admit fixed boolean flag signatures in unevaluated type queries on
   pending descriptor bindings, with checker tests for aliases, budgets and preserved
   gates. The phase contract permits these signatures independently of answers.
2. Native origin/ordinary-error coverage and the admitted-boundary guide; the full
   compiler gate passes all ten checks (`/tmp/meowy-proof-signatures-gate.log`).

`expressions.rs::hint` resolves fixed flag types through the existing safe binding
lookup without entering `symbol`'s flag-value gate. It does not prepare calls,
expose answers or add constants. Inline observation calls stay gated.
Transitive data/control tracking remains the next milestone after this prerequisite.
The regression reproduced B001 at `((copy).always)<>` before the fix. All 972
library tests now pass (`/tmp/meowy-proof-signatures-library.log`), including four
new groups for fixed signatures, aliases, cost limits and preserved lookup/value
gates. Type-query accounting charges dispatch plus boolean materialization
(two steps, one type); no observation is prepared or recharged. Both new native
groups pass in debug/release (`/tmp/meowy-proof-signatures-native.log`). Coverage
checks ownership in dynamic branches and types in literal-false branches: existing
loan checking discards literal-false paths. No outstanding failures remain.

- `src/foundation.rs` now includes partial `proof` module identity;
  `check/names.rs::symbol` provides typed revision metadata. Module/member aliases
  retain identity and same-spelling user bindings remain ordinary bindings.
  Descriptor type names resolve to `Spec::Descriptor`. Type-only `can_copy` calls
  now queue metadata by resolved `Item::CanCopy` identity; evaluation stays gated.
- `check.rs::Value::Pending` indexes an immutable query record without runtime
  storage. Copies retain call span, owner, target, revision and checked type argument. `Spec::Descriptor` names static descriptor
  types without adding runtime `hir::Type` variants. `Spec::Meta` still represents
  `core.Type`. Results need a fixed declared `proof.Result` type distinct from
  the active nominal alternative. Do not encode them as ordinary runtime records
  or add a native foundation layout. Copies retain origin/target/revision metadata.
- `check/functions.rs::call` evaluates ordinary arguments through `expr` and builds
  runtime calls. Observation arguments must bypass that path only after intrinsic
  identity is resolved. Type-only queries need no place observation machinery;
  future value/place queries need validated source descriptions without loads,
  loans, index evaluation or last-use effects.
- `check.rs::check_imports` checks the body before borrow and loan validation.
  Pending query bindings now have fixed Result signatures while checking; a B001
  gate after borrow/loan validation prevents unresolved queries reaching codegen. Type capability answers need a type walk,
  not a scalar CFG analysis. They still must not discharge assertions early or
  suppress ordinary failures in uncalled/runtime-skipped checked bodies.
- Initializer evidence now retains proof-dependency marks and required reads
  reject marked evidence with E225. Scalar constants and the ordinary base graph
  still need transitive data/control dependency tracking before exposing flags;
  type formation and query availability must reject E225. Ordinary runtime
  conditions derived from flags retain both successors for base typing/ownership.
  Existing constant folding or `inputs` evidence cannot provide this guarantee.
- `type_values/work.rs::Work` retains bootstrap limits (4096 visits, 64 levels,
  16384 nodes), with B001 failures. Its separate `required::Budget` currently charges
  type materialization, required statements/blocks and integer/boolean evaluation,
  including scalar/record projection ancestors and retained record reads. Type
  expression dispatch and required source constructors now charge separately.
  Pending queries retain statement/outer-root budgets. Descriptor materialization,
  text/helper admission and phase/dependency tracking remain prerequisites.
  Never relabel B001 as E220.
- `hir::Type::is_copy` is a reuse candidate for admitted concrete runtime types;
  audit its domain before dispatch. `<never>` and compile-time-only types are
  explicitly Never, but `Type::is_copy` currently returns true for `Type::Never`;
  a direct call would therefore give the wrong proof result. Unsupported type
  representations remain B001, and malformed
  types retain ordinary errors; neither is an Indeterminable result.
- `diagnostic.rs` carries one span and a message; `driver.rs::report_at` maps it
  to an owning source. E224 also needs deterministic query origins, required/actual
  alternatives, capability facts and revision. Preserve private file boundaries;
  any source-note support should be an independently validated prerequisite.

### Pending descriptor statement roots

`queries.rs::pending_form` separates recognition/arity checks from argument
construction. `queries/statements.rs::pending_statement` starts a construction
root at each admitted binding/expression statement, or joins the active root.
The statement, call/copy read and explicit annotation share one ledger. Grouping
adds no steps. Calls retain the final ledger, including annotation charges and
sticky failures; independent statements reset budgets. Copies preserve the
original query ID without requerying or altering its original ledger.

Completed dependency-ordered slices:

1. `84ba2bc`: separate recognition from preparation; no constructor work or query
   reservation during admitted call recognition. All 964 library tests passed.
2. `835386a`: share pending statement/annotation roots, charge reads and preserve
   limits, error order and required-branch gates. All 967 then-current library
   tests passed, plus the subsequent required-branch case in 27 focused tests.
3. Native checked-body/origin and skipped-required-branch checks, guide and handoffs.
   All ten compiler checks pass (`/tmp/meowy-pending-statement-gate.log`);
   the unused test import found by Clippy has been removed.

Logs: `/tmp/meowy-pending-recognition.log`, `/tmp/meowy-pending-statements.log`,
`/tmp/meowy-pending-statements-focused.log`.

Required-block descriptor construction, descriptor outcome materialization and
transitive proof data/control dependencies remain unimplemented. Pending metadata
has no descriptor aggregate payload; do not claim materialization/analysis charges
from these preparation roots. Text/helper execution remains unsupported and needs
its own counters when admitted. Text type queries do not materialize bytes.

### Pending-query argument roots and retained budgets

Type-only queries now charge one invocation step and construct written arguments
through `source_spec` in a `construction_root` at the call span. Existing roots
share their remaining counters; ordinary extent gates and temporary computed-input
modes remain intact. Arity checks precede construction. Malformed/unsupported
arguments reserve no query metadata or budget slot.

`Work.query_root` reserves one stable index into `Checker.query_budgets` on the
first admitted query. Nested queries share it. `required_root` moves the final
logical ledger into that slot when the outer root exits, preserving tail charges,
all three counters, the original span and sticky E220. Independent calls start
fresh ledgers even at identical source spans. Copies keep their query/root IDs
without replaying argument construction. Roots without queries retain no ledger.
Bootstrap traversal counters remain local resource guards. The final pending gate
reads the retained ledger after ordinary typing and ownership checks.

Completed dependency-ordered slices:

1. `66dfbca`: argument roots, invocation/type charges, exact-limit, mode and
   first-error tests. All 957 library tests passed.
2. `bbc71bf`: shared budget identity and retention through outer-root completion,
   copies, tail work/failures and independent roots. All 960 library tests pass.
3. Native facade targets, descriptor/meta arguments, call origins, argument modes
   and dependency failures; guide and handoffs. Both native groups pass in both
   profiles, reaching the expected pending-evaluation B001 gate for valid arguments.

Logs: `/tmp/meowy-query-arguments.log`, `/tmp/meowy-query-budgets.log`,
`/tmp/meowy-query-budget-native.log`. The full compiler gate passes all ten
checks, including 960 library/899 native tests, formatting, Clippy and bootstrap
conformance (`/tmp/meowy-query-budget-gate.log`).

Required-block query syntax, evaluated outcomes, scalar flags, descriptor result
construction and phase/dependency tracking remain gated. Annotation accounting
and statement roots are integrated; descriptor materialization remains open.

### Symbol-probe audit and field-hint isolation

`symbol(TypeQuery)` runs `type_value`, and computed `type_literal` operands also
enter required evaluation. Field hints previously called `symbol` twice while
looking for heap/static metadata: a computed field base spent work and could leave
sticky E220 despite its hint result being discarded. `hint_symbol` now resolves
only name/import chains; computed bases and nested type queries spend no operand
work. Successful metadata/record hints and selected constructor errors remain intact.

Function-call classification, unannotated export classification and required scalar
field classification can also inspect computed bases, but those forms currently
reject. Preserve their diagnostics separately; do not globally charge `symbol`.
Required comparison form checks already restrict field bases to names/imports.
Direct bindings/ascriptions retain their existing execution paths. Pending query
arguments now construct and retain their outer budgets as described above.

Dependency-ordered slices:

1. `19cff61`: hint-only resolution with zero-work, sticky-limit, metadata and
   error-order regressions. All 953 library tests pass.
2. Native imported record/metadata widths, source query/constructor error spans,
   guide and handoffs. Both focused native groups pass in debug/release.

Logs: `/tmp/meowy-field-hint-library.log`, `/tmp/meowy-field-hint-native.log`.
The complete compiler gate passes (`/tmp/meowy-field-hint-gate.log`), including
953 library/897 native tests, formatting, Clippy and bootstrap conformance.
Pending-query argument construction and retained budgets are integrated above;
proof outcomes stay gated.

### Type-use execution accounting

Ascription/type-test target construction uses `construct_type` after checking its
value once. The old `ty` lookup wrapper is now test-only. `binding_symbol` handles
ordinary unannotated identity bindings separately from shared `symbol` probes:
literals construct once in the ordinary mode, type/member/nominal reads charge
read steps and payload traversal, and queries retain their existing evaluator
without a second charge. Groups and synthetic wrappers add no logical work.

Completed slices:

1. `3876c34`: target construction, exact/overflow costs, mode/error order and
   lookup isolation for ascriptions and type tests.
2. `93f21d4`: type-identity binding execution, including copies, members, nominal
   identities, queried types, grouping, subtraction, input gates and no storage.
3. Native imported identities, ascriptions/type tests and source-file errors;
   guide and both handoffs. The complete compiler gate passes.

Six new checker groups and two native groups pass. The complete gate passes
950 library/895 native tests, Clippy and formatting. Native accepted programs run
in debug/release, retaining startup order, widths and original errors. No
outstanding failures remain. Logs:
`/tmp/meowy-ascription-roots-library.log`, `/tmp/meowy-type-binding-library.log`,
`/tmp/meowy-type-use-integration.log`, `/tmp/meowy-type-use-gate.log`.

The field-hint audit and isolation above follow this series. Shared `symbol`
remains unchanged: direct query bindings execute their existing evaluator, while
rejecting call/export/required-field forms retain their diagnostics. Pending-query
argument roots and retained budgets are integrated above; text/helper admission
and phase/dependency tracking remain prerequisites to proof outcomes.

### Function signature accounting

Definitions construct written annotations in a root spanning the source header;
result annotations precede parameters. The root ends before body checking unless
an outer required root already exists. Forward reservations and explicit function
re-export annotations root their source function-type expressions. Reservations
and definitions are distinct source work; omitted result annotations reuse the
reserved/inferred result without additional construction. Ordinary/computed modes
and nested extent budgets follow `construction_root`/`mode_root`.

Completed dependency-ordered slices:

1. `d4f4573`: body locals reuse checked parameter types. Annotation evaluation is
   not repeated after parameter names enter scope; outer constant widths survive
   parameter shadowing. Duplicate names and signature mismatches retain checks.
2. `9157ec3`: definition/forward roots, exact counts, type limits and mode tests.
   Forward definitions now evaluate written result annotations before parameters.
3. Explicit re-export roots, native facade/forward/error integration, guide and
   both handoffs; the complete compiler gate passes.

Six focused checker groups and two new native groups pass, alongside existing
signature/documentation tests. The full gate passes 944 library/893 native tests,
Clippy, formatting and bootstrap conformance. No outstanding failures remain. Logs: `/tmp/meowy-signature-reuse-library.log`,
`/tmp/meowy-signature-roots-library.log`, `/tmp/meowy-signature-integration.log`,
`/tmp/meowy-signature-gate.log`.

Ascription and ordinary identity-binding execution is integrated above. Remaining
computed-type/query probes and deferred-query budgets still need auditing; proof
outcomes and full release qualification stay gated.

### Ordinary constructor roots

`construction_root` starts a budget at an ordinary alias or data annotation's type
expression, or joins the current root. `mode_root` restores the prior ordinary/
required mode on success or failure; `type_value` temporarily enables required
input evaluation for computed operands. `Work.ordinary` controls input eligibility,
not whether work is charged. Nested extents share the enclosing constructor budget.

Completed dependency-ordered slices:

1. `b293502`: scoped modes with shared-budget and restoration regressions.
2. `a22fbdc`: ordinary alias roots, source-constructor and extent cost coverage.
3. `5aa67cf`: data binding/emission annotation roots; runtime initialization stays
   outside fresh annotation roots. `spec`, `ty` and symbol probes retain lookup mode.
4. Native integration, guide and both handoffs; the complete compiler gate passes.

Two mode groups, four alias groups and three data-annotation groups pass. The full
gate passes 938 library/891 native tests, Clippy and formatting. Native coverage passes in debug/release,
retaining imported alias/export widths, startup, input gates and original errors.
Logs: `/tmp/meowy-constructor-modes.log`, `/tmp/meowy-alias-roots-library.log`,
`/tmp/meowy-data-roots-library.log`, `/tmp/meowy-constructor-roots-integration.log`,
`/tmp/meowy-constructor-roots-gate.log`. No outstanding failures remain. Ordinary block checking still fails fast
without scope recovery; statement-level tests verify root cleanup independently.

Function signatures and direct type-use execution are now integrated above.
Field hints and pending-query argument budgets are now integrated above.

### Ordinary extent roots

`list_extent` creates a `required_root` for ordinary permitted extent expressions.
`Work.ordinary` separates accounting from retained initializer proof access;
`proven_inputs` gates local/module/field evidence in `expressions.rs`. Ordinary
constants and form restrictions retain their prior behavior. `extent_value` keeps
reachability and required-state restoration shared between both paths.

Completed slices:

1. `fd47931`: ordinary extent roots, input-mode separation and four focused groups.
2. Integration: native capacity checks, runtime-skipped error checks and imported
   source spans; documentation and both handoffs. The complete compiler gate passes.

The full gate passes 929 library/889 native tests, Clippy and formatting. Two native groups
pass in debug/release, preserving capacity/width checks, required-block inputs,
ordinary form/mutable-input rejections and original imported E107 diagnostics.
Logs: `/tmp/meowy-extent-roots-library.log`, `/tmp/meowy-extent-roots-clippy.log`,
`/tmp/meowy-extent-roots-integration.log`, `/tmp/meowy-extent-roots-gate.log`.
No outstanding failures remain.

Independent extents reset their budget; active required roots share counters and
retain broader input eligibility. Extent roots do not charge surrounding type
construction. Ordinary aliases and data annotations now supply enclosing roots;
pending-query arguments now construct and retain outer budgets. Descriptor
execution roots and text/helper admission remain separate; proof outcomes stay gated.

### Source-constructor accounting

`source_spec` explicitly charges construction for required literals, aliases and
scalar/block/record annotations; `spec` retains ordinary lookup behavior. Mode is
passed through the existing resolver, not stored in mutable checker state. Source
composites charge before children; duplicate union inputs, implicit primaries and
repeated named payload substitutions retain complete logical type/step costs.
Computed operands retain nested evaluation, and synthetic wrappers stay transparent.
Completed literal/alias payloads retain bootstrap traversal without duplicate
logical construction charges. Scalar annotation/error order remains unchanged.

Completed dependency-ordered slices:

1. `1bbf3b8`: source type-literal traversal and exact/failure/lookup/limit tests.
2. `504c37a`: required aliases and scalar/block annotations, including function
   aliases and selected/skipped declaration coverage.
3. `ca17338`: record field/copy annotations, independent copy costs and scope/root
   restoration after failed construction or logical exhaustion.
4. Integration: native facades, repeated extent blocks, source errors, guide and
   both handoffs; the complete compiler gate passes.

Eleven source-accounting groups and two native groups pass, including debug/release
startup and original-file errors. The complete gate passes 925 library/887 native
tests, Clippy, formatting and bootstrap conformance. No outstanding failures remain.
Logs: `/tmp/meowy-source-types-focused.log`, `/tmp/meowy-source-types-library.log`,
`/tmp/meowy-source-aliases-library.log`, `/tmp/meowy-source-records-library.log`,
`/tmp/meowy-source-types-integration.log`, `/tmp/meowy-source-types-gate.log`.

Ordinary aliases/data annotations and their extents now have explicit roots;
`spec` remains a lookup path. Function signatures are now rooted; other type-use
sites still need an execution-boundary audit. Text/helper values are
not admitted by the bounded required interpreter; audit gates before adding unused
counters. Pending queries now retain shared root budgets. Keep outcomes
gated until these accounting and phase/dependency prerequisites finish.

### Type-expression accounting

`type_value_inner` charges evaluated literals, reads, queries and subtraction;
member execution also charges projection ancestors. Blocks charge at entry.
`type_result` excludes groups and parser-generated subtraction wrappers from logical
materialization; bootstrap node/depth/visit guards retain their prior behavior.
Synthetic wrappers are identified by the contained expression's identical source
span. Explicit computed type constructors retain their own materialization work.

Dependency-ordered slices:

1. `d5a58b5`: expression dispatch, transparent wrappers and focused regressions.
2. Integration: selected/skipped constructors, first-failure and native facade/query
   coverage, guide and both handoffs; the complete compiler gate passes.

All ten logical-type checker tests and two native groups pass, including both
profiles, startup order, query operand isolation and original source errors.
The complete gate passes all 914 library/885 native tests, Clippy, formatting and
bootstrap conformance. No outstanding failures remain.
Logs: `/tmp/meowy-type-expression-focused.log`,
`/tmp/meowy-type-expression-library.log`, `/tmp/meowy-type-expression-clippy.log`,
`/tmp/meowy-type-expression-integration.log`, `/tmp/meowy-type-expression-gate.log`.

Required source construction is now integrated above. Ordinary root domains,
text/helper counters and deferred descriptor execution accounting remain open.

### Aggregate-slot accounting

Revision 1 counts initialized fields and primary slots, including recursive
copies, separately from expression/type steps. Required records support bounded
immutable integer/boolean/record fields with unit primaries. `record_shape`
retains existing depth/field eligibility limits; no new value shapes are admitted.

Completed dependency-ordered slices:

1. `7656b4b`: add the 1,048,576 aggregate-slot counter, atomic overflow-safe charges
   and sticky E220 failures, with shared/reset root tests.
2. `4ee4675`: charge recursive copies in `type_record`, explicit fields on
   successful insertion and implicit primaries at completion, with focused tests.
3. Integration: read/error/root-sharing coverage, native facade and partial
   composition programs, documentation and both handoffs after the complete gate.

Composition transfers already constructed/copied slots into the flattened result,
including its primary. Forwarded fields (`bind: false`) do not charge again;
additional named fields charge normally. Scalar/record reads, type queries and
list type extents allocate no slots. Cached initializer work is bootstrap-only.

Three ledger groups, six record-slot checker groups and native integration in both
profiles pass. The complete compiler gate passes with no outstanding failures.
Logs: `/tmp/meowy-slot-ledger.log`, `/tmp/meowy-record-slots.log`,
`/tmp/meowy-record-slots-regressions.log`, `/tmp/meowy-record-slot-integration.log`,
`/tmp/meowy-record-slot-gate.log`.
Next: audit remaining text/helper/root domains. Proof outcomes and required list
values stay gated.

### Retained record-read accounting

Audit: revision 1 charges reads of already available immutable inputs, not their
initializer traversal. `Input.work` and record ancestor evidence retain bootstrap
eligibility/error costs only. `required_record` is execution-only; `required_path`
also serves hints/composition checks and remains uncharged. The aggregate-slot
ledger now charges recursive record copies separately from read steps.

Completed dependency-ordered slices:

1. `c57282b`: charge record name/field execution and projection ancestors, with
   exact/grouped/repeated/cached-read, lookup isolation and root-limit tests.
2. Integration: ancestor-error precedence, skipped/type-query and native facade
   coverage; documentation and both handoffs refreshed after the complete gate.

Five checker groups and two native groups pass, including debug/release startup
and original-file errors. The complete compiler gate passes with no outstanding
failures. Logs: `/tmp/meowy-record-accounting-focused.log`,
`/tmp/meowy-record-accounting-integration.log`, `/tmp/meowy-record-accounting-gate.log`.
Record construction/copy slot accounting is now integrated; remaining work is
expression/text/helper domains and deferred-query roots. Proof outcomes stay gated.

### Scalar projection charging

Investigation: `required_path` and `required_field` serve both eligibility and
execution. Charging there would count form/hint walks repeatedly. Integer field
execution in `expressions.rs` and boolean field execution in `booleans.rs` own
ancestor charges, after their existing outer-node charge and before lookup.
Parentheses contribute no steps; each nested field and named root contributes one.
Retained Input.work remains a bootstrap guard, not a logical cost.

Dependency-ordered commits:

1. `5f716ec`: add a shared ancestor-charge traversal at the two execution sites, with focused
   exact/grouped/repeated-read, form isolation and budget-failure regressions.
2. The integration slice adds retained-error, type-query, static metadata and
   real import coverage, and refreshes both handoffs after the complete gate.

The shared traversal now charges each non-group ancestor sequentially at both
scalar execution sites. Three focused groups pass (local/module identity, exact
and grouped/repeated/skipped reads, lookup/runtime isolation and E220 cleanup).
Validation: `cargo test --manifest-path compiler/Cargo.toml --lib logical_projection`
and cargo fmt pass. Log: `/tmp/meowy-projection-focused.log`.
Implementation commit: `5f716ec`. Six focused library groups and one native
group now pass, including retained first-error spans, type-query non-evaluation,
static import metadata and real facade startup/widths in debug/release.
Log: `/tmp/meowy-projection-integration.log`. All ten full compiler gate checks pass; log:
`/tmp/meowy-projection-gate.log`. No outstanding failures remain. Record-read accounting now extends this work;
record-slot accounting is also integrated; other expression/text/helper domains
remain open.

### Logical integer charges

`a9d2c9e` adds `charge_integer` at raw_expression, guarded by required mode and an
active work root. It charges integer literal/name/outer-field and arithmetic nodes.
Grouping adds no charges. Immediate negative literals charge operator and literal
sequentially, preserving consumed work on failure and the signed-minimum rule.
Eligibility, hint/form walks and runtime folding are not charged as evaluation.

`623c451` adds charges for unary/binary nodes manually evaluated by integer_operand.
Fallback leaves still use the shared evaluator; scoped_output owns block charges.
Integer comparisons and list extents inside required roots reuse these paths.
Plain extents without a work root retain their existing behavior and remain outside
logical-root accounting. Legacy Input.work is not copied into logical counters.

All 129 required-evaluation tests passed after the shared hook; all 134 passed
after delegated operators. Ten focused integer groups and one native group pass:
exact/grouped costs, first-error order, repeated reads, negative literals, block
arithmetic, skipped comparisons, extents, root/depth/mode restoration, type-query
non-evaluation and imported widths/startup in debug/release. Logs:
`/tmp/meowy-integer-node-charges.log`, `/tmp/meowy-integer-block-charges.log`,
`/tmp/meowy-integer-charge-integration.log`. The full compiler gate passes.
No outstanding failures remain.

Remaining domains: other type-use roots, text/helper counters and
deferred descriptor execution accounting. Proof evaluation stays gated.

### Statement and boolean logical charges

`92a0dfc` charges one logical step in `type_statement` for each evaluated required
statement and in `scoped_output` for each evaluated type/scalar/record block.
Structural branch checks and skipped bodies do not spend these charges. Delegating
wrappers such as `scalar_block` and `inferred_block` do not count blocks again.

`5f0cace` charges evaluated non-group/non-block outer nodes in `required_boolean`.
Short-circuit selection determines which operands spend work. Block execution
owns its own charge; parentheses and form validation spend no logical steps.
Boolean equality reads each operand separately, and an input failure retains its
original span. Legacy Input.work still enforces bootstrap eligibility/bounds and
is not copied into logical step counters.

All 119 required-evaluation tests passed after statements; all 123 passed after
booleans. Nine accounting groups now cover exact counts, selected/skipped paths,
grouping, repeated reads, primitive/block equality, logical step limits, nested
failure cleanup and independent roots. Logs: `/tmp/meowy-statement-charges.log`,
`/tmp/meowy-boolean-charges.log`, `/tmp/meowy-charge-integration.log`.
The complete compiler gate passes. No outstanding failures remain.

Integer evaluation now extends these charges. Remaining domains include other
type-expression dispatch, text/helper charges and deferred descriptor
execution accounting. Proof evaluation stays gated.

### Logical type accounting

`5d03d9b` extracts `type_values/work.rs` and centralizes required-root lifetimes.
The same 111 baseline tests plus a nested-sharing/cleanup regression passed.
`cecf7c6` adds `check/required.rs::Budget`, separate from bootstrap Work counts.
Each successful type node materialization charges one logical step and one type
node. Nested roots keep the outer source span; independent roots reset counters.
Charges are atomic, overflow-safe and sticky on E220. Failed roots cannot start
more nested evaluation, and outer completion/failure discards their state.

The ledger uses the revision 1 step/type limits (1,000,000 and 65,536). Source
programs still encounter lower bootstrap limits first; those remain B001. Logical
boundary tests inject consumed budgets internally and are not source-level E220
qualification. No query outcome is evaluated.

The ledger field required default initialization in nine existing test modules;
all fourteen affected files had to change together for a buildable struct update.
That validated slice contained 203 changed lines. The integration exercises real
metatype bindings, repeated/nested construction, skipped constructors and retained
original E107 spans. All 223 checker tests passed after the ledger; ten focused
logical tests pass after integration. Logs: `/tmp/meowy-logical-type-ledger.log`,
`/tmp/meowy-logical-root-integration.log`. The complete compiler gate passes;
no outstanding failures remain. Span is now imported directly by the legacy test
module that uses it, so production and test lint checks both pass.

Statement/block and boolean charging now extend this ledger. Remaining logical
domains must not copy bootstrap traversal counts; other type-expression dispatch,
text/helper counters and deferred descriptor
execution accounting are still open. Preserve B001 infrastructure limits and keep proof evaluation gated.

### Deferred copy-query integration

`6acc657` retains explicit type-call suffixes as `ExprKind::Specialize`, including
ordered type arguments and original spans. It preserves ascription/query/reference
contexts and the 64-argument/256-tree-depth bootstrap guards. Generic type binders,
nested generic types, value arguments and explicit union arguments remain gated.
This parser slice needed nine files because AST visitors and both affected existing
parser test groups had to change together; 28 parser tests passed.

`72e7483` connects resolved `Item::CanCopy` identity to `Value::Pending` and
`check/queries.rs::Query`. Calls retain a checked type argument, call span, owner,
target and revision. Copies share their original queue index; no active outcome is
invented. Immutable locals accept the fixed Result annotation; direct `result<>`
type aliases return `Spec::Descriptor(Result)`. Mutable/runtime storage, truthiness,
formatting and captures remain E223. Narrowed pending annotations are E207.

The pending-evaluation B001 gate runs after ordinary typing and borrow/loan checks.
It also applies to uncalled functions and runtime-skipped checked bodies. Every
pending-query program is rejected before code generation. Required-block query
construction, flags, assertions and general generic execution remain unavailable.
A 4096-call queue bound is bootstrap capacity, not E220 logical accounting; copies
add no entries. Independent calls retain distinct original locations.

`b620759` preserves the reference's distinction between unsupported static metadata
exports (B001) and invalid runtime-typed exports/record storage (E223).

The final integration keeps type-argument imports visible to graph discovery,
reports owning-file/local spans, and prevents check/build/run from starting the
program. Documentation passes the declaration parent through specialized callees,
so inline type fields and computed bindings retain their attachments and spans.
Six pending checker groups, three native groups in both profiles and the
additional documentation regression pass. Logs: `/tmp/meowy-pending-query-export-gates.log`,
`/tmp/meowy-type-call-doc-tests.log`. `4aa3ea3` retains record type arguments so
invalid query arity reaches E212. `b7cfeb0` keeps known flag projections B001 and
unknown fields E201. Their focused tests and the complete final gate pass.
No outstanding failures.

Next: complete logical required-root accounting and proof-dependency tracking. Pending
origins are ready, but evaluated results, canonical combined origin sets and
outcomes must remain gated until those prerequisites are connected.

### Descriptor identity slice

Step 2 starts with a separate static type namespace representation, not a new
`hir::Type` variant: `Spec::Descriptor` retains opaque `proof.Always`, `Never`,
`Indeterminable`, `Result` and `Flags` identities through type aliases. Runtime
storage conversion reports E223. First-class descriptor type values, helpers and
explicit descriptor union construction remain B001 until their metadata evaluator
exists. No active result or observation origin is constructed in this slice.

Commit order: (1) nominal type resolution/storage boundaries with checker tests;
(2) native file-alias/privacy and documentation integration, full compiler gate.
Preserve known-good revision metadata. Next within step 2: result values with fixed
declared type, active alternative and retained origins, before public construction.
Nominal identity/storage boundaries pass five proof checker groups and the two
revision native groups. Required descriptor aliases charge one bootstrap type node
and restore lexical scope. No outstanding failures. Log:
`/tmp/meowy-proof-descriptor-tests.log`. Native facade/privacy/documentation
integration and the full compiler gate pass. Evaluated result values remain
unimplemented.
Identity commit: `2f09968`. Native facade/privacy and rendered documentation
regressions pass. Five proof checker groups and four native groups are now covered;
log: `/tmp/meowy-proof-descriptor-integration.log`. No outstanding failures.
The alias guide passes checking in both profiles. Pending query metadata now
extends this representation; evaluated results remain gated.

### Dependency-ordered commit series

Each numbered item is a review concern, not permission for an oversized commit.
Split integration further if it exceeds the repository review threshold. Keep
source-visible queries unsupported until their prerequisite checks are connected.

1. Add partial module identity and typed `proof.revision` (`uint32`, value 1).
   Reuse static scalar resolution. Test import/member aliases, lexical shadowing,
   exact queried width, required reads and no module initialization effects.
   Query members remain B001. This is package metadata, not an executable proof.
2. Add internal nominal descriptor identity, fixed signatures and retained origins.
   Cover Result versus active alternative, metadata copying, forbidden truthiness,
   forgery and runtime escape (E223), with no HIR runtime storage. Keep public
   construction gated until obligation evaluation is ready.
3. Implement logical required-root accounting independently of bootstrap guards.
   Test exact/below/above limits, nested root sharing, independent roots, restored
   state after errors and identical cached/uncached charges. E220 must name root,
   counter and limit; include aggregate/origin materialization, not just visits.
4. Add deferred obligations after ordinary checking and proof-dependency propagation.
   Keep fixed query signatures usable before answers exist. Cover copied/arithmetic
   flags, controlled query availability, type extents and both runtime branches;
   E225 must precede affected query evaluation. Preserve original ordinary errors.
5. Add bounded type-only `can_copy<T>()` classification and charged origin creation.
   Connect only after steps 2-4. Cover integers, shared/exclusive references,
   supported aggregates, never and metadata types; audit nominal capabilities.
   Reject bad arity with E212, malformed types with their original error, and
   unsupported place/generic forms with B001. Do not fabricate Indeterminable.
6. Add direct result flags, `assert` and `expect<S>` with diagnostic evidence.
   Test A/N outcomes through source, I assertion behavior with internal descriptors
   until an I-producing query exists, invalid expectation types (E223), mismatch
   (E224), fixed type queries and failures in uncalled/runtime-skipped bodies.
   Add source-note rendering separately first if needed for multi-file origins.
7. Integrate aliases, file facades/privacy, repeated roots and debug/release native
   checks; compare runtime behavior and HIR with unused queries removed. Add the
   supported-subset guide, run `python3 -B tools/verify.py --compiler`, and update
   both handoffs. Unsupported proof fixtures do not count as qualification passes.

Stop condition: type-only copy queries can be checked, inspected and asserted with
correct phase/dependency/budget behavior and no runtime query effects. Broader
profile 1 value/ownership analysis requires its own frozen graph and canonical
transfer plan; existing optimizer or borrow answers are not substitutes.

Step 1 metadata is complete in two slices: `9ddefca` adds module/static scalar
identity; `ae7fde7` adds direct required member reads and structural checks.
Immutable unannotated aliases remain static; annotated/mutable scalar
bindings can materialize runtime values. HIR tests verify unused aliases create no
statements, functions or locals. Named descriptor types are recognized; query
evaluation remains B001.

Three checker groups and two native groups pass, including debug/release, exact
widths, skipped failures, lexical shadowing, function-local required reads and
facade startup order. Static reads carry no initializer work beyond the existing
expression visit. No outstanding focused failures. Logs:
`/tmp/meowy-proof-metadata-tests.log`, `/tmp/meowy-proof-required-tests.log`.
The full compiler gate passes. The [foundation guide](docs/FOUNDATION.md#proof-revision-metadata)
documents the subset; its example prints `1` and `7` in debug/release.
Pending query metadata now extends this completed slice; evaluated results remain gated.

Planning validation: source/reference inspection and all four default
`python3 -B tools/verify.py` checks pass (16 tooling tests, local links, catalog and
schemas). Log: `/tmp/meowy-proof-plan-checks.log`. No proof example was executed;
this planning-only run did not execute compiler/runtime behavior. The later
metadata implementation has its own compiler gate evidence below.

## Type subtraction series

Dependency-ordered slices:

1. `bc3030c`: suffix parsing for annotations/expressions, original spans, chain bounds
   and existing unary/operator behavior. Evaluation remained gated in that commit.
2. `ae68e3b`: normalized concrete subtraction, required binding/equality integration
   and checker/native kind, query, empty-set and runtime-value regressions.
3. `e5579d3`: imports, source order, repeated work, skips, budgets, scope, startup
   and runtime/helper boundary coverage.
4. This documentation commit integrates the guide/README and both handoffs. Guide
   execution and the full `python3 -B tools/verify.py --compiler` gate pass.

Seven library/five native focused groups, fmt and Clippy pass. No implementation
failures remain. Logs: `/tmp/meowy-subtraction-parser.log`,
`/tmp/meowy-subtraction-evaluator.log`, `/tmp/meowy-subtraction-integration.log`.
No outstanding failures. Proof revision metadata follows this completed series.

## Current compiler slice

`!<U>` subtracts normalized supported alternatives in annotations, aliases, metatype
bindings and required expressions. `parser/types.rs` reuses `ExprKind::Binary("!")`,
wrapped by `TypeKind::Computed` in annotations; existing AST visitors retain imports,
documentation traversal and original spans. Expression suffixes bind before equality;
repeated subtraction is left-to-right. Unary `!`, `!=` and existing context rules stay
intact. Each suffix takes one bracketed type. Named/computed union operands work;
adjacent union suffixes after subtraction remain B001 pending a mixed-precedence
contract. Annotation chains cap at 64 operations; evaluator depth can stop them earlier.

`type_values/subtraction.rs` checks bounded operand forms and infers bare type blocks.
`type_value_inner` evaluates operands once in source order and uses `Type::subtract`.
Empty left sets still evaluate the right; absent removals preserve identity. Known
non-type operands report E222, inferred scalar block results E207. Each operand/result
materializes against the node budget; repeated constructor inputs retain full work.
A primitive lexical-type subtraction costs three visits/three nodes. Failed budgets
restore state; independent roots reset counters. Short-circuits skip construction
while preserving known operand/form checks. Operand-local declarations do not escape.

Imports in removed operands remain discovered and initialized, even for an empty
source set. Facades retain privacy/identity and source failures keep original paths
and spans. Subtraction cannot validate or convert nullable runtime data; ordinary
assignment checks still reject unproven narrowing. Literal subtypes, broad bases
such as `core.error`, full E209 representability, helpers and runtime type storage
remain outside this subset.

Required `==`/`!=` accepts bare type-producing blocks, including grouped forms and
block/type-value pairs in either order. `type_comparison_form` defers such pairs to
`block_equality_form`; structural checks do not infer skipped block results.
`equality_operand` reuses `operand_block`/`inferred_block` and accepts `Value::Type`
when no scalar context is required. Both type payloads compare by normalized identity.

Known integer/boolean operands retain contextual widths and result kinds. Selected
mixed type/scalar results report E207; scalar blocks emitting known type values also
report E207 after checking the constructor. Original constructor errors remain intact.
Direct known type/scalar or ordered type comparisons retain E222; runtime type values,
record-value equality in required blocks, helpers and first-class metatypes remain gated.

Both operands evaluate once in source order, including selected tails. First failures
stop the right operand and preserve original source paths/spans through facades.
Each inferred type block materializes its payload against the shared node budget;
input reads retain their full work on every read. Two grouped primitive type blocks
cost 11 visits/four nodes; a literal/block pair costs seven visits/three nodes. Failed
budgets restore scope/depth, independent roots reset budgets and skipped operands
spend no evaluation work/nodes. Skipped forms still check supported statements and
outer lexical/member lookup while deferring block-local names, types and result kinds.

Literals, supported queries, aliases and core/foundation/file-module type members
retain normalized `hir::Type` identity: record fields/mutability/primary, union sets,
widths, list capacities, reference modes and nominal identities. No runtime locals
are created, and native module initialization order/privacy are preserved.

Named immutable type-value exports are implemented in `exports::export_type_value`.
Unconditional top-level metatype-annotated emissions start/join bounded required
roots and register only `Value::Type` in lexical/module maps. No runtime field,
emission, local or scalar input is created. Function/data exports retain their paths.

`Module.values` and `module_member` preserve payload identity through imports,
ordinary aliases, explicit value re-exports and type-namespace aliases. The type/value
namespaces remain separate. Metatype aliases use lexical lookup; private names stay
private. Runtime composition does not forward compile-time exports. Mutable, nested,
labeled, conditional and unannotated type-value emissions remain gated.

Each initializer starts a fresh root unless required evaluation is already active.
Nested work shares budgets; eligible scalar input failures keep original source spans
through facades and tails. Runtime module initialization remains once-only and ordered;
panics still stop dependent modules and entry execution. Imported type payloads can
construct function-local types without granting runtime captures.

Documentation emission metadata now retains the annotation location, so named type
values display core.Type while aliases display the concrete data type. The
[guide](docs/COMPUTED_TYPES.md#named-type-value-exports) covers syntax and boundaries.
Commits: `93ef8d1` (exports), `6f9cd05` (facades), `bf76f46` (integration).

Ordinary immutable metatype bindings also work at module/function scope. They remain
private without explicit exports and have no runtime storage. Runtime branch
reachability does not skip required roots; skipped required matcher bodies keep their
prior semantics. First-class metatype values, runtime type containers and type-producing
helpers remain separate. Proof queries remain specification-only.

`Module.primary` retains an emission ID and typed `Primary::Int`/`Primary::Bool`
evidence. `primary_input` captures eligible direct unconditional emissions;
`Primary::forward` adds two visits for each module composition. Integer and boolean
lookups enforce the checked primary kind. Predicate traversal reads scalar module
locals and projected mixed-module primaries, preserving value/error/work evidence.
Integer required contexts, runtime HIR and ordinary scope/ownership rules are unchanged.

Boolean operations project mixed-module primaries. Identity aliases/type queries keep
named fields; comparing two complete module records does not project their primaries.
Annotated ordinary identity bindings remain gated. Boolean copies and primary/named
re-exports retain source errors and work. A checked outer integer initializer may
supply required types inside functions; this does not enable runtime module captures.

Direct named boolean exports retain checked local IDs in `Module.inputs` and evidence
in `Checker.bool_inputs`. `boolean_field_input` resolves empty paths from scalar
boolean evidence and nonempty paths from typed record evidence. Eligibility remains
per initializer: unrelated file/named-export effects do not invalidate a pure primary.
Private dependencies remain private; runtime initialization is neither run nor removed
by checking. Initializer failures still stop dependent and entry execution.

`inputs/blocks.rs::scalar_binding` shares immutable integer, boolean and bounded record
evidence between `Block<i128>` and `Block<bool>`. Declared record kinds call `record_expr`
with existing caller depth/count counters and store whole evidence in `Sources.records`.
Only work and errors enter the enclosing result; scratch records never replace scalar
primaries. Aliases, projections, compositions and unused tails retain complete ancestor
evidence. Branch-local record shadowing restores the prior scope.

Typed statement traversal lives in `inputs/blocks/{integers,booleans}.rs`. Eligible
matcher branches share primary/error/work state and restore branch-local bindings.
Conditions retain their evaluated work even when false; skipped bodies add none.
The first evaluated failure stops traversal, clears the value and keeps its original
E107 span through copies and projections. E207/E205/E204/E201 type/primary/scope checks
and ordinary runtime HIR, initialization, flow and ownership analysis are unchanged.

`Input<T>` retains value, error and transitive work separately from runtime folding.
`Checker.inputs` and `Checker.bool_inputs` retain ordinary declarations; `Sources`
provides scoped initializer evidence. Known declared noninteger bindings never become
integer value inputs. Inferred `never` retains existing error-only behavior; nested
unreachable boolean blocks need explicit boolean types when inference loses their kind.

`inputs/predicates.rs` proves boolean literals/locals, negation, short-circuit logic,
boolean equality/inequality and exact-width integer comparisons. Runtime parameters, mutable or effectful initializers
and evaluated helpers remain unavailable. Required reads across function scopes do
not grant ordinary runtime captures. Repeated field comparisons remain unstable flow
atoms; an immutable boolean binding can supply complementary matcher arms.

Boolean `==`/`!=` evaluates eligible operands left-to-right and adds each operand's
complete work, even for identical cached sources. Only a failing left operand stops
the right read; a true/false value does not. Logical `&&`/`||` retain short circuiting.
The first error keeps its source span without inventing a result. Boolean inputs in
scalar/record initializers reuse this proof; ordinary runtime HIR and typing are unchanged.

`records.rs::Leaf` distinguishes optional integer and boolean values, including
error-only paths. `Record::field` and `Record::boolean` reject the wrong leaf kind;
boolean leaves are never encoded as integers. Record accumulation preserves these
kinds through copies, subrecord projection, composition and failed initialization.
`boolean_field_input` resolves boolean record paths with complete ancestor work/errors.
It supports local records, record-derived exported paths and direct named boolean
module exports. Scalar scratch and equality/branch predicates reuse that evidence.

Record initializers retain unit primaries and immutable integer/boolean/record fields. Complete
ancestor work/errors survive field paths, subrecord aliases and local compositions.
`inputs/records/build.rs` accumulates selected statements; `Record::failed` keeps
error-only paths without inventing branch values. Evaluated unsupported siblings or
tails prevent eligibility for every selected leaf; unrelated file initialization does not.

Direct module compositions forward independently eligible named inputs and integer/boolean
primaries. Local-record compositions export a checked record ID and bounded field path;
`inputs/records/paths.rs::input_path` retains paths/work through facades. Synthetic
module namespaces never gain whole-record eligibility. Mixed integer primaries need
integer context in required scratch/arithmetic; ordinary aliases keep record identity.
Identity-only module aliases/imports emit no runtime binding and may read eligible
exports inside scalar initializers. Whole-module record construction remains gated.
Privacy, collisions, widths, startup order and borrowed-export gates remain intact.

Every required read charges retained work again; independent roots reset their budget.
Scalar blocks/predicates share 64 active levels and 4096 visits. Records retain 256 total
fields and 32 record/evidence levels; type traversal retains 16384 nodes. Frontend and
ownership limits remain independent; these bootstrap limits are not language E220 counters.

Unsupported record shapes, selected standalone expression blocks, named/outer emissions,
mutable scratch and helpers remain unavailable inside scalar initializers. Float/text
comparisons and conditional module exports remain separate. See [COMPUTED_TYPES.md](docs/COMPUTED_TYPES.md#block-initializers).

## Actual validation

- Forward-group completion passes all ten compiler checks: 1960 library/913
  native tests, 32 tooling and 30 harness groups. Conformance: 100 passed,
  19 pinned gaps, zero failures in debug/release; `/tmp/meowy-forward-groups-gate.log`.
  Strict mode rejects only those gaps; `/tmp/meowy-forward-groups-strict.log`.
  Prior fixtures/expectations and reference contracts are unchanged;
  `/tmp/meowy-forward-groups-preservation.log`. Exported forward definitions remain
  a separately verified acceptance gap. All four final documentation checks pass;
  `/tmp/meowy-forward-groups-docs.log`.

- Runtime-panic conformance passed all ten compiler checks: 1890 library/913 native,
  32 tooling and 30 harness groups. Conformance: 69 passed, 19 pinned gaps, zero
  failures in debug/release; `/tmp/meowy-runtime-panic-conformance-gate.log`.
  Strict mode correctly rejects known gaps; `/tmp/meowy-runtime-panic-strict.log`.
  Prior fixtures, compiler source and reference contracts are unchanged.

- Companion conformance: all ten compiler checks pass, including 1890 library/913
  native tests, 29 tooling and 22 harness groups. Conformance: 55 passed, 19 pinned
  gaps, zero failures in both profiles; `/tmp/meowy-companion-conformance-gate.log`.
  Strict mode correctly rejects those gaps; `/tmp/meowy-companion-strict.log`.
  Prior fixture/reference bytes and capability exceptions are preserved.

- Coverage audit: all ten compiler checks pass, including 1890 library/913 native,
  28 tooling and 11 harness test groups. Conformance: 45 passed, 19 pinned gaps,
  zero failures in both profiles; `/tmp/meowy-conformance-coverage-gate.log`.
  Strict mode correctly rejects those gaps; `/tmp/meowy-conformance-strict.log`.
  All 37 reference documents and 33 proof obligations have checked evidence/gap
  records. This measures traceability, not complete language or release coverage.

- Indirect-store effects passed all ten checks in
  `python3 -B tools/verify.py --compiler`: 1890 library/913 native tests, formatting,
  Clippy, build and conformance (10 passed, 13 unsupported, 0 failed in debug/release).
  Log: `/tmp/meowy-indirect-effects-gate.log`. Other erased-statement connectivity,
  callee effects, precise write locations and propagation stay pending.
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

## Prior capabilities and other areas

Carried reference-free lists retain whole initialized length/payload through inner
restarts. Shape limits remain 256 parts/32 levels; list construction retains capacity
65,536, layout 1 MiB and one-based initialized-length checks. Nullable/union/reference-
bearing/foundation/owning and top-level unit carried slots retain their gates.

Shared whole-list views, nested element/record-field projections and reborrows
retain canonical Slot/Field/Element sources and parent reference identity. Index
expressions execute once and check current initialized length. Views can survive
inner restarts and alias scope exit while their result owner lives; owner expiry
cannot be undone by reinitializing the same physical site.

Selected-slot mutability is independent of whole-binding replacement. `Proofs.mutable`
tracks replaceable roots and `Proofs.fields` mutable owned descendants; `variable`
drives snapshots/refinements without granting writes through shared references.
Pointer syntax uses tight `&`/`&!`/`*`, immediate-field `.&`/`.&!`/`.*` and grouping
for complete targets.

Standalone documentation supports attachment, checked links/signatures, E801-E805,
`doc check`/`doc build`, local API pages and checked/opt-in examples. Net/HTTP/TLS
remain specified library work; module/type/I/O/task foundations and capability-typed
lifecycle implementation precede executable adapters.

## Architecture and proof boundaries

Graph loading/discovery remains in `src/modules/{load,discover}.rs`; snapshots and
compilation are in `src/modules.rs`. Literal imports cover the supported AST,
retain source order and initialize dependencies once before the entry. Canonical
paths, manifest boundaries, depth/file/edge/source/discovery budgets remain enforced.
`src/check/exports.rs` owns file export identity, privacy and signatures.
`src/check.rs::check_imports` retains complete graph ownership checking.
`src/driver.rs` protects graph inputs from output replacement and maps diagnostics.
Native file sites remain in `src/backend/sites.rs` and `native/runtime.cpp`.

Type-value resolution lives in `src/check/type_values.rs`; bootstrap work and root
lifetimes are in `type_values/work.rs`, with logical type charges in `required.rs`.
Ordinary type construction and symbol lookup remain in `src/check/names.rs`. Required list extents
still use `src/list.rs::list_extent` and scalar checks in `src/check/scalars.rs`.

Static integer validation/folding is in `src/check/type_values/scalars.rs`; `Value::Static`
in `src/check.rs` carries exact types into expression hints and documentation.

Immutable initializer evidence is in `src/check/inputs.rs`, recorded by ordinary
binding checking in `src/check/statements.rs` and consumed by scalar required reads.

Shared scalar block state lives in `src/check/inputs/blocks.rs`; typed traversal lives
in `src/check/inputs/blocks/{integers,booleans}.rs`. Required-only constant
materialization is in `src/check/expressions.rs`. Runtime constant folding remains separate.

Whole-record evidence is in `src/check/inputs/records.rs`; direct required field lookup
is in `src/check/type_values/fields.rs`. Native coverage is `tests/native/computed_fields.rs`.

Nested paths/subrecord evidence are in `src/check/inputs/records/paths.rs`.
`Sources` in `src/check/inputs.rs` carries scoped integer, boolean and record inputs.

## Still outside this compiler

Whole-record module inputs, conditional module exports, helper
initializer eligibility, module-data captures, borrowed module storage, package/manifest
resolution, full required evaluation and generic specialization, public FFI, wider
ownership/cleanup, executable networking, public artifacts/replay and LSP remain separate. Host execution does not qualify minimum
platforms or bundled distributions. That earlier validation used Rust 1.98.1 and
LLVM/Clang/LLD/LLVM ar 22.1.8; the current host toolchain is recorded above.

## Next steps

Explicit ascriptions, uniform type predicates and the four bit functions are
complete; their remaining bootstrap limits are documented above. Bounded type
subtraction retains its documented limits. Never operand contexts are repaired
above. Explicit group and observed non-projecting Forward consumer resolution are
implemented, including unchanged observed narrowing and bounded local eligibility.
Exact Bind qualification and bounded ordinary initializer indexing are complete.
Validated local-read forwarding and coercion-owned primary consumer links are
complete, including output- and contextual-list-owned primary parts and composed-emission
source slots and unchanged observed ascriptions. Field Operation/Normal observations
are now independent, and bounded result-candidate inputs retain qualified composed
source slots. Candidate traversal and observed field-result source links are
implemented, including direct-input lookup through unchanged narrowing and a
separate expanded field-source forest. Explicit-group direct-source resolution is
complete. Observed non-projecting Forward coercions are next, following the ordered
plan above; value selection and broader field-value provenance remain separate.

1. Extend `check/dependencies.rs`, alias/storage tracking and function checking:
   direct local and owned-path writes now retain conservative whole-owner marks.
   Emitted-slot aliases share marks through `Alias::root`; immutable scalar
   references retain known owners and indirect stores propagate marks or gate
   unknown dependency-bearing origins with B001. Mutable scalar-reference
   retargeting now retains bounded conservative owner sets and completeness.
   Named scalar-reference emissions now share bounded origins at their canonical
   slot root. Flat ordinary record bindings now retain scalar-reference field
   origins through direct blocks/copies and local field projections. Whole-record
   replacement and direct mutable reference-field writes now merge those origins.
   Nested construction, named-emission snapshots, record/subrecord copies and
   scalar/subrecord writes now share bounded ordered paths. Composition now retains
   temporary snapshots and maps field names to source paths. Nullable wrapping/
   narrowing of a single record shape retains those paths; known null has no owners.
   Shared element borrows now retain container origins through reference-free
   list/record views and reborrows, including aggregate-stored views and supported
   mutable record-view fields. Temporary storage now retains its existing ID and
   initializer/control marks without extending lifetime. Direct temporary-carrier
   copies now recover snapshotted reference/record pointees, including transparent
   reborrows. Immutable one-level aliases to named reference cells now retain
   root/field locations and recover stored pointee origins. Mutable one-level
   carriers now retain bounded location sets/completeness through retargets and
   preserve prior copies. Bounded deeper named/temporary chains now resolve cell
   layers and preserve dependency marks with cycle-safe traversal. Lexical emitted
   carriers now register and share canonical cell sets through sibling aliases.
   Completed-record carrier fields now retain bounded `Cells` snapshots through
   construction/copies, composition, nullable wrappers and field/subrecord updates.
   Projection reads and deeper cell expansion use the same path metadata. Next
   extend broader reference-bearing aggregates,
   heterogeneous record unions and broader returned origins while
   preserving explicit incomplete sets. Scalar-reference return origins now reuse
   compatible argument candidates from the existing borrow contract; this does not
   provide callee effect/data/control summaries. Shared returned scalar/list/record
   views now also reuse general-contract field/element projections when referenced
   components have no borrowed values. Concrete by-value record arguments now
   retain nested named shared-reference fields, including nullable record wrappers
   and nested nullable fields. Known null contributes no owners; unknown matching
   fields remain incomplete. One-level shared carrier arguments now resolve stored
   view owners through named/temporary cells and record-stored carrier snapshots.
   Deeper shared chains now use bounded type/cell-layer traversal with complete
   known owner sets and preserved unknown alternatives. Storage locations for
   references to borrowed records now survive aliases, copies, retargets and
   record-stored views; dependency reads traverse addressed field prefixes. Direct
   shared borrowed-record arguments now match nested stored shared-reference fields
   and owned-field projections using those locations. Carrier-valued fields now
   resolve through bounded shared cell expansion, including nested fields and
   retargeted/unknown alternatives. Shared chains ending in concrete borrowed
   records now expand to record locations before matching stored/owned candidates.
   Nested borrowed-record view fields now use a shared bounded type/location
   worklist, including empty/unknown locations and mixed candidate owners. Shared
   references to nullable single-record targets now reuse those locations; known
   null contributes no stored owners and heterogeneous layouts remain incomplete.
   Returned shared chains now retain exact and compatible inner argument cells
   through copies and nested calls, preserving unknown alternatives and bounded
   type/cell traversal. By-value record arguments now contribute nested/nullable
   stored carrier candidates through bounded field traversal and existing snapshots.
   Direct borrowed-record arguments now contribute owned reference-cell projections
   and stored carriers through bounded nested/nullable field traversal. Shared
   chains ending in borrowed-record arguments now expand those locations before
   returned-cell matching. Nested borrowed-view fields now use a shared bounded
   type/location worklist, including unknown/empty descendants and nullable views.
   Direct shared concrete-record results now retain exact-compatible argument
   locations through copies/nested calls; known locations do not imply known
   contents. By-value nested/nullable record containers now supply stored exact
   view candidates through existing cell snapshots. Shared input chains now expand
   to exact-compatible record-view locations, including record-stored chains.
   Returned concrete-record views now also retain projected/stored candidates from
   borrowed-record arguments through the bounded location worklist. Nullable
   single-record result targets now retain their actual storage locations, even
   when null, without inventing contained owners. Returned shared carrier chains
   ending in borrowed records now retain matching intermediate locations before
   terminal expansion, including stored/projected candidates. Direct
   record-valued calls now retain ordinary shared-reference field origins through
   public argument matching, including nested calls and copies. Carrier-cell fields
   now retain bounded snapshots too, including record views and direct carrier-field
   access on calls. Bounded owned-field paths now preserve direct reference reads
   and subrecord call snapshots, including composition and carrier fields.
   Shape-preserving record-call coercions now share that bounded walk, including
   nullable wrapping/narrowing and carrier fields. Unknown and null remain distinct.
   Path discovery now retains bounded shape selections at each heterogeneous
   union boundary in `records/paths.rs`, including unsupported alternatives.
   The positional adapter intentionally excludes qualified paths. Bounded owned
   shape keys and origin/carrier snapshots now exist alongside positional metadata;
   local narrowing reads select exact keys. Immutable bindings with immutable
   fields now capture root record-to-union widening, known null and exact union
   copies. Whole-container/prefix traversal follows shaped origins and carrier
   locations, so later marks reach copies, guards and pending query control.
   Nested immutable union fields now capture checked binding alternatives, including
   nullable fields and conditional sources. Subrecord projections preserve field
   prefixes and outer/inner shape selections. Unknown alternatives keep snapshots
   incomplete. Composition temporaries now capture immutable shapes;
   block reads remap source field names/indices and merge their snapshots with
   direct alternatives. Unknown inputs stay incomplete. Immutable named emissions
   now capture snapshots before alias registration; siblings merge at `Alias::root`
   and shaped reads/dependency traversal use that canonical storage. Ordinary copies
   keep prior snapshots. Ordinary mutable bindings with immutable fields now capture
   initial shapes and merge whole-value replacements, retaining old/new owners and
   incomplete alternatives without replaying RHS work. Self-assignment reads prior
   metadata; failed shape merges preserve it. Mutable named slots with immutable
   contents now capture initial snapshots and merge retargets at `Alias::root`;
   siblings see the shared owners while prior copies remain independent. Completed
   mutable fields and mutable descendants now capture final canonical slot snapshots.
   Owned concrete-record field/subrecord writes merge bounded qualified prefixes,
   preserving siblings and prior copies; whole-value replacement retains mutable
   record shapes too. Union-interior writes retain the ordinary concrete-storage
   B001 gate. Statement-owned temporaries now capture shaped snapshots; direct
   dereferences and reborrows recover explicit storage IDs and concrete field paths,
   preserving shape offsets and E303 expiry. Named shared record/null-union views
   now retain bounded storage locations; shaped dereference reads merge exact
   snapshots across valid concrete prefixes and retain unknown alternatives.
   Shared carrier chains now retain those locations through bounded shared-only
   classification and existing cell expansion, including named/stored chains,
   retargets and prior copies. Returned carriers with at least two shared layers
   now match supported record/null-union terminals through the public contract,
   including direct/deeper inputs and supported stored/projected record fields.
   Unknown candidates and untraversed unmatched union inputs remain incomplete.
   Direct returned shared record/null-union views now match exact direct/deeper
   and stored shared inputs, preserving unknowns and unmatched-input guards.
   Exact owned-union projections from borrowed records now preserve concrete
   owner paths alongside stored candidates. The walker skips only an owned union
   identical to the direct shared result target after public projection matching;
   unmatched owned unions use the bounded discovery path below.
   Hidden exact/deeper shared terminal candidates in owned record/null-union
   fields now resolve through variant-qualified snapshots. Unknown leaves remain
   incomplete, and owned targets inside variants or borrowed contents needing
   further traversal remain unsupported. Direct shared-union input support is below.
   Concrete-record reference-field origins now resolve through known shared view
   locations when direct storage lookup is unavailable. Returned calls, hidden union
   candidates, nullable/nested records and copies retain all stored origins; unknown
   owners remain incomplete. Concrete prefix/type checks prevent layout mixing.
   Shared carrier-field contents now resolve through the same concrete owner/path
   validation, retaining stored cells across returned views, copies, nullable records
   and hidden candidates. Unknown owners or contents remain incomplete.
   Direct one-layer unmatched shared-union arguments now reuse bounded variant-key
   discovery and snapshot resolution for concrete-record, union-view and carrier
   results. Unknown owners/contents remain incomplete; exact terminals retain
   their existing matching path. Record-result prerequisite: `f5d001a`.
   Deeper direct shared-union inputs now use bounded shared-terminal recognition
   and cell-layer expansion before variant-aware matching. Direct/deeper mixtures,
   unknown intermediates and union/carrier outputs retain all known candidates;
   exclusive edges remain unsupported and prior record-depth diagnostics are kept.
   Union inputs stored in owned record arguments now resolve selected concrete
   field cells before shared-layer expansion and variant-aware matching. Nested
   fields, mixed direct/stored candidates, unknown contents and both result paths
   are covered; no union variant indices become ordinary storage paths.
   Borrowed-record union fields now resolve concrete field addresses, read their
   stored references and expand remaining shared layers through the same location
   helper. Nullable/unknown owners and fields preserve completeness; union/carrier
   result reads retain all supported hidden and direct candidates. Location helper:
   `40fc5e0`.
   Typed borrowed-record continuations now resolve their variant-qualified
   snapshots, expand shared layers and traverse concrete fields. Cumulative depth
   spans union-to-record transitions; the existing flow ledger bounds total work.
   Unknown owners/contents retain known alternatives without becoming complete.
   Bound prerequisite: `4c0b67b`. Nested transitions, nullable contents and lifetimes
   are covered; exclusive edges and variant-address projections remain gated.
   Typed continuations now include supported heterogeneous record/null-union
   terminals, reusing exact variant snapshots and cumulative traversal bounds.
   Differing layouts, unknown/null contents and deeper reference chains preserve
   all supported candidates; mixed unsupported members and exclusive edges stay gated.
   By-value union calls now use bounded exact-variant result-leaf lookup and
   public-contract matching for shared references with reference-free pointees.
   Copies and nested result paths retain variant-specific/all/unknown origins;
   no runtime branch or callee body is selected. Type lookup prerequisite: `b50f87a`.
   By-value union carrier leaves now retain public-contract cell locations,
   including deeper references and shared record/union views. Origins and cells
   keep independent completeness; unknown inputs retain known alternatives.
   By-value union arguments now contribute shared-reference origins through
   exact variant-qualified snapshots, including nested owned fields, copies and
   inline union call wrappers. Borrowed primary values and carrier leaves remain
   unsupported; null variants contribute no origins and unknowns retain known roots.
   Shared carrier leaves in by-value union arguments now expand exact cell
   snapshots through `call_stored_origins` to reference-free terminal origins.
   Mixed/deeper/null/unknown inputs and compatible record projections retain known
   roots; borrowed record/union terminal contents remain incomplete.
   Borrowed concrete/nullable-record terminal contents in by-value union arguments
   now use location-based record traversal from exact cell snapshots. Structural
   depth includes the enclosing variant path; all/unknown contents and owned-field
   projections preserve known roots. Traversal prerequisite: `1482d20`.
   Borrowed heterogeneous record/null-union terminal contents now resolve typed
   leaves from location-backed snapshots. Record/union transitions share cumulative
   structural depth and the work ledger; exact keys keep differing layouts apart.
   Null/unknown contents retain known roots conservatively. Discovery prerequisite:
   `c9c5325`.
   Direct/deeper/record-stored borrowed-union origin arguments now enter the
   same bounded location traversal through `call_origin_view`. Caller-depth
   prerequisite `d600bb6` prevents input location lookup from restarting the call
   budget. Exact variants and unknown intermediates retain conservative origins.
   By-value union arguments now contribute returned cell locations through exact
   expression-backed variant snapshots, including concrete owned prefixes, record/
   union views, deeper carriers and typed continuations. Resolver prerequisite:
   `1e97d1d`. Unknown candidates preserve incompleteness; no variant positions are
   manufactured as ordinary storage paths.
   Forward derived leaves now retain per-frame continuation control across
   subsequent statements and matcher arms until the target scope joins. Intervening
   successors, writes and pending queries retain marks; enclosing control and
   continuation state restore on errors, and independent joined successors stay clear.
   Expression boundaries now refresh leave continuation control, including later
   call operands, indexed assignment RHS temporaries and pending-query blocks.
   Operand-local target joins remain independent; final coercion errors restore
   enclosing control. Scope prerequisite: `bac55cf`.
   Validated restart sites now retain bounded RestartId/target/function-owner/
   source-span/control evidence separately from forward continuation flags.
   Scope-operation errors precede registration; repeated matching metadata is stable
   and conflicting site identities are rejected. No loop-carried propagation is
   claimed by this prerequisite.
   Pending queries now retain bounded active block IDs within their function and
   associate with matching restart sites in either source order. Copies preserve
   the original query scope; recognition and failed preparation add no observations.
   The links do not change query control flags or replay source.
   Completed runtime bodies now retain bounded structural inventories of bindings,
   reads, writes, stores, calls, nested blocks and exits under their block/function
   identities. Restart targets address these inventories directly. Successful
   required scalar/boolean/record input uses retain storage-root IDs, read/root spans
   and lexical control under the nearest same-function block; fixed-signature
   queries and skipped evaluation add no uses. Runtime prerequisite: `87e7b20`.
   Body facts now retain bounded parent/operand roles, both matcher arms and
   short-circuit condition/RHS roles. Local/slot-alias facts and required uses retain
   canonical storage IDs without conflating reference carriers with their pointees.
   Relations: `fe19008`; storage: `5099fd1`. Indirect stores stay unresolved.
   Checked statement sites now retain function/block ownership, same-function
   parents and completion state (`d8f7f64`). Required reads and original pending
   queries retain those sites; query copies keep their original source. All ten
   compiler checks pass, including 1478 library/910 native tests.
   Runtime expression points (`10a45ec`) and individual required-read/query points
   (`b4a79de`) retain same-function parent/statement/block identities and explicit
   completion. Matcher and short-circuit points now retain condition/taken/skipped
   regions, including inline erased uses and runtime-skipped operands.
   HIR matchers (`1038bbc`) and short-circuit binaries (`a66a363`) retain optional
   point IDs. Body facts validate kind/function/block/completion and retain direct
   source links (matcher integration: `f2dcfcf`); synthetic sources stay unknown.
   Point boundaries return exact IDs (`e201bd7`). Branch decisions now retain
   explicit entry, true/false, empty bypass and normal-join ports in
   `dependencies/edges.rs` (matcher prerequisite: `1e30544`). Both structural
   alternatives remain, and normal ports do not imply reachable completion.
   Expression roots (`ba3debb`) and matcher-body statement points (`ee6c2bc`)
   now link region entry/normal ports to checked contents through a shared bounded
   edge ledger (matcher-content prerequisite: `e981b0b`). Empty bypasses remain
   separate; child normal ports are not presumed reachable.
   General statements now retain exact roots through lifetime checking (`aaffd39`),
   and completed sites identify those roots. Core blocks retain ordered statements
   and explicit forward-group barriers (`4286bae`). Ordinary binary operands retain
   exact direct roots and normal-to-next-entry edges; composed roots stay unknown.
   `dependencies/sequences.rs` validates ownership and shares the edge budget.
   Scope exits now retain exact statement entries, target ports and control metadata
   (`1f26948`); HIR leaves and restart evidence retain source IDs (`90a2065`). Body
   facts validate source identity and exact targets, preserving unknown synthetic
   origins and original call spans. No normal-fallthrough edge is introduced.
   Core block entry/normal/result ports (`b45829a`) now retain empty completion,
   sequence endpoints and target-leave joins, preserving receiver/forward barriers.
   Plain block consumers and expression statements use exact source links
   (`4f266e4`); body producer IDs remain function-owned. Restart reentry edges are
   separately marked Backedge and publish atomically with their exit source.
   Matcher statement roots now connect ordered branch points (`ed3baf3`). Ordinary
   bindings (`0e93b48`) and direct local/slot-alias writes retain explicit operations
   between value-normal and statement-normal ports, preserving source roots,
   canonical storage IDs, owner and control metadata. Unknown module producers
   remain explicit; annotations/required-use links are still separate.
   Composed roots (`e4374cc`) now precede emission staging. Direct emissions
   (`6bb0f49`) and record fanout (`c6fd7e5`) retain exact EmitId, slots, aliases and
   primary/field projections. Body facts use bounded point/target indices to link
   those operations without span matching. Emission initializes components; it
   does not exit its block. Never/static outputs retain their existing boundaries.
   Field/indexed operations now retain canonical targets and exact WriteStep paths,
   index/RHS roots, address stages and containing-list reservation/length stages.
   `Checked` edges admit the next address only on bounds success; write effects
   follow normal RHS completion. These metadata do not grant loan authority.
   Indirect scalar stores now retain exact target/RHS roots, capture/write stages
   and bounded canonical pointee-owner snapshots before RHS retargeting. Unknown
   origins stay incomplete; whole-owner sets are not precise projected locations.
   Direct calls now retain exact callee/site identities and argument roots,
   ordered links to an opaque effect, and conditional `Returned` edges. Declared
   `never` calls omit normal continuation. Debug formatting, list methods and
   required/type-only calls remain separate; no effect summary is inferred.
   List indices now retain exact receiver/position roots, explicit list-value and
   length snapshots before position evaluation, and bounds-success result stages.
   Nonreturning receivers skip position checking; nonreturning positions have no
   projection/result edge. Shared-list dereference and loan checks are unchanged.
   Concrete/inferred and union-context lists now retain element sequences by source
   slot, preserving deferred scalar checking, construction endpoints and `never`
   boundaries. Custom effect-block roots now follow form recognition and own
   checked body sequences/result links. Probes allocate no expression points; failures
   restore active point, scopes/frames, owner and reach.
   List/string `size` now retain receiver/operation roots; `add` retains receiver
   snapshots, item roots and capacity-success result stages. Known and unknown
   lengths remain distinct; nonreturning operands do not produce result edges.
   Shared element borrows now retain exact parent/index roots, checked place paths,
   view boundaries and direct temporary/statement IDs. Address/length stages
   precede indices, and bounds-success stages precede reference results. No new
   loan authority or lifetime extension is inferred.
   Exclusive paths now retain canonical storage, prefix fields and exact index
   roots (`f078e24`, `3920c12`). Each reservation/length capture precedes its index;
   checked success advances the address and completed paths acquire the reference.
   Nonreturning indices omit success and final acquisition/result edges.
   Debug output now retains exact formatting roots (`ff58f82`) and streamed stages
   (`99208a3`). Static text stays explicit; each operand precedes its output and
   that effect's return precedes the next part. Panic prefix comes first; only
   completed messages reach print newline or outer panic publication. Nonreturning
   operands omit suffix output and completion edges; panic has no normal result.
   Ordinary groups now reuse region links from group entry to exact child entry
   and child normal to group normal (`a3a44e3`). Same-owner/block identities, HIR,
   expected typing and logical charges are preserved; no return is inferred.
   Ordinary scalar unary operands now retain exact roots (`7932a09`) and validated
   scalar operation/result stages (`fdeb564`). Integer negation results follow
   overflow success; stopped operands omit operations/results. Context, primary
   projection and outer union coercion remain distinct. Signed literals and
   required-only construction keep their existing paths and logical charges.
   Explicit dereferences now retain exact pointer roots (`89d255f`) and shared/
   exclusive mode with load/result stages (`15f1c74`). Stopped pointers omit loads;
   uninhabited referents omit normal results. Metadata does not copy aggregate
   shapes, infer pointee storage from cells or grant authority.
   Exclusive scalar reborrows now retain exact parent roots (`03f5e54`), existing
   sites and mode, and parent-before-reborrow/result stages (`a6e9d05`). No referent
   load or new loan authority is inferred; stopped parents allocate no site.
   Direct shared reborrows retain exact parent roots (`5332455`), existing sites,
   requested result/actual parent modes and result links (`4c6c381`). Aggregate
   comparisons are bounded and retain no shape copies; stopped parents preserve
   requested mode without allocating a site or adding operation/result edges.
   Projected shared borrows now retain exact roots (`0a540a8`), explicit post-source
   materialization (`3133214`), bounded checked plans (`ea6a625`) and stage/result
   edges (`7ca8a3a`). Owned field reads and intermediate loads remain distinct from
   final address projections; source return conditions are not bypassed. Plans
   retain temporary identities, field indices, final parent modes and existing
   reborrow sites without adding source point IDs or loan authority.
   Ordinary shared place borrows now retain checked places, canonical alias roots,
   shared mode and address/reference stages (`6f34439`). No operand point or pointee
   read is invented. Paths use source-local types so canonical aliases can retain
   different member types; reference-cell identities and bookkeeping are preserved.
   Ordinary exclusive scalar places now reuse that operation with exclusive mode
   (`898a519`). Existing mutability, final-field permission and record-shape checks
   precede publication; alias-exclusive marking and later backing validation stay
   intact. Indexed exclusive paths and reborrows retain their own operations.
   Standalone temporary borrows now retain exact initializer roots (`2085cd4`) and
   existing local/statement identities with materialization/result links (`4443aac`).
   Never inputs allocate no cell; reference-valued initializers remain distinct
   cells. Projected and element-parent staging receives no duplicate result edges.
   Implicit exclusive-to-shared conversion now retains distinct raw source roots
   (`62a8a65`) and existing sites/result stages (`c2c4336`). Raw operations cannot
   bypass conversion; unchanged shared results forward through transparent links.
   Stopped contexts allocate no site or result edge. Generic coercions remain unknown.
   Ordinary runtime fields now retain exact receiver roots and captured implicit
   load decisions (`ed85184`), with receiver/load/field/result stages (`b806c44`).
   Resolved indices are recorded before narrowing; explicit loads, call returns,
   never fields and required/static early exits retain their existing boundaries.
   Predicates and explicit ascriptions now retain exact operand roots (`e5d5653`)
   and ordered result stages (`5ad03a7`), including erased no-op wrappers. Existing
   target construction, E208, stopped inputs and required-read identities remain
   unchanged; metadata does not evaluate answers or store target descriptors.
   Ordinary dispatch blocks now retain receiver point/local/body identities
   (`b0bee6b`) and receiver initialization/body/result stages (`63367e7`). The
   synthetic `None` prefix remains explicit, later barriers are never skipped,
   and stopped inputs/bodies gain no invented completion or source statement.
   Composed Group/Block roots now link exact children and partial body results
   (`85a59dd`). Composed dispatch captures receiver/local/body identities and
   reuses bounded prefix stages (`0ade9ed`) while preserving partial records,
   expected slots and caller-specific errors.
   Record-context equality now retains both composed operand roots (`498f9ca`)
   and links their checked evaluation order through the existing sequence ledger.
   Contextual primary selection, scalar projection, hints and diagnostics remain
   unchanged; the discard-only `composed` helper has no remaining callers.
   Composed fallback now retains exact source roots and pre-coercion decisions
   (`dd9be32`), with distinct forwarding/conversion/stopped stages (`b063010`).
   Never inputs have no result link even when existing coercion changes HIR type.
   Other generic coercion callers remain separate.
   Ordinary non-short-circuit binaries now retain captured primary/normal/success
   plans (`f0162b3`), and atomically publish entry/projection/operation/result stages
   with adjusted operand sequences (`26f4860`, prerequisite `07722b5`). Integer
   arithmetic results require Checked success; stopped inputs/projections have no
   later stages. Short-circuit graphs and required sequence-only checking remain
   separate, including required AST evaluation through `integer_result`.
   Expected-value decisions (`a9add72`) and non-required raw-source boundaries
   (`548eaaf`) now feed forwarding/primary/coercion stages (`bc18f19`). Caller
   roots stay distinct from raw branches/calls; direct and projected Never gain
   no result edge. Shared reborrows and required/source-free paths remain separate.
   Unary checking now captures its actual additional primary projection
   (`9fdbde3`) and sequences it before the operation/result (`4f26bbd`). Existing
   expected projections are not repeated; direct Never, invalid primaries, signed
   literals, required helpers and checked negation retain their prior boundaries.
   Formatting now captures source/primary pairs (`39670ee`) and places validated
   projections before per-part output (`d660709`). Literal None entries, recursive
   flattening, panic prefixes, returned-I/O conditions and stopped suffixes remain
   intact; a Never primary has extraction but no output/result edge.
   `list.rs::list_receiver_point` now captures its implicit shared-list load
   decision (`da35e36`). Index metadata sequences that load before the receiver
   snapshot and index evaluation, preserving explicit dereferences, stopped
   positions, bounds-success and shared element-borrow handling. Eight index
   groups pass (`0b6193f`). Method loads precede size operations or add snapshots
   (`4993f8d`), preserving effects, stops, errors and capacity-success stages.
   All 11 method groups and the compiler gate pass; the guide documents coverage.
   Final element-conversion decisions in `list_context.rs::list_union` are now
   captured (`fcc193f`) and sequenced before successors/construction using
   prepared list sequences (`0c05427`). Stopped primaries retain no suffix or result
   edges; input/sequence/endpoint publication is atomic. The guide documents the
   scope and the compiler gate passes. Narrowing decisions (`1fb0156`) and
   ordinary local/field raw-source roots (`29a5e6f`) now feed bounded forwarding/
   conversion/result stages (`699fc42`). Never results are omitted and shared-
   budget failures publish no narrowing operation. The compiler gate passes; the
   guide documents the scope. Owned-field narrowing decisions are now captured
   alongside `ProjectionStep::Field` (`108b41d`) and conversion stages now precede
   further traversal/reborrow (`e909bd0`) without changing path-step counts. All 14
   focused groups and the compiler gate pass; the guide documents the scope. Raw
   local-read point/local/canonical-storage identities and completion are captured
   inside the narrowing boundary (`371d866`) and feed bounded entry/read/result
   stages (`d1cfcf6`). All eight focused groups and the compiler gate pass; the
   guide documents the scope. Runtime Value::FileModule reads now reuse bounded
   storage-read machinery after existing required-primary and capture gates. Four
   focused groups and 1813 library tests pass (`ea57a8f`). Three native groups
   verify real imports/facades, aliases, once-only startup and required/capture
   boundaries (`ed70fbb`); the compiler gate passes and the guide documents scope.
   Ordinary scalar leaves now retain checked kind/type and exact roots at actual
   construction branches in `expressions.rs` (`a4b385a`), including signed integers
   and resolved constants. Bounded construction/result stages (`7a42833`) and the
   compiler gate pass; the guide documents the scope. Static Heap leaf identities
   are captured at resolved branches (`46ea8f3`) and bounded handle/result stages
   (`c5e248b`) pass the compiler gate; the guide documents scope. Bounded read-only
   inventory (`2eec4bd`, `95fba66`) preserves duplicate stored edges, routes and
   Backedge markers while auditing all 31 counts. Full-family coverage (`42b04e8`)
   and the compiler gate pass; the guide documents inventory limits. Port anchors
   (`f543ec8`), endpoint owner agreement (`cad2261`) and stage selectors (`ec8677c`)
   now validate retained identities. The producer registry (`53d54af`) validates
   Operation ports without repeated invocation scans. The compiler gate passes; the
   guide documents scope. Bounded forward lookup (`9adb15c`) now retains exact
   source ports, original entry positions and separate forward/Backedge buckets.
   Full-family, duplicate, owner, route and boundary coverage (`b838be4`) passes the
   compiler gate; the guide documents scope. Bounded structural walks (`1732485`)
   now retain visited ports and original forward/Backedge positions, report missing
   sources and stop repeated-port cycles. Boundary coverage (`f932bf5`) and all ten
   compiler checks pass; the guide documents scope.
   Independent HIR program/function reports (`eaa1c2c`) now retain one validated
   index with exact roots and owner checks. The allowance prerequisite (`80feac6`)
   bounds report growth; unused/recursive, identity and aggregate-budget tests and
   all ten compiler checks pass. The guide documents collection scope.
   Validated operation owners now stay with the index (`bbed8d8`). Encountered
   Operation ports receive bounded direct Bind/Write or explicit Unknown effects
   (`9f83e01`), preserving exact storage/RHS/control and conservative marks.
   Identity/alias/stopped/budget coverage and all ten compiler checks pass; the
   guide documents scope.
   Owned Path effects (`dd532e3`) now preserve exact ordered field/index metadata
   with per-path and aggregate copy bounds. Boundary coverage (`f705732`) and all ten
   compiler checks pass; the guide documents scope. Indirect snapshots (`7110adb`)
   now retain pre-RHS origins/completeness and target/RHS/control with bounded copies.
   Boundary coverage (`8983112`) and all ten compiler checks pass; the guide
   documents snapshot scope and the remaining declaration boundary.
   Ordinary function Bind definitions now retain exact-ID bounded completion
   endpoints (`b2dd92c`, `8509cc9`), without entering their bodies or treating a
   Never body as a non-completing declaration. Selected immutable function/foundation
   identities now have bounded completion (`29db896`, `6387133`), including self/forward
   aliases; source conformance (`cabfb71`) and the compiler gate pass. Exported
   definitions/re-exports now return exact IDs (`6fce856`) and publish completion
   after all export checks (`447c0e4`), preserving module startup and independent
   bodies. Source conformance (`3506f03`) and the compiler gate pass. Explicit
   TypeAlias statements now connect only after required construction and declaration
   checks (`133ba70`); required-only helper aliases gain no runtime endpoints.
   Source conformance (`90d6263`) and the compiler gate pass. Ordinary type-valued
   bindings (`3692a56`) and meta exports (`1c4d254`) now connect after all required/
   declaration/export checks. Source coverage (`d43aab9`) and the compiler gate pass.
   Immutable static aliases now connect after declaration (`c2f7d7b`), preserving
   required-only producers, payload widths and typed/mutable runtime storage paths.
   Source conformance (`46d450e`) and the compiler gate pass. Scoped-control aliases
   now connect after bounded active-target/owner validation (`58fd3cf`), keeping
   declaration separate from real leave/restart calls. Source coverage (`8d47955`)
   and the compiler gate pass. File-module aliases now retain exact backing IDs
   and validate local/exports registration before completion (`1143ce5`), preserving
   module order, privacy, required reads and caller ownership. Source conformance
   (`43a4308`) and the compiler gate pass. Synthetic initializer storage now keeps
   exact module-expression roots (`514324d`, `97209a5`), exposing ordered initializer
   effects while preserving stopped inputs and successful-check boundaries. Source
   conformance (`6ea543a`, `90791b1`) and the compiler gate pass. Ordinary forward
   groups now retain reservation IDs and checked first-signature sites, validate all
   definitions before completion, and connect block/dispatch sequences (`82595c3`,
   `a687419`, `4940924`). Source coverage (`6791859`) and the compiler gate pass.
   Exported forward definitions now fulfill existing IDs with validated module-level
   scope/signatures, whole-group publication and peer documentation (`9ee8cb6`,
   `1f995e4`). Required source coverage (`a8f8988`, `25aa9e0`) and the compiler gate
   pass. Record alias auditing confirms ordinary copies already retain storage
   operations and required scratch stays within checked construction roots;
   regressions and source coverage (`f8d7aed`, `48298eb`, `e5a5f39`) preserve those
   boundaries. Pending statement IDs and bounded completion now preserve original
   query origins, owner/site/root checks and final proof gates (`4506888`, `8a019dc`).
   Required rejection coverage (`532c3a5`) and the compiler gate pass. Direct-call
   effect records now retain exact CallId/FunctionId, argument/control and return
   metadata with bounded validation (`16c700a`, `ab9a03a`); source coverage (`8c2cf12`)
   and the compiler gate pass. Bounded call adjacency (`84324c6`) now retains exact
   site/owner membership and missing/backedge boundaries. Iterative components
   (`d64de6c`) preserve independent owners and identify self/mutual recursion;
   source coverage (`2983acc`) and the compiler gate pass. Condensation (`37be1e7`)
   preserves internal/cross-group sites, and bounded analysis ordering (`d28ce65`,
   `37ef903`) places callees before callers with deterministic ties. Source coverage
   (`a0635f6`) and the compiler gate pass. Bounded output-stage capture (`0ad7f11`)
   and typed aggregation (`c018728`, `cae9858`) now retain partial print/panic effects
   without inventing terminal operations. Source coverage (`44e3073`) and the compiler
   gate pass. Local storage context (`4f51efb`) and typed read effects (`cb389e2`)
   now preserve canonical storage, normal/control flags and required-only boundaries;
   source coverage (`04ba6d8`) and the compiler gate pass. Explicit dereference
   result decisions (`e9aa13a`) and typed effects (`f3f4e4c`) now preserve exact
   pointer roots/modes and normal/control flags. Boundary tests (`f6044ab`), source
   coverage (`cfc7d62`) and the compiler gate pass. Checked field counts (`f834ed9`)
   and typed field effects (`e0b6732`) now retain validated receiver/index/load/result
   boundaries. Identity/budget coverage (`deb2b53`), source cases (`e984699`) and the
   compiler gate pass. Separate index result decisions (`b29e9a0`), stage capture
   (`03f4681`) and typed aggregation (`fcb6b88`) now retain partial receiver loads/
   snapshots before stopped positions and conditional read/result boundaries. Identity/
   budget tests (`b04783a`), source cases (`776f00a`) and the compiler gate pass.
   Method-stage validation (`0412249`) and typed reports (`c9d4361`) now retain
   list/string size and list-add observations, including partial receiver stages.
   Boundary tests (`d83f647`), source cases (`a1f0875`) and the compiler gate pass.
   Unary-stage validation (`97cec7c`) and typed observations (`77976fa`) now retain
   compact scalar descriptors and separate projection/operation/result flags. Type/
   boundary tests (`ec2d6c6`), source cases (`9dbc4ae`) and the compiler gate pass.
   Binary type capture (`5503576`), stage validation (`3ae26e3`) and typed observations
   (`98babec`) now retain scalar descriptors, partial projections and checked results
   across both operation and sequence ledgers. Boundary tests (`cb5a88d`), source
   cases (`37726ee`) and the compiler gate pass; nonscalar comparisons remain opaque.
   Scalar construction/result reports (`3be764d`) now preserve checked scalar kinds,
   numeric widths and independent observations without copying literal values.
   Boundary regressions (`28ec4c2`), required source cases (`6b1d6c1`) and the compiler
   gate pass. Heap reports (`4800e0d`) retain nominal Allocator identity and independent
   handle/result observations. Boundary regressions (`f8fca7b`), source cases
   (`89d7d1f`) and the compiler gate pass. Narrowing observations (`c6e092c`) now retain
   exact raw sources and separate forwarding/conversion/result boundaries. Boundary
   regressions (`a86b02f`), source cases (`ef17f36`) and the compiler gate pass.
   Coercion reports (`912e62d`) now retain Forward/Convert/Stopped decisions and
   independent projection/conversion/result visits. Boundary regressions (`16d183f`),
   source cases (`00fb650`) and the compiler gate pass. Predicate/ascription reports
   (`a031d11`) now retain checked kinds, operands and independent operation/result
   observations. Boundary regressions (`da43cb7`), source cases (`c68e7f9`) and the
   compiler gate pass. Place-borrow record counts (`ac590ac`), validation (`97bda15`)
   and independent address/acquisition/result reports (`6de8c96`) now preserve
   canonical alias storage, reference-cell identity and checked modes. Additional
   boundary coverage (`f4ebd60`), source cases (`44e8b39`) and the compiler gate pass.
   Temporary cell/site validation (`7b28c38`) and independent acquisition/result
   reports (`80d6d78`), boundary coverage (`eea5989`) and source cases (`fd9c16d`)
   and the compiler gate pass. Reborrow validation (`6702ace`) and independent
   acquisition/result reports (`133ed62`), boundary coverage (`ad414a1`) and source
   cases (`3314092`) and the compiler gate pass. Shared element source capture
   (`d013921`), validation (`e1fa743`) and observations (`8644137`) now retain partial
   addresses and bounded owned/view/temporary metadata. Source/loan coverage
   (`c5abe07`), limits (`9d696e8`), source cases (`3fa8ab0`, `658a01c`), lint repair
   (`16cd21e`) and the compiler gate pass. Exclusive field counts (`bf99d6f`), index
   facts (`29abc4b`), validation (`b5cdaef`) and observations (`e8debce`) now retain
   independent address/reservation/acquisition/result visits. Boundary coverage
   (`720e062`), source cases (`c7abffc`, `3d0a9bc`) and the compiler gate pass.
   Projected field counts (`dc0777e`), validation (`d7735b0`) and reports (`73be259`)
   now retain independent materialization/field/load/address/conversion stages and
   acquisition/results. Boundary and budget coverage (`f821bad`, `c0b741c`), required
   source cases (`39423eb`, `c6ae594`) and the compiler gate pass. List producer
   identity (`3450dfd`), validation (`7344c0f`), reports (`6ff637d`), boundaries/limits
   (`520b8d2`, `e90b966`) and required source cases (`9b32d9f`, `13f8aaa`) now pass
   the compiler gate. Dispatch completion (`833ab1a`), validation (`68334bb`),
   reports (`73511c7`), boundaries/limits (`7768051`, `a6fae62`) and source cases
   (`9f5dcc5`, `6d1ddbc`) now pass the compiler gate. Emission composition locals
   and field counts (`b54c7d6`), validation/site repair (`298e75b`, `de88cc4`),
   reports (`2261568`), boundary/limit coverage (`7c7e172`, `e28d8ea`) and required
   source cases (`acb7fe0`, `36cead6`) now pass the compiler gate. Checked equality
   categories (`3db93eb`), reports (`308fcbc`), boundary/limit tests (`240cec7`,
   `c685bf3`) and required source cases (`4735fb1`, `ccebb5d`) now pass the compiler
   gate. Never operand contexts (`418e681`) and their source cases (`e6621f4`) are
   repaired. Checked block completion (`1816109`), validation (`0d5b5de`), reports
   (`e5191fc`), boundaries/limits (`84e8d09`, `16a4e99`), source cases (`fb8b386`,
   `b8b80a0`) and identity/span repairs (`e00f349`, `8d53ba2`) are implemented.
   Result-slot layouts (`0592307`), budget plumbing (`3395b59`), validation (`716f535`),
   candidate links (`16cdf31`), boundaries/limits (`3a30543`, `cda38c4`) and source
   cases (`cb60d46`, `c41d71c`) now pass the compiler gate. Consumer indexing
   (`ec42f09`), direct fields/primaries (`e07bd3d`, `2fba3c2`), boundaries/limits
   (`120c657`, `2ac60c2`) and source cases (`06b7c8b`, `f0dba6d`) now pass the gate.
   Explicit group capture/resolution (`eb7e6d0`, `b1311c1`), identity/limit coverage
   (`a790e3e`, `d1f839d`) and source cases (`4817fba`, `5da70d0`) now pass the gate.
   Observed Forward qualification and mixed chains (`97032a4`, `c2552a2`) plus
   typed source cases (`31f9912`, `4523ee7`) pass the gate. Unchanged narrowing
   (`a8442b3`, `554cbfb`) and source cases (`9a251dd`) pass the gate. Bounded local
   eligibility (`5af4c18`, `407c6d7`) and source cases (`5e7e3a5`) pass the gate.
   Exact Bind qualification (`7ec1583`, `970c6a3`) and source cases (`6d910f8`) are
   complete. Parameter/index collection (`d3b22ec`, `869e5c7`), boundary coverage
   (`a433f00`) and source cases (`2d460d2`) pass the gate. Collection budgets
   (`f83ba59`), read forwarding (`1308c62`), mixed bounds (`b2d0033`) and source cases
   (`8e5c5d1`) pass the gate. Coercion-owned primary consumers (`5001917`), boundary
   coverage (`0399df9`) and source cases (`9748410`) pass the gate. Output edge/report
   qualification (`0c5c184`, `1bf878c`), streamed links (`7ac15f3`), limits (`4509d0e`)
   and source cases (`8e1269d`) pass the gate. List validation/report qualification
   (`6694ada`, `d5e966b`), contextual links (`c214833`), boundaries (`5d908ad`) and source
   cases (`0c48b3e`) pass the gate. Shared emission qualification (`2a2de97`), source-slot
   consumers (`bcf8eb0`), boundaries (`2ec24cf`) and source cases (`b13d8fc`) pass the gate.
   Ascription capture (`beaade7`), report qualification (`af3641a`), consumer forwarding
   (`406d35f`), boundaries (`09a5498`) and source cases (`d4d105b`) pass the gate.
   Field validation (`b395515`), independent observations (`582a7f2`), boundaries
   (`da2b050`) and source cases (`b9031c2`) pass the gate. Candidate header sharing
   (`98140f0`), input qualification (`4475569`), collection (`71b1d6a`), boundaries
   (`2f23847`) and source cases (`3afaa03`) pass the gate. Composed source sharing
   (`c489bd1`), candidate slot qualification (`74671ed`), boundaries (`590cb3e`) and
   source cases (`e04ad8e`) pass the gate. Stored graph qualification (`322d4ed`),
   traversal/forest integration (`b3a6516`), boundaries (`60fd5e0`) and source cases
   (`01baf4a`) pass the gate. Field-result qualification (`aeefeed`), collection
   (`03e46c3`), boundaries (`1ff75ba`) and source cases (`9373325`) pass the gate.
   Stored lookup (`1cd353b`), narrowing resolution (`444a66a`), direct reports
   (`324fc89`), boundaries (`f4e12ff`) and source cases (`5f602e9`) pass the gate.
   Direct-source graph qualification (`2f043ea`), explicit traversal (`7972426`),
   expanded-forest integration (`4104d99`), boundaries (`cfe9512`) and source cases
   (`cc43868`) pass the compiler gate; strict results are recorded above.
   Shared group qualification (`39032ab`), direct resolution (`d75b744`), identity/
   cycle boundaries (`fa55889`), limits (`1790a65`) and source cases (`b2005f1`)
   pass the compiler gate; strict/final docs are recorded above. Next qualify direct
   field sources through observed non-projecting Forward coercions, following the
   ordered plan, before wider value provenance work.
   Indexed/projected/temporary borrows and reborrows stay separate; no observation
   may grant new loan authority, extend a lifetime or infer a proof outcome.
   Other contextual builders and required evaluation remain separate.
   Preserve owners and required roots. Keep result availability
   separate from field/value provenance, and exclude backedges from acyclic walks
   until the loop-header analysis is implemented.
   Do not turn a completed check or missing effect metadata into normal completion.
   Generic coercions and contextual builders remain
   coverage gaps; missing sequences are not independence.
   Never add a generic entry-to-normal bypass across exits or unknown effects.
   Keep independent matcher arms, nested targets and function ownership distinct.
   Record source identities during checking; do not infer links or runtime order from spans,
   point IDs or inventory indices. Required evaluator control regions beyond
   captured read leaves remain separate. Preserve recognition purity, original
   roots, fixed signatures and both structural successors.
   Connect nested block/emission result sources to their consumers using existing
   HIR/slot identities; retain unknown reference/store/call effects explicitly.
   These remain prerequisites to a complete transfer graph. Then propagate over
   restart backedges/headers using these facts and query links,
   without re-evaluating initializers or resetting retained logical budgets. Preserve
   nested targets, function ownership, ordinary errors and unknown/derived conditions.
   Validate seeded before/after-restart reads, writes, required uses and queries,
   then run the full compiler gate. E225 enforcement and outcomes stay gated until
   this propagation is validated; termination dependence remains separate.
   Owned projections through unselected heterogeneous prefixes remain separate.
   Broader aggregate returned shapes remain separate. Precise overwrite/branch joins and function result
   dependencies remain separate; old owners/marks are retained conservatively.
   Keep flags gated until these analyses are complete.
   Structural reads and lexical matcher control are tracked; required reads
   and pending query availability enforce E225 for those marks. Conditional
   leave control is now retained at statement and expression boundaries.
   Restart backedges and termination dependence remain incomplete;
   lexical restoration alone is not complete control analysis. Add seeded write/call/continuation regressions and
   run the full compiler gate. Preserve answer-independent fixed flag type queries.
   Preserve ordinary typing/ownership in skipped runtime bodies and E225 separation for
   type formation and observation availability. Plan independently reviewable
   representation, propagation and enforcement slices, then run the compiler gate.
   When enabling outcomes in `check/queries.rs`, charge descriptor construction and
   type capability inspection to retained roots. Pending metadata has no result
   payload yet. Required-block descriptor admission and text/helper execution stay
   gated until their execution/accounting foundations exist.

2. Keep mixed union/subtraction precedence and unsupported literal/base subtraction
   parked until their language/representation prerequisites are established. Keep
   first-class metatypes, runtime type containers and type-producing helpers separate.

Do not push or bump release versions here.
