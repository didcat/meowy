# meowy project status

Updated: 2026-09-21. This is the current project handoff; Git retains prior work.
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

Initializer evidence now preserves a proof-dependency mark through evaluated
scalar arithmetic, copies, conditions, tails and record projections. Five seeded
checker groups and all 977 then-current library tests pass. The required-input
boundary now rejects marked evidence with E225; three additional focused checker
groups pass. All ten compiler checks pass. Source-level flag projections
remain gated. Base-graph propagation through both successors, calls and mutable
state, plus observation-availability enforcement, still precede evaluated query outcomes.

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

- `python3 -B tools/verify.py --compiler`: all ten checks passed, including 980
  library/903 native tests (1883 total), 20 Python harness tests, fmt, Clippy,
  build, links and catalog/schema checks. Conformance: 10 passed, 13 unsupported,
  0 failed in debug/release. Log: `/tmp/meowy-proof-dependencies-gate.log`.
- Eight new seeded checker groups cover dependency propagation through evaluated
  scalar/record evidence, E225 required-input rejection, fixed signatures,
  original errors, budget precedence and root restoration. No source-level proof
  flag is enabled; full base-graph/control propagation remains unfinished.
- Proof outcomes remain B001-gated. Runtime source, reference fixtures,
  dependencies and versions are unchanged; editor and separate runtime/sanitizer
  gates were not rerun. Full release qualification remains open.

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
