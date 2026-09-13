# Compiler handoff and work tracker

Updated: 2026-09-13. Annotated required record construction and integration pass.
The final compiler gate passed. Full v0.0.1 remains incomplete.
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

Explicit record annotations now construct bounded immutable records during required
evaluation. `Output` carries expected type, primary value and completed field slots;
`type_values/records/build.rs` checks named emissions, builds typed leaf paths and
publishes a `Value::Record` only after every expected field is initialized.

Selected fields retain exact kinds/widths and optional matching annotations. Missing
fields report E204, duplicates E205, unknown/incompatible fields E207 and literal
range errors E216. Each emitted name enters the current lexical scope only after its
initializer completes; matcher-local names do not escape. Nested record/scalar field
blocks inherit expected types, and eligible record values can initialize record fields.

Record shapes keep implicit unit primaries, immutable integer/boolean/nested fields,
256 total fields and 32 record levels. Explicit primary emissions/composition, empty/
nullable/mutable/float/text/reference shapes and unannotated construction remain gated.
Selected tails retain work/errors; skipped matcher expressions are not evaluated.
All required contexts retain shared scope/work/depth/node budgets and no runtime locals.

Existing source materialization preserves complete ancestor errors/work before field
copies, and record construction/copies charge retained type shapes. Documentation
keeps completed field widths and resulting type signatures. Function-scoped required
reads do not grant captures, and check/build retain silent eager module initialization,
including failures behind skipped field reads.

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

- `66bd7c1`: consolidated output state; 769 library/810 native tests, fmt and Clippy
  passed. Log: `/tmp/meowy-required-output-tests.log`.
- `3fe7299`: record construction; 771 library/811 native tests, fmt and Clippy passed.
  Log: `/tmp/meowy-record-construction-tests.log`.
- `345f98b`: integration; 773 library/814 native tests, fmt and Clippy passed.
  Log: `/tmp/meowy-record-construction-integration.log`.
- Four constructor checker tests and four native groups cover leaf kinds, no runtime
  storage, field/annotation/init errors, completed-field scope, nested/source copies,
  256-field/32-level boundaries, source/tail work, source spans, docs and module startup.
- The guide prints `7` in debug/release:
  `/tmp/meowy-record-construction-doc-_d1o3ak4/main.mwy`.
- `python3 -B tools/verify.py --compiler`: all ten checks passed, including 773
  library/814 native Rust tests (1587 total), 20 Python tests, fmt, Clippy and build.
  Log: `/tmp/meowy-record-construction-gate.log`.
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

Whole-record module inputs, conditional module exports, unannotated required scalar blocks, helper
initializer eligibility, module-data captures, borrowed module storage, package/manifest
resolution, full required evaluation and generic specialization, public FFI, wider
ownership/cleanup, executable networking, public artifacts/replay and LSP remain separate. Host execution does not qualify minimum
platforms or bundled distributions. Toolchain: Rust 1.98.1 and LLVM/Clang/LLD/LLVM ar 22.1.8.

## Next steps

Annotated record construction is complete across `66bd7c1` (output state), `3fe7299`
(construction) and `345f98b` (integration). The guide and full compiler gate pass.

Next, plan primary record composition from existing eligible records inside annotated
required constructors. Read ordinary composition rules and reuse `records/build.rs`,
required record sources and checked field slots. Record ordered commits first:

1. Share field-slot insertion/type checking where useful, preserving explicit named
   emission bindings, duplicate/init diagnostics and existing scalar/type behavior.
2. Support `-> source` for eligible immutable record values, mapping their field names
   into the expected shape and charging source evidence once before forwarding fields.
   Preserve ordinary composition's lexical-binding rules and whole-module namespace gates.
3. Verify partial/full composition, nested records, collisions, budgets and staging;
   update guides/handoffs and run `python3 -B tools/verify.py --compiler` across the series.

Keep unannotated constructors, inline partial composition, explicit nonrecord primaries,
skipped documented declarations, fallback arms, mutable/float/text/reference fields,
whole-module records, helpers and borrowed storage separate. Do not push.
