# meowy documentation

This documentation defines the language's semantics and the foundational library
contracts used in its programs. The reference is authoritative; the guide teaches
those rules by building small programs, and the design notes explain their intent.

[Pawterns](pawterns/README.md) is the practical cookbook: 39 recipes for the
language and its ecosystem, from your first project to a greeting that survives
its own concurrency design. Pick a problem, follow the code, and learn why the
working version works.

## Learn the language

1. Read the [guide](guide/README.md) for bindings, functions, blocks, and loops.
2. Read [values and blocks](reference/values-and-blocks.md) to understand what
   emissions do, when code executes, and how primary values compose with fields.
3. Read [types](reference/types.md) and [collections](reference/collections.md)
   before defining public data structures.
4. Read [memory](reference/memory.md) for ownership and predictable storage.
5. Read [tasks and channels](reference/tasks-and-channels.md) before introducing
   concurrent work.
6. Follow the [manifest guide](guide/mod.md) to configure `mod.mwy`, local path
   aliases, package dependencies, and the project entry.
7. Use the [CLI guide](cli/README.md) to check and run programs, then inspect and
   reproduce failures from their saved executable capsules.
8. Set a [gatostyle policy](guide/gatostyle.md) for layout, code quality, and the
   expression forms your project prefers.
9. Follow the [time and calendar guide](guide/time-and-date.md), then build a
   typed application command line with the standard library's `cli` module.
10. Set up the [language server](reference/lsp.md) for editor diagnostics,
    navigation, and repairs, with project configuration in `mod.mwy`.

## Look something up

