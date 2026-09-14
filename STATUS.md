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

Required type-value equality is in progress. The first slice adds normalized `==`/`!=`
for known type operands; 832 library/858 native tests, fmt and Clippy pass.
See the compiler handoff for the remaining source/work/facade integration and final gate.


The compiler entry guide is [compiler/README.md](compiler/README.md); detailed
guides live in `compiler/docs/`. The compiler root keeps `README.md`, `AGENTS.md`
and `STATUS.md`. Links and Cargo metadata follow this layout.

Named immutable `core.Type` exports now preserve concrete type payloads through
imports and explicit facades without runtime fields. Private names, type/value
namespaces, required budgets, source errors and module startup remain intact.
Ordinary metatype bindings continue to work at module/function scope without storage.

Commits: `93ef8d1` (exports), `6f9cd05` (facades), `bf76f46` (integration).
The [guide](compiler/docs/COMPUTED_TYPES.md#named-type-value-exports) covers syntax,
explicit forwarding and remaining boundaries.

Net/HTTP/TLS still needs broader generic-type/I/O/task foundations. Full v0.0.1
release qualification remains incomplete.

## Actual validation

- Rust integration: 830 library/856 native tests (1686 total), fmt and Clippy pass.
  Log: `/tmp/meowy-type-exports-slice3.log`.
- Three checker/eight new native groups cover type exports, privacy, collisions,
  required budgets, source errors, documentation and module startup/failure.
- `python3 -B tools/verify.py --compiler`: all ten checks passed, including 1686
  Rust tests, 20 Python tests, fmt, Clippy, build, links and catalog/schema checks.
  Conformance: 10 passed, 13 unsupported, 0 failed in debug/release.
  Log: `/tmp/meowy-type-exports-gate.log`. The named-export guide prints `7` in
  both profiles; source: `/tmp/meowy-type-exports-doc-nsew30hi/main.mwy`.
- Proof remains specification-only; its examples were not compiled or executed.
- Runtime implementation, reference fixtures and dependencies are unchanged. Editor
  and separate runtime/sanitizer gates were not rerun; full release qualification
  remains open. Project version and release tags were not changed.

## Area handoff

| Area | Current boundary |
| --- | --- |
| Compiler | Named core.Type exports retain compile-time identity through explicit facades. |
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

## Release naming

[VERSIONING.md](VERSIONING.md) defines the project release policy: keep major/minor
at zero, use the patch field as a chosen release identifier, make releases without a
fixed cadence, and label pre-releases as uncertain. Numbers do not promise compatibility
or chronological order. The existing full-release target remains an implementation
milestone, and protocol/schema/contract revision rules remain independent.
README and COMPILER link this policy. No package version or release tag was changed.
All four default repository checks pass, including 16 tooling tests, local links,
catalog metadata and schemas. Log: `/tmp/meowy-versioning-docs.log`. Compiler/runtime
execution was not part of this documentation edit.

## Next steps

1. Investigate normalized type-value equality inside required evaluation;
   see the [compiler handoff](compiler/STATUS.md#next-steps).
2. Implement proof only in separately planned slices against its qualification contract.
   Do not push, bump versions automatically or claim full release qualification.
