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

Explicit record annotations now construct immutable records in required scopes.
Typed named fields, nested records, source copies and matcher-selected emissions retain
scope, initialization checks and budgets without runtime storage. Field names become
available after initialization; explicit primary composition remains gated.

Commits: `66bd7c1` (result state), `3fe7299` (construction), `345f98b` (integration).
The [supported guide](compiler/docs/COMPUTED_TYPES.md#annotated-record-construction)
covers expected shapes, field diagnostics, scope and remaining boundaries.

Net/HTTP/TLS still needs broader generic-type/I/O/task foundations. Full v0.0.1
release qualification remains incomplete.

## Actual validation

- `python3 -B tools/verify.py --compiler`: all ten checks passed, including 1587
  Rust tests (773 library/814 native), 20 Python tests, fmt, Clippy and build.
- Four constructor checker tests and four native groups pass, covering fields, kinds,
  initialization, nested copies, shape limits, source errors, documentation and startup.
- The guide prints `7` in debug/release. Conformance: 10 passed, 13 unsupported,
  0 failed in both profiles. Local links and catalog/schema checks pass.
  Log: `/tmp/meowy-record-construction-gate.log`.
- Runtime implementation, reference fixtures and dependencies are unchanged.
  Editor and separate runtime/sanitizer gates were not rerun; full release qualification
  remains open. Evaluator and record-shape bounds are not native support guarantees.

## Area handoff

| Area | Current boundary |
| --- | --- |
| Compiler | Annotated required records construct checked fields without runtime locals. |
| Documentation tooling | Constructed signatures and file graphs are checked; multi-file doc commands/indexes remain separate. |
| Editor integration | Pointer syntax previously passed Vim/Neovim; unchanged here. |
| Standard library | Net specifies peers and HTTP adapters; type/I/O/task foundations precede implementation. |
| Runtime and release | File-site runtime helpers passed native probes; platform/distribution qualification remains open. |

## Next steps

1. Plan primary composition from eligible records inside annotated required constructors,
   preserving field checks, lexical scope, initialization and budgets. See the
   [compiler handoff](compiler/STATUS.md#next-steps).
2. Keep whole-module records, unannotated constructors, skipped documented declarations,
   fallback arms and helpers separate. Commit validated slices using [AGENTS.md](AGENTS.md);
   do not push.
