# Compiler handoff and work tracker

Updated: 2026-09-21. Pending queries retain charged argument/outer-root budgets.
Proof evaluation remains unimplemented. Full v0.0.1 is incomplete.
[../STATUS.md](../STATUS.md) tracks the project; [../COMPILER.md](../COMPILER.md)
records the plan. Keep this handoff current; Git holds history. Do not recreate STEP logs.

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

## Executable proof plan

The next milestone is bounded type-only `proof.can_copy<T>()`, with opaque static
results, direct flags, `assert` and `expect<S>`. Begin with concrete types already
represented by the bootstrap; defer value observations, place probes, generic
analysis, bounds, composition helpers and static descriptor exports. This is a
partial package milestone, not revision 1 qualification. Only module/revision
metadata and descriptor type aliases are implemented so far. Pending copy-query metadata is retained, but no evaluated result or observation
outcome is constructed. The reference remains authoritative.

### Prerequisites and current integration

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
- Scalar constants and initializer evidence currently have no proof-dependency
  marks. Add transitive data/control dependency tracking before exposing flags;
  type formation and query availability must reject E225. Ordinary runtime
  conditions derived from flags retain both successors for base typing/ownership.
  Existing constant folding or `inputs` evidence cannot provide this guarantee.
- `type_values/work.rs::Work` retains bootstrap limits (4096 visits, 64 levels,
  16384 nodes), with B001 failures. Its separate `required::Budget` currently charges
  type materialization, required statements/blocks and integer/boolean evaluation,
  including scalar/record projection ancestors and retained record reads. Type
  expression dispatch and required source constructors now charge separately.
  Pending queries now retain outer-root budgets. Remaining descriptor execution
  roots, text/helper admission and phase/dependency tracking remain prerequisites.
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

### Descriptor annotation accounting

Explicit pending-result annotations now use `construction_root` and charged
`source_spec`, preserving lookup-only classification elsewhere. An active outer
root supplies its ledger; otherwise the annotation starts an independent root.
Aliases charge their constructed type, while unannotated copies do not reconstruct
arguments or create another query. Ordinary extent restrictions remain intact.

Dependency-ordered slices:

1. `b43a23c`: annotation construction roots and three checker groups covering
   aliases, exact/overflow limits, original errors and root restoration.
2. Native file-origin/error checks, the foundation guide and both handoffs.

All 963 library tests pass (`/tmp/meowy-descriptor-annotations-lib.log`). The new
native group passes in debug/release (`/tmp/meowy-descriptor-annotations-native.log`)
after correcting its expected annotation spans. The final compiler gate passes all ten checks.

Audit: required text values and source-helper execution remain unsupported in
`check/type_values`; text type queries do not materialize bytes. Add their counters
when admitting execution. Pending descriptor execution roots and transitive
proof data/control dependencies remain prerequisites before outcomes or flags.

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
is integrated; descriptor execution roots remain open before query analysis.

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

- `python3 -B tools/verify.py --compiler`: all ten checks passed, including 963
  library/900 native tests (1863 total), 20 Python harness tests, fmt, Clippy,
  build, links and catalog/schema checks. Conformance: 10 passed, 13 unsupported,
  0 failed in debug/release. Log: `/tmp/meowy-descriptor-annotations-gate.log`.
- Three checker groups cover descriptor annotation aliases/copies, exact and
  exceeded step/type limits, retained outer failures, independent root restoration
  and constructor error accounting. Log: `/tmp/meowy-descriptor-annotations-lib.log`.
- A native group verifies original query/file locations, wrong alternatives,
  missing names, arithmetic failures and ordinary extent gates in both profiles.
  Log: `/tmp/meowy-descriptor-annotations-native.log`.
- Valid queries still reach the pending-evaluation B001 gate. No proof outcome,
  text/helper execution or full release qualification is claimed. Runtime source,
  reference fixtures, dependencies and versions are unchanged; editor and separate
  runtime/sanitizer gates were not rerun.

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
platforms or bundled distributions. Toolchain: Rust 1.98.1 and LLVM/Clang/LLD/LLVM ar 22.1.8.

## Next steps

The bounded subtraction series is complete; its syntax/representation limits remain
explicitly documented. No outstanding failures remain.

1. Establish pending descriptor execution roots in `check/statements.rs` and
   `check/queries.rs` before enabling deferred query analysis. Argument roots,
   annotation construction and shared budget retention are integrated. Text/helper
   values remain unsupported; add their counters only when admitting execution.
   Preserve unsupported gates. Record ordered
   slices and test selected/skipped work, original errors and outer-root limits.
   Then add transitive data/control proof dependencies before producing outcomes
   or flags; type formation and query availability must retain E225 separation.
   Run the full compiler gate after each integrated series.

2. Keep mixed union/subtraction precedence and unsupported literal/base subtraction
   parked until their language/representation prerequisites are established. Keep
   first-class metatypes, runtime type containers and type-producing helpers separate.

Do not push or bump release versions here.
