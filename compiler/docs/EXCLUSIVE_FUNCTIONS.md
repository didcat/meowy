# Scalar exclusive function arguments

This extends [scalar exclusive references](EXCLUSIVE_REFERENCES.md) using the existing
[memory contract](../../docs/reference/memory.md) and
[reference conversion rules](../../docs/reference/types.md#inference-and-assignment).
The bounded argument contract is implemented and covered by native execution and
exact-code rejection tests. [Bare scalar-reference results](REFERENCE_RETURNS.md)
now also retain explicit guarded argument-to-result authority.

## Bounded contract

Support direct functions whose exclusive parameters are scalar references and whose
remaining parameters are primitive values or shared/exclusive scalar references.
Primitive values here are null, never, booleans, integers and floats. Results may
be primitive values or one shared/exclusive scalar reference. Shared-only signatures retain their wider existing support;
exclusive-derived shared arguments may cross only this bounded flat scalar
contract. No new syntax, ABI or LLVM alias promise is added.

An exclusive argument moves its holder. Explicit `&!*p` delegates a child for the
call and leaves `p` available afterward. An expected shared parameter reborrows `p`
without moving it. Reborrowing may shorten the usable period; it does not extend the
owner lifetime. Carrier results, reference cells, indexed/non-scalar pointees,
exclusive dispatch blocks and restart bodies remain separate capabilities.
Receiver syntax for direct functions uses the same argument contract.

## Caller obligations

- Evaluate and capture every argument once, left to right. Preserve earlier moves
  and side effects when a later argument leaves or panics. Such paths never enter
  the call or retain a fictitious final argument use.
- After every argument returns, record each scalar-reference parameter's maximum
  access: Read for shared, Write for exclusive. Keep all captured argument values
  demanded through every entry access and the call itself, even for an unused
  parameter. This catches conflicting arguments and suspended parents passed by move.
- Validate entry accesses against guarded actual sources, existing loan ancestry,
  other arguments and future live references. Public bounds remain dependencies;
  they never grant permission. Existing acquisitions alone do not prove call entry.
- Forget mutable source refinements after a potentially mutating call. A returned
  primitive has no reference authority; ordinary caller loans can end at call return.
- Non-returning callees still require valid, non-conflicting call entry. Recursive
  direct calls use the same checked signature and argument rules at every call site.

## Callee obligations

A scalar exclusive parameter begins as initialized non-Copy storage with an exclusive
input loan. Its physical referent is `Source::Input { id, component, fields }`, distinct
from the parameter's local pointer cell. Reborrows preserve the symbolic input root;
parent/child conflicts on that root must be checked exactly as for local scalar roots.

Different scalar input roots may be treated as disjoint for an exclusive access only
because every direct caller proves the exclusive argument does not overlap another
borrow-carrying argument. Shared arguments may alias each other; this is not a general
noalias assumption about function parameters. Nested forwarding must repeat entry
validation using symbolic roots and preserve the distinction from local storage.

The body scan includes exclusive parameters even when they are unused, so a resolved
restart cannot bypass the first-slice gate. Captures and indirect calls remain gated.
Bare scalar-reference results use explicit guarded argument transfer and conservative
bounds. Returning a carrier or crossing a wider exclusive signature remains B001.

## Acceptance matrix

| Expected behavior                                | Source                                                                                |
| ------------------------------------------------ | ------------------------------------------------------------------------------------- |
| Accept scalar mutation                           | `set <null> : (p <&!int32>) { *p = 2 }; x := 1; set(&!x); v : x`                      |
| E301 after a moved argument                      | `set <null> : (p <&!int32>) { *p = 2 }; x := 1; p : &!x; set(p); v : *p`              |
| Accept explicit child argument                   | `set <null> : (p <&!int32>) { *p = 2 }; x := 1; p : &!x; set(&!*p); v : *p`           |
| Accept implicit shared reborrow                  | `read <int32> : (p <&int32>) { -> *p }; x := 1; p : &!x; v : read(p); *p = 2`         |
| E302 at a suspended-parent call                  | `set <null> : (p <&!int32>) { *p = 2 }; x := 1; p : &!x; s : &*p; set(p); v : *s`     |
| E302 inside a symbolic input body                | `f <null> : (p <&!int32>) { s : &*p; *p = 2; v : *s }`                                |
| E302 between call arguments                      | `f <null> : (p <&!int32>, s <&int32>) { *p = *s }; x := 1; p : &!x; s : &*p; f(p, s)` |
| Accept shared reference return                   | `f <&int32> : (p <&!int32>) { -> &*p }`                                               |
| B001 on unused exclusive input in a restart body | `f <null> : (p <&!int32>) { 'again { 'again.restart() } }`                            |

Also verify nested forwarding, recursive direct calls, aliases of function bindings,
unknown branch conditions, definite/uncertain moves, shared child copies, public
bounds, scalar widths/boolean layout, discarded calls, argument-side owner mutation,
Leave/panic and independent shared-only functions with restarts.

## Implementation and validation

`borrow_contract.rs` classifies the narrow signature; bare scalar-reference
results use `borrow_contract/returns.rs`. Wider shared substitution is unchanged. `borrow/mutable.rs` includes input modes in its scan; `loans/solve.rs`
grants them. `loans/values.rs` builds caller entry accesses on the existing CFG,
and `loans/permissions.rs` compares symbolic-root overlap.

Seventeen groups in `tests/native/exclusive_functions.rs` execute accepted programs
in debug/release and check primary rejection codes in both build profiles. They
include direct receiver syntax, mutation invalidating caller facts, shared-only
restart callees and no-return entry/argument effects. The existing scalar-local
matrix and reference fixtures remain enabled. The compiler gate passes 626 Rust
tests and 20 Python tests; full conformance still has 13 unsupported cases.

The [exclusive functions example](../examples/exclusive-functions.mwy) demonstrates
shared argument reborrowing, nested mutation, a consumed holder and receiver syntax.
Generated cleanup and wider result shapes remain distinct work.
