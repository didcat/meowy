# meowy project status

Updated: 2026-09-12. This is the current project handoff; Git retains prior work.
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

Required type blocks now support boolean negation, short-circuit logic and equality.
All operand names/types are checked; evaluation skips unnecessary right-hand reads.
Equality retains both-read costs and first-error order. Required reads preserve module
identity, runtime capture restrictions and normal initialization.

Commits: `e637137` (operand checks and logic), `0ff01b1` (equality).
Integration tests and the [supported guide](compiler/docs/COMPUTED_TYPES.md#boolean-operators)
cover truth values, root budgets, skipped inputs, dependency errors and silent staging.

Net/HTTP/TLS still needs broader generic-type/I/O/task foundations. Full v0.0.1
release qualification remains incomplete.

## Actual validation

- `python3 -B tools/verify.py --compiler`: all ten checks passed, including 1543
  Rust tests (753 library/790 native), 20 Python tests, fmt, Clippy and build.
- Nine focused checker tests and 13 native groups pass, including operand bounds,
  truth values, skipped work/errors, equality costs, scopes and dependency errors.
- The guide prints `true` in debug/release. Conformance: 10 passed, 13 unsupported,
  0 failed in both profiles. Local links and catalog/schema checks pass.
  Log: `/tmp/meowy-required-operators-gate.log`.
- Runtime implementation, reference fixtures and dependencies are unchanged.
  Editor and separate runtime/sanitizer gates were not rerun; full release qualification
  remains open. Evaluator and record-shape bounds are not native support guarantees.

## Area handoff

| Area | Current boundary |
| --- | --- |
| Compiler | Required boolean operators retain checked types and evaluated source work. |
| Documentation tooling | Constructed signatures and file graphs are checked; multi-file doc commands/indexes remain separate. |
| Editor integration | Pointer syntax previously passed Vim/Neovim; unchanged here. |
| Standard library | Net specifies peers and HTTP adapters; type/I/O/task foundations precede implementation. |
| Runtime and release | File-site runtime helpers passed native probes; platform/distribution qualification remains open. |

## Next steps

1. Plan integer comparisons in required type blocks, preserving exact operand widths,
   source errors, skipped evaluation and root budgets. See the
   [compiler handoff](compiler/STATUS.md#next-steps).
2. Keep float/text comparisons, conditional type selection, inline boolean blocks,
   whole-module records, conditional exports, required record scratch and helpers separate.
   Commit validated slices using [AGENTS.md](AGENTS.md); do not push.
