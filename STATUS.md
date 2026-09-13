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

Eligible boolean module primaries now supply predicates alongside named boolean inputs.
Typed evidence retains values, source errors and work through scalar copies and facade
chains. Boolean operations project mixed-module primaries; aliases and type queries
keep named fields. Privacy, runtime capture gates and initialization remain intact.

Commits: `39aacdd` (typed evidence storage), `77ca671` (boolean primary inputs).
Integration tests and the [supported guide](compiler/docs/COMPUTED_TYPES.md#boolean-primary-imports)
cover repeated reads, independent roots, dependency errors and startup behavior.

Net/HTTP/TLS still needs broader generic-type/I/O/task foundations. Full v0.0.1
release qualification remains incomplete.

## Actual validation

- `python3 -B tools/verify.py --compiler`: all ten checks passed, including 1521
  Rust tests (744 library/777 native), 20 Python tests, fmt, Clippy and build.
- Seven boolean-primary groups pass in debug/release, including independent exports,
  repeated work, silent staging, diamond initialization and runtime startup failure.
  The guide prints `flags`, `7`, `ready` in both profiles.
- Conformance: 10 passed, 13 unsupported, 0 failed in debug/release. Local links
  and catalog/schema checks passed. Log: `/tmp/meowy-boolean-primary-gate.log`.
- Runtime implementation, reference fixtures and dependencies are unchanged.
  Editor and separate runtime/sanitizer gates were not rerun; full release qualification
  remains open. Evaluator and record-shape bounds are not native support guarantees.

## Area handoff

| Area | Current boundary |
| --- | --- |
| Compiler | Named and primary boolean module inputs retain source evidence through predicates. |
| Documentation tooling | Constructed signatures and file graphs are checked; multi-file doc commands/indexes remain separate. |
| Editor integration | Pointer syntax previously passed Vim/Neovim; unchanged here. |
| Standard library | Net specifies peers and HTTP adapters; type/I/O/task foundations precede implementation. |
| Runtime and release | File-site runtime helpers passed native probes; platform/distribution qualification remains open. |

## Next steps

1. Plan immutable boolean scratch in required type blocks, preserving typed input
   evidence, source errors and root budgets. This precedes conditional type selection.
   See the [compiler handoff](compiler/STATUS.md#next-steps).
2. Preserve package, borrowed-export and ownership gates. Keep whole-module inputs,
   conditional exports, required record scratch and wider comparison types separate.
   Commit validated slices using [AGENTS.md](AGENTS.md); do not push.
