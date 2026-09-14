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

## Type capabilities and ownership probes

The following resolved intrinsics each have two call forms. This is an intrinsic
signature family, not a new general function-overloading rule. Zero value arguments
require explicit `T`; one observation argument selects the place form and infers
its type. Other combinations are E212.

| Type query | Proposition; no particular runtime object is examined |
| --- | --- |
| `can_copy<T>()` | `T` satisfies the existing `memory.Copy` capability. |
| `can_move<T>()` | An initialized owned `T` admits ordinary by-value transfer to a fresh local. |
| `can_borrow<T>()` | The shared reference type `<&T>` is well-formed for runtime storage. |
| `can_exclusive_borrow<T>()` | `<&!T>` is well-formed for a fresh mutable owned location of `T`. |

All return `proof.Result`. Concrete runtime data types use their normative type
and capability rules, including opaque types' published capabilities. These are
not answers about an object's current initialization, mutability, outstanding
references, destination lifetime, thread, or allocation. In particular, a type
being copyable does not prove a particular instance is currently readable.

Revision 1 ordinary runtime data types admit by-value transfer. For a copyable
type, that transfer copies according to the language; `can_move` does not introduce
an explicit force-move operation or invalidate an otherwise copyable source.
Compile-time-only types and `<never>` produce `Never` for these runtime-operation
queries. Ill-formed types remain type errors, not `Never`.

For a generic parameter, use only facts entailed by its declared constraints.
An unconstrained runtime data parameter has indeterminable copyability but admits
ordinary generic moves and references. A `memory.Copy` constraint proves copyability.
Monomorphization, inlining, or inspecting one caller must not strengthen the answer
inside the generic declaration. Generic bodies must still be valid for every type
admitted by their constraints; proof flags cannot replace a required constraint.

| Place query | Hypothetical immediate operation |
| --- | --- |
| `can_copy(place)` | Read a copy into a fresh local, without invalidating the source. |
| `can_move(place)` | Transfer into a fresh local using ordinary copy/move rules. |
| `can_borrow(place)` | Create a shared reference used only through the end of this full statement. |
| `can_exclusive_borrow(place)` | Create an exclusive reference for that same statement interval. |

These also return `proof.Result`. The probe creates no local, loan, reservation,
read, move, cleanup, or runtime check. Its hypothetical destination has the same
scope and thread as the call and introduces no retained reference beyond the stated
interval. A longer loan or escaping result requires its own actual ownership check.

A place probe accepts a resolved place with a known type even if it is moved,
uninitialized, immutable, or blocked by a live loan. That is the point of the probe.
Unknown names, inaccessible fields, malformed selectors, or non-place expressions
are still errors. A constant integer index or stable integer index place is allowed;
unknown position/initialized-length relationships can make the answer indeterminable.
Indices are observed, not executed. The probe is not an indexed read.

Revision 1 evaluates probes with this ordered, conservative rule:

1. Return `Never` for a definite type prohibition, definitely absent initialization
   or owner lifetime, definite out-of-bounds position, forbidden extraction from
   borrowed/opaque storage, or missing write authority for an exclusive borrow.
2. Return `Never` for an incompatible loan that is definitely live and definitely
   overlaps the place throughout the proposed operation's required access.
3. Return `Always` only when the type operation is permitted, initialization/lifetime
   and bounds are definite, required authority exists, and every possibly live loan
   is compatible or has a definitely disjoint canonical place.
4. Otherwise return `Indeterminable`. A possible conflict is not a definite failure;
   absence of a definite conflict is not permission.

Use exact binding/field origins and known element indices for canonical places.
Unknown index relationships and unresolved alias overlap are possible overlap.
Disjoint fields/elements count as disjoint only where the ordinary memory contract
permits independent access; a whole-owner access overlaps its contained storage.
Copy probes inspect read permission. Moving a non-copyable owner requires consuming
permission and absence of dependent loans. Shared/exclusive probes retain the
ordinary mutability, provenance, initialization, and exclusivity requirements.

Loan liveness is the backwards may-use analysis of the frozen source control-flow
graph and declared reference-result contracts. Probe/observation arguments are not
runtime uses and do not themselves extend liveness. Definite liveness requires the
loan on every admitted path; possible liveness requires at least one. Dynamic overlap
is never decided from pointer addresses or allocator behavior. All probes obey the
same contract in debug and release.

No query provides a usable reference or a proof token accepted by an unsafe API.
No result suppresses E208, E210, ownership diagnostics, or a runtime check that lacks
its own ordinary justification. A compiler must not use a probe to legalize the
operation it is analyzing: circular permission is not a proof.

