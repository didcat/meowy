# Testing

[Library index](README.md) · [Pawterns](../../pawterns/testing.md) · [Worked suite](../../programs/testing/README.md)

`@"testing"` supplies assertions and ordinary values describing test cases and
suites. `meowy test` discovers those descriptions, builds case harnesses, and
runs each selected case in a fresh process. Tests keep normal type, ownership,
allocation, and task rules. There are no test keywords, attributes, magic function
prefixes, or implicit exception conversions.

Assertions can also be used in ordinary programs. Importing `testing` does not
start a runner, create an executor, or retain every test in an application binary.
The [CLI](../../cli/README.md) operates the suite; all project policy belongs in
the manifest's `test` record.

## Write a suite

Create `tests/arithmetic_test.mwy` in a project:

```meowy
testing : @"testing"

-> tests : testing.suite({
    -> adds : testing.case(() {
        actual <int32> : 2 + 2
        expected <int32> : 4
        testing.equal(&actual, &expected, "two plus two")
    })

    -> keeps_literal_text : testing.case(() {
        testing.text_equal("meowy", "meowy", "the label stays unchanged")
    })
})
```

A minimal `mod.mwy` for this test-only project is:

```meowy
-> test : {
    -> version : 1
    -> paths : ["./tests"]
}
```

From the project directory:

```sh
meowy test --list
meowy test
```

The listing contains these case IDs, in this order:

```text
tests/arithmetic_test.mwy::tests::adds
tests/arithmetic_test.mwy::tests::keeps_literal_text
```

Each callback returns `<null>` on normal completion. Successful completion,
including its required joins and cleanup, passes the case. Returning an error
union instead is a descriptor error; callers must inspect expected domain errors
or explicitly fail on unexpected results. This avoids both ignored error results
and implicit boxing into a universal test outcome.

`tests`, `adds`, and `keeps_literal_text` are ordinary export names. A differently
named Suite export works the same way. A function merely named `test_something`
is not a registered case.

## Descriptors and callable rules

| API                                                   | Result          | Contract                                                                                                 |
| ----------------------------------------------------- | --------------- | -------------------------------------------------------------------------------------------------------- |
| `testing.case(body <() -> null>)`                     | `testing.Case`  | Describe one callback; do not invoke it during construction.                                             |
| `testing.panics(spec, body <() -> null>)`             | `testing.Case`  | Describe a callback whose boundary must observe the specified recoverable panic.                         |
| `testing.skip(reason <string>, value <testing.Case>)` | `testing.Case`  | Add a nonempty static skip reason. The skipped callback is still checked.                                |
| `testing.suite(cases)`                                | `testing.Suite` | Build a suite from an immutable, null-primary record of named Case values, or `null` for an empty suite. |

These are pure compile-time constructors with opaque nominal result types.
Aliases retain their resolved identities. Callbacks are non-capturing function
pointers: an imported function or compile-time value can be referenced normally,
but a runtime closure environment cannot be hidden in a descriptor. Acquire
runtime fixtures inside the callback. Cases cannot contain nested Suite values;
export several named suites when a module needs separate groups.

The compiler checks all callback bodies, including skipped cases, before running
anything. Invalid descriptor shapes, callback signatures, runtime-dependent
metadata, unknown fields, unsupported panic codes, and repeated skip decoration
use `E218`, with ordinary type/name diagnostics where those are the actual cause.
An empty suite is allowed: `testing.suite({})` supplies the ordinary `<null>`
empty-block value through this constructor's explicit empty-input contract.
Selecting no cases is a separate runner decision.

Descriptor construction cannot execute application module initialization or
discover cases by running filesystem, environment, or network operations. A
callback may have those effects when the runner actually invokes it. Suite
exports are test-discovery roots, not native ABI exports. An ordinary application
build retains only code and data required by its own reachable operations.

## Static proof contracts

