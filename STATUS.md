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

Required type blocks now accept immutable boolean scratch from literals, eligible
locals, record fields and module named/primary inputs. Static aliases retain values
without runtime storage; source reads preserve work and original errors. Function-scoped
required reads retain the existing runtime capture and initialization rules.

Commits: `1e997b0` (paths/work), `ab37924` (local scratch), `a9bd0dc` (fields),
`27bca1f` (primaries). Integration tests and the
[supported guide](compiler/docs/COMPUTED_TYPES.md#boolean-scratch) cover aliases,
root budgets, dependency errors and silent staging.

Net/HTTP/TLS still needs broader generic-type/I/O/task foundations. Full v0.0.1
release qualification remains incomplete.

## Actual validation

- `python3 -B tools/verify.py --compiler`: all ten checks passed, including 1532
  Rust tests (748 library/784 native), 20 Python tests, fmt, Clippy and build.
- Four focused checker tests and seven native groups pass, including work/depth,
  static-alias reuse, function scopes, dependency errors and silent diamond startup.
- The guide prints `false` in debug/release. Conformance: 10 passed, 13 unsupported,
  0 failed in both profiles. Local links and catalog/schema checks pass.
  Log: `/tmp/meowy-required-booleans-gate.log`.
- Runtime implementation, reference fixtures and dependencies are unchanged.
  Editor and separate runtime/sanitizer gates were not rerun; full release qualification
  remains open. Evaluator and record-shape bounds are not native support guarantees.

## Area handoff

| Area | Current boundary |
| --- | --- |
| Compiler | Required boolean scratch retains source evidence without runtime storage. |
| Documentation tooling | Constructed signatures and file graphs are checked; multi-file doc commands/indexes remain separate. |
| Editor integration | Pointer syntax previously passed Vim/Neovim; unchanged here. |
| Standard library | Net specifies peers and HTTP adapters; type/I/O/task foundations precede implementation. |
| Runtime and release | File-site runtime helpers passed native probes; platform/distribution qualification remains open. |

## Next steps

1. Plan boolean operators in required type blocks, preserving operand typing, source
   errors, short-circuit behavior and shared root budgets. See the
   [compiler handoff](compiler/STATUS.md#next-steps).
2. Keep conditional type selection, wider comparisons, inline boolean blocks, required
   record scratch, whole-module inputs, conditional exports, helpers and borrowed storage
   separate. Commit validated slices using [AGENTS.md](AGENTS.md); do not push.
