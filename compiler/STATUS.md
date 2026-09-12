# Compiler handoff and work tracker

Updated: 2026-09-12. Branch-aware boolean initializer inputs are implemented.
All ten compiler gate checks passed. Full v0.0.1 remains incomplete.
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

`inputs/blocks/booleans.rs` evaluates boolean initializers using immutable eligible
integer/boolean bindings and one selected boolean primary. Nested
blocks, aliases, comparisons and imported integer leaves reuse typed input evidence.
Successful blocks retain every visited tail statement; the first evaluated failure
stops evaluation and clears the result value. Integer-block eligibility is unchanged.

`boolean_stmts` shares primary/error/work state across selected branches and restores
branch-local `Sources`. Conditions always retain their evaluated work; skipped bodies
add none. Predicate failures stop selection without inventing a primary; selected
body/tail failures keep their first source error even after an earlier emission.
Ordinary duplicate/missing primary and lexical-scope checks remain E205/E204/E201.
Runtime HIR and ordinary flow, type and ownership checking remain unchanged.

`boolean_input` uses the declared binding type, allowing error-only `never` HIR from
unreachable boolean blocks. Nested unreachable blocks need explicit boolean types
when inference loses their result kind. Failures retain their original E107 span
through boolean aliases and record/subrecord projections. Inline predicate blocks
and skipped short-circuit block operands preserve evaluation order and work.

`Input<T>` shares source-error and work accounting between integer and boolean
values. `inputs/predicates.rs` proves literals, immutable boolean locals, negation,
short-circuit logic and exact-width integer comparisons. `Checker.bool_inputs`
retains ordinary binding evidence; `Sources.booleans` retains scoped record scratch.
Runtime constant folding stays separate from eligibility.

`inputs/records/build.rs` accumulates selected statements and restores branch-local
evidence after each branch. Predicate operands and selected unused bindings retain
work; skipped operands/branches contribute none. An evaluated predicate failure stops
selection, and `Record::failed` keeps error-only paths for the complete checked shape.
Aliases and subrecord projections therefore retain the first E107 source span without
inventing a branch value. Selected effectful or mutable inputs remain unavailable.

Original runtime HIR, initialization, capture gates and ordinary flow/type/ownership
checks are unchanged. Repeated field comparisons remain unstable flow atoms; a saved
immutable boolean can supply complementary matcher arms without changing that analysis.
Record branches share the 32-level record-evidence recursion bound. Boolean branches,
nested blocks and predicates share 64 active levels and 4096 visits. Nested record
and branch evaluation shares depth with its containing evaluator.

Eligible unit-primary local records now preserve computed-input evidence through
composition, including inline sources, aliases, projected subrecords and extensions
with named fields. `inputs/records/build.rs` recognizes the checked temporary Bind plus
unit-primary/field projection HIR and retains complete ancestor eligibility, errors
and work. The runtime HIR and evaluation order are unchanged.

Direct top-level local-record compositions export fields through a shared checked
record ID and a bounded field path. `exports.rs::composed_record_inputs` records the
source evidence; `inputs/records/paths.rs::input_path` combines the retained path with
an importer projection. Subsequent module compositions preserve that identity/path
and add forwarding work. Every required read charges all retained work again.

File-module namespace eligibility stays distinct: direct module composition forwards
individually eligible exports and an eligible integer primary. It never grants
whole-record eligibility to a synthetic namespace. A local record constructed from
individual eligible exports can qualify. Mutable, effectful or unsupported siblings
and tails evaluated inside a record prevent eligibility for all composed fields;
unrelated file initialization does not. Privacy, exact widths, ordinary collision
checks, startup order, runtime captures and borrowed-export gates remain intact.

Required integer primary projections of scalar/mixed modules retain their prior
context rules: explicit integer scratch/arithmetic works; unannotated mixed-module
scratch and bare mixed-module extents remain unavailable. Required reads inside
functions work without granting ordinary runtime captures.

Nested records retain unit primaries, immutable integer/record fields, 256 total fields
and 32 record levels. Declared record shapes preserve known E107 source spans on
unreachable paths; unannotated unreachable shapes can lose field identity and remain
B001. Other bootstrap limits remain 4096 visits, 64 resolver/validation levels and
16384 type nodes; these are not language E220 counters. Native ownership analysis
may exhaust its own budget before a shape reaches its input-field limit.

Selected standalone expression blocks, named/outer emissions and record scratch
inside boolean blocks remain gated.
Boolean/float/text comparisons, boolean record-field/export inputs, boolean required
scratch, conditional module exports and integer-block branches remain separate, along
with helper purity and full required evaluation. See
[COMPUTED_TYPES.md](docs/COMPUTED_TYPES.md#conditional-record-initializers).

## Actual validation

- `2ffb238`: boolean accumulator extraction; all 736 library/733 native tests, fmt
  and Clippy passed unchanged. Log: `/tmp/meowy-boolean-accumulator-tests.log`.
- `c67bd12`: boolean branch evaluation; all 737 library/736 native tests, fmt and
  Clippy passed. Log: `/tmp/meowy-boolean-branch-tests.log`. Native cases cover both
  primary choices, scope, selected/skipped effects and errors, and E205/E204/E201.
- The 62/63 branch-depth evaluator boundary passes direct HIR tests. The equivalent
  source probe hit the earlier frontend expression limit; it is not native support
  at that depth. Existing record shape/ownership limits remain independent as well.
- Five focused branch groups pass. Debug/release integration verifies condition work
  even when false, selected versus skipped tail work, independent required roots,
  silent check/build and dependency startup through module forwarding.
- The guide example prints `7` in debug/release. Extracted file:
  `/tmp/meowy-boolean-branch-doc-tlwju30r/main.mwy`.
- `python3 -B tools/verify.py --compiler`: all ten checks passed, including 737
  library/738 native Rust tests (1475 total), 20 Python tests, fmt, Clippy and build.
  Log: `/tmp/meowy-boolean-branch-gate.log`.
- Conformance: 10 passed, 13 unsupported, 0 failed in debug/release. Local links,
  catalog/schema and whitespace checks passed. Full release qualification remains open.
- Runtime implementation, reference fixtures and dependencies are unchanged. Editor
  and separate runtime/sanitizer gates were not rerun; release qualification is open.

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

Integer block evidence lives in `src/check/inputs/blocks.rs`; boolean block state and
branch traversal live in `src/check/inputs/blocks/booleans.rs`. Required-only constant
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

1. Plan conditional integer-block evidence in `inputs/blocks.rs`, using the proven
   boolean branch/predicate approach. Preserve selected integer width/value, primary
   uniqueness, tail work/errors and branch-local scope. Inspect the existing integer
   failure path before sharing an accumulator; separate structural changes from
   new accepted behavior and retain integer-only scratch unless separately planned.
2. Keep selected standalone expression blocks, record scratch, named/outer emissions,
   boolean field/export inputs, required boolean scratch, helpers, packages and borrowed
   storage separate. Record reviewable slices before implementing; do not push.
