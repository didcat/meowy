# meowy project status

Updated: 2026-09-13. This is the current project handoff; Git retains prior work.
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

## Current milestone

The compiler entry guide is [compiler/README.md](compiler/README.md); detailed
guides live in `compiler/docs/`. The compiler root keeps `README.md`, `AGENTS.md`
and `STATUS.md`. Links and Cargo metadata follow this layout.

Required integer comparisons now accept inline block operands while preserving
short-circuiting. Known outer operands and statement forms are checked up front;
block-local values and unresolved widths wait until evaluation. Selected operands run
once in source order with shared budgets, original errors and no runtime storage.

Commits: `803eda8` (comparison extraction), `00f68eb` (block comparisons),
`287459d` (integration). The [supported guide](compiler/docs/COMPUTED_TYPES.md#integer-block-comparisons)
covers deferred widths, short circuits and remaining boundaries.

Net/HTTP/TLS still needs broader generic-type/I/O/task foundations. Full v0.0.1
release qualification remains incomplete.

## Actual validation

- Last compiler gate, before proof documentation: all ten checks passed, including 1640
  Rust tests (805 library/835 native), 20 Python tests, fmt, Clippy and build.
- Five checker tests and three native groups cover relations, widths, exact work,
  structural bounds, source errors, skipped values, documentation and staging.
- Block-comparison guide prints `7` in debug/release. Conformance: 10 passed,
  13 unsupported, 0 failed in both profiles. Local links and catalog/schema checks pass.
  Log: `/tmp/meowy-block-comparisons-gate.log`.
- Proof documentation: `python3 -B tools/verify.py` passed all four checks, including
  16 tooling tests, local links, catalog metadata and artifact schemas. No proof source
  was compiled or executed. Log: `/tmp/meowy-proof-reference-verify.log`.
- Runtime implementation, reference fixtures and dependencies are unchanged. Editor
  and separate runtime/sanitizer gates were not rerun; full release qualification
  remains open. Evaluator/record bounds are not native support guarantees.

## Area handoff

| Area | Current boundary |
| --- | --- |
| Compiler | Integer block comparisons preserve short circuits, deferred widths and shared budgets. |
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

Commits: `12d21df` (results/value APIs), `415f7c4` (ownership/canonical analysis).
The final slice completes testing, work accounting, qualification and cross-links.
The docs, library index, compile-time rules, testing and memory APIs link the contract.
Default repository verification and whitespace checks pass. The package, observation
parameter mode, proof-only descriptor unions and executable proof fixtures are not
implemented; no runtime/compile-time execution of proof examples was claimed.

## Next steps

1. Resume the compiler's planned boolean-result blocks in required logical operators;
   see the [compiler handoff](compiler/STATUS.md#next-steps).
2. Implement proof in separately planned slices against its qualification obligations,
   starting with fixed signatures/meta-values and frozen source-point analysis. Do not
   expose heuristic optimizer facts or treat unsupported diagnostics as proof outcomes.
3. Do not push or claim proof implementation or full release qualification.
