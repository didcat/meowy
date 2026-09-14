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

- `python3 -B tools/verify.py --compiler`: all ten checks passed, including 1640
  Rust tests (805 library/835 native), 20 Python tests, fmt, Clippy and build.
- Five checker tests and three native groups cover relations, widths, exact work,
  structural bounds, source errors, skipped values, documentation and staging.
- Block-comparison guide prints `7` in debug/release. Conformance: 10 passed,
  13 unsupported, 0 failed in both profiles. Local links and catalog/schema checks pass.
  Log: `/tmp/meowy-block-comparisons-gate.log`.
- Runtime implementation, reference fixtures and dependencies are unchanged. Editor
  and separate runtime/sanitizer gates were not rerun; full release qualification
  remains open. Evaluator/record bounds are not native support guarantees.

## Area handoff

| Area | Current boundary |
| --- | --- |
| Compiler | Integer block comparisons preserve short circuits, deferred widths and shared budgets. |
| Documentation tooling | Constructed signatures and file graphs are checked; multi-file doc commands/indexes remain separate. |
| Editor integration | Pointer syntax previously passed Vim/Neovim; unchanged here. |
| Standard library | Net specifies peers and HTTP adapters; type/I/O/task foundations precede implementation. |
| Runtime and release | File-site runtime helpers passed native probes; platform/distribution qualification remains open. |

## Current documentation task

Specify `@"proof"` as a strict, compile-time standard-library query package. No compiler
implementation is part of this task. Keep the completed compiler handoff above intact.

Dependency-ordered documentation commits:

1. Complete: `docs/reference/stdlib/proof.md` defines staging, result types, inspection,
   assertions/expected-status tests, composition and value/bounds APIs. E223/E224 are
   registered with their owning contracts. Local links and whitespace checks pass;
   the proof API has not been compiled or executed. Committed as `12d21df`.
2. Complete: type/place query contracts, snapshot invalidation, fixed abstract domains,
   transfers/joins/loop handling, outcome rules and phase ordering are specified. E225
   rejects backward proof dependencies. Local links and whitespace checks pass.
   Remaining: testing workflows, budgets, diagnostic precedence and qualification.
3. Complete budgets, diagnostics, examples and qualification obligations; register codes,
   connect library/reference indexes and the compile-time contract, and run the default
   repository verifier. Report documentation validation separately from implementation.

## Next steps

1. Complete the proof reference and its documentation checks in the slices above.
2. Resume the compiler's planned boolean-result blocks in required logical operators
   after this specification task; see the [compiler handoff](compiler/STATUS.md#next-steps).
3. Do not push or claim the new proof API is implemented or release-qualified.