## Observation points and invalidation

All subjects of one query are observed in the same frozen pre-call state. Arguments
have no observation-time evaluation order because none executes; malformed argument
diagnostics use source order. Static metadata is validated before classification.
Each separate query has its own point, even when its arguments have identical spelling.

A result describes the subject's version at that point. Assignment creates a new
version. A possible mutation through an alias invalidates affected scalar facts;
an unknown call invalidates facts for places it may mutate under its declared
access/capture contracts. Earlier descriptors remain historical results, not facts
about the replacement value.

```meowy
proof : @"proof"

x <int32> := 5
before : proof.is(x, 5)
x = 6
after : proof.is(x, 5)
proof.expect<proof.Always>(before)
proof.expect<proof.Never>(after)
```

Asserting `before` after the assignment checks the old proposition and succeeds;
it cannot establish that the current `x` is 5. Copying `before.always` into a
runtime boolean likewise creates no persistent relationship with `x`.

Ordinary guards may establish facts for a later query. Mutations invalidate them
according to the same snapshot rules. A guard involving a proof-derived flag has
no such authority; the phase rules below prevent self-justifying analysis.

## Canonical analysis profile 1

This section fixes observable precision. An implementation may use different internal
data structures, but must return exactly the answer produced by these rules. It
must not substitute additional optimizer, SMT, interprocedural, profile-guided,
link-time, host-dependent, or speculative facts. Stronger private analysis may still
optimize code when valid; it cannot change a `proof` result.

### Abstract values

Use the following domain for each initialized scalar version:

- A non-null integer is either a nonempty exact finite set `S` of at most 16
  representable values, or one inclusive integer interval `[lower, upper]`.
- A boolean is a nonempty subset of `{false, true}`. Null contributes its own
  alternative. A supported nullable scalar pairs its non-null domain with a
  may-be-null bit; an absent non-null alternative is represented separately.
- No reachable values is bottom, used internally for unreachable control flow.
  An unknown scalar uses its full declared domain, not bottom.
- Retain same-value identity only for the same version or a direct scalar copy
  of that version. Other expressions create a fresh identity, even `x + 0`.
- Type alternatives and structural capabilities come from declared types and
  the ordinary stable type-predicate refinements. Unresolved generic facts remain
  unresolved; a runtime type tag never becomes a compile-time type argument.

Canonicalize a finite set with more than 16 members to its interval hull.
Canonicalize any interval containing at most 16 integers to its enumerated set.
Do not retain exclusions, congruences, symbolic polynomials, relations between
different integer versions, or multiple integer intervals. Null stays a separate
alternative and never occupies an integer interval endpoint.

Join finite sets by union when the union fits; otherwise use their hull. Joining
an interval with another domain uses the hull, then canonicalizes. Boolean/null
alternatives join by union. Same-value identity survives a join only when all
incoming live paths retain the same origin. Bottom contributes no values.

### Transfers and guards

Process the source control-flow graph after name/type resolution and before
optimization, using mathematical integers to calculate abstract bounds. Each
transfer is clipped to the declared integer type; arithmetic never wraps.

| Source operation | Required transfer |
| --- | --- |
| Scalar literal or an already established compile-time scalar constant | Its singleton domain. |
| Direct scalar copy | Copy domain and same-value identity. |
| Integer `+`, `-`, `*` | For finite sets, calculate all pairs; otherwise calculate the mathematical interval hull. Keep representable normal results, then canonicalize. |
| Unary integer `-`, `~` | Calculate finite images or exact interval images, intersect with the result type, then canonicalize. |
| Integer `/`, `%`, `&`, `|`, `^` | Calculate finite-set pairs exactly; otherwise use the full result-type domain. |
| Boolean operators | Apply ordinary boolean tables to the admitted values, with short-circuit control flow. |
| Scalar comparison | Apply the ordinary comparator to all admitted pairs: all true/all false gives that singleton, otherwise both booleans. Same-value identities use diagonal pairs. |
| Call result, runtime input, or unsupported fact transfer | Full declared result domain; do not inspect the callee body. |
| Assignment | Create a fresh version with the RHS domain; invalidate overlapping place facts. |

Discard arithmetic pairs that cannot complete normally; do not infer anything about
states after a panic. Statically invalid source must still report its ordinary
error before proof evaluation. If no normal results remain, the successor is bottom.
Copy/conversion intrinsics are not special transfers in revision 1 unless their
result is already a compile-time constant under the language's normal rules.