[`@"proof"`](proof.md#testing-with-proof-queries) tests compiler-established facts
without executing the observed program. `proof.assert(result)` requires `Always`;
`proof.expect<S>(result)` tests an exact outcome, including `Indeterminable`.
These are compile-time checking obligations, not `testing.assert` runtime panics.

A proof failure in a checked test callback prevents building that test program.
Skipped callbacks remain checked, so `testing.skip` cannot hide a failed proof
assertion. Standalone positive/negative checking fixtures can test the package
without a runtime suite; this adds no test-discovery convention or CLI flag.
Runtime tests still cover actual input-dependent behavior, effects, and cleanup.
The proof reference lists the separate implementation/qualification requirements.

## Assertions borrow their evidence

| API                                                                           | Result  | Comparison and failure                                                    |
| ----------------------------------------------------------------------------- | ------- | ------------------------------------------------------------------------- |
| `testing.assert(condition <boolean>, message <string>)`                       | `null`  | Continue if true; otherwise raise `P005`. No truthiness.                  |
| `testing.fail(message <string>)`                                              | `never` | Raise `P005` unconditionally.                                             |
| `testing.equal<T>(actual <&T>, expected <&T>, message <string>)`              | `null`  | Require equality of the complete values; otherwise raise `P005`.          |
| `testing.not_equal<T>(actual <&T>, expected <&T>, message <string>)`          | `null`  | Require inequality of the complete values; otherwise raise `P005`.        |
| `testing.text_equal(actual <string>, expected <string>, message <string>)`    | `null`  | Compare UTF-8 bytes exactly, with escaped text evidence on failure.       |
| `testing.bytes_equal(actual <uint8[]>, expected <uint8[]>, message <string>)` | `null`  | Compare length and every initialized byte, including embedded zero bytes. |

`equal` and `not_equal` require one concrete type supporting ordinary equality
and diagnostic formatting. A union must satisfy those requirements for every
alternative. These are statically checked constraints, like the formatting
requirements of `debug.print`, not a new runtime interface table. Unsupported
comparisons use `E210`; incompatible argument types use the normal call/type
diagnostics. No implicit numeric conversion or error erasure occurs.

Pass references to existing bindings for generic equality. The helper borrows
its arguments without copying a large list or consuming an owner. Ordinary
argument evaluation happens once, in order, before the comparison. Aggregate
equality checks its complete shape, including metadata fields; it does not merely
compare primary values. Floating-point equality retains the language's numeric
rules; use `assert` with an explicit tolerance predicate for approximate results.

Text equality does not normalize Unicode, fold case, or apply a locale. Byte and
text differences report a one-based byte position, plus lengths and an escaped,
bounded excerpt where available. If a prefix is equal but lengths differ, the
position is one beyond that prefix. Allocation failure while recording rich
evidence must preserve the basic `P005`, source span, and failure status.

An assertion's message argument is a formatting boundary, like `debug.print`.
Direct interpolation such as `"Wrong value in row {index}"` streams into bounded
failure evidence without constructing an owned intermediate string. Interpolated
expressions are evaluated once with the call arguments; successful assertions
may omit formatting, but cannot omit their observable evaluation effects.
Borrowed message parts remain valid through the call and do not escape into
stored diagnostics. This behavior follows the resolved assertion identity through
aliases. A string stored separately still needs constant interpolation or an
explicit text builder under the ordinary string rules.

Successful assertions allocate no application-owned payload. Failure recording
uses bounded runtime diagnostic storage and labels omitted evidence. Borrows do
not escape into a saved diagnostic: any retained evidence is copied while valid,
then normal unwinding releases the original values.

### Assertions and narrowing

An assertion is an ordinary call and does not refine the caller's union type.
Use a matcher whose failure path cannot continue:

```meowy
strings : @"strings"
testing : @"testing"

-> tests : testing.suite({
    -> parses_a_byte : testing.case(() {
        parsed : strings.to_uint8("42")
        | parsed <error> | testing.fail("valid decimal input must parse")

        expected <uint8> : 42
        testing.equal(&parsed, &expected, "parsed value")
    })
})
```

`fail` returns `<never>`, so the later statement can only see the success type.
A boolean passed to `assert` carries no such caller-side proof. Matcher conditions
still use `<T>` as a predicate regardless of spaces; ordinary expression contexts
still use it as a proven ascription.

`testing.assert(false, "reason")` and `testing.fail("reason")` deliberately raise
runtime `P005`, even when the condition is constant. The compiler may lower that
call to an unconditional failure path but must not reject it merely for failing
an assertion. This does not legalize a known division by zero or out-of-bounds
access inside an argument: ordinary static violations remain diagnostics before
the callback can run.

All assertions are fatal to the current normal control path. They unwind with
the existing recoverable panic rules; there is no accumulating soft-check mode.
Use separate cases for independent checks or loop through a table when one
failure should stop that case at the first bad row.

## Expected failures, panics, and skips

A library error is an ordinary result. Match it and check the facts the API
promises. A panic expectation instead describes a panic reaching a case boundary:

```meowy
debug : @"debug"
strings : @"strings"
testing : @"testing"

-> tests : testing.suite({
    -> rejects_text : testing.case(() 'result {
        parsed : strings.to_uint8("forty-two")
        | parsed <strings.ParseError> | 'result.leave()
        testing.fail("expected a decimal parse error")
    })

    -> deliberate_panic : testing.panics({
        -> code : "P006"
        -> message : "fixture panic"
    }, () {
        debug.panic("fixture panic")
    })

    -> large_matrix : testing.skip("Run the small cases while this fixture is rebuilt",
        testing.case(() {
            testing.assert(true, "placeholder fixture is still checked")
        }))
})
```

The panic specification accepts exactly `code` and optional `message`.
`code` must name a supported recoverable `P...` code; fatal `P008` is rejected.
`message` defaults to `null`, meaning code-only matching. A supplied string must
equal the diagnostic's message exactly, excluding its code label, locations,
terminal color, and stack display. For `debug.panic("fixture panic")`, that
message is the supplied string. Assertions use their supplied message; expected
and actual values are separate evidence fields.

A matching callback panic passes only after the required unwind and joins
complete successfully. A normal return produces `T001`; a different panic
retains its original code with an expected-versus-observed note. Prefer an exact
message when unrelated operations could raise the same code.

The expectation applies to callback execution and a child panic propagated by
its required joins. A panic swallowed into an explicitly handled
`tasks.Panicked` value does not reach that boundary. Module initialization failures,
module teardown failures, destructor panics, fatal cleanup failures, watchdog
termination, and output-limit failures cannot satisfy an expected panic.
Returning an error or printing a panic-looking line also cannot satisfy it.

Skipped cases remain in the selected list with their reason, but get no case
process. A skipped-only file therefore needs no runtime initialization. If an
active case needs the same module graph, that graph still initializes for that
active case. Skipping is not a way to hide invalid source or suppress required
initializers from other cases.

## Fixtures, cleanup, and concurrent work

Fixture setup is normal code at the start of the callback. Scoped owners provide
teardown on success and recoverable panic. Each case process initializes its
module graph anew; one case cannot retain an in-memory fixture for the next.
Factor setup into functions returning owners, and retain those owners while
views or adapters borrow them.

An in-memory writer is enough to exercise partial I/O without a flaky filesystem:

```meowy
io : @"io"
testing : @"testing"

-> tests : testing.suite({
    -> keeps_committed_prefix : testing.case(() {
        buffer <uint8[3]> := []
        result : 'write {
            writer := io.buffer_writer(&!buffer)
            -> io.write_all(writer.write, "hello".bytes())
        }

        expected <usize> : 3
        testing.equal(result.&count, &expected, "committed byte count")
        | result.error <null> | testing.fail("a full buffer must report failure")
        testing.bytes_equal(buffer.slice(), "hel".bytes(), "retained prefix")
    })
})
```

The writer's exclusive borrow ends before the assertion reads `buffer`. Fixture
and byte storage stay inline; the harness's own process/capture costs are a
separate runtime budget. See [I/O contracts](io-and-system.md#readers-and-writers).

For host files, use explicit paths and ordinary file owners. Call `sync` or
`close` and assert their results when durability or close errors matter;
implicit non-throwing cleanup cannot report those results for the test.
Parallel cases may still share files, services, ports, or foreign state on the
host. Process separation resets application memory; it does not isolate those
external resources.

A synchronous case runs at its process root without a task executor. If tested
code uses `>>`, supply `build.executor` exactly as for an application. Explicitly
join and inspect each outcome. A child assertion becomes `tasks.Panicked`; an
ignored observed error value does not fail its parent automatically. Implicit
scope cleanup still reports unobserved panic/admission failures under the normal
task rules. A case cannot pass while required joins or cleanup remain unfinished.

Drain channels or start consumers before joins that depend on them. A runner
watchdog can stop a stuck process, but it is not a substitute for a correct
[shutdown protocol](../../pawterns/deadlocks-and-shutdown.md).

## Configure a run in mod.mwy

```meowy
-> test : {
    -> version : 1
    -> paths : ["./tests"]
    -> processes : 1
    -> timeout_ms : 30_000
    -> output_bytes : 1_048_576
    -> fail_fast : false
    -> allow_empty : false
    -> seed : 0
}
```

An omitted `test` block uses these defaults. An explicit block requires
`version : 1`; unknown fields, unsupported schema versions, wrong types, invalid
sizes, and forbidden path forms use `E505`. This schema version is independent
of the meowy distribution, suite source, and diagnostic artifact versions.
Additive options require a distribution that recognizes them; older versions
must reject unknown settings rather than silently ignore them.

| Field          | Default                                            | Contract                                                                                                                           |
| -------------- | -------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| `version`      | Implicitly `1` only when the whole block is absent | Supported manifest schema; required inside an explicit block.                                                                      |
| `paths`        | `["./tests"]`                                      | List of project-relative directories or explicit `_test.mwy` files; no globs, import aliases, or paths outside the owning project. |
| `processes`    | `1`                                                | Positive `<uint32>` maximum simultaneously executing case processes.                                                               |
| `timeout_ms`   | `30_000`                                           | Positive `<uint32>` case watchdog budget, or `null` to disable it explicitly.                                                      |
| `output_bytes` | `1_048_576`                                        | Positive `<uint32>` combined captured stdout/stderr byte limit per case.                                                           |
| `fail_fast`    | `false`                                            | Stop launching new cases after the first observed terminal case failure; reap already launched cases.                              |
| `allow_empty`  | `false`                                            | Permit zero selected cases to succeed when explicitly true.                                                                        |
| `seed`         | `0`                                                | `<uint64>` base for deterministic case seeds.                                                                                      |

Build target, CPU, optimization, native inputs, debug information, compilation
jobs, and task executor settings still come from `build`. A library needs no
`build.entry` to run tests. A test harness does not execute the application entry;
put shared code in importable helper modules rather than importing that entry.
Preserve the ordinary entry/import separation rules.

Keep the budgets separate: `build.jobs` limits compilation, `test.processes`
limits concurrent OS case processes, and `build.executor` describes each case's
task runtime. Two case processes with two workers each can have two independent
executors. Runner parallelism never supplies an implicit executor to test code.
Harness capture and supervision storage belong to the tool runtime, whose
identity and configured limits are recorded alongside application build inputs.

## Discovery, identity, and selection

`meowy test [PATH...]` finds the owning manifest from the first explicit file or
directory, or from the current directory with no paths. A file starts discovery
at its parent. Every selector must belong to that same nearest manifest; missing
project ownership or conflicting project selectors are command-usage errors.
The command never combines nested projects into one implicit suite run.

Configured directories recursively discover files ending in `_test.mwy`.
Discovery skips `.git`, the project's `build` tree, and nested manifest roots,
and does not walk symlinked directories. Explicit CLI paths narrow the configured
roots; they cannot broaden them. Resolve file identity canonically, deduplicate
overlapping selections, and reject file symlinks or explicit targets escaping
the configured roots or crossing an owning-manifest boundary. Directory aliases
and alternative spellings must not create new identities or seeds.

Missing explicitly declared paths produce `E501`. An absent default `./tests`
directory yields zero discovered cases; it is not an error just to open such a
project in an editor. `meowy test` uses `E509` when its final selection is empty
unless `allow_empty` is true. A nonempty selection consisting entirely of skipped
cases succeeds and reports the skipped count.

Only Suite values in public named exports of discovered files register cases.
Imported helpers need no filename suffix. An imported Suite is not independently
registered unless a discovered file exports it. Normal canonical module identity
prevents repeated import initialization; exporting one Case under two case names
intentionally registers two different cases.

A case ID has three components:

```text
canonical/project-relative_test.mwy::suite_export::case_field
```

Use `/` separators in the canonical project-relative path. Escape `%` and `:`
inside that path as `%25` and `%3A`; suite and case fields use ordinary identifier
names. Sort IDs by their UTF-8 bytes for listing and final reporting. Editing a
body keeps its ID; moving the file or renaming either exported field changes it.
Store the unescaped source path and component names separately in metadata too.

Check every discovered suite graph before applying name filters. Skips and
filters cannot turn bad source into a successful build. A filter chooses cases
for execution, not which language rules apply to their files. Explicit path
selection may choose a smaller set of files to discover and check.

| Invocation                 | Behavior                                                                                                                                                                                                |
| -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `meowy test`               | Check, build, and execute the selected cases.                                                                                                                                                           |
| `meowy test --filter TEXT` | Select IDs containing this case-sensitive substring. Repeated filters are alternatives (OR).                                                                                                            |
| `meowy test --list`        | Check/discover and print selected IDs with skip reasons; no linking, execution, or test-session publication. Static failures print without saved occurrences/capsules; use `--no-run` to preserve them. |
| `meowy test --no-run`      | Build the selected harnesses and publish a test build session with zero case executions. Useful for a different target.                                                                                 |
| `meowy test --show-output` | Also display the captured streams of passing cases.                                                                                                                                                     |

`--list` and `--no-run` cannot combine; `--show-output` requires execution.
`--profile`, `--target`, `--offline`, and normal terminal options retain their
CLI meanings. `--entry`, `--output`, and `--report` do not apply to `test`.
Project runner policy has no environment-variable or extra CLI overrides.
Ordinary `meowy check` keeps its application/entry checking scope; use
`meowy test --no-run` to check and build the test graph explicitly.

## Case lifecycle and watchdogs

After successful checking/building, the supervisor launches each active case in
a fresh process. It initializes only that case's required module graph, invokes
the callback, completes callback cleanup/joins, performs module teardown, and
reaps the case process. No case borrows an owner from the supervisor or another
case. Skipped cases launch no process.

Each case starts in the canonical project root with closed standard input;
use explicit reader fixtures for input. It inherits the invocation's host
environment under the ordinary system API rules. Environment values can be test
inputs, but cannot override manifest runner policy. Capture relevant inputs under
the existing diagnostic capture rules; inheritance alone does not make them
replayable.

A child sends its completion record after callback and module teardown. The
supervisor reports a pass only after receiving that valid record, reaping a
normally completed process, and checking captured output against its limit.
Buffered excess output cannot become a pass because a completion record arrived
first. The completion channel is separate from stdout/stderr: printing `PASS`,
calling a foreign process-exit function, or exiting with status zero before that record
cannot impersonate a passing test. Unexpected exits or signals use `T003` and
retain the native status and phase when known.

The supervisor measures `timeout_ms` from launching the case, including startup,
module initialization, callback execution, joins, and teardown. Build time and
time waiting for a process slot are outside that budget. On expiry it requests
process termination and reaps the process, reporting `T002`. Host termination
and reaping can take longer than the budget; this is not a hard bound on the
entire command.

`T002` is not `tasks.Timeout`. A case may handle a cooperative task timeout and
pass normally, while a watchdog may stop a blocked foreign call or an incorrect
join order. A killed process cannot promise cleanup, flushed output, descendant
process cleanup, or a complete replay capsule. If an assertion was observed
before cleanup became stuck, retain it as related evidence while the terminal
case outcome describes the watchdog failure. Count that as one failed case.

Capture stdout/stderr concurrently under the combined output limit. Crossing it
fails the case with `T004`, retains the available prefix marked incomplete, and
terminates/reaps the process. A failure cannot become success just because some
captured diagnostic text was omitted. Basic structured failure metadata must
remain available when richer evidence storage is exhausted.

An abruptly terminated case may leave descendants holding capture pipe ends.
The supervisor must not wait indefinitely for EOF from those descendants: retain
the available bytes, close capture endpoints after bounded draining, and mark
the streams incomplete. Report any known remaining host effects; process isolation
does not claim to undo them or automatically contain arbitrary native code.

With `fail_fast`, finish observing already launched cases and mark selected cases
not launched as `not_started`. Such a case is neither a pass nor an explicit
skip. Final counts and per-case summaries use sorted ID order even when parallel
completion arrives in another order.

## Repeat random inputs deliberately

`testing.seed()` returns the active case's immutable `<uint64>` seed, including
when called by that case's child tasks. It is available in the case process from
initialization through teardown; calling it outside that runtime raises `P005`
with an unavailable-case-context message. It is not a compile-time value.

Construct `random.seeded(testing.seed())` inside a case to generate repeatable
inputs. This does not replace `random.secure`, change a global generator, or
stabilize task scheduling, clocks, file contents, or remote services. Pass a
generator or derived inputs explicitly to helpers rather than sharing mutable
random state between children.

Seed derivation version 1 is target-independent. Hash these concatenated bytes
with SHA-256: UTF-8 `meowy-test-v1`, one zero byte, the manifest seed as an unsigned
eight-byte big-endian integer, the case ID's UTF-8 byte length in the same form,
and the case ID's UTF-8 bytes. Interpret the first eight digest bytes as unsigned
big-endian `<uint64>`. Record base seed, derived seed, derivation version, and the
selected library's pseudorandom algorithm identity in the case metadata.

Filtering or reordering other cases therefore does not change this case's input
seed. Renaming its identity does. Reproduction uses the saved identity and seed,
not a newly calculated value from the edited checkout. A bounded table/loop of
generated examples is still one case, stopping at its first failed assertion;
record the iteration and input in the assertion message when useful.

## Reports and reproducible failures

The following shortened transcript illustrates one failed comparison; it is not
a measured run. Case ordinals and diagnostic occurrence numbers are separate:

```text
$ meowy test

meowy v0.0.1
testing 2 cases

ok 1 tests/arithmetic_test.mwy::tests::adds
not ok 2 tests/arithmetic_test.mwy::tests::keeps_literal_text

1 - panic[P005]: the label stays unchanged
  --> tests/arithmetic_test.mwy:11:9
   = case: tests/arithmetic_test.mwy::tests::keeps_literal_text
   = expected: "meowy"
   = actual:   "mewy"
   = first difference: byte 3

1 passed; 1 failed; 0 skipped; 0 not started

reproduce: meowy err reproduce 1 --test
```

The transcript assumes the second case's actual text was edited to `"mewy"`.
Reports label callback, initialization, and teardown failures distinctly. Failed
cases show available captured output; passing output is shown only when requested.
Human summaries, diagnostics, and labeled case streams go to stderr; `--list`
uses stdout for its listing. Live output from concurrent cases is not interleaved
into the parent's streams. Durations, native failures, and scheduling evidence
remain observations rather than fixed expected output.

Each normal `test` or `test --no-run` invocation publishes one completed project
test session, including successful runs; its children never update last-run
pointers individually. `--list` does not publish a session. The test namespace
is separate from application entry sessions:

```sh
meowy err summary --test
meowy err explain 1 --test
meowy err inspect 1 --test --trace ownership --trace tasks
meowy err reproduce 1 --test --verbose
```

Run these from the owning project. `--test` cannot combine with `--entry`.
`--session ID` selects an older test session and follows the normal exclusivity
with `--previous`. Cleanup obeys the same namespace selection: default entry
cleanup does not remove test sessions, and `err cleanup --test` selects only test
sessions. A source repair still validates against the saved case's source graph.

A case capsule records its stable ID, source and toolchain inputs, selector and
policy, seeds, observed phase, streams, supervision outcome, and the exact case
binary when one was produced. A source/build failure instead captures that
earlier phase. An assertion panic is a normal panic artifact with test context.
A handled error, matching expected panic, or skipped case creates no failure
occurrence merely because its code exercises a failure path.

The harness enables [recording profile 1](../replay-recording.md) for each executed
case by default, with that profile's fixed event/input budgets and instrumentation
costs. There is no test recording toggle in this revision. Listing, checking,
skipped cases and `--no-run` do not execute recording boundaries. Unsupported
effects still run in the original case and mark its capture external; capture
limits do not turn passing case results into failures. The separately configured
case output limit and watchdog remain enforced test outcomes.

Reproduction selects the failing case under its preserved supervision, not the
whole current suite. Case replay keeps its labeled captured streams on stderr,
as the test runner does. Exported case capsules preserve that mode. They retain
existing [integrity and replay fidelity rules](../diagnostics.md#replay-fidelity).
Watchdog/host failures may lack a closed recording; report the missing evidence
instead of claiming a deterministic replay of a killed process or a proof of
deadlock. Existing trace areas cover the compiler, owners, tasks, channels, and
clocks; case supervision is additional metadata, not an invented trace source.

Manifest/configuration, source/build/case failures, and disallowed empty selections
return status `1`. Invalid command usage returns `2`. A fully passing selection, all-skipped
nonempty selection, or explicitly allowed empty selection returns `0`.
Successful `--list` and `--no-run` also return `0`, without claiming executed cases.
See [test diagnostic codes](../diagnostic-codes.md#test-runner-failures) and
[the testing cookbook](../../pawterns/testing.md) for failures you can exercise.
