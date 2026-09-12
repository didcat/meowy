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

Boolean initializers now select branches using eligible predicate evidence, sharing
one primary result while preserving lexical scope and first errors. Conditions and
selected tail statements retain work; skipped bodies add none. Module forwarding,
runtime HIR and ordinary flow/type/ownership checks remain unchanged.

Commits: `2ffb238` (accumulator extraction), `c67bd12` (branch evaluation).
Integration checks and the supported guide are complete. Integer-block branches and
selected standalone expression blocks remain separate. See
[the supported slice](compiler/docs/COMPUTED_TYPES.md#boolean-block-initializers).

Net/HTTP/TLS still needs broader generic-type/I/O/task foundations. Full v0.0.1
release qualification remains incomplete.

## Actual validation

- `python3 -B tools/verify.py --compiler`: all ten checks passed, including 1475
  Rust tests, 20 Python tests, fmt, Clippy and build.
- Five focused branch groups pass. Execution, work and module-staging probes run
  in debug/release; scope/error checks also pass. The guide prints `7` in both profiles.
- Evaluator depth is checked directly; the equivalent deep source reaches an earlier
  frontend limit. Evaluator and record-shape bounds are not native support guarantees.
- Conformance: 10 passed, 13 unsupported, 0 failed in debug/release. Local links
  and catalog/schema checks passed. Log: `/tmp/meowy-boolean-branch-gate.log`.
- Runtime implementation, reference fixtures and dependencies are unchanged.
  Editor and separate runtime/sanitizer gates were not rerun; full release qualification
  remains open.

## Area handoff

| Area | Current boundary |
| --- | --- |
| Compiler | Boolean branches retain selected primaries, scope, errors and work. |
| Documentation tooling | Constructed signatures and file graphs are checked; multi-file doc commands/indexes remain separate. |
| Editor integration | Pointer syntax previously passed Vim/Neovim; unchanged here. |
| Standard library | Net specifies peers and HTTP adapters; type/I/O/task foundations precede implementation. |
| Runtime and release | File-site runtime helpers passed native probes; platform/distribution qualification remains open. |

## Next steps

1. Plan conditional integer-block evidence with exact widths, selected-primary,
   scope and tail-work/error proofs. See the [compiler handoff](compiler/STATUS.md#next-steps).
2. Preserve package, borrowed-export and ownership gates. Keep record scratch,
   selected standalone expression blocks, boolean field/export inputs and required
   boolean scratch separate. Commit validated slices using [AGENTS.md](AGENTS.md); do not push.
