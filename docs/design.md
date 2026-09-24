# Design decisions

[Documentation index](README.md)

meowy's small vocabulary is useful only when its combinations have predictable
meaning. These decisions keep the composable surface while making storage,
execution, and failure visible.

## Preserve the core

Blocks construct values with a primary and named fields. Emitters initialize
those components without ending execution. Dispatch composes transformations.
Independent matchers control which statements run. Named scopes express early
completion and repetition.

The grammar has no reserved keywords. Punctuation supplies structure; names
refer to ordinary values, type values, or scoped operations. `true`, `false`,
`null`, and primitive types come from a small predefined environment. `$` denotes
the nearest dispatch receiver; `self` is an ordinary name. Ordinary names support
lexical shadowing and aliasing; only a nested dispatch introduces another `$`. Compiler knowledge follows an intrinsic's identity,
not the letters in its name.

Lower-level features follow that rule too: `&!` marks an exclusive borrow,
`!{ ... }` marks a caller-proven safety boundary, and constraints appear after
`:` in a generic binder. Lifetimes follow the borrow contract without a word-based
clause. `ffi.record` chooses native layout through an ordinary compile-time call.

This provides the common vocabulary for functions, records, modules, and
configuration. It does not make their execution rules interchangeable: a block
evaluates now, a function evaluates on a call, a field access reads data, and a
module initializes once. Parentheses are required for calls, including zero-argument
calls. Reading a value must never accidentally repeat I/O.

## Make the type system explain the storage

A mutable binding retains its declared or inferred type. Unions represent actual
alternatives; subtraction operates on types, and narrowing requires control-flow
evidence. Generic parameters describe compile-time specialization rather than a
placeholder that can turn any string into any requested type.

Record shape is structural and exact. A record with a null primary remains a
record. Type predicates inspect the complete value, while scalar operations can
inspect a compatible primary. Named composition cannot conceal collisions.
Owned type erasure is explicit and preserves destruction information.

Fixed-width numbers, checked arithmetic, concrete union storage, and explicit
native layout allow a programmer to reason about costs before running a program.
The native ABI is a boundary, not a property inferred from similar-looking syntax.

## Distinguish capacity from allocation

`T[N]` preserves the bounded-list meaning: up to `N` initialized elements inline.
An exact-size array is a different type. `T[]` is a borrowed slice; a growable
vector takes an allocator. Named list positions remain bounded literal aliases,
while runtime key lookup uses a map with its own storage and ordering contract.

One-based indexing remains part of the language. Binary offsets are translated
explicitly, and raw pointers do not reuse list indexing as unchecked arithmetic.
Large value copies are possible and visible; references avoid them when needed.

Imports expose values; reachable uses determine the required machine code, data,
and runtime services. Optimization preserves initialization, failures, ownership,
and cleanup. [Memory and binary optimization](reference/optimization.md) connects
those rules to storage lifetimes, linking, and deployment budgets.

## Give work an owner

`>>` starts a task and `<<` joins it. A task has one result owner and a lexical
lifetime. A group is bounded, returns outcomes in submission order, and retains
slots for errors and cancellation. Groups are not globals or implicitly lazy
iterators. A function cannot hide an escaping child behind an ordinary result.

Cancellation requests cleanup; it does not destroy a running thread. A deadline
is a monotonic value with explicit units and does not waive a join's lifetime
obligation. The compact timed forms `>1000>` and `<1000<` are replaced by
`.deadline(time.after(time.Second.scale(...)))`, keeping timing policy out of
punctuation.

Channels carry many messages, use explicit capacity for backpressure, and move
message ownership. Separate sender and receiver endpoints make shutdown precise.
The receiver gets `Item<T>` or `Closed`, so sending a nullable payload does not
make end-of-stream ambiguous. Failed sends return their unsent owner.

## Keep ordinary programs safe at a low level

Inline storage and explicit allocators avoid automatic garbage collection.
Moves, borrows, and deterministic cleanup keep ownership inspectable. Explicit
`!{ ... }` blocks permit native memory operations with stated preconditions; they
cannot excuse a data race or extend a dead allocation's lifetime.

Formatting can stream output. Allocated strings, vectors, erased owners, task
storage, and channel queues have visible construction boundaries. A library
cannot promise allocation-free behavior while hiding a dynamic lookup table
behind field syntax.

## Compose a useful standard library

The [standard library](reference/stdlib/README.md) applies the same rules to text,
collections, system services, and application command lines. Callables and
allocators are explicit; typed command descriptions produce statically known
parse results. Convenience does not require hidden maps, process exits, or
background work.

Elapsed durations, monotonic instants, civil timestamps, and calendar dates have
different types. A fixed Day is 24 hours; a calendar day follows date and zone
rules. Calendar profiles preserve leap-month identity and pin their rule data,
including Chinese lunisolar conversions. A selected calendar never guesses a
zone, locale, or observational religious convention.

## Keep configuration and tools small

A manifest declares a graph of inputs and exports. Dependency selectors resolve
to locked immutable content. Compile-time evaluation is pure and bounded; it
cannot execute arbitrary dependency-provided build scripts.

[Gatostyle](guide/gatostyle.md) separates layout from configurable coding-style
and quality rules. Projects can prefer different equivalent expression forms,
including call chains, dispatch, or intermediate bindings. Layout preserves the
syntax tree; semantic fixes must prove that types, evaluation, ownership, cleanup,
and task boundaries remain equivalent. An unproven rewrite stays a suggestion.

No style policy changes the language grammar or weakens its checks. Spaces do not
distinguish a type test from an ascription: matcher context does. Source may use
zero spaces, with punctuation supplying annotations and statement boundaries.

## Keep failures executable and explainable

A diagnostic should survive the terminal session that printed it. Numbered
occurrences keep their source spans, repair candidates, and an executable replay
capsule containing the failing inputs and tools. A single replay command can show
the violated type constraint, ownership path, concrete storage cost, or recorded
task and channel events that explain the failure.

The capsule preserves the failing phase: checking errors replay checking, native
link failures replay linking, and runtime failures replay the built program.
Recorded evidence stays distinct from facts derived during replay. Capturing a
schedule or external input has an explicit cost and a stated completeness limit.
The [diagnostic contract](reference/diagnostics.md#replay-capsules) and
[CLI workflows](cli/README.md) define those boundaries.

## Extension boundaries

The following belong behind explicit library contracts or a separate language
proposal, rather than unspecified behavior inside the core:

- Shared ownership and user-defined destruction protocols, including cycle policy.
- Completion-order selection over several channels, including fairness rules.
- General lazy pipelines and asynchronous streams beyond the standard library
  cursor contracts.
- Dynamic library loading, managed callbacks, and foreign exception translation.
- Extra numeric formats, SIMD, packed data, and platform-specific intrinsics.

Any extension must state its representation, lifetime, allocation behavior,
failure outcomes, and interaction with scope cleanup. Surface shorthand comes
after those rules are clear.
