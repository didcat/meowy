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

Required block equality now supports integer and boolean results with deferred kind
selection and exact widths. Equality evaluates both sides once; outer logical short
circuits still skip blocks. Scope, source errors and shared budgets are preserved
without runtime storage. Ordered comparisons remain integer-only.

Commits: `60bc234` (shared operands), `95d3180` (equality), `a414ec7` (integration).
The compiler guide now covers scalar equality and the previously deferred logical
block operands. The user's documentation formatting is preserved.

Net/HTTP/TLS still needs broader generic-type/I/O/task foundations. Full v0.0.1
release qualification remains incomplete.

## Actual validation

- `python3 -B tools/verify.py --compiler`: all ten checks passed, including 1656
  Rust tests (815 library/841 native), 20 Python tests, fmt, Clippy and build.
- Four equality checker tests and three native groups cover kinds, exact work, eager
  reads, scope, source errors, aliases and module/function staging.
- Logical/equality guides each print `7` in debug/release. Conformance: 10 passed,
  13 unsupported, 0 failed in both profiles. Local links and catalog/schema checks pass.
  Log: `/tmp/meowy-block-equality-gate.log`.
- Proof remains specification-only; its examples were not compiled or executed.
- Runtime implementation, reference fixtures and dependencies are unchanged. Editor
  and separate runtime/sanitizer gates were not rerun; full release qualification
  remains open. The user's documentation formatting is preserved.

## Area handoff

| Area | Current boundary |
| --- | --- |
| Compiler | Scalar block equality preserves deferred kinds, eager reads and shared budgets. |
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

Commits: `12d21df` (results/value APIs), `415f7c4` (ownership/canonical analysis),
`947c56b` (testing, work accounting, qualification and cross-links).
The docs, library index, compile-time rules, testing and memory APIs link the contract.
Default repository verification and whitespace checks pass. The package, observation
parameter mode, proof-only descriptor unions and executable proof fixtures are not
implemented; no runtime/compile-time execution of proof examples was claimed.

## Next steps

1. Investigate explicit `core.Type` bindings in required scopes while retaining their
   compile-time-only representation; see the [compiler handoff](compiler/STATUS.md#next-steps).
2. Implement proof only in separately planned slices against its qualification contract.
   Do not push or claim full release qualification.