| Question                                                    | Reference                                                                                           |
| ----------------------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| What does this punctuation mean?                            | [Syntax](reference/syntax.md)                                                                       |
| How do I document APIs without repeating their types?       | [Checked documentation comments](reference/documentation.md)                                         |
| Does `->` return? Does `{ ... }` create a function?         | [Values and blocks](reference/values-and-blocks.md)                                                 |
| Can a mutable binding change type?                          | [Types](reference/types.md)                                                                         |
| How do I declare `D<K, V, Y, Z>` or a generic function?        | [Multiple type parameters](reference/types.md#multiple-type-parameters)                             |
| How do I construct types or check pure helpers?             | [Compile-time evaluation](reference/compile-time.md)                                                |
| How do I accept capturing callbacks without boxing?         | [Callable capabilities](reference/types.md#callable-environments)                                   |
| How do errors and nullable values narrow?                   | [Types](reference/types.md#unions-and-narrowing)                                                    |
| How do I create and read my own errors?                     | [Errors and custom failures](reference/stdlib/errors.md)                                            |
| How do I write and run a test suite?                        | [Testing](reference/stdlib/testing.md)                                                              |
| What can the compiler prove, and how do I test those guarantees? | [Proof queries and static tests](reference/stdlib/proof.md) |
| What can I configure in mod.mwy?                            | [Fully commented manifest](guide/mod.full.mwy)                                                      |
| Where does memory come from, and when is it released?       | [Memory](reference/memory.md)                                                                       |
| How do stack, heap, and static storage affect peak memory?  | [Memory and binary optimization](reference/optimization.md)                                         |
| Why does importing time not link the whole stdlib?          | [Import reachability](reference/optimization.md#how-imports-become-executable-bytes)                |
| How do I deploy on a small machine or in a container?       | [Constrained deployment](reference/optimization.md#docker-images-old-machines-and-virtual-machines) |
| Is `T[4]` an array or a capacity limit?                     | [Collections](reference/collections.md)                                                             |
| Who joins a task? What does cancellation guarantee?         | [Tasks and channels](reference/tasks-and-channels.md)                                               |
| How are native data and functions declared?                 | [Modules and FFI](reference/modules-and-ffi.md#native-interfaces)                                   |
| Which C types and calling conventions are accepted?         | [Native ABI profile](reference/native-abi.md)                                                       |
| Which machine and runtime can run the initial distribution? | [Target profile](reference/target-profile.md)                                                       |
| What does the standard library provide?                     | [Standard library](reference/stdlib/README.md)                                                      |
| How do I compose TCP/UDP peers with sender/receiver capabilities? | [Networking and peers](reference/stdlib/net.md)                                                  |
| How do I add typed HTTP requests, handlers and routers to a peer? | [HTTP protocol contracts](reference/stdlib/http.md)                                              |
| How are encrypted connections and peer identities checked?  | [TLS transports](reference/stdlib/tls.md)                                                             |
| How do durations, dates, and zones work?                    | [Time and date](reference/stdlib/time-and-date.md)                                                  |
| How do I use Chinese or Hebrew leap months?                 | [Calendars](reference/stdlib/calendars.md)                                                          |
| How do I build my own CLI application?                      | [CLI library](reference/stdlib/cli.md)                                                              |
| How do I configure mod.mwy and local import aliases?        | [Manifest guide](guide/mod.md)                                                                      |
| Which dependency revision and build settings win?           | [Package identity and build graphs](reference/packages-and-builds.md)                               |
| How do I enforce coding style and code quality?             | [gatostyle](guide/gatostyle.md)                                                                     |
| How do I configure editor analysis and compatibility?       | [Language server](reference/lsp.md)                                                                 |
| Which failures are values?                                  | [Diagnostics](reference/diagnostics.md)                                                             |
| What does a diagnostic code mean?                           | [Code catalog](reference/diagnostic-codes.md)                                                       |
| How do I check, build, and run a program?                   | [Command line](cli/README.md#check-build-and-run)                                                   |
| How do I review and apply suggested fixes?                  | [Repair workflow](cli/README.md#preview-and-apply-repairs)                                          |
| Can I see the original compiler and runtime evidence?       | [Replay capsules](reference/diagnostics.md#replay-capsules)                                         |
| How do I replay a failure without the project?              | [Export a replay executable](cli/README.md#export-one-executable)                                   |
| Which effects can a recording reproduce?                    | [Recording boundaries and budgets](reference/replay-recording.md)                                   |
| What does each recorded operation contain?                  | [Runtime event registry](reference/runtime-events.md)                                               |
| How do tools read locks, diagnostics and capsules?          | [Artifact formats and schemas](reference/artifact-formats.md)                                       |
| Which small programs pin the language rules?                | [Conformance cases](conformance/README.md)                                                          |

## Read complete programs

The [worked projects](programs/README.md) each include a local manifest, entry,
helper files, and instructions. Start with validation, composition, and binary
headers, then define and inspect [custom errors](programs/custom-errors/README.md).
Continue with Unicode, JSON, maps, randomness, concurrency, timers, and complete
CLI applications. File tools include small fixtures and explicit
instructions for their I/O effects.

The [testing project](programs/testing/README.md) shares a helper between an
application and its suite, then checks domain errors, byte fixtures, panic
expectations, and an owned channel message. Follow the
[testing recipes](pawterns/testing.md) to build your own suite.

[Design notes](design.md) record the language's main decisions.
The [manifest guide](guide/mod.md) explains project configuration alongside its
[sample manifest](guide/mod.sample.mwy).

## Conventions

Code fences marked `meowy` use language syntax. A snippet explicitly labeled
**invalid** demonstrates a diagnostic. API tables describe contracts; angle
brackets in prose denote types, not command placeholders.

Reference rules use **must** for requirements and **may** for permitted choices.
Target-dependent properties, such as pointer width and C layout, must be supplied
by the selected build target. No sample module version or performance measurement
is implied by a code listing.

Follow the [documentation conventions](guide/documentation-style.md): four-space
indentation, readable punctuation spacing and lowercase `meowy`. Compact examples
are reserved for explicitly labeled syntax or formatter demonstrations. Gatostyle's
layout engine preserves structure, while optional coding-style fixes require
proof that their rewrites preserve behavior. Spaces never select language meaning.

Shell examples use uppercase placeholders such as `PATH` and `TRIPLE`; help
output may use `<id>` for a command argument. Those command placeholders are
separate from meowy's type syntax. Diagnostic transcripts illustrate the format
and contract rather than reporting a particular local run.