A stable integer guard against a compatible constant intersects its domain with
`==`, `<`, `<=`, `>`, or `>=`; the opposite edge uses the complementary constraint.
For `!=`, remove a finite-set member or an interval endpoint; discard an interior
interval exclusion. Apply normalization after each intersection. A type predicate
intersects/removes declared alternatives. A stable boolean guard selects its value.
All other guards leave scalar domains unchanged unless already constant.

`&&` passes true-edge facts to its right operand; `||` passes false-edge facts.
Matcher arms follow the language's independent-arm and continuation rules, not an
invented `else-if` chain. Joins use the rule above. Emission is not return. Calls,
writes, and alias mutation invalidate guards before later queries use them.

At each restart/loop header, forget numeric, boolean, same-value, and refined-tag
facts for every place that the loop may modify, including possible alias writes;
use declared domains for them on every entry. Retain facts for unaffected places.
Analyze each iteration body's acyclic paths from that header state, without bounded
unrolling or guessing an iteration count. Exits join normal exit-edge states.
Nested loops apply this rule from the outside in. Ownership initialization and
loan probes additionally use their must/may state rules above.

Functions start from declared parameter domains and constraints. Do not specialize
facts using runtime call arguments. Imported runtime data supplies its declared
type domain; imported compile-time constants/capabilities retain published metadata.
No module initializer is executed to improve a result. External, concurrent, raw,
or otherwise untracked mutation forgets affected facts; it cannot create a stronger
answer. Unsupported alias relationships remain possible overlap for place probes.

### Predicate classification

First validate the query, then check reachability, then apply these rules. For value
queries, bottom yields `Indeterminable`. For reachable admitted domains:

| Query | `Always` | `Never` | Otherwise |
| --- | --- | --- | --- |
| `is(x, y)` | Same-value identity, or every admitted pair is equal. | No admitted pair is equal. | `Indeterminable` |
| `in(x, S)` | Every admitted subject value belongs to `S`. | No admitted value belongs to `S`. | `Indeterminable` |
| `between(x, l, h)` | The integer domain is contained in `[l, h]`. | It is disjoint from `[l, h]`. | `Indeterminable` |
| `has_type<T>(x)` | All admitted alternatives satisfy the predicate. | None can satisfy it. | `Indeterminable` |

Different identities use the Cartesian product of their domains; do not recover
relations lost at joins. Finite-set membership is exact against the static metadata
set, including coverage of every integer in an interval. This does not add interval
holes to the subject domain. `notin` and result combinators use their fixed tables.
`boundaries` takes the minimum and maximum of the integer domain, or full type
bounds at bottom. Type capabilities and place probes use their preceding rules.

## Phase ordering and circularity

An observation result must not change the analysis that produced it. Implement
these semantic phases regardless of the compiler's internal pass organization:

1. Resolve and check ordinary program structure, types, generic constraints,
   ownership, and source effects without assuming any proof answer. Proof exports
   have fixed signatures. Validate observation syntax and static argument kinds.
2. Freeze the canonical source graph and base facts. Compute queries only from this
   graph and profile 1. Proof-derived runtime conditions provide no refinements;
   retain both successors when constructing base facts.
3. Form descriptors, perform metadata composition/inspection, and discharge proof
   assertions/expectations. Materialize any requested ordinary scalar projections.
4. Optimize and generate runtime code. No optimization result feeds back into phase 2.

A query whose subject, static metadata, type argument, or availability depends on
an earlier proof answer is E225, even when a compiler could guess a fixed point.
Dependency includes arithmetic, copies, calls, and control dependence. A query in
a runtime branch controlled by `.always` is therefore rejected. Descriptor-only
`inspect`, `not`, `all`, `any`, `assert`, and `expect` may consume prior descriptors;
they do not create another observation or change the frozen base graph.

Proof-derived means transitively dependent on an observation answer, including flags
and bounds. The fixed `proof.revision` constant and exported type identities are not
proof-derived.

Proof-derived scalars may control ordinary runtime behavior when both branches are
independently well-typed and ownership-valid. They must not determine type formation,
array/list capacities, generic arguments, overload resolution, imports, manifests,
capability constraints, or ownership acceptance. Those backward dependencies are
E225. A type query on the fixed `proof.Result` signature remains permitted.

A query cannot introduce an unsafe assumption, establish its own precondition, or
hide invalid ordinary code in an answer-dependent branch. Ill-formed programs retain
their original diagnostics rather than returning `Indeterminable`. Host failure,
missing compiler support, and exhausted evaluation budgets are not proof outcomes.

Changing any rule, precision limit, transfer, or outcome table requires a contract
revision. Debug/release, caching, worker scheduling, compiler memory availability,
source paths, and optional diagnostics cannot change the answer. Target-dependent
integer ranges use the declared target, not the machine running the compiler.
