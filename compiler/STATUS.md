# Compiler handoff and work tracker

Updated: 2026-09-13. Required boolean logical block operands are in progress.
The previous compiler gate passed. Full v0.0.1 remains incomplete.
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

## Current compiler slice

Required integer comparisons now accept inline blocks and arithmetic containing them.
`comparisons.rs` owns integer comparison validation/execution; `comparisons/blocks.rs`
validates deferred operand forms without evaluating block-local initializers. Existing
block-free comparisons retain their original eager form checks and evaluation costs.

Known outer names/types and supported block statement forms are checked before logical
short-circuiting. Unresolved block-local values, annotations and dependent literal widths
wait until the comparison runs. Selected operands use `integer_arithmetic` once, left
to right, preserving exact widths and source errors. All six integer relations work.

Short-circuited blocks charge no evaluation visits or constructed type nodes. Structural
validation remains bounded by the existing form count/depth limits. Selected blocks
retain scope, work/depth/node budgets, tail errors and original dependency spans.
Known incompatible widths remain E213; evaluated block width mismatches remain E207
and deferred literal range errors remain E216.

Block-local declarations do not escape to the other operand. Materialized aliases reuse
values, original sources retain complete work, and independent roots reset budgets.
Required comparisons produce no runtime locals or HIR statements. Check/build remain
silent and normal module startup and function capture restrictions are preserved.

Selected boolean-result block operands, empty/null/float/text results, scalar-primary
records, mutable scratch, whole-module namespaces, fallback arms and helpers remain
separate. Skipped documentation retains its existing unanalyzed-declaration gate.

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

- `803eda8`: comparison extraction; 800 library/832 native tests, fmt and Clippy
  passed. Log: `/tmp/meowy-comparison-refactor.log`.
- `00f68eb`: deferred block comparisons; 802 library/833 native tests, fmt and
  Clippy passed. Log: `/tmp/meowy-block-comparisons.log`.
- `287459d`: integration; 805 library/835 native tests, fmt and Clippy passed.
  Log: `/tmp/meowy-block-comparisons-integration.log`.
- Five block-comparison checker tests and three native groups cover all relations,
  deferred/known widths, exact work, structural bounds, scope, source costs/errors,
  skipped values/types, documentation and module/function staging.
- Block-comparison guide prints `7` in debug/release:
  `/tmp/meowy-block-comparisons-doc-c7rut46c/main.mwy`.
- `python3 -B tools/verify.py --compiler`: all ten checks passed, including 805
  library/835 native Rust tests (1640 total), 20 Python tests, fmt, Clippy and build.
  Log: `/tmp/meowy-block-comparisons-gate.log`.
- Conformance: 10 passed, 13 unsupported, 0 failed in debug/release. Local links,
  catalog/schema and whitespace checks passed. Full release qualification remains open.
- Runtime implementation, reference fixtures and dependencies are unchanged. Editor
  and separate runtime/sanitizer gates were not rerun; full release qualification
  remains open. Evaluator/record bounds are not native support guarantees.

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

Whole-record module inputs, conditional module exports, required boolean-result block operands, helper
initializer eligibility, module-data captures, borrowed module storage, package/manifest
resolution, full required evaluation and generic specialization, public FFI, wider
ownership/cleanup, executable networking, public artifacts/replay and LSP remain separate. Host execution does not qualify minimum
platforms or bundled distributions. Toolchain: Rust 1.98.1 and LLVM/Clang/LLD/LLVM ar 22.1.8.

## Next steps

Inspection: required boolean form checking has no block case, while execution already
short-circuits logical operators. Reuse comparison block statement validation and
`scalar_block(..., boolean)` for selected operands. The block form has an expected
boolean context; its local initializers remain deferred until execution. Grouped
negation must use `!({ ... })`; bare `!{ ... }` is the existing unchecked-block syntax.

Dependency-ordered commit plan:

1. Complete: common operand-block statement validation preserves comparison diagnostics
   and structural accounting. 806 library/835 native tests, fmt and Clippy pass.
   Log: `/tmp/meowy-boolean-block-form.log`.
2. Add boolean block forms and selected execution for `!`, `&&`, `||` and required
   matcher conditions; keep focused acceptance/rejection tests with this behavior.
3. Verify exact work, skipped initializers, bounds, scope, source errors and staging.
4. Update guides/handoffs and run the final compiler gate across the series.

Keep direct boolean block equality/comparison disambiguation, empty/null results,
scalar-primary records, skipped documented declarations, fallback arms, mutable/
float/text/reference fields, whole-module records and helpers separate. The proof
package remains a specified future API, not part of this implementation. Do not push.

Unrelated documentation edits appeared during this work; preserve them unstaged.
