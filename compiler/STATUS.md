# Compiler handoff and work tracker

Updated: 2026-09-12. Boolean equality predicate evidence is in progress.
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

`inputs/predicates.rs` proves boolean literals/locals, negation, short-circuit logic and
exact-width integer comparisons. Runtime parameters, mutable or effectful initializers
and evaluated helpers remain unavailable. Required reads across function scopes do
not grant ordinary runtime captures. Repeated field comparisons remain unstable flow
atoms; an immutable boolean binding can supply complementary matcher arms.

Record initializers retain unit primaries and immutable integer/record fields. Complete
ancestor work/errors survive field paths, subrecord aliases and local compositions.
`inputs/records/build.rs` accumulates selected statements; `Record::failed` keeps
error-only paths without inventing branch values. Evaluated unsupported siblings or
tails prevent eligibility for every selected leaf; unrelated file initialization does not.

Direct module compositions forward independently eligible named inputs and integer
primaries. Local-record compositions export a checked record ID and bounded field path;
`inputs/records/paths.rs::input_path` retains paths/work through facades. Synthetic
module namespaces never gain whole-record eligibility. Mixed module primaries still
need integer context in required scratch/arithmetic; ordinary aliases keep record identity.
Identity-only module aliases/imports emit no runtime binding and may read eligible
exports inside scalar initializers. Whole-module record construction remains gated.
Privacy, collisions, widths, startup order and borrowed-export gates remain intact.

Every required read charges retained work again; independent roots reset their budget.
Scalar blocks/predicates share 64 active levels and 4096 visits. Records retain 256 total
fields and 32 record/evidence levels; type traversal retains 16384 nodes. Frontend and
ownership limits remain independent; these bootstrap limits are not language E220 counters.

Unsupported record shapes, selected standalone expression blocks, named/outer emissions,
mutable scratch and helpers remain unavailable inside scalar initializers. Boolean/float/text
comparisons, boolean record-field/export inputs, required boolean scratch and conditional
module exports remain separate. See [COMPUTED_TYPES.md](docs/COMPUTED_TYPES.md#block-initializers).

## Actual validation

- `2e017ef`: bounded record scratch; all 741 library/751 native tests, fmt and Clippy
  passed. Log: `/tmp/meowy-record-scratch-tests.log`. Native cases cover integer/boolean
  results, field order, projected aliases, compositions, branch scope, unused records,
  ancestor errors/effects and immutable supported shapes.
- The 256/257-total-field boundary passes checker-only tests for integer and boolean
  initializers. Frontend/evaluator/ownership limits remain independent. Unreachable
  typed aliases preserve nested errors; erased inline field identities remain limited.
- Five record-scratch native groups pass. Debug/release integration verifies retained
  ancestor/projection/tail work on repeated reads, independent roots, imports, silent
  check/build and startup. Identity-only module aliases/imports remain valid; actual
  whole-module record construction rejects required evaluation.
- The guide example prints `7` in debug/release. Extracted file:
  `/tmp/meowy-record-scratch-doc-l35p7qyk/main.mwy`.
- `python3 -B tools/verify.py --compiler`: all ten checks passed, including 741
  library/753 native Rust tests (1494 total), 20 Python tests, fmt, Clippy and build.
  Log: `/tmp/meowy-record-scratch-gate.log`.
- Conformance: 10 passed, 13 unsupported, 0 failed in debug/release. Local links,
  catalog/schema and whitespace checks passed. Full release qualification remains open.
- Runtime implementation, reference fixtures and dependencies are unchanged. Editor
  and separate runtime/sanitizer gates were not rerun; full release qualification is open.

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

Whole-record module inputs, conditional module exports, boolean-field/export inputs, helper
initializer eligibility, module-data captures, borrowed module storage, package/manifest
resolution, full required evaluation and generic specialization, public FFI, wider
ownership/cleanup, executable networking, public artifacts/replay and LSP remain separate. Host execution does not qualify minimum
platforms or bundled distributions. Toolchain: Rust 1.98.1 and LLVM/Clang/LLD/LLVM ar 22.1.8.

## Next steps

Inspection: typed boolean evidence already retains values, first errors and work.
Equality must read both operands in order unless the left fails; a false/true value
alone cannot skip the right operand as `&&`/`||` can. Reuse existing predicate recursion
and counters without changing runtime HIR or ordinary typing.

Dependency-ordered commits:

1. Complete: boolean `==`/`!=` uses ordered typed operand evidence. All 741 library/756
   native tests, fmt and Clippy passed. Log: `/tmp/meowy-boolean-equality-tests.log`.
   Truth tables, nested records/blocks, first errors and eager operand gates pass.
2. Add repeated operand-work and runtime/module-staging integration, update guides
   and both handoffs, then run `python3 -B tools/verify.py --compiler`.

Keep boolean record fields/module exports, required boolean/record scratch, float/text
comparisons, standalone expression blocks, helpers, packages and borrowed storage
separate. Do not push.
