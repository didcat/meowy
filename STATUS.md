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

Eligible records now retain typed boolean leaves alongside integer leaves. Predicate
field reads preserve complete ancestor work/errors through aliases, scratch records,
projections and record-derived exports. Integer access refuses boolean leaves; runtime
HIR, shape bounds, privacy and ordinary ownership checks remain intact.

Commits: `bed0554` (typed storage), `631ede0` (boolean leaves), `998f029` (field lookup).
Integration checks and the supported guide are complete; see
[the supported slice](compiler/docs/COMPUTED_TYPES.md#typed-boolean-record-fields).

Net/HTTP/TLS still needs broader generic-type/I/O/task foundations. Full v0.0.1
release qualification remains incomplete.

## Actual validation

- `python3 -B tools/verify.py --compiler`: all ten checks passed, including 1507
  Rust tests, 20 Python tests, fmt, Clippy and build.
- Seven boolean-field groups pass. Debug/release integration checks ancestor work,
  true/false values, record-derived exports and silent staging. Private dependencies
  and direct scalar boolean-export gates pass. The guide prints `7` in both profiles.
- Conformance: 10 passed, 13 unsupported, 0 failed in debug/release. Local links
  and catalog/schema checks passed. Log: `/tmp/meowy-boolean-fields-gate.log`.
- Runtime implementation, reference fixtures and dependencies are unchanged.
  Editor and separate runtime/sanitizer gates were not rerun; full release qualification
  remains open. Evaluator and record-shape bounds are not native support guarantees.

## Area handoff

| Area | Current boundary |
| --- | --- |
| Compiler | Typed boolean record leaves retain ancestor evidence through predicates. |
| Documentation tooling | Constructed signatures and file graphs are checked; multi-file doc commands/indexes remain separate. |
| Editor integration | Pointer syntax previously passed Vim/Neovim; unchanged here. |
| Standard library | Net specifies peers and HTTP adapters; type/I/O/task foundations precede implementation. |
| Runtime and release | File-site runtime helpers passed native probes; platform/distribution qualification remains open. |

## Next steps

1. Plan direct immutable named boolean module inputs with retained source IDs,
   initializer evidence and forwarding work. See the [compiler handoff](compiler/STATUS.md#next-steps).
2. Preserve package, borrowed-export and ownership gates. Keep boolean module primaries,
   whole-module inputs, required boolean/record scratch and wider comparison types
   separate. Commit validated slices using [AGENTS.md](AGENTS.md); do not push.
