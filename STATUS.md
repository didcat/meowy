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

Bounded type subtraction is in progress. Its gated parser slice passes 843 library/
866 native tests, fmt and Clippy; evaluator and integration work follow.

Required `==`/`!=` now accepts bare type-producing blocks on either side of another
block or known type value. Selected blocks retain normalized identity, scoped locals,
source errors and work budgets. Scalar contexts preserve widths; mixed evaluated
type/scalar results report E207. Short-circuiting checks block structure without
resolving local initializers or result kinds. No runtime type storage is created.

Commits: `f2a1962` (block inference), `055d8cf` (integration evidence).
The [guide](compiler/docs/COMPUTED_TYPES.md#type-producing-block-operands) covers syntax,
result kinds and evaluation boundaries. Its example prints `7` in debug/release.
The full compiler gate passes.

Named immutable `core.Type` exports continue to retain concrete payloads through
explicit facades. Private names, separate namespaces and module startup stay intact.
The compiler entry guide is [compiler/README.md](compiler/README.md); detailed guides
live in `compiler/docs/`.

Net/HTTP/TLS still needs broader generic-type/I/O/task foundations. Full v0.0.1
release qualification remains incomplete.

## Actual validation

- `python3 -B tools/verify.py --compiler`: all ten checks passed, including 840
  library/866 native tests (1706 total), 20 Python tests, fmt, Clippy, build, links
  and catalog/schema checks. Conformance: 10 passed, 13 unsupported, 0 failed in
  debug/release. Log: `/tmp/meowy-type-blocks-gate.log`.
- Five checker/five native block-equality groups cover inferred type identity,
  contextual scalar kinds, E207 type emissions, skipped structure/values, source
  order, imported privacy/startup, repeated input costs, independent roots and
  work/node/depth failures with restored state. Focused logs:
  `/tmp/meowy-type-blocks-slice1.log`, `/tmp/meowy-type-blocks-slice2.log`.
- The type-producing block equality guide prints `7` in debug/release.
  Extracted source: `/tmp/meowy-type-blocks-guide.mwy`.
- Runtime implementation, reference fixtures, dependencies and release versions are
  unchanged. Editor and separate runtime/sanitizer gates were not rerun. Full release
  qualification remains open; proof examples remain unimplemented/unexecuted.

## Area handoff

| Area | Current boundary |
| --- | --- |
| Compiler | Bare type-block equality preserves inferred identity, scalar context and bounded work. |
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

1. Investigate bounded type subtraction syntax and required evaluation;
   see the [compiler handoff](compiler/STATUS.md#next-steps).
2. Implement proof only in separately planned slices against its qualification contract.
   Do not push, bump versions automatically or claim full release qualification.
