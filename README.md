<img src="assets/m.png" alt="m—meowy's mascot (aka my cat)" width="220">

# meowy

**meowy** is a small, composable systems programming language built around
values, blocks, emissions, dispatch, and matching.

Its guiding principle is **what you write is what you get**: data has a concrete
representation, allocation has a visible cause, and concurrent work has an owner.
The same block vocabulary describes a calculation, a record, a module, or a
function body.

The grammar is punctuation-based and has no reserved keywords. `true`, `false`,
`null` and types are well-known values and bindings. `$` denotes the dispatch
receiver; `self` is an ordinary name. See [names and syntax](docs/reference/syntax.md#no-keywords).
Spaces are optional: `value<T>` tests a type and `value~<T>` explicitly ascribes it.
Punctuation can delimit a complete program without whitespace.

```meowy
<Reading> : <{
    sensor <uint16>
    value <int32>
}>

reading <Reading> : {
    -> sensor : 7
    -> value : 24
}

double <int32> : (value <int32>) {
    -> value * 2
}

sample : {
    -> reading.value.(double)
    -> sensor : reading.sensor
}

sample         # primary value: 48 #
sample.sensor  # named value: 7 #
```

Emitting does not end execution. A block produces at most one primary value and
one value for each emitted name on any execution path. Its fields have a static
shape; composing blocks does not require a dynamic object table.

Current version: v0.0.0

v0.0.1 is the first full release target, covering the documented language and
ecosystem for the initial supported platform. Full release scope belongs to
v0.0.1, not a future v1.0.0. The major and minor numbers have retired; the patch
number handles everything. See [Versioning, allegedly](VERSIONING.md).

## Documentation

Start with the [guide](docs/guide/README.md), or use the
[documentation index](docs/README.md) to browse the full reference.

To work on the compiler, read [Building the first meowy compiler](COMPILER.md)
for the recommended languages, toolchain, architecture, and release gates.
Use [repository verification](tools/README.md) to check documentation contracts,
editor support, and explicitly selected compiler/runtime checks. The
[native cleanup prototype](runtime/README.md) exercises ownership cleanup outside
the compiler; [STATUS.md](STATUS.md) tracks implementation and remaining work.

For something to build, open [Pawterns](docs/pawterns/README.md), the meowy
cookbook: 39 recipes from a first greeting through owned buffers, CLI tools,
calendar surprises, and getting concurrent work to actually finish.

| Topic                                                                | What it covers                                                                         |
| -------------------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| [Syntax](docs/reference/syntax.md)                                   | Bindings, literals, operators, functions, and scopes                                   |
| [Values and blocks](docs/reference/values-and-blocks.md)             | Evaluation, emissions, dispatch, matching, and control flow                            |
| [Types](docs/reference/types.md)                                     | Inference, unions, narrowing, records, generics, and conversions                       |
| [Compile-time evaluation](docs/reference/compile-time.md)            | Type-producing helpers, purity, staging, and evaluation budgets                        |
| [Memory](docs/reference/memory.md)                                   | Ownership, borrowing, allocation, raw pointers, and cleanup                            |
| [Memory and binary optimization](docs/reference/optimization.md)     | Storage lifetimes, linker reachability, build policy, and constrained deployment       |
| [Collections](docs/reference/collections.md)                         | Bounded lists, arrays, slices, vectors, and maps                                       |
| [Tasks and channels](docs/reference/tasks-and-channels.md)           | Structured concurrency, deadlines, cancellation, and communication                     |
| [Modules and FFI](docs/reference/modules-and-ffi.md)                 | Imports, exports, reproducible dependencies, and native boundaries                     |
| [Package build graphs](docs/reference/packages-and-builds.md)        | Root lock authority, revision identity, and transitive native inputs                   |
| [Target and native ABI](docs/reference/target-profile.md)            | Initial platform requirements and the explicit C boundary                              |
| [Standard library](docs/reference/stdlib/README.md)                  | Text, data, collections, time, calendars, system services, and CLI apps                |
| [Errors and custom failures](docs/reference/stdlib/errors.md)        | Construct failures, read codes and payloads, preserve ownership, and box explicitly    |
| [Testing](docs/reference/stdlib/testing.md)                          | Assertions, ordinary suite values, isolated cases, fixtures, and reproducible failures |
| [Diagnostics](docs/reference/diagnostics.md)                         | Labeled errors, ranked repairs, replay capsules, and failure behavior                  |
| [Recording and artifact formats](docs/reference/artifact-formats.md) | Versioned schemas, reader compatibility, and recorded event contracts                  |
| [Diagnostic codes](docs/reference/diagnostic-codes.md)               | Rule catalog, required evidence, and repair guidance                                   |
| [Command line](docs/cli/README.md)                                   | Check, build, run, inspect internals, apply fixes, and replay failures                 |
| [Language server](docs/reference/lsp.md)                             | Editor setup, compatibility, analysis, repairs, and manifest configuration             |
| [gatostyle](docs/guide/gatostyle.md)                                 | Configurable layout, code quality, expression styles, and safe fixes                   |

The [worked projects](docs/programs/README.md) put these rules together in 16
self-contained directories, each with its own manifest and run instructions.
They cover composition, custom errors, Unicode, JSON, maps, randomness, tasks,
channels, timers, calendars, nested CLIs, streaming file tools, and a complete
[test suite](docs/programs/testing/README.md). The [design notes](docs/design.md)
explain the choices and the boundaries of the core.
The [conformance cases](docs/conformance/README.md) pin parsing, typing, ownership,
and native-signature decisions with accepted and rejected source fixtures.
The [time and calendar guide](docs/guide/time-and-date.md) builds toward a small
CLI that converts dates between six calendar systems, including Chinese.

## Project files

- `.mwy` files contain meowy source.
- `mod.mwy` describes imports, local path aliases, exports, build settings, and
  optional gatostyle and language-server policy. All project language-server
  configuration lives here.
  See the [manifest guide](docs/guide/mod.md) and its
  [sample](docs/guide/mod.sample.mwy).
- [Vim and Neovim support](editor/nvim/README.md) provides syntax highlighting,
  file detection, and buffer settings.

Inspired by Smalltalk, OCaml, Rust, Zig, Go, Lisp, and my cat.

## License

Licensed under [MPL 2.0](./LICENSE)
