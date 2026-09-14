# The meowy command line

[Documentation index](../README.md)

The CLI takes a source file from checking to execution, then keeps failures
available as executable replay capsules. You can inspect a diagnostic, preview
its repairs, and replay the original failure after editing or moving the project.

This chapter defines the command contract. Terminal transcripts illustrate that
contract; version numbers, session IDs, and machine details are examples. The
[diagnostics reference](../reference/diagnostics.md#diagnostic-presentation)
contains the full three-error example used below.

## Choose a command

| When                                     | Command                                        | Why                                                                     |
| ---------------------------------------- | ---------------------------------------------- | ----------------------------------------------------------------------- |
| Learn the available options              | `meowy help` or `meowy help err reproduce`     | Read help for one command without running it                            |
| Identify a toolchain                     | `meowy --version`                              | Print the version and build identity                                    |
| Check source while editing               | `meowy check main.mwy`                         | Check the module graph without linking or executing the application     |
| Connect an editor                        | `meowy lsp`                                    | Serve current buffers with project configuration from `mod.mwy`         |
| Diagnose editor setup                    | `meowy lsp doctor main.mwy`                    | Check toolchain, manifest, and local analysis inputs                    |
| Check project coding policy              | `meowy style check`                            | Inspect layout, preferred expression forms, and code quality            |
| Review safe style changes                | `meowy style fix --diff`                       | Preview proven rewrites and layout without changing source              |
| Apply only source layout                 | `meowy fmt`                                    | Use gatostyle's layout settings without semantic rewrites               |
| Run a program                            | `meowy run main.mwy`                           | Build and execute the selected entry                                    |
| Exercise the project's test suites       | `meowy test`                                   | Check suites, then run each selected case in its own process            |
| See which cases would run                | `meowy test --list`                            | Check static suite descriptors and list case IDs without execution      |
| Check API documentation                  | `meowy doc check main.mwy`                     | Check attached prose, symbol links and example source without execution |
| Run explicitly runnable documentation    | `meowy doc check main.mwy --run-examples`      | Execute selected doc examples through the ordinary test runner          |
| Build a local API reference              | `meowy doc build main.mwy --output build/docs` | Render checked documentation without publishing it                      |
| Produce a native executable              | `meowy build main.mwy --output build/main`     | Build without executing the result                                      |
| Explain executable size and dependencies | `meowy build --report`                         | Save section sizes, retention evidence, and runtime requirements        |
| Resolve newly declared dependencies      | `meowy deps resolve`                           | Fill missing lock entries while preserving existing locked revisions    |
| Deliberately update a dependency         | `meowy deps update geometry`                   | Re-resolve that alias and the transitive changes it requires            |
| Read less after a failed run             | `meowy err summary`                            | List occurrences, locations, and fix counts                             |
| Understand one rule violation            | `meowy err explain 1`                          | Explain the rule against the saved source                               |
| Look up a rule without a saved run       | `meowy err explain E303`                       | Read its meaning, evidence, and repair guidance                         |
| See compiler or runtime evidence         | `meowy err inspect 1 --verbose`                | Open the recorded internals without replaying the failure               |
| Review proposed edits                    | `meowy err fix all --diff`                     | Show every candidate without changing source                            |
| Replay what failed                       | `meowy err reproduce 1 --verbose`              | Verify and execute the capsule, with the relevant evidence views        |
| Carry a failure to another machine       | `meowy err export 1 --output error-1.replay`   | Write one self-contained executable                                     |
| Prepare a compiler bug report            | `meowy err report 1 --diff`                    | Review the exact outgoing payload locally                               |
| Recover diagnostic cache space           | `meowy err cleanup --diff`                     | List the sessions and bytes eligible for deletion                       |

`--help` on any command is equivalent to asking for its help. CLI subcommand names
are shell arguments; they do not introduce reserved words into meowy source.

## Documentation commands

`meowy doc check` and `meowy doc build` follow the
[checked documentation contract](../reference/documentation.md). They share the
source graph, target/profile selection and offline dependency policy with checking.
`--require-public` requires documentation on exported declarations and named record
members. `--run-examples` belongs to doc check; neither ordinary compilation nor
doc build runs examples. `--assets DIR` explicitly permits bounded local image assets
for doc build; it does not fetch remote content or copy arbitrary source files.

Doc build requires `--output DIR`, stages its output safely and never uploads it
or opens a browser implicitly. Failed or unsupported required checks return nonzero;
unexecuted examples are not reported as executed passes. The bootstrap implements
standalone doc check/build and explicit example execution; package/default-entry
selection, assets and full LSP integration remain unavailable. See the documentation
chapter's implemented profile before relying on broader full-language options.

## Check, build, and run

Start with a file when trying a small program:

```sh
meowy check docs/programs/age/main.mwy
meowy run docs/programs/age/main.mwy
meowy build docs/programs/packet/main.mwy --profile release --output build/packet
```

`check` resolves imports, evaluates compile-time declarations, and validates types,
ownership, capacities, and task contracts. It does not execute application module
initialization. It cannot catch a native link failure or a problem dependent on
runtime input. Use it for a short editing loop or source validation in automation.

`build` also lowers, generates, and links the program. With no `--output`, the
result goes under the project root's `build/<target>/<profile>/` directory, using
the entry's basename and the target's executable suffix. With `--output`, the
path is relative to the shell's current directory; parent directories are created
as needed. Build outputs may be replaced on later successful builds. Outputs
must not overwrite a source file, manifest, lockfile, or replay input.

`run` builds before execution. It never runs a stale executable when checking or
linking fails. Program arguments follow `--`:

```sh
meowy run main.mwy -- --input records.bin
```

Everything after `--` belongs to the program, even if it looks like a meowy option.
The application inherits the caller's working directory and standard streams.
The saved session records the selected entry, arguments, target, and profile.

### Projects and entry selection

For a project, put its configuration in `mod.mwy`; the
[manifest guide](../guide/mod.md) explains each section and its
[sample](../guide/mod.sample.mwy); the
[fully commented manifest](../guide/mod.full.mwy) lists every setting. Then use:

```sh
meowy check
meowy build --profile release
meowy run
```

With an explicit entry, the CLI resolves the path relative to the current working
directory and searches its directory and ancestors for the nearest `mod.mwy`.
Without an entry, it searches from the working directory and uses `build.entry`.
Manifest paths remain relative to the manifest. An explicit entry overrides only
`build.entry`; import aliases, native inputs, and executor settings still come
from that project. Without a manifest, an explicit file is a standalone program
whose root is its directory. Omitting both is a usage error.

The project rules are in [modules and configuration](../reference/modules-and-ffi.md).
A task-using standalone program still needs an explicit executor configuration;
`run` does not infer a worker pool from task or channel capacity.

### Profiles, targets, and dependencies

| Option                                   | Applies to                                                                               | Effect                                                                            |
| ---------------------------------------- | ---------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| `--profile debug` or `--profile release` | `check`, `build`, `run`, `test`, `doc check`, `doc build`                                | Override `build.profile` for this invocation                                      |
| `--target TRIPLE`                        | `check`, `build`, `run`, `test`, `doc check`, `doc build`                                | Override `build.target` and record the chosen architecture, OS, and ABI           |
| `--output PATH`                          | `build`, `err export`, `doc build`                                                       | Select the output file                                                            |
| `--report`                               | `build`                                                                                  | Write a build report and link map beside the executable                           |
| `--offline`                              | `check`, `build`, `run`, `test`, `doc check`, `doc build`, `deps resolve`, `deps update` | Require all dependency content and resolution metadata locally                    |
| `--record-replay`                        | `run`                                                                                    | Record supported runtime inputs and scheduling decisions for deterministic replay |

Defaults are the manifest's values, then `debug` and the host target. Both profiles
preserve checked arithmetic, bounds, ownership, and cleanup. `release` is an
optimization choice, not a way to suppress language rules. A target incompatible
with the host can be checked and built; `run` and executing `test` reject an
incompatible host. Use `test --no-run` to build tests for another target.

The initial distribution supports only `x86_64-unknown-linux-gnu` and baseline CPU,
with explicit static/shared closure requirements. Other triples, including the illustrative AArch64
test command below, are `E507` until a distribution supplies that profile. See
[target/runtime requirements](../reference/target-profile.md).

Optimization goals, CPU baseline, build concurrency, debug information, and
linkage come from `build.optimize`, `build.cpu`, `build.jobs`, `build.debug_info`,
and `build.link` in `mod.mwy`. Profile-dependent defaults follow the effective
profile; explicitly set fields are preserved. Target compatibility includes the
expanded CPU requirements and native runtime dependencies, not just the target
triple. See [build policy](../reference/optimization.md#select-build-policy-in-modmwy).

Ordinary checking, building, running, and testing use the existing `mod.lock`. They may
fetch content identified by that lock, but never advance a revision or rewrite the
lockfile. A missing required entry or conflicting entry is a diagnostic. A
standalone file using only foundational and relative imports needs no remote
lockfile; its foundational libraries come from the selected toolchain.
Project-local prefixes in `import.aliases` also require no remote lock entries;
dependency commands operate on the package records beside that table. Resolve or
update remote dependencies deliberately:

```sh
meowy deps resolve
meowy deps update geometry
meowy check --offline
```

`resolve` fills missing entries and fails on conflicts with existing ones;
`update ALIAS` permits changes for that dependency and necessary transitive entries.
`update` without an alias updates all declared remote dependencies. Both display
revision and digest changes and replace the lockfile only after resolution and
integrity checks succeed. They do not rewrite manifest selectors. Foundational
modules do not need remote dependency entries. Use `--offline` when a check must
not contact a source server, for example after preparing a CI dependency cache.

The root lock owns the whole graph; imported locks are ignored, different remote
revisions may coexist, and dependency native artifacts contribute to the root
build. A lock pins the complete distribution digest as well as dependency content.
Only `deps update` without an alias may adopt a different running distribution;
it displays that identity change alongside revisions. The
[package graph contract](../reference/packages-and-builds.md) defines selector
normalization, transitive update limits and build-setting precedence. The lock's
[schema 1](../reference/artifact-formats.md#lockfiles) is UTF-8 JSON.

### Explain a build's size and dependencies

```sh
meowy build --profile release --output build/app --report
```

This produces the executable plus `build/app.build.json` and `build/app.link.map`.
With default output selection, the suffixes follow the resolved executable path,
including its target suffix. The report records effective inputs, section sizes,
retention evidence, debug companions, and required runtime dependencies. It helps
answer why a helper or calendar table is present and which bytes belong to code,
static data, zero-fill storage, or debug information.

`--report` is available only on `build`; it neither changes optimization nor runs
the program. JSON and map output go to files, while human summaries remain on
stderr. The [optimization reference](../reference/optimization.md#measure-and-explain-the-result)
defines their identities, evidence limits, and independent inspection commands.
Reports explain executable storage; measure runtime memory with the actual workload.

A requested report write failure makes the build unsuccessful with status `1`
and identifies any completed artifacts. A failed build's partial report names the
failed phase and never claims a successful build. Protected output paths remain
usage errors. A previous report stays tied to its original binary digest.

## Test a project

Declare suites with [`@"testing"`](../reference/stdlib/testing.md), then run:

```sh
meowy test
meowy test --list
meowy test tests/arithmetic_test.mwy --filter halves_table
meowy test --profile release --offline
meowy test --no-run --target aarch64-unknown-linux-gnu
```

`meowy test [PATH...]` finds the nearest `mod.mwy` from the first explicit path,
or from the working directory when paths are omitted. Every selected path must
belong to that same project; a manifest is required, but `build.entry` is not.
The application entry is never implicitly added or executed. Put shared behavior
in importable modules and let the application and tests call those modules.

The manifest's `test.paths` selects directories or exact `_test.mwy` files;
the default is `./tests`. Directory discovery finds `_test.mwy` files recursively,
excluding `.git`, `build`, nested projects, and symlink directory traversal.
CLI paths narrow this configured selection; they cannot add another root.
Canonical paths must remain inside those roots and belong to the same manifest,
including after resolving a symlinked file; explicit paths cannot enter a nested
project or escape through a symlink.
Discovery checks the selected files and their dependency graphs, then collects
public suite descriptors without running module initializers or callbacks.
All discovered graphs are checked before applying name filters: a filter cannot
hide a type error in the discovery set. Ordinary `meowy check` still checks the
application graph; use `test --list` or `test --no-run` for suites.

| Option          | Effect                                                                                                   |
| --------------- | -------------------------------------------------------------------------------------------------------- |
| `--filter TEXT` | Select canonical case IDs containing this exact substring; repeated filters are combined with OR.        |
| `--list`        | Check and list IDs with skip reasons on stdout; do not build, execute, or publish a test session.        |
| `--no-run`      | Check and build the test harness without executing it; publish a session recording zero case executions. |
| `--show-output` | Show captured output from passing cases too; failed case output is shown by default.                     |

`--list` and `--no-run` are mutually exclusive; neither accepts `--show-output`.
`test` accepts the build-selection flags above and terminal options, but not
`--entry`, `--output`, `--report`, or application arguments after `--`.
Execution policy comes from [`test` in `mod.mwy`](../guide/mod.md#configure-test-suites),
including process concurrency, timeout, output limit, seed, and fail-fast behavior.
There are no environment or command-line overrides for those policy fields.

A canonical case ID names its canonical project-relative file, exported suite, and case:
`tests/arithmetic_test.mwy::tests::halves_table`. Alias spellings do not create duplicate
module identities. IDs use the escaping and sorting rules
in the [testing reference](../reference/stdlib/testing.md). The runner reports
final results in ID order, even when processes finish in another order.

Each active case runs in a fresh process, initializing only its own module graph.
Its callback, child joins, and cleanup must complete before it passes. Process
isolation resets program memory; files and external services can still be shared.
The watchdog covers launch through cleanup, and may terminate a stuck process.
That is a test timeout, not `tasks.Timeout` or proof of a deadlock, and forced
termination cannot promise cleanup or complete replay evidence.

The runner captures bounded stdout and stderr per case instead of interleaving
live output. It prints labeled captured streams, case results, and its summary to
stderr. Counts distinguish passed, failed, skipped, and not-started cases.
Fail-fast stops new launches after an observed failure and waits for started
cases; it does not count the remaining work as passed. Zero selected cases fails
with `E509` unless `test.allow_empty` is true. A nonempty selection consisting
entirely of skipped cases succeeds and reports the skipped count.

## Coding style and quality with gatostyle

Gatostyle reads the optional `gatostyle` policy in `mod.mwy`. Projects can configure
layout and rules for call versus dispatch notation, intermediate bindings, type
annotations, naming, discarded values, allocations, and task capture. Presets are
starting values; individual rules, custom selectors, and path overrides express
the project's choices. See the [gatostyle guide](../guide/gatostyle.md) for the
complete schema and examples, including a zero-spacing layout.

```sh
meowy style check
meowy style fix --diff
meowy style fix
meowy style explain call_form
meowy style config --resolved main.mwy
```

`style check` reports policy findings without changing source; `style` alone means
`style check`. `style fix --diff` previews eligible safe fixes and layout, while
`style fix` validates and writes the patch. An automatic semantic edit must
preserve resolved names, types, evaluation, ownership, cleanup, and task boundaries.
A preference without such a proof remains a finding for the author.

Use `meowy style check --offline --quiet --color never` in CI. Quality analysis
uses the selected files' module graphs and existing lock; offline mode requires
dependencies locally. It never updates the lock or executes the application.
Missing analysis inputs fail the operation rather than skipping enabled checks.

For layout alone:

```sh
meowy fmt main.mwy cmd/
meowy fmt --check
meowy fmt --diff
meowy fmt --stdin --stdin-filepath ./main.mwy
```

`fmt` writes selected files by default. Its `--check`, `--diff`, and `--stdout`
modes write no source; `--stdout` requires one file. Stdin defaults to source on
stdout and can instead use `--check` or `--diff`. Layout requires syntax and local
policy, without resolving dependencies. A successful format is not a type check.

Both workflows accept files or directories and discover project source when no
paths are supplied. Discovery skips build outputs, configured exclusions, and
nested projects, without following imports to select files for editing. The guide
defines [file selection and exits](../guide/gatostyle.md#commands-for-an-editing-loop-and-ci)
and [buffer analysis](../guide/gatostyle.md#work-with-an-editor-buffer).

These commands do not replace a saved check/build/run/test session or modify replay
capsules. Style findings use `G...` codes and `meowy style explain`, independently
of saved compiler occurrences. A source edit can make an old repair stale; check
again before applying that saved repair.

## Language server and editor analysis

Use `meowy lsp` (or `meowy lsp --stdio`) as the command launched by an editor's
LSP client. Standard input/output carry protocol messages; startup logs and
failures go to stderr. The server checks synchronized buffers, including unsaved
modules and manifests, without executing the application or publishing ordinary
CLI sessions.

Editor settings belong in `mod.mwy` under `lsp`; build inputs still come
from `build` and `import`, suite roots come from `test`, and coding policy comes
from `gatostyle`. Inspect the saved configuration and prerequisites with:

```sh
meowy lsp config --resolved main.mwy
meowy lsp doctor main.mwy
```

These commands read disk, so they do not inspect another process's editor
buffers. The [language-server reference](../reference/lsp.md) defines startup,
configuration fields and defaults, version compatibility, project discovery,
client capability fallbacks, and live inspection. It also defines explicit
capture of an unsaved static failure into an executable replay capsule.
`--profile`, `--target`, `--preset`, and alternate configuration files are not
LSP overrides; edit the manifest instead.

## Saved sessions

To look up a rule without a project or saved failure, use its code:

```sh
meowy err explain E303
```

This reads the selected toolchain's [code catalog](../reference/diagnostic-codes.md).
Providing `--session ID` instead uses the explanation preserved with that session.
An occurrence number, such as `meowy err explain 1`, always refers to saved source.

Every `check`, `build`, or `run` publishes a completed session for its project and
entry, including a successful run with zero errors. This prevents old failures
from masquerading as current ones. A session contains numbered occurrences,
patch candidates, the original command, and replay capsules for failures that
were captured successfully. Capture failures are visible in the summary.

`test`, including `test --no-run`, publishes one completed session in a separate
project test namespace. Successful sessions replace that namespace's latest
selection too. Case processes never publish separate last-run pointers, and
`test --list` changes neither namespace. Use `--test` with error commands:

```sh
meowy err summary --test
meowy err inspect 1 --test --verbose
meowy err reproduce 1 --test
meowy err export 1 --test --output failing-case.replay
```

A case failure records its full case ID, seed, phase, and execution policy.
Its capsule replays that case under the preserved supervision, or the failing
checking/build phase if execution never began. An expected, matched panic is a
passing case and creates no failure occurrence. Case ordinals and diagnostic
occurrence numbers are separate; commands above select an occurrence.

Background language-server diagnostics have no saved occurrence numbers and do
not change this selection. An explicit
[editor capture](../reference/lsp.md#inspect-facts-and-capture-a-failure) stores a
session from its frozen buffer graph without advancing the last-run pointer.
Select it with the returned `--session` and `--entry`; it may contain source that
has never been saved to disk.

`meowy err` is shorthand for `meowy err summary`. Error commands discover the
project from the working directory. Without `--test` they use that project's
most recently completed entry session and print its entry and session ID. With
`--entry PATH`, they use the same project discovery as an explicit `run` entry and
restrict selection to that entry. A standalone session can therefore be found
from its root directory or by providing its entry path. Use the exported capsule
when carrying the failure elsewhere.

`--test` selects the discovered project's test namespace and is mutually
exclusive with `--entry`. It can combine with `--session ID` or `--previous`,
following their existing exclusivity rules. A normal entry failure and a test
failure therefore remain independently selectable after either command runs.

`--previous` explicitly means the last completed session for that selection. It
is the same default used by `err`; it does not start another build or step back
one additional run. For an older session, use its ID:

```sh
meowy err summary --entry main.mwy
meowy err explain 1 --session proj-1788649910
meowy err fix 1.2 --session proj-1788649910 --diff
```

`--session ID` selects a session in the discovered namespace and can combine with
`--entry` to disambiguate entry sessions. IDs ambiguous across entries require
that selector; selecting a test session requires `--test`.
It cannot combine with `--previous`. A command resolves its session once, before
work begins, so a concurrently completed build cannot change its selection.

Occurrence `1` and candidate `1.2` are local to that session. Error code `E103`
identifies a rule across runs. `explain`, `inspect`, `reproduce`, and repair
validation never advance the last-run pointer. A missing session, absent
occurrence, or deleted capsule gives an explicit error rather than choosing a
different failure.

## Preview and apply repairs

Begin with the complete source explanation and candidate list:

```sh
meowy err summary
meowy err explain 1
meowy err fix all --previous --diff
```

`all --diff` lists alternatives separately; it does not pretend mutually exclusive
repairs form one patch. Add `--recommended` for a combined preview of the preferred
edits. In the [three-error example](../reference/diagnostics.md#diagnostic-presentation),
two local capacity increases are recommended. The text-versus-integer decision
still belongs to the programmer:

```text
$ meowy err fix all --previous --diff --recommended

meowy v0.0.1

session proj-1788649910 / main.mwy
3 errors from the last run
2 recommended fixes selected
1 error skipped: error 2 has no recommended fix

fix 1.2 [recommended]
E103: list capacity exceeded
main.mwy:1

  increase capacity from 3 to 4
  reserves inline storage for one more string view

--- main.mwy
+++ main.mwy
@@ -1 +1 @@
-list <string[3]> := ["A", "B", "C"]
+list <string[4]> := ["A", "B", "C"]


fix 3.2 [recommended]
E103: list capacity exceeded [same as 1]
main.mwy:10

  increase capacity from 2 to 3
  reserves inline storage for one more string view

--- main.mwy
+++ main.mwy
@@ -10 +10 @@
-names <string[2]> := ["pato", "ari"]
+names <string[3]> := ["pato", "ari"]


2 fixes selected
1 file would be changed

main.mwy
  2 insertions
  2 deletions

no changes were written.

apply this selection with:
  `meowy err fix 1.2 3.2 --session proj-1788649910`
```

For this session, `meowy err fix all --previous --recommended` applies the same
two candidates. Pin the session ID and explicit candidate IDs when a preview
must survive intervening builds. Removing `--diff` writes the selected source
edits; it does not run the application. Follow with `meowy check` or `meowy run`.

Without `--recommended`, applying `all` requires exactly one applicable candidate
for each error that has fixes. If an error has alternatives, the command asks for
explicit IDs instead of selecting one arbitrarily. A batch cannot contain two
alternatives for the same occurrence. Errors with no fixes are listed as skipped;
an empty selection makes no changes and reports that fact.

Every apply checks saved source hashes, validates the combined changes, and
refuses stale or conflicting edits before writing. Internal validation may leave
unselected errors in place; in this example `E207` remains after the two capacity
fixes. A manually edited file requires a fresh check and new candidates. The
[repair contract](../reference/diagnostics.md#repair-contracts) defines validation
and recovery after interrupted writes.

## Reproduce a failure

One command opens the original failure and its useful internals:

```sh
meowy err reproduce 1 --verbose
```

The replay executable contains the exact toolchain and captured inputs. Source
edits since the failure are irrelevant to reproduction. The original command
can have failed in parsing, type checking, ownership analysis, lowering, linking,
or execution; the runner re-enters that phase and checks the saved failure's
identity. It never executes an old program to simulate a static error.

For the capacity error, a verbose replay looks like this. Digests are shortened
for display; verification uses their full values. The layout shown belongs to
this illustrative toolchain and target:

```text
$ meowy err reproduce 1 --verbose

meowy v0.0.1
reproducing error 1 [E103]

capsule proj-1788649910/error-1/replay
capture: closed
phase:   type checking

original:
  main.mwy:3:13
  command: meowy run main.mwy

toolchain:
  compiler      0.0.1 (bundled)
  host          x86_64-unknown-linux-gnu
  target        x86_64-unknown-linux-gnu
  profile       debug
  dependencies  bundled; no network resolution

preserved payload:
  tools/meowy-compiler
  tools/runtime-libraries/
  source/main.mwy
  diagnostics/error-1.json
  evidence/types.json
  evidence/layout.json
  build/commands.json
  build/environment.json
  stdout
  stderr
  manifest.json

application binary: not produced; checking rejected the source

verifying capsule...
  tools/meowy-compiler       sha256 85f4...c921  ok
  source/main.mwy            sha256 91c0...72a1  ok
  diagnostics/error-1.json   sha256 a882...07ef  ok
  evidence/types.json        sha256 0d19...f5c2  ok
  remaining payload entries                     ok

artifact integrity: ok
host requirements: compatible

workspace:
  fresh scratch directory
  original source paths remapped
  declared environment restored
  live project and user build cache excluded

--- types [recorded] ---

main.mwy:1:6    list: <string[3]>
main.mwy:1      initialized length: 3
main.mwy:3:13   list.add("D") requires length: 4
constraint:    4 <= 3
result:        false -> E103

--- layout [derived from saved inputs] ---

target word:          8 bytes
string view:         16 bytes (address + length)
<string[3]>:          56 bytes, alignment 8
<string[4]>:          72 bytes, alignment 8
fix 1.2 storage:     +16 inline bytes; no allocation

--- replay ---

invoking bundled compiler with recorded checking inputs

1 - error[E103]: list capacity exceeded
  --> main.mwy:3:13
   |
 1 | list <string[3]> := ["A", "B", "C"]
   |      ----------- limited to 3 items
 2 |
 3 | list = list.add("D")
   |             ^^^^^^^^ cannot add a fourth item
   |
   = attempted to insert `"D"`

--- result ---

compiler exit status: 1
saved failure:       E103 / type checking / main.mwy:3:13
replayed failure:    E103 / type checking / main.mwy:3:13
match:               yes
other diagnostics:   2, retained in replay output
source changes:      none

reproduction complete: matched
```

The capsule runner returns success because it matched the failure, even though
the bundled compiler exited with an error. A different error, a changed panic
location, or a successful compilation is a divergence, not a successful replay.

### Inspect types, ownership, and concurrent work

Use `inspect` when the recorded explanation is enough. It verifies and reads the
capsule without executing the failing compiler or program:

```sh
meowy err inspect 1 --verbose
meowy err inspect 1 --trace types --trace layout
meowy err reproduce 1 --trace ownership
```

Available trace areas are `types`, `ownership`, `layout`, `lowering`, `tasks`,
`channels`, and `clocks`. Each reports whether evidence was recorded, derived,
or unavailable.
`inspect` reads previously stored views; computing a new derived view requires
`reproduce`. Selecting a trace never manufactures missing historical events.

Use `--trace clocks` for monotonic and civil readings, scheduled deadlines, timer
wakeups, skipped ticker slots, and the captured time-zone/calendar data versions.
Recorded replay advances a virtual clock instead of waiting through the original
delays. The [time contract](../reference/stdlib/time-and-date.md#failures-storage-and-replay)
defines which timing inputs must be present.

For concurrency investigations, enable recording before the failing run:

```sh
meowy run main.mwy --record-replay
meowy err reproduce 1 --verbose --trace tasks --trace channels
```

The runtime recording links child admission, cancellation, joins, and cleanup to
channel transfers and endpoint closure. For example, a captured wait state might
show a parent waiting to join a producer, the producer waiting on a full queue,
and the only receiver still owned by that parent. That explains the wait cycle
without guessing from thread stacks. It is not a promise to detect every deadlock.

An illustrative trace from a program that panics after a timed-out producer makes
the ownership consequences visible:

```text
--- tasks + channels [recorded] ---

e41  task#2  send message#3 -> channel#1: waiting (occupancy 2/2)
e42  task#0  join task#2: waiting
     channel#1 receiver is still owned by task#0
e43  task#2  deadline reached: cancellation requested
e44  task#2  send checkpoint: cancellation acknowledged
     message#3 was not transferred; released by task#2
e45  task#2  cleanup: last sender for channel#1 released
e46  task#2  settled: tasks.Timeout
e47  task#0  join completed: tasks.Timeout
e48  task#0  debug.panic("producer timed out"): P006
e49  task#0  cleanup: receiver released; queued messages#1 and #2 released

cause [derived from recorded events]:
  parent waited for producer before draining its full channel
  deadline ended the wait through cooperative cancellation
  application chose to turn the timeout outcome into a root panic
```

The timeout is an ordinary task outcome; the root panic is what creates the
runtime diagnostic here. Events distinguish an attempted send from an ownership
transfer, and a cancellation request from its later acknowledgment. A replay can
show why a message was released and which task owned it at that point.

Recording adds instrumentation and storage costs and can affect timing. Complete
recordings replay supported input boundaries and scheduling decisions. Partial
recordings name the missing events; native calls or external services may require
live I/O. The default replay stops at an uncaptured boundary. Use
`--allow-live-io` only when deliberately rerunning those effects; the result is
labeled a best-effort rerun. A hang without a captured failure or completed
session does not automatically have an error ID.

[Recording profile 1](../reference/replay-recording.md) lists exactly which APIs
can be replayed, operation/argument matching, the fixed environment allowlist, and
256 MiB input / 64 MiB event / 8 MiB application-output capture budgets. These
are fixed distribution policy, without manifest or CLI overrides. Capture limits
mark evidence incomplete; they do not truncate the original application's I/O.

### Export one executable

```sh
meowy err export 1 --output error-1.replay
./error-1.replay --inspect
./error-1.replay --verbose
```

Export gathers all shared cache blobs into the one executable, verifies them,
and prints its digest and host requirements. An existing export path is refused
rather than overwritten. Copy the file with its executable permission to a
compatible host; the source checkout and meowy installation are unnecessary.
The runner supports `--inspect`, `--verbose`, repeatable `--trace AREA`, and
`--allow-live-io`; its default action is replay. Standalone replay uses the same
exit statuses as `err reproduce`.

The original source, compiler, and relevant native/runtime libraries travel in
the capsule. Declared host and application-target requirements remain visible.
A capsule marked incomplete stays incomplete after export; the exporter reports
exactly which inputs are missing. Inspecting remains useful even when replay is
unavailable.

The [versioned artifact contract](../reference/artifact-formats.md) defines the
ELF capsule container and structural/integrity checks. If even its runner is
missing, export returns `E704` and writes no executable-looking artifact. A newer
unsupported artifact schema also returns `E704`; malformed structure or payload
integrity failures use `E705`. Neither case silently migrates saved evidence.

## Report a compiler bug

A source error usually calls for a source edit. Report a compiler crash, an
incorrect diagnostic, a violated language rule, or a replay that diverges despite
a closed recording. Prepare the report locally first:

```sh
meowy err report 2 --diff
meowy err report 2
```

`--diff` lists the exact outgoing files, metadata, redactions, and resulting replay
limitations. It neither uploads nor starts authentication. The report includes
the selected occurrence and the capsule inputs needed to investigate it, which
can include surrounding project source. Source minimization is not automatic.

Submission requires login and confirmation of that payload. If no reviewed
payload exists, `report` presents it before requesting confirmation; if the
payload changes after preview, it must be reviewed again. In a noninteractive
shell, submission requiring authentication or confirmation fails with instructions.
Local explanation, replay, and export never require an account. Report previews
exclude tool transport credentials and disallowed environment values, and mark
any omission that affects replay. Source/argv/output remain reviewed exact inputs;
this is not a promise to recognize secrets embedded in arbitrary project bytes.

## Clean up saved failures

```sh
meowy err cleanup --diff
meowy err cleanup --session proj-1788649910
```

Without `--session`, cleanup targets completed entry sessions for the current
project; `--entry` restricts it to an entry, while `--test` selects test sessions.
`--diff` lists sessions, capsule counts, and
bytes that would be freed. Actual deletion asks for confirmation; `--yes` provides
explicit confirmation for automation. Active capture, replay, and export sessions
are excluded. Shared payloads are collected only when no retained capsule uses
them. Successful deletion clears any last-run pointer to a removed session.

Cleanup never removes source, `mod.lock`, normal build outputs, or exported replay
files. Those are outside the diagnostic cache lifecycle.

## Streams and exit statuses

Human diagnostics, banners, progress, summaries, and replay explanations go to
standard error. During `run`, application standard output remains standard output,
and application standard error remains standard error. Application stdout during
runtime replay also remains stdout; its streams are captured in the replay result.
Tests and case replays use labeled captured streams on stderr; `test --list`
prints its case listing on stdout.
`help` and `--version` print to standard output.
Style configuration queries emit JSON on stdout; style/fmt diffs and formatted
source also use stdout, with their findings and summaries on stderr.

Global `--color auto|always|never` controls terminal styling; `auto` uses color
only on a terminal and honors `NO_COLOR`. `--quiet` suppresses banners, progress,
and conversational hints, while keeping errors, fix effects, and command results.
Neither option changes the recorded diagnostic or occurrence IDs. Use
`meowy check --offline --color never --quiet` for a predictable source check in CI.
The serving `meowy lsp` process rejects these terminal options; its protocol
stream has no terminal presentation and its tracing policy comes from `mod.mwy`.
The `lsp config` and `lsp doctor` inspection commands accept them for human output.

| Command outcome                                                                         | Exit status                                            |
| --------------------------------------------------------------------------------------- | ------------------------------------------------------ |
| Successful check, build, query, export, cleanup, report, or repair action               | `0`                                                    |
| Successful replay matching the saved failure                                            | `0`, regardless of the failing child's recorded status |
| Source/build failure, rejected repair validation, or replay divergence                  | `1`                                                    |
| Invalid usage, stale/conflicting edits, integrity failure, or unavailable replay inputs | `2`                                                    |
| Completed application launched by `run`                                                 | Application's process status                           |
| Passing test run, successful test listing, or successful `test --no-run`                | `0`                                                    |
| Test manifest/source/build failure, failed case, or disallowed empty selection          | `1`                                                    |
| Style check finds violations at/above policy threshold; style fix leaves such findings  | `1`                                                    |
| Fmt check finds layout differences                                                      | `1`                                                    |
| Invalid style/fmt input, policy, incomplete analysis, failed proof, or failed write     | `2`                                                    |
| LSP configuration output or ready/disabled doctor result                                | `0`                                                    |
| LSP doctor finds unavailable inputs, mismatched toolchain, or degraded operation        | `1`                                                    |
| Invalid LSP configuration or command usage                                              | `2`                                                    |

A style/fmt command otherwise succeeds with `0`, including an empty selection.
A serving LSP process uses its [protocol shutdown statuses](../reference/lsp.md#wire-lifecycle-and-compatibility);
source diagnostics do not terminate that connection.
A diff preview returns success when it can present the requested candidates;
the saved errors do not make the preview fail. Failure to save a capsule is
reported alongside the original failure and does not replace its exit status.
On POSIX hosts, signal termination of a program launched by `run` follows the
shell convention `128 + signal`. A failed test case instead contributes to the
runner's status `1`, with its actual child status or signal recorded separately.
Command output distinguishes a tool failure from a returned application status;
numeric codes alone cannot tell those apart for `run`.

The entry's `<null>` primary means success; an `<int32>` primary supplies the
process status, subject to the host's status representation. See
[entry execution](../reference/modules-and-ffi.md#build-settings) for the language
contract and [replay capsules](../reference/diagnostics.md#replay-capsules) for the
artifact and evidence contracts.
