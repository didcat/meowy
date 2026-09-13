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

Required type blocks now select scalar, record and list types with boolean matchers.
Selected branches share the primary result and keep local scope; skipped expressions
are not evaluated. Condition work, tail checks and original source errors are retained.
Module initialization and runtime capture restrictions remain intact.

Commits: `12f4799` (statement state), `b180fcc` (selection), `161489e` (integration).
The [supported guide](compiler/docs/COMPUTED_TYPES.md#conditional-type-selection)
covers nested construction, skipped-branch rules and documentation boundaries.

Net/HTTP/TLS still needs broader generic-type/I/O/task foundations. Full v0.0.1
release qualification remains incomplete.

## Actual validation

- `python3 -B tools/verify.py --compiler`: all ten checks passed, including 1561
  Rust tests (761 library/800 native), 20 Python tests, fmt, Clippy and build.
- Five checker tests and five native groups pass, covering selection, scope, budgets,
  skipped expressions, dependency errors, exported types and silent initialization.
- The guide prints `7` in debug/release. Conformance: 10 passed, 13 unsupported,
  0 failed in both profiles. Local links and catalog/schema checks pass.
  Log: `/tmp/meowy-conditional-types-gate.log`.
- Runtime implementation, reference fixtures and dependencies are unchanged.
  Editor and separate runtime/sanitizer gates were not rerun; full release qualification
  remains open. Evaluator and record-shape bounds are not native support guarantees.

## Area handoff

| Area | Current boundary |
| --- | --- |
| Compiler | Boolean matchers select required types with scoped bodies and shared budgets. |
| Documentation tooling | Constructed signatures and file graphs are checked; multi-file doc commands/indexes remain separate. |
| Editor integration | Pointer syntax previously passed Vim/Neovim; unchanged here. |
| Standard library | Net specifies peers and HTTP adapters; type/I/O/task foundations precede implementation. |
| Runtime and release | File-site runtime helpers passed native probes; platform/distribution qualification remains open. |

## Next steps

1. Plan explicitly annotated integer/boolean blocks in required type bindings,
   preserving result kinds, lexical scope, tail checks and shared budgets. See the
   [compiler handoff](compiler/STATUS.md#next-steps).
2. Keep skipped documented declarations, standalone branch blocks, fallback arms,
   float/text comparisons, required record scratch and helpers separate. Commit
   validated slices using [AGENTS.md](AGENTS.md); do not push.
