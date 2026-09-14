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

Bounded `!<U>` subtraction now constructs normalized concrete type sets in annotations,
aliases and required expressions. Static queries, imported values and bare type blocks
retain identity, original failures and work. Removing all members produces `never`;
both operands are still evaluated. Subtraction does not validate nullable runtime data.

Commits: `bc3030c` (parser), `ae68e3b` (evaluator), `e5579d3` (integration).
The [guide](compiler/docs/COMPUTED_TYPES.md#type-subtraction) covers syntax, budgets
and remaining limits. Its example prints `7` in debug/release. The full compiler gate
passes. Mixed adjacent union suffixes, literal subtypes and broad bases remain gated.

Required type equality, including bare type blocks, remains supported with scoped
inference and scalar-context checks. No runtime type storage is created.

Named immutable `core.Type` exports continue to retain concrete payloads through
explicit facades. Private names, separate namespaces and module startup stay intact.
The compiler entry guide is [compiler/README.md](compiler/README.md); detailed guides
live in `compiler/docs/`.

Net/HTTP/TLS still needs broader generic-type/I/O/task foundations. Full v0.0.1
release qualification remains incomplete.

The first executable proof series is now planned: bounded type-only copy queries,
static descriptors and assertions. Phase/dependency tracking and logical root
accounting must precede query execution. Partial module/revision metadata passes
focused checker/native tests, but proof queries remain unimplemented. Direct
required member reads and the final compiler gate are next.

## Actual validation

- Proof planning: source/reference inspection and all four default
  `python3 -B tools/verify.py` checks pass, including 16 tooling tests.
  Log: `/tmp/meowy-proof-plan-checks.log`. No compiler or runtime execution was
  rerun for this documentation slice; the following evidence is from subtraction.
- `python3 -B tools/verify.py --compiler`: all ten checks passed, including 847
  library/871 native tests (1718 total), 20 Python tests, fmt, Clippy, build, links
  and catalog/schema checks. Conformance: 10 passed, 13 unsupported, 0 failed in
  debug/release. Log: `/tmp/meowy-subtraction-gate.log`.
- Three parser/four checker/five native subtraction groups cover suffix spans and
  bounds, normalized sets, queries, kinds, runtime-value rejection, operand scope,
  skipped construction, original errors, import discovery/startup/privacy, retained
  input work, independent roots and restored budget state.
  Focused log: `/tmp/meowy-subtraction-integration.log`.
- The subtraction guide prints `7` in debug/release.
  Extracted source: `/tmp/meowy-subtraction-guide.mwy`.
- Runtime implementation, reference fixtures, dependencies and release versions are
  unchanged. Editor and separate runtime/sanitizer gates were not rerun. Full release
  qualification remains open; proof examples remain unimplemented/unexecuted.

## Area handoff

| Area | Current boundary |
| --- | --- |
| Compiler | Bounded type subtraction retains normalized identity, source evidence and work. |
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

1. Complete direct required reads of the typed proof revision constant and run
   the final compiler gate; module/static alias metadata passes focused checks. The
   [compiler handoff](compiler/STATUS.md#executable-proof-plan) records the ordered
   descriptor, accounting, phase/dependency and type-only query slices.
2. Broaden subtraction only after its remaining syntax/representation prerequisites
   are established. Do not push, bump versions or claim full release qualification.
