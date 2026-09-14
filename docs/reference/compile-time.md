# Compile-time values and evaluation

[Documentation index](../README.md)

Compile-time evaluation constructs types and closed descriptions using ordinary
punctuation and checked arithmetic. It does not run module initializers or acquire
application resources. Failed required evaluation is a checking error, never an
operation deferred to program startup.

## Type values and user helpers

`core.Type` is the well-known, compile-time-only type of a concrete type value.
`<int32>`, a type query `value<>`, and a type constructor's result have this type.
Its name is shadowable. Type values have no runtime size, address, serialization,
or allocator. They cannot inhabit runtime records, unions, boxes, task messages,
or callable environments. `memory.size_of<core.Type>()` is `E211`.

`<Name>` resolves a type name; `<(expression)>` evaluates a value expression whose
result must be `core.Type`. Type literals can be value arguments. There are no
implicit string-to-type conversions or runtime reflection tables.

Compile-time type values support `==` and `!=` by normalized type identity, and
type literals compose unions/subtraction as in the type reference. These
operations have no runtime `core.Type` counterpart.

```meowy
core : @"core"
-> optional <core.Type> : (element <core.Type>) { -> <(element)><null> }
-> bounded <core.Type> : (element <core.Type>, capacity <usize>) {
    -> <(element)[capacity]>
}
<MaybeCount> : optional(<uint32>)
<Counters> : bounded(<uint32>, 8)
```

A computed type atom `(expression)` takes the same extent suffix as a named
atom: `<(element)[capacity]>` constructs a bounded-list type, and `<(element)[]>`
a slice. The capacity is a non-negative representable compile-time integer;
overflowing the target layout is `E104`. This is type construction, not indexing.

A function with a compile-time-only parameter or result is a compile-time-only
callable. All its arguments must be compile-time values, including ordinary
integers such as `capacity`. Its body is checked symbolically, then evaluated
for each concrete call; its parameters may supply computed types and extents.
A public helper has explicit parameter/result annotations. Exporting it exports
a compile-time binding without runtime code or initialization.

Closed descriptors for `cli.command`, `testing.suite`, `errors.define`, and
`ffi.record` use their documented compile-time constructors. Only an intrinsic
may create its own opaque descriptor kind or grant nominal error/native identity.
User helpers may compose or return their results without acquiring those
privileges. Generic `:T` still binds a type, not an integer; use a type-producing
helper for reusable capacity construction.

## Required evaluation and staging

Required evaluation roots are type aliases/annotations/extents needing evaluation,
module-level compile-time-only bindings, manifests, and closed intrinsic
descriptions. Nested helper/constructor calls belong to the enclosing root and
do not reset its budgets. Overlapping roots form one outer root.

Literals, immutable values computed from eligible inputs, and pure helper
declarations are available. Local mutable scratch and loops are allowed; mutable
state cannot escape the root. Runtime parameters, mutable module globals,
environment queries and application initializers are not compile-time inputs.
An immutable module binding is eligible only if its initializer is transitively
evaluable under these rules, without executing module initialization.

A pure ordinary function called at runtime remains a runtime operation. Optimizer
folding does not make a runtime-dependent type legal. Type queries inspect types
without evaluating operands. Target queries and pointer-width arithmetic use the
selected target, never the compiler host's representation.

## Proof observations

[`@"proof"`](stdlib/proof.md) defines compiler-known, non-evaluating observation
parameters. A query can describe a runtime place and return an opaque compile-time
`proof.Result` without reading that place or making its runtime value available to
required evaluation. Ordinary helpers do not acquire this parameter mode.

The proof package fixes its own canonical analysis revision and three outcomes;
its compile-time descriptor types may form metadata unions/collections but have no
runtime representation. Scalar projections may be materialized as constants. They
cannot determine types, extents, specialization, imports, or ownership acceptance,
or feed another observation. These backward dependencies use E225. Invalid proof
observations/descriptors use E223 and failed proof expectations use E224.

Proof roots share the logical limits below; budget exhaustion is E220, never an
indeterminable answer. The package's specified interval abstraction is a precision
rule, not permission to replace an exhausted computation with an unknown result.

## Purity

`core.Pure` is a compiler-checked callable capability, not a keyword or an opt-in
promise. Pure callables may read arguments and immutable captures, construct
inline results, mutate fresh local scratch, compare, branch, restart, and call
other proven-pure callables. They cannot mutate caller storage, mutable captures,
or globals; perform I/O; read clocks or ambient randomness; submit/join tasks;
acquire locks; use raw-pointer/foreign operations; or acquire/release runtime
resources or allocator-backed owners. A local counter is fine; incrementing one
supplied by reference is not.

Checked arithmetic/bounds failures do not make a function impure. Required
evaluation reports their existing static diagnostics; recoverable error results
remain values. `debug.panic` is not a compile-time operation. Intrinsic descriptor
validation reports checking errors without running runtime diagnostics.

Derive purity from a source function's body, captures and transitive calls.
Recursive source definitions are checked together: operations outside calls to
the group must be pure. A forward group's statically resolved internal calls
participate in this check; its complete definitions establish the item capabilities.
Indirect calls are effectful unless their generic contract supplies
both a callable signature and `core.Pure`. Retain the resulting capability in
the concrete function item's compiled interface, available without inspecting its
body. A plain function-pointer type alone does not preserve purity; a constrained
generic parameter retains the item type and its proof. A pointer whose identity
was erased cannot regain purity from its signature. Intrinsics qualify only if their contract
explicitly allows compile-time evaluation or declares `core.Pure`.

This capability also governs runtime pure callbacks such as CLI validators.
Purity does not mean argv is known before execution. Static-message constructors
such as `cli.invalid` may appear in pure runtime bodies when their static
arguments are available. Forbidden compile-time effects use `E219` with the call
chain; runtime dependencies of static arguments use `E211`.

## Evaluation budgets

Language contract revision 1 uses fixed per-root limits. They are not manifest
settings and cannot be relaxed by an optimization profile.

| Budget                      | Limit      | Accounting                                                                                    |
| --------------------------- | ---------- | --------------------------------------------------------------------------------------------- |
| Evaluation steps            | 1,000,000  | One per evaluated statement/expression node; repeated visits count again.                     |
| Active source-helper calls  | 256        | Increment on entry, decrement on return.                                                      |
| Constructed aggregate slots | 1,048,576  | Cumulative initialized elements, fields and primary slots, including materialized copies.     |
| Materialized text bytes     | 16,777,216 | Cumulative UTF-8 bytes produced by literals, copies or formatting.                            |
| Constructed type nodes      | 65,536     | Cumulative primitive references and composite nodes visited during construction/substitution. |

Evaluate in source order; unevaluated branches and type-query operands are not
charged. Parentheses/comments add no expression nodes. Literal elements are
evaluated expressions as well as constructed slots. Intrinsics charge one call
step plus one step per descriptor/list element or UTF-8 byte inspected; results
also charge construction budgets. Type construction charges one step and one
type node per visited node. An array extent does not construct its elements.
Reading an available immutable input charges a read, not a copy of the object;
materializing a copy charges its slots/text recursively.

These are logical counters, not elapsed time or compiler-memory limits. Caching,
sharing, folding and parallelism must not change acceptance: charge as if the
root were evaluated afresh. Exhaustion reports `E220`, identifying the root,
counter and helper stack. Host resource failure is a tool failure, not evidence
of source-budget exhaustion. An infinite pure restart therefore fails
deterministically. Changing these limits requires a language contract revision.
