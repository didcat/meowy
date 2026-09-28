# Language conformance cases

[Documentation index](../README.md)

These cases pin observable rules to small source files. `cases.json` is version 1
of the fixture catalog and targets language contract revision 1. Every case is
independent: do not concatenate sources or let a failed declaration contaminate
another case's name lookup. This is a core regression set, not a claim to cover
every library operation or every possible program.

For each case, run checking on its `source` using the catalog's target and the
bundled foundational library, outside any ancestor application's manifest policy.
For accepted `run` cases, additionally build and execute the source with default
runtime settings and compare stdout bytes exactly. Successful checking/execution
must exit zero. Rejected cases must fail during checking with the indicated
primary diagnostic code; diagnostic wording, extra explanatory notes and display
columns are not golden strings. A missing toolchain/target or host failure is an
infrastructure failure, never a passing rejection test.

The catalog fields are closed: `version`, `language_contract`, `target`, `cases`.
Each case has a unique ASCII `id`, `phase` (`check` or `run`), relative `source`,
`expected`, and a documentation `reference`. `expected` is either
`{"accepted":false,"code":"E..."}` or `{"accepted":true}`; an accepted run
additionally has `stdout`. No source, reference or command is fetched remotely.
The files intentionally include invalid source; formatting or repairing them
changes the test input and must be reviewed as a contract change.

Every case is required by default. The bootstrap's
[support manifest](../../compiler/tests/conformance_support.json) lists only
temporary unsupported exceptions with their exact B001 diagnostic. A different
capability failure, a new unsupported case, or mixed error diagnostics fails the
gate. When an exception begins satisfying its reference outcome in both profiles,
the runner requires removing that exception; it then remains required. Reference
acceptance/output/error expectations never change to match a bootstrap limitation.
`python3 compiler/tests/conformance.py --strict` rejects every unsupported case.
Passing this small catalog alone does not qualify the complete language.

[Coverage inventory](COVERAGE.md) separates required source cases, pinned capability
gaps and internal test evidence across every file in `docs/reference/`. The
[document map](documents.json) records the reviewed content hash, evidence kind
and remaining scope. Reference edits require reviewing the affected evidence/gap
and updating its SHA-256; new documents require a new entry. Then regenerate with
`python3 -B docs/conformance/coverage.py --write`. Catalog validation checks the
inventory and rejects stale generated reports. Counts measure traceability only;
linked internal tests and seeded graph metadata do not qualify entire documents.

```sh
python3 docs/conformance/check.py
```

This command validates the **fixture catalog**, source paths, expected diagnostic
codes and documentation references. It does not run the language cases and must
not be reported as a successful language conformance run. Artifact-format examples
have separate [JSON schemas](../reference/artifact-formats.md) that consumers can
validate with a Draft 2020-12 JSON Schema validator.

Compiler releases should run accepted cases under both debug and release profiles;
checking results and defined output must agree. Add boundaries alongside each
language change: zero-space parsing, type construction, callable captures, union
joins, borrow rejection and foreign signatures. Task/replay suites additionally
need controlled scheduling and saved event fixtures; the [replay contract](../reference/replay-recording.md)
defines matching independently of a particular operating-system schedule.
