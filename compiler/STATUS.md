# Compiler handoff and work tracker

Updated: 2026-09-14. Type-only copy queries retain pending metadata until ordinary checks finish.
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
- `type_values.rs::Work` is bootstrap work (4096 visits, 64 levels, 16384 nodes),
  with B001 failures. It is not the revision 1 logical budget. Proof needs shared
  required-root counters for steps, aggregate slots, text, types and helper depth;
  keep infrastructure limits separate. Never relabel bootstrap exhaustion E220.
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

### Logical accounting continuation

The existing Work visits are not language steps: they include bootstrap traversal
and retained initializer evidence. They must not be relabeled E220. The first
logical charging domain will be actual type materialization, whose nodes already
have one owning traversal; scalar/control/input, aggregate/text and helper charging
remain separate follow-ups. Proof evaluation stays gated.

Dependency-ordered commits:
1. Extract bootstrap work ownership and centralize required-root lifetime handling
   for computed types and metatype bindings. Preserve all counts/errors; verify
   nested sharing and cleanup. Baseline: 111 required-evaluation tests pass in
   `/tmp/meowy-required-root-before.log`.
2. Add an independent logical ledger with per-root origin and atomic counter
   checks. Wire type-node materialization only: one step plus one type node each.
   Keep earlier bootstrap limits B001. Test exact/below/above logical limits,
   overflow, repeated charges, nested roots and failure cleanup.
3. Validate metadata exports/imports, independent source roots and the unchanged
   query gate; document the partial charging boundary and run the full compiler gate.

Root lifetime extraction passes the same 111 baseline tests plus a new nested
sharing/error-cleanup regression (112 total). Log: `/tmp/meowy-required-root-after.log`.
Bootstrap counts and diagnostics are unchanged. Next: the independent type ledger.
Refactor commit: `5d03d9b`. The type ledger is implemented locally: independent
step/type counters, original root span, atomic checked additions and sticky E220
failures. Work.node charges one step/type after the existing bootstrap guard.
Existing test snapshots require default initialization of the new ledger field;
those struct literals must change with the field to keep the commit buildable.
Two additional full Work literals were found by the compiler and now initialize
the new field. This field change requires all nine existing snapshot-test files
in the same buildable slice; splitting their defaults from the new field would
produce either missing-field errors or needless-update lint failures. The logical
root gate also preserves a caught budget failure so hints cannot suppress E220.
All 223 checker tests pass (`/tmp/meowy-logical-type-ledger.log`). The slice is
under 300 changed lines across 14 files; nine test modules directly initialize
Work and must provide its new field in the same buildable change. Next: source
root/metadata integration checks and documentation, then the full compiler gate.
Logical type limits are internally boundary-tested; source programs still hit
lower bootstrap limits first. No claim of complete revision 1 accounting is made.

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

Next: logical required-root accounting and proof-dependency tracking. Pending
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

- `python3 -B tools/verify.py --compiler`: all ten checks passed, including 861
  library/878 native tests (1739 total), 20 Python tests, fmt, Clippy, build, links
  and catalog/schema checks. Conformance: 10 passed, 13 unsupported, 0 failed in
  debug/release. Log: `/tmp/meowy-pending-query-complete-gate.log`.
- All 28 parser tests passed. Six pending-query checker groups cover fixed Result
  identity, shared copy origins, call capacity, ordinary-error precedence, captures,
  runtime escape rejection and explicit metadata/flag gates. Record argument
  arity and documentation attachment regressions also pass.
- Three native groups exercise check/build/run in both profiles: original file
  spans, type-argument import discovery, preserved ownership/type errors, and no
  startup before unsupported evaluation is reported. Logs:
  `/tmp/meowy-pending-query-integration.log`, `/tmp/meowy-type-call-doc-tests.log`,
  `/tmp/meowy-pending-query-export-gates.log`, `/tmp/meowy-pending-query-flags.log`.
- No proof outcome is evaluated and no unsupported query counts as successful
  conformance. Runtime implementation, reference fixtures, dependencies and versions
  are unchanged. Editor and separate runtime/sanitizer gates were not rerun.
  Full v0.0.1 release qualification remains incomplete.

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

Type-value resolution/work bounds live in `src/check/type_values.rs`; ordinary type
construction and symbol lookup remain in `src/check/names.rs`. Required list extents
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

1. Extend `check/queries.rs` beyond pending origins only after logical root
   accounting and proof-dependency handling are connected. Next implement the
   [planned accounting prerequisite](#dependency-ordered-commit-series): separate
   language counters/root identity from bootstrap guards; share nested roots,
   restore state on failure and test exact/below/above limits and retained work.
   Do not relabel existing Work exhaustion E220. Pending metadata already supplies
   call spans/target/revision; keep active outcomes, flags and assertions gated
   until phase/dependency and charging rules can be enforced together.
2. Keep mixed union/subtraction precedence and unsupported literal/base subtraction
   parked until their language/representation prerequisites are established. Keep
   first-class metatypes, runtime type containers and type-producing helpers separate.

Do not push or bump release versions here.
