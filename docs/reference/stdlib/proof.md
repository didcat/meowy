# Proof queries

[Library index](README.md) · [Compile-time evaluation](../compile-time.md) · [Ownership](../memory.md)

Contract revision: **1**. `@"proof"` exposes bounded, deterministic queries about
facts established during checking. This is a language/library reference, not an
implementation claim. The bootstrap does not implement this package. All source
examples in this chapter are specification examples awaiting compiler qualification.

```meowy
proof : @"proof"

x <int32> : 5
result <proof.Result> : proof.is(x, 5)
proof.assert(result)
```

The assertion succeeds during checking. No read of `x`, comparison, allocation,
borrow, or assertion code is emitted for these operations.

## Purpose and non-goals

A query asks what the specified analysis establishes **at its source point**.
It does not ask for the value the program will observe on one execution. Ordinary
`x == 5` remains the operation for a runtime equality test. Booleans still contain
only `true` and `false`; uncertainty belongs to proof results.

Revision 1 provides value equality, finite membership, integer intervals, type
capabilities, and probes of immediate ownership operations. It is not an arbitrary
theorem prover, an optimizer API, a runtime verifier, or a way to suppress errors.
Proof results never create a capability, refine a runtime value, waive a bounds
check, authorize a move/borrow, or extend a lifetime.

The analysis must be sound and follow this revision's precision rules exactly.
`Always` and `Never` are guarantees, not guesses. `Indeterminable` is an ordinary
successful query result, not a compilation failure or permission to proceed.

## Module identity and staging

Import `proof : @"proof"`. Exports resolve through ordinary bindings and can be
aliased or shadowed. No export name is a keyword. Intrinsic behavior follows the
resolved package member, including through aliases, never its spelling.

`proof.revision` is the compile-time `uint32` constant `1`. The language contract
selects this revision; a build flag or optimization profile cannot select a
stronger analysis. This package has no initialization effects, host dependencies,
allocator parameter, background worker, or runtime solver.

An observation intrinsic receives a description of its observation arguments,
not their runtime values. This is a compiler-known parameter mode, like the
non-evaluating nature of a type query; it introduces no new source syntax.
Only the intrinsics explicitly listed as observations have this privilege.

```meowy
proof : @"proof"
is_five : proof.is

x <int32> : 5
result : is_five(x, 5)
```

The query is anchored at the `is_five` call. An ordinary wrapper does not acquire
observation parameters: a helper returning a proof descriptor is compile-time-only
and can receive only compile-time arguments. It cannot accept a runtime value
and silently quote the caller's expression. Helpers can combine existing results.

Proof results, bounds, and inspection records are compile-time-only. They have
no native representation, size, address, destructor, equality operator, or runtime
serialization. They cannot enter runtime storage, `any`, captures, tasks, FFI,
or a module's runtime exports. Static metadata exports are permitted; private
source identities must not be exposed by inspecting an exported descriptor.

Their scalar projections, such as `.always` or `.lower`, are ordinary compile-time
constants and may be materialized in runtime scalar storage. This does not carry
proof authority into that storage. A result computed in a function body depends
on that body's declared inputs and local facts, not on an optimizer's chosen caller.

## Result types and meaning

| Type | Meaning for an admissible proposition at its observation point |
| --- | --- |
| `proof.Always` | The proposition holds for every state admitted by the canonical analysis. |
| `proof.Never` | Its negation holds for every admitted state. |
| `proof.Indeterminable` | Neither universal conclusion is established. |
| `proof.Result` | The closed union `<proof.Always><proof.Never><proof.Indeterminable>`. |

The three alternatives are disjoint, opaque nominal descriptor types. Only proof
intrinsics construct them. A same-shaped record or a type ascription cannot forge
one. All observation APIs declare `proof.Result` as their return type, regardless
of the answer; ordinary inference does not change that signature to one alternative.

```meowy
proof : @"proof"
x <int32> : 5
result : proof.is(x, 5)
<Result> : result<>
```

`Result` denotes `proof.Result`. Its compile-time active alternative is
`proof.Always`. An ordinary type predicate can inspect that alternative in a
compile-time context. The descriptor union is not a runtime tagged union.

Every alternative and `proof.Result` expose these immutable scalar projections:

| Projection | `Always` | `Never` | `Indeterminable` |
| --- | --- | --- | --- |
| `.always <boolean>` | `true` | `false` | `false` |
| `.never <boolean>` | `false` | `true` | `false` |
| `.indeterminable <boolean>` | `false` | `false` | `true` |

Exactly one flag is true. `!result.always` means either `Never` or
`Indeterminable`; it is not equivalent to `result.never`. Results have no
truthiness and cannot be supplied directly as matcher conditions.

Each descriptor retains an internal proposition, observation origin, and analysis
revision for diagnostics. These are compiler metadata, not user-readable addresses
or a public serialized format. Copying a descriptor preserves its origin; it does
not requery the current state. A descriptor remains inspectable after its subject
changes or leaves scope because it retains no runtime reference to that subject.

A value query at an analysis-unreachable point yields `Indeterminable`, never a
vacuous `Always` or `Never`. Invalid syntax, types, or observations still fail
checking. Type-only capability queries do not depend on point reachability.

## Inspection, assertions, and composition

The API tables use the library's usual signature notation. `Result` means
`proof.Result`, and `message` must be a compile-time string. A message may be
omitted; the compiler then generates one from the query and assertion origins.

