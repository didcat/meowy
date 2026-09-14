# Standard library

[Documentation index](../../README.md)

The standard library gives meowy programs a shared vocabulary for storage, text,
time, data, and operating-system services. Import a foundational module with
`@"name"`; its contract is versioned with the language. These names resolve without
an entry in `mod.mwy`. They are ordinary values, so functions, types, constants,
and constructors can be aliased or shadowed without adding keywords.

An import makes the module's API available; it does not request the whole standard
library in the executable. Reachable operations retain their helpers, data,
cleanup, and runtime services. Runtime lookup may require a complete data set,
while a direct operation can need much less. [Memory and binary optimization](../optimization.md)
connects these API choices to stack, heap, static storage, and linker behavior.

## Modules

| Import                                 | Use it for                                                                           | Contract                                                                 |
| -------------------------------------- | ------------------------------------------------------------------------------------ | ------------------------------------------------------------------------ |
| `@"core"`                              | Predefined values and fundamental type values                                        | [Core](core.md#predefined-values)                                        |
| `@"debug"`                             | Diagnostic printing and panics                                                       | [Diagnostic output](core.md#output-and-text)                             |
| `@"errors"`                            | Custom failure types, codes, messages, typed payloads, and explicit boxing           | [Errors and custom failures](errors.md)                                  |
| `@"testing"`                           | Assertions, suite values, expected panics, skipped cases, and reproducible test runs | [Testing](testing.md)                                                    |
| `@"proof"` | Compile-time value/capability queries, bounded analysis, and static contract tests | [Proof queries](proof.md) |
| `@"bytes"`                             | Initialized byte storage, views, copying, and searching                              | [Bytes](text-and-data.md#bytes-and-utf-8-strings)                        |
| `@"strings"`                           | Borrowed text, parsing, and explicitly owned text                                    | [Text](text-and-data.md)                                                 |
| `@"unicode"`                           | Scalars, grapheme boundaries, normalization, and case folding                        | [Unicode](text-and-data.md#unicode-operations)                           |
| `@"fmt"`                               | Streaming formatted output through a supplied writer                                 | [Formatting](text-and-data.md#owned-strings-and-formatting)              |
| `@"numbers"`, `@"bits"`, `@"math"`     | Checked conversions, numeric utilities, bit operations, and mathematics              | [Numeric utilities](text-and-data.md#numbers-mathematics-and-randomness) |
| `@"random"`                            | Reproducible generators and explicit cryptographic entropy                           | [Randomness](text-and-data.md#numbers-mathematics-and-randomness)        |
| `@"encoding"`, `@"hash"`               | Hex, base64, and stable byte digests                                                 | [Binary data](text-and-data.md#binary-encodings-and-digests)             |
| `@"json"`                              | Bounded typed/dynamic decoding and streaming encoding                                | [JSON](json.md)                                                          |
| `@"memory"`, `@"values"`, `@"dynamic"` | Allocators, layout, aggregate ownership, and explicit type erasure                   | [Storage APIs](memory.md)                                                |
| `@"collections"`, `@"iter"`            | Lists, arrays, vectors, maps, sorting, and pull cursors                              | [Collections](collections.md)                                            |
| `@"time"`                              | Fixed durations, monotonic deadlines, sleeps, timers, and tickers                    | [Time](time-and-date.md#fixed-durations)                                 |
| `@"date"`                              | Civil dates, timestamps, time zones, arithmetic, parsing, and formatting             | [Date and time](time-and-date.md#civil-dates-and-times)                  |
| `@"calendars"`                         | Gregorian, Julian, Hebrew, Chinese, Islamic civil, and Buddhist calendars            | [Calendars](calendars.md)                                                |
| `@"tasks"`, `@"channel"`               | Owned work, cancellation, and bounded message transfer                               | [Concurrency APIs](tasks-and-channels.md)                                |
| `@"io"`                                | Readers, writers, buffers, streams, and partial progress                             | [I/O](io-and-system.md#readers-and-writers)                              |
| `@"path"`, `@"fs"`                     | Lexical paths, files, metadata, and directory traversal                              | [Paths and files](io-and-system.md#paths-are-data)                       |
| `@"env"`, `@"process"`                 | Explicit environment snapshots, argv, and child processes                            | [Processes](io-and-system.md#environment-and-processes)                  |
| `@"net"`                               | Addresses, DNS, TCP/UDP, capability-typed peers and HTTP protocol adapters             | [Networking and peers](net.md), [HTTP contracts](http.md)                |
| `@"tls"`                               | Explicit trust, authenticated encrypted streams and negotiated protocols             | [TLS transports](tls.md)                                                 |
| `@"cli"`                               | Typed application options, subcommands, help, and usage errors                       | [CLI applications](cli.md)                                               |
| `@"ffi"`                               | Declared native layouts and foreign symbols                                          | [Native APIs](ffi.md)                                                    |

All library API chapters live in this directory. The surrounding reference defines
language rules such as [ownership](../memory.md), [collection representation](../collections.md),
and [task lifetimes](../tasks-and-channels.md); a library call follows those rules.

[Pawterns](../../pawterns/README.md) puts these APIs to work in small recipes:
text, typed errors, CLI apps, files, calendars, owned batches, and orderly shutdown.
Its [testing chapter](../../pawterns/testing.md) combines these pieces into suites
with explicit fixtures, failure expectations, and case-process budgets.

## Read the contracts

In API tables, `T` is a compile-time type parameter and `N` is a compile-time
capacity. Names such as `K` and `V` are independent type parameters, as in
`collections.Map<K, V>`; [generic declarations](../types.md#multiple-type-parameters)
explain how to declare your own types and functions with several parameters.
`T or Error` denotes a concrete union, written `<T><Error>` in source.
`&T` borrows and `&!T` borrows exclusively. An unborrowed non-copyable argument
transfers ownership. Collection, task, and endpoint operations borrow their
receiver unless explicitly described as consuming it. Stateful mutation requires
exclusive access; selecting a method does not execute it.

A callable parameter describes a statically checked signature and a concrete
capture environment. Passing `file.read`, a hash function, or a CLI validator
never requests implicit interface lookup, callback boxing, or an allocator.
Generic construction may specialize its result type at compile time, as with
`cli.command`; it cannot inspect live process inputs during that construction.

Tables abbreviate names within their module. For example, `Duration` in the time
chapter means `time.Duration`, and `RangeError` there means the named module's
concrete error type. All library errors satisfy an `<error>` predicate; this does
not erase their payloads into a dynamically allocated common error object.
Use [errors and custom failures](errors.md) to define a failure type, construct
and inspect its values, and choose concrete storage or explicit erasure.

## Storage, effects, and failure

- Inline values, bounded lists, and borrowed views allocate no language heap
  storage. Growing text, vectors, maps, and decoded documents take an allocator.
  An owner retaining an allocator cannot outlive it.
- Host resources expose their acquisition and cleanup explicitly. OS handles and
  executor facilities can consume host/runtime resources even when a result has
  an inline representation. Their failure contracts remain observable.
- Constructors release partial acquisitions on failure. Operations receiving an
  owned payload return it in their failure type where retry is meaningful;
  ownership does not disappear because an insertion failed.
- Fallible I/O reports committed progress. Buffer-formatting APIs instead validate
  before mutation and preserve the caller's bounded output on failure. Their
  returned views borrow that output and expire before it is changed or dropped.
- Positions in collections, argument vectors, and source text are one-based.
  Raw file/wire offsets, timestamps, and cyclic counters use the units and origin
  stated by their own contracts.
- Domain errors are values. Checked arithmetic or a violated proven precondition
  may panic as specified. Library errors do not automatically become compiler
  diagnostics or terminate the application.

Unicode, time-zone, calendar, and pseudorandom algorithm identities are versioned
inputs. Code must not silently take different rules from a host locale or network
lookup. External observations follow the [replay contract](../diagnostics.md#replay-fidelity);
a saved executable records which effects it can reproduce.

## Start with a complete program

Follow the [time and calendar guide](../../guide/time-and-date.md) to distinguish
elapsed time from civil arithmetic. The [calendar CLI](../../programs/calendar-cli/main.mwy)
combines typed options, explicit argv storage, calendar conversion, and writer
errors. The [custom-errors project](../../programs/custom-errors/README.md) defines
nominal failures, retains a typed parsing cause, and inspects errors without
allocation. The [worked projects](../../programs/README.md) include dedicated Unicode,
JSON, map, random, ticker, nested-command, and file-streaming examples. Each has
its own manifest, inputs, failure walkthrough, and ownership explanation.
