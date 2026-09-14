# Compiler handoff and work tracker

Updated: 2026-09-13. Bare type-block equality is implemented.
The final compiler gate and guide execution pass. Full v0.0.1 remains incomplete.
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

## Active plan: bounded type subtraction

The contract defines `!<U>` on compile-time type sets. `hir::Type::subtract`
already handles the supported normalized concrete alternatives; literal subtypes
and broad bases such as `error` remain unsupported. Reuse `ExprKind::Binary` with
operator `!`, wrapped by `TypeKind::Computed` in annotations. Existing graph/docs
visitors then retain both operands and original spans without new AST variants.

Dependency-ordered commits:
1. Parse subtraction suffixes in annotations and value expressions; preserve unary
   `!`/`!=`, bound chains and test spans/shape. Evaluation remains gated in this slice.
2. Evaluate supported concrete operands left-to-right with shared work/node budgets;
   integrate required bindings and equality forms, with focused checker/native tests.
3. Verify imports, original errors, repeated work, skipped constructors, scope and
   runtime value preservation with integration regressions.
4. Document supported syntax/boundaries, execute the guide, update both handoffs and
   run the full compiler gate. Commit each validated slice.

The first bounded grammar accepts one explicit bracket per subtraction suffix;
use a named union or computed type inside that bracket to remove multiple members.
Unparenthesized union suffixes after subtraction remain gated rather than assigning
an unconfirmed mixed union/subtraction precedence. Repeated subtraction is left-to-right.
Parser slice validation: 843 library/866 native tests, fmt and Clippy pass.
Three new parser groups cover query/literal/annotation forms, Unicode-offset spans,
left-to-right chains, unchanged unary `!`/`!=`, chain limits and the mixed-union gate.
Annotation lookahead now retains subtraction budget diagnostics. Evaluation remains
B001 until the next slice. Log: `/tmp/meowy-subtraction-parser.log`.
Parser committed as `bc3030c`. The evaluator reuses bounded type operands and
normalized `Type::subtract`, including bare type blocks. Required bindings and
equality forms recognize subtraction. Two checker/one native groups cover normalized
sets, widths, mutability, references, capacities, empty results, queries and runtime
value rejection. Broad bases/literal subtypes retain their existing gates.

All 845 library/867 native tests, fmt and Clippy pass.
Log: `/tmp/meowy-subtraction-evaluator.log`. No outstanding failures.
Evaluator committed as `ae68e3b`. Integration tests now cover form-only skips,
work/node/depth boundaries and state restoration, facade identity/privacy, removed
operand import discovery/startup, source failure order and repeated input/root costs.
All seven library/five native focused groups, fmt and Clippy pass, including
selected operand scope and runtime/helper boundaries. Log:
`/tmp/meowy-subtraction-integration.log`. No outstanding failures. Next: commit
integration, document the supported subset and run the final compiler gate.

## Current compiler slice

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
helpers remain separate. Proof remains specification-only.

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

- `python3 -B tools/verify.py --compiler`: all ten checks passed, including 840
  library/866 native tests (1706 total), 20 Python tests, fmt, Clippy, build, links
  and catalog/schema checks. Conformance: 10 passed, 13 unsupported, 0 failed in
  debug/release. Log: `/tmp/meowy-type-blocks-gate.log`.
- Five checker/five native block-equality groups cover inferred type identity,
  contextual scalar kinds, E207 type emissions, skipped structure/values, source
  order, imported privacy/startup, repeated input costs, independent roots and
  work/node/depth failures with restored state. Focused logs:
  `/tmp/meowy-type-blocks-slice1.log`, `/tmp/meowy-type-blocks-slice2.log`.
- The type-producing block equality guide prints `7` in debug/release.
  Extracted source: `/tmp/meowy-type-blocks-guide.mwy`.
- Runtime implementation, reference fixtures, dependencies and release versions are
  unchanged. Editor and separate runtime/sanitizer gates were not rerun. Full release
  qualification remains open; proof examples remain unimplemented/unexecuted.

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

The bare type-block equality series is complete; no outstanding failures remain.

1. Investigate bounded type subtraction against `docs/reference/types.md` and
   `docs/reference/compile-time.md`. Trace `parser/types.rs` (currently unsupported),
   expression parsing, `type_values.rs` and `hir::Type::subtract`; plan syntax/AST,
   required evaluation and integration as separate reviewable slices. Preserve
   normalized identity, left-to-right source/work evidence and short-circuit behavior.
2. Implement proof only with a separate dependency-ordered plan against its
   qualification contract. Keep first-class metatype values, type-of-type queries,
   runtime type containers and type-producing/generic helpers separate.

Do not push or bump release versions here.
