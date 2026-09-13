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

Required type blocks now support integer equality and ordering, including arithmetic,
field and module-primary operands. Exact widths and literal ranges are checked even
when comparisons are skipped; evaluated reads preserve work and source failures.
Runtime capture restrictions and normal initialization remain intact.

Commits: `0a0e600` (shared integer materialization), `e290793` (operand checks),
`917dd51` (evaluation). Integration tests and the
[supported guide](compiler/docs/COMPUTED_TYPES.md#integer-comparisons) cover budgets,
skipped arithmetic, dependency errors and silent staging.

Net/HTTP/TLS still needs broader generic-type/I/O/task foundations. Full v0.0.1
release qualification remains incomplete.

## Actual validation

- `python3 -B tools/verify.py --compiler`: all ten checks passed, including 1551
  Rust tests (756 library/795 native), 20 Python tests, fmt, Clippy and build.
- Focused comparison integration passes: two checker tests and five native groups
  cover exact widths, arithmetic, source work, operand error order and initialization.
- The guide prints `true` in debug/release. Conformance: 10 passed, 13 unsupported,
  0 failed in both profiles. Local links and catalog/schema checks pass.
  Log: `/tmp/meowy-required-comparisons-gate.log`.
- Runtime implementation, reference fixtures and dependencies are unchanged.
  Editor and separate runtime/sanitizer gates were not rerun; full release qualification
  remains open. Evaluator and record-shape bounds are not native support guarantees.

## Area handoff

| Area | Current boundary |
| --- | --- |
| Compiler | Required integer comparisons retain exact widths and evaluated source work. |
| Documentation tooling | Constructed signatures and file graphs are checked; multi-file doc commands/indexes remain separate. |
| Editor integration | Pointer syntax previously passed Vim/Neovim; unchanged here. |
| Standard library | Net specifies peers and HTTP adapters; type/I/O/task foundations precede implementation. |
| Runtime and release | File-site runtime helpers passed native probes; platform/distribution qualification remains open. |

## Next steps

1. Plan conditional type selection with checked boolean predicates, scoped branches,
   primary-result rules and shared budgets. See the
   [compiler handoff](compiler/STATUS.md#next-steps).
2. Keep float/text comparisons, inline boolean blocks, whole-module records, conditional
   module exports, required record scratch and helpers separate. Commit validated slices
   using [AGENTS.md](AGENTS.md); do not push.