| API | Result | Contract |
| --- | --- | --- |
| `inspect(result <Result>)` | `proof.Flags` | Return the three boolean flags without asserting any outcome. |
| `assert(result <Result>, message <string>)` | `proof.Always` | Require `Always`; otherwise fail checking with E224. |
| `expect<S>(result <Result>, message <string>)` | `null` | Require exactly alternative `S`, one of the three nominal result types; mismatch is E224. |
| `not(result <Result>)` | `Result` | Negate the anchored proposition using the table below. |
| `all(results)` | `Result` | Conjoin an immutable compile-time bounded list of results. |
| `any(results)` | `Result` | Disjoin such a list. |

`proof.Flags` is an opaque compile-time inspection record with exactly the same
three projections as a result. `inspect` is redundant with direct projections
but supplies an explicit inspection boundary. There is no assertion hidden in it.
`expect` is intended for tests of all outcomes, especially `Indeterminable`.

```meowy
proof : @"proof"
x <uint8> : 5
result : proof.is(x, 5)
flags : proof.inspect(result)
proof.expect<proof.Always>(result)
proof.assert(result).always
```

The final expression is the constant `true`. `proof.assert(result).indeterminable`
can never yield `true`: an indeterminable argument fails before field selection.
Assertions are compile-time obligations in every checked body, even one that is
not called at runtime. They are not runtime panics and cannot be hidden behind
a runtime condition. Testing an observation does not execute its subject.

| Input | `not` |
| --- | --- |
| `Always` | `Never` |
| `Never` | `Always` |
| `Indeterminable` | `Indeterminable` |

`all` returns `Never` if any input is `Never`, `Always` if all are `Always`, and
`Indeterminable` otherwise. `any` returns `Always` if any input is `Always`,
`Never` if all are `Never`, and `Indeterminable` otherwise. Empty `all` is `Always`;
empty `any` is `Never`. Their propositions are respectively true and false.

List elements are formed in source order before composition. `all`/`any` do not
suppress invalid queries or failed assertions while constructing their inputs.
They combine outcomes only: `all([r, not(r)])` remains `Indeterminable` when `r`
is indeterminable. They do not invoke another solver or recover correlations.
Inputs may describe different source points; the result combines those historical
propositions, not simultaneous claims about one current state.

## Admissible value observations

Value observations accept scalar literals, parenthesized observations, or resolved
runtime places: bindings, field paths, dereferences, and list/array elements.
A negative integer literal is accepted as one literal observation. An index must
be a compile-time integer or a stable integer place whose singleton index is
known in the canonical analysis. Ordinary privacy and name resolution apply.

The place must be live, initialized, and readable in the frozen base analysis.
Dereference provenance, index bounds, and initialized list length must be proved
without performing a runtime check. Failure to establish these prerequisites is
an invalid observation (E223), not `Indeterminable`. Ownership probes below can
inspect a resolved unavailable place without performing this data observation.

An observation performs no load, copy, move, borrow, bounds check, accessor call,
interpolation, or argument side effect. In particular, `proof.is(read(), 5)` is
not a request to run or symbolically execute `read`. Calls, operator expressions
other than negative literals, dispatch blocks, and arbitrary executable blocks
are not observation terms. Bind their result normally, then query the binding.
Known forbidden effects use E219; other inadmissible terms use E223.

Revision 1 value domains are fixed-width integers, `boolean`, `null`, and closed
unions of one such non-null scalar type with `null`. Floats, strings, references,
records, lists as whole values, resources, and user-defined equality are not value
predicate domains; querying them with these APIs is E223. Type predicates and
ownership probes have their separately defined, wider domains.

| API | Result | Meaning |
| --- | --- | --- |
| `is(x, value)` | `Result` | Prove equality of the two observed scalar values, including their null alternatives. |
| `in(x, values)` | `Result` | Prove membership in a compile-time finite scalar set. |
| `notin(x, values)` | `Result` | Exactly `not(in(x, values))`, including validation and uncertainty. |
| `between(x, low, high)` | `Result` | Prove `low <= x && x <= high`, with inclusive static integer endpoints. |
| `has_type<T>(x)` | `Result` | Prove that the existing language type predicate `x <T>` holds. |
| `boundaries(x)` | `proof.Bounds<T>` | Return conservative inclusive bounds for a non-null integer observation of type `T`. |

`is` observes both operands at the same source point. Non-null scalar types must
match exactly; different integer widths/signedness use E213, other incompatible
non-null domains use E222. A literal receives the unique compatible non-null
operand context, otherwise its ordinary default type. Unrepresentable literals
use E216. There are no implicit numeric conversions.

Null is an ordinary value: null equals null; null differs from a non-null value.
It does not mean unknown. A nullable integer known to be either `5` or `null`
compared with `5` is indeterminable; one known to be null compared with `5` is never.
A known same-version scalar place equals itself, even if its value is unknown.

`values` is an immutable compile-time bounded list of compatible scalar constants;
its initialized elements form the set. Duplicates and order do not affect the
answer. They still count toward construction and inspection work. An empty set
makes `in` never and `notin` always at a reachable point. A runtime collection is
not static metadata and is rejected with E211; no runtime scan is inserted.

`between` accepts a non-null integer subject and compatible compile-time integer
endpoints. Reversed endpoints are malformed (E223), not an empty predicate.
Range and width checks precede classification. `has_type` follows existing type
predicate compatibility and nominal identity rules, including resource types;
it does not inspect representation or grant an ascription/ownership permission.

`Bounds<T>` is opaque compile-time metadata with `lower <T>`, `upper <T>`, and
`singleton <boolean>`. Bounds contain every admitted value and need not be tight.
`singleton` is exactly `lower == upper`. With no narrower knowledge, bounds are
the declared type's minimum and maximum. At an unreachable point, return those
full type bounds. Nullable/noninteger subjects use E223; narrow them first.
