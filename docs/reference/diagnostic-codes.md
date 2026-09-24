# Diagnostic code catalog

[Documentation index](../README.md) · [Diagnostic presentation](diagnostics.md)

A code names a rule or failure class. An occurrence adds the source, values,
constraints, and execution evidence that explain this particular failure.
`meowy err explain E303` describes a rule without a saved session;
`meowy err explain 1` explains occurrence 1 using its captured source.

This catalog assigns the codes below. Unlisted numbers remain unassigned. Codes
are stable across changes to wording, source syntax, or compiler phase names;
a retired code is never reused for a different rule. A capsule records the
catalog version and explanation that accompanied its original toolchain.

## Families and presentation

| Family | Area                                     | Normal presentation |
| ------ | ---------------------------------------- | ------------------- |
| `E0xx` | Source text and punctuation              | `error[E001]`       |
| `E1xx` | Collection and numeric constraints       | `error[E103]`       |
| `E2xx` | Names, types, and value construction     | `error[E207]`       |
| `E3xx` | Ownership, borrows, and storage          | `error[E303]`       |
| `E4xx` | Tasks and channels                       | `error[E403]`       |
| `E5xx` | Projects, imports, and build inputs      | `error[E503]`       |
| `E6xx` | Native boundaries and linking            | `error[E601]`       |
| `E7xx` | Diagnostic sessions, repairs, and replay | `error[E702]`       |
| `E8xx` | Documentation and documentation tools    | `error[E801]`       |
| `P0xx` | Runtime panics                           | `panic[P003]`       |
| `T0xx` | Test-runner expectations and supervision | `failure[T002]`     |
| `F0xx` | Compiler faults                          | `fatal[F001]`       |
| `G...` | Gatostyle policy findings                | `warning[G101]`     |

Gatostyle assigns `G001` to layout drift, `G002` to unresolved custom-rule facts,
and the built-in rule codes in its [policy catalog](../guide/gatostyle.md#choose-how-the-code-is-written).
Custom rule IDs use `G:project:NAME`. Their severity is configurable; explain them
with `meowy style explain`, and inspect their patches with `meowy style fix --diff`.
They do not become occurrences in the last check/build/run session or weaken any
compiler rule. Invalid gatostyle configuration uses the existing `E505` code.

The [language server](lsp.md) uses these same codes. Invalid `lsp` fields or an
unsupported configuration schema use `E505`; a pinned toolchain mismatch or
unavailable target input uses `E507`. Indexing limits, disconnected clients, and
truncated diagnostic presentation are server status, not new language rules.
JSON-RPC/LSP error numbers describe protocol requests and are separate from this
catalog and from saved occurrence IDs.

The prefix is part of the code. An exit status, OS error number, occurrence ID,
or library error type is not interchangeable with a diagnostic code. Severity,
phase, and capture status are separate fields in the saved diagnostic.

Report the most specific established cause. For example, use `E403` for a borrowed
channel message rather than also emitting a generic transfer error. Attach the
underlying type or lifetime constraint as a note. Suppress downstream errors that
exist only because an earlier expression has no valid type. Independent failures
still get separate occurrences, even when they share a code.

Each entry below gives the trigger, the minimum useful evidence, and a repair
direction. A direction is not automatically an applicable patch or a recommended
fix. The [repair contract](diagnostics.md#repair-contracts) governs actual edits.

## Source text and punctuation

See [syntax](syntax.md). Identifier spellings are never rejected as reserved
keywords; an unknown name is a lookup problem under `E201` or `E202`.

| Code   | Diagnostic and trigger                                                          | Evidence and repair direction                                                                             |
| ------ | ------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| `E001` | Invalid source token: bytes or punctuation cannot form a token in this position | Show the offending span and expected token forms; identify invalid UTF-8 by byte offset                   |
| `E002` | Unclosed or mismatched delimiter                                                | Show the opener and mismatched closer or end of file; suggest a closer only when pairing is unambiguous   |
| `E003` | Unsupported string escape                                                       | Mark the full escape and list supported forms; escape a literal backslash explicitly if that was intended |
| `E004` | Incomplete expression or declaration                                            | Show the punctuation that requires an operand, body, or binding value; do not insert a guessed expression |

Unclosed strings, ordinary `# ... #` comments and documentation fences use `E002`,
including when they span multiple lines. For documentation, show the opener and
required bar count/closer family; a shorter or longer run is not a mismatched closer
but payload text. A quote inside interpolation is parsed as part of that
expression; nested strings are not reported as an unclosed outer string merely
because they contain another quote.

## Documentation

| Code   | Diagnostic and trigger                                              | Evidence and repair direction                                                |
| ------ | ------------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| `E801` | Orphaned, duplicate or misplaced documentation                      | Show the doc opener and expected declaration/module position                 |
| `E802` | Malformed, unresolved or inaccessible documentation link            | Show the link span and resolve against checked bindings, not prose spelling  |
| `E803` | Invalid example metadata or missing required public documentation   | Identify the fence attribute or undocumented declaration                     |
| `E804` | Documentation example check/output/exit/time/output-budget mismatch | Show the containing fence and example-local diagnostic or execution evidence |
| `E805` | Unsafe or failed documentation publication                          | Identify the destination and preserve unrelated or previous valid output     |

Unsupported language features and unavailable infrastructure retain B/F diagnostics;
they never satisfy an expected E-code rejection. These codes do not imply LSP,
package documentation or release-compatible index support.

## Collection and numeric constraints

See [collections](collections.md) and [numeric behavior](types.md#numeric-behavior).
These codes apply when the compiler proves the violation. Runtime checks still
apply when values are dynamic. A checked API that returns an error, such as
`get`, `try_add`, or `numbers.checked_add`, keeps that result even when the failure
is statically known. These codes reject operations that would otherwise trap;
they do not turn an expected error value into a rejected program.

| Code   | Diagnostic and trigger                       | Evidence and repair direction                                                                                                                              |
| ------ | -------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `E101` | Index out of bounds at compile time          | Show the requested one-based position and proven length; use a valid position or checked `get`                                                             |
| `E102` | Duplicate named-list alias                   | Mark both literal names; rename or remove the duplicate alias                                                                                              |
| `E103` | Bounded-list capacity exceeded               | Show the capacity declaration, proven length, and append requiring another slot; increase capacity or redesign storage explicitly                          |
| `E104` | Invalid collection extent                    | Show a negative or nonconstant `N` where a non-negative compile-time capacity or array length is required                                                  |
| `E105` | Fixed-array element count mismatch           | Show the required `N` and actual initializer count; provide exactly that many elements                                                                     |
| `E106` | Invalid named-list lookup                    | Show a nonliteral key or an undeclared alias; use a declared literal alias or a map for runtime keys                                                       |
| `E107` | Checked arithmetic violation at compile time | Show operands, operation, and representable range, including division by zero; choose valid operands or an explicit checked/wrapping API where appropriate |
| `E108` | Invalid shift count at compile time          | Show the count and operand width; use a representable count with the bit-operation API                                                                     |

An alias declared by the type but removed at runtime is a bounds case, not an
unknown-name case. Assignment to `size() + 1` never appends. `<T[]>` denotes a
borrowed slice, so an `E103` repair cannot turn an inline list into a growable
owner simply by deleting `N`.

## Names, types, and value construction

See [types](types.md) and [values and blocks](values-and-blocks.md).

| Code   | Diagnostic and trigger                                  | Evidence and repair direction                                                                                                                                                                 |
| ------ | ------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `E201` | Unknown value name or member                            | Show the lookup and its scope or receiver type; identify a missing import or binding without treating the spelling as syntax                                                                  |
| `E202` | Unknown type name                                       | Show the type lookup and available namespace; import, qualify, or declare the intended type                                                                                                   |
| `E203` | Binding already declared in this scope                  | Mark both declarations in the same namespace; choose another name or an intentional inner scope                                                                                               |
| `E204` | Required result component is uninitialized              | Show the required primary or field and a completing path that does not initialize it or provide a permitted default                                                                           |
| `E205` | Result component may be emitted twice                   | Mark both emissions and the path connecting them; emit once or use control flow that excludes the second write                                                                                |
| `E206` | Conflicting expanded fields                             | Show each origin and incompatible field declarations; give the composition one unambiguous shape                                                                                              |
| `E207` | Incompatible initializer or assignment                  | Show the required binding type and actual value type; choose a compatible value or explicitly change the contract                                                                             |
| `E208` | Type ascription lacks a proof                           | Show the current flow type and requested type; establish narrowing before using `value~<T>`                                                                                                    |
| `E209` | Type subtraction cannot represent the result            | Show the source set and removed alternative; use a predicate when subtracting a literal from an unrestricted primitive                                                                        |
| `E210` | Generic capability requirement is not satisfied         | Show the instantiated type, constrained binder, and operation needing `Copy`, `Send`, or another declared capability                                                                          |
| `E211` | Type depends on a runtime value                         | Show the runtime dependency reaching a type expression; supply a compile-time argument or keep the choice in ordinary value storage                                                           |
| `E212` | Call or generic arguments do not match the signature    | Show required arity/types or unresolved/conflicting type binders beside the supplied arguments; nullable parameters still require arguments                                                   |
| `E213` | Numeric operands require an explicit conversion         | Show both already-typed operand types; convert deliberately rather than silently changing width or signedness                                                                                 |
| `E214` | Public function signature is incomplete                 | Mark missing parameter or result annotations at the exported boundary; state the public contract                                                                                              |
| `E215` | Matcher condition is not boolean                        | Show the condition type; use an explicit comparison or type predicate rather than implicit truthiness                                                                                         |
| `E216` | Literal is not representable in its expected type       | Show the literal and target range; choose the intended width or a representable value                                                                                                         |
| `E217` | Invalid error definition or static constructor metadata | Mark the offending error descriptor field, code or static-message constructor argument and show the required metadata contract                                                                |
| `E218` | Invalid testing descriptor or callback                  | Mark invalid Suite/Case shape, callback result/captures, panic expectation, skip reason, or runtime-dependent metadata                                                                        |
| `E219` | Forbidden compile-time effect                           | Show the required evaluation root and transitive call reaching I/O, runtime resources, mutation of external storage, or another forbidden effect.                                             |
| `E220` | Compile-time evaluation budget exceeded                 | Show the root, exhausted logical counter, limit and active helper stack; bounded evaluation does not depend on wall-clock time.                                                               |
| `E221` | Invalid forward function group                          | Show the reserved signature and missing, mismatched, capturing or generic definition, or the statement interrupting the group.                                                                |
| `E222` | Operator is not defined for these operands              | Show the operation and complete operand types; use a supported comparison or explicit library operation.                                                                                      |
| `E223` | Invalid proof observation or descriptor                 | Show the resolved query, unsupported subject/domain, unavailable place, malformed static metadata, or illegal descriptor use; follow the [proof contract](stdlib/proof.md).                   |
| `E224` | Compile-time proof expectation failed                   | Show the assertion and original observation, required outcome and actual Always/Never/Indeterminable result; strengthen ordinary evidence or correct the expectation.                         |
| `E225` | Proof query has a forbidden backward dependency         | Show the dependency from a proof answer into another observation, its availability, type formation, specialization, or ownership acceptance; remove the cycle or use ordinary declared facts. |

`E207` covers initialization as well as later assignment. Changing `"twenty"` to
`20` is not a type conversion defined by the language. `E208` concerns a value
ascription; assigning a still-nullable value into a narrower binding is `E207`.

`E212` includes generic function and type specialization: a wrong argument count
or kind, conflicting evidence for one binder, or a parameter that cannot be
inferred. Identify the binder and contributing arguments without guessing a
union or conversion. Duplicate binders remain `E203`, capability failures `E210`,
and missing exported result annotations `E214`. See
[generic inference](types.md#inference-and-specialization).
Flow evidence invalidated by a write or mutating call must be shown at that
operation, so a programmer can see why an earlier type test no longer suffices.

`E217` covers malformed static [error definitions](stdlib/errors.md#define-and-construct-an-error)
and static metadata for `cli.invalid`/`io.error`. Runtime-dependent type
construction or a runtime value supplied as static metadata remains `E211`. A custom application's
error code is metadata on a returned value, not an entry in this compiler catalog.

## Ownership, borrows, and storage

See [memory](memory.md). Diagnostics should name the owner and the operation
that changed access rights, not just say that a value is unavailable.

| Code   | Diagnostic and trigger                              | Evidence and repair direction                                                                                                              |
| ------ | --------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| `E301` | Use after move or explicit release                  | Mark the original owner, consuming operation, and later use; borrow before transfer or obtain a new owner                                  |
| `E302` | Conflicting accesses to borrowed storage            | Show the active shared/exclusive loan, the conflicting access, and its last required use; shorten the overlap or use disjoint fields       |
| `E303` | Borrow outlives its storage                         | Show the storage lifetime, borrowed view, and escaping return/capture; return an owner or borrow caller-owned storage                      |
| `E304` | Moving a non-copyable value through a borrow        | Show the borrowed place and attempted move; keep a borrow or use an explicit consuming/take operation on an owner                          |
| `E305` | Mutation requires a mutable, exclusive location     | Show the selected immutable slot or shared access path; request replacement permission on that slot                                        |
| `E306` | Storage operation would require implicit allocation | Show the runtime string construction, escaping storage, or other hidden allocation boundary; construct an explicit owner with an allocator |
| `E307` | Allocator lifetime is too short                     | Show the allocator and allocations that can outlive it; keep the allocator alive until its owners are released                             |
| `E308` | Owned payload cannot be erased without boxing       | Show the concrete payload and requested erased representation; retain the concrete union or explicitly box it                              |
| `E309` | Storage may be uninitialized                        | Show the read and a path lacking initialization, including a partially moved aggregate; initialize the needed component before access      |

`E303` includes returning a slice of a local list and using a capture after its
referent's lifetime ends. It does not ban all task borrows: a valid borrow can
survive until the owning join. `E309` concerns ordinary storage; missing block
emissions use `E204`. No repair may extend a lifetime by silently moving storage
to the heap or copying a non-copyable owner.

Deferred actions participate in the same ownership diagnostics. `E301` includes
moving an owner into an emission while a registered action still needs the local,
or consuming it in an action that runs earlier in cleanup order. Show the
registration, invalidating move/release and delayed use. `E302` and `E303` retain
their ordinary access/lifetime meanings at delayed execution; registration alone
does not establish an implicit borrow. `E309` includes a required local that is
uninitialized on a cleanup path. `E303` also covers a labeled action whose delayed
access requires an inner binding after its scope ends, including copyable values
that were not explicitly saved in longer-lived storage. Show the registration,
target scope and shorter-lived binding. See
[deferred actions](values-and-blocks.md#deferred-actions).

## Tasks and channels

See [tasks and channels](tasks-and-channels.md). These are contract violations;
`Cancelled`, `Timeout`, `SpawnFailed`, `Full`, and `Closed` are ordinary typed
outcomes when their APIs promise them.

| Code   | Diagnostic and trigger                          | Evidence and repair direction                                                                                          |
| ------ | ----------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| `E401` | Ownership cannot transfer between tasks         | Show the capture, result, or message type that lacks `tasks.Send`, including the nontransferable field                 |
| `E402` | Shared task borrow requires `tasks.Sync`        | Show the shared referent and failing capability; transfer an owner or use an explicitly synchronized representation    |
| `E403` | Channel message contains a non-static borrow    | Show the borrowed payload and its owner; send owned data or a valid static view                                        |
| `E404` | No executor is established for task start       | Show the task operation and entry/host configuration; supply an explicit executor                                      |
| `E405` | Invalid join operand                            | Show the operand type; join a task handle or group, not an ordinary value or a borrowed ticket                         |
| `E406` | Task handle, group, or ticket escapes its owner | Mark the owning scope and escaping return/store; collect outcomes before that scope ends                               |
| `E407` | Task or group has already been joined           | Show the consuming join and later join, submission, or ticket access; use the collected result instead                 |
| `E408` | Task-group capacity exceeded at compile time    | Show the group capacity and submission count, including slots holding failed admissions; bound work or process batches |

An exclusive borrow can move into one child if its referent is transferable and
conflicting parent access is excluded until the borrow ends. Diagnose a lifetime
violation with `E303`, and an overlapping parent access with `E302`. A full channel
is backpressure or a `try_send` result, not `E408`. A wait cycle is evidence for
investigation, not proof of a static source error in every scheduling context.

## Projects, imports, and build inputs

See [modules and configuration](modules-and-ffi.md) and
[CLI dependency commands](../cli/README.md#profiles-targets-and-dependencies).

| Code   | Diagnostic and trigger                                        | Evidence and repair direction                                                                                                           |
| ------ | ------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| `E501` | Module cannot be resolved                                     | Show the literal import, importing file, and resolution base; correct the path or declared alias                                        |
| `E502` | Import cycle                                                  | Show the ordered import/re-export edges closing the cycle; separate the shared dependency                                               |
| `E503` | Required lock entry is missing or conflicts with the manifest | Show the alias, selector, and lock entry; resolve or explicitly update dependencies                                                     |
| `E504` | Dependency content digest mismatch                            | Show the expected and observed digest and canonical source identity; recover verified content instead of silently relocking             |
| `E505` | Invalid manifest configuration                                | Mark the unknown field, incompatible selectors, invalid size, or forbidden compile-time effect and its configuration path               |
| `E506` | Entry module contract is invalid                              | Show an entry imported as a module or an unsupported primary result; keep the entry distinct and handle top-level errors explicitly     |
| `E507` | Requested target or toolchain input is unavailable            | Show the requested target/input and available host context; provide the declared input or select a compatible target                    |
| `E508` | Import alias conflicts with a foundational module             | Mark the alias and foundational identity; rename the package or local path alias                                                        |
| `E509` | No test cases selected when an empty run is forbidden         | Show configured roots, explicit path selectors, name filters, and discovery count; select real cases or deliberately allow an empty run |

A lock mismatch is not permission to move a branch or tag. Diagnostics retain
the source identity, revision, and digest involved in the failure. A missing
dependency can make a capture incomplete; the tool must not claim a closed
capsule while required content is absent.

For local path imports, `E501` includes the prefix, mapped directory, resolved
suffix, and both declaration and import sites. An invalid `import.aliases` value
or a name shared by a package and a path alias uses `E505`. `E508` applies to both
package aliases and local path aliases that conflict with foundational names.

[Optimization settings](optimization.md#select-build-policy-in-modmwy) use `E505`
for invalid values or incoherent combinations, and `E507` for unavailable target,
CPU, runtime, or LTO support. Include the manifest field, effective profile,
requested capability, and relevant toolchain identity. Optimization does not
change source-error severities or justify suppressing a runtime check without proof.

## Native boundaries and linking

See [native interfaces](modules-and-ffi.md#native-interfaces) and
[raw pointers](memory.md#raw-pointers-and-unsafe-operations).

| Code   | Diagnostic and trigger                               | Evidence and repair direction                                                                                                       |
| ------ | ---------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| `E601` | Operation requires a caller-proven safety boundary   | Show the raw-memory or unchecked-function operation and its preconditions; justify it inside `!{ ... }` or expose a safe wrapper    |
| `E602` | Foreign signature contains a non-ABI-compatible type | Mark the offending parameter/result and convention; use explicit native representations                                             |
| `E603` | Native record definition is invalid                  | Show nonconstant fields, duplicate names, or disallowed field types; pass a valid ordered field list to `ffi.record`                |
| `E604` | Unchecked function requirement would be erased       | Show the `<!(...) -> R>` source and ordinary function destination; preserve the requirement or validate inside a safe wrapper       |
| `E605` | Declared native symbol cannot be resolved            | Show the symbol, convention, and exact selected link inputs; supply the correct artifact without guessing from ambient search paths |
| `E606` | Native artifact is incompatible with the target      | Show the artifact's architecture/ABI and the selected target; rebuild or select compatible declared content                         |

`E601` does not say that adding punctuation makes an arbitrary address valid.
The unsafe operation's lifetime, alignment, bounds, initialization, and access
preconditions still have to hold. A native fault at runtime is recorded as an
observed process failure when possible; it is not retroactively certified as a
safe meowy panic or assigned a guessed cause.

## Diagnostic tools and replay

These failures belong to the requested tool action. They do not replace the
saved source occurrence, allocate a misleading new occurrence in the old session,
or recursively produce more failure capsules.

| Code   | Diagnostic and trigger                               | Evidence and repair direction                                                                                                          |
| ------ | ---------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| `E701` | Session, occurrence, or candidate cannot be selected | Show the selector and project/entry; choose an existing unambiguous ID                                                                 |
| `E702` | Repair source is stale                               | Show the affected path and saved/current digest; run a fresh check before applying new candidates                                      |
| `E703` | Repair candidates conflict                           | Show both IDs and overlapping ranges or mutually exclusive alternatives; select one coherent set                                       |
| `E704` | Replay input or host requirement is unavailable      | Name the missing captured input or incompatible host requirement; inspect the capsule or replay on a compatible host                   |
| `E705` | Capsule integrity check failed                       | Show the affected payload and digest mismatch; recover an intact artifact before executing its payload                                 |
| `E706` | Replay diverged                                      | Show saved and observed failure identities, or the first recorded event that no longer matches                                         |
| `E707` | Failure capture is incomplete                        | Show the missing files, exhausted capture budget, or cache write failure; preserve the original diagnostic and mark replay limitations |
| `E708` | Combined repair validation failed                    | Show the selected candidates and remaining target/new errors; keep source unchanged and revise the selection                           |

`E707` can accompany an original error as a capture-status note. It does not
replace that error or its command exit status. Tool errors without source spans
point to their session, capsule member, or edit range. Authentication and command
usage failures are CLI results, not invented language-rule violations.

## Test-runner failures

See [testing](stdlib/testing.md) and [test sessions](diagnostics.md#test-cases-and-assertions).
These codes describe the runner's expected outcome or supervision, not a new
application error union. Unexpected recoverable panics retain their original
`P...` code; a failed equality assertion is still `P005`.

| Code   | Failure                                           | Required evidence                                                                                                                                            |
| ------ | ------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `T001` | Expected panic was not observed                   | Case ID, expected code/message, and normal callback completion; a handled error or child panic outcome does not satisfy the expectation.                     |
| `T002` | Case watchdog expired                             | Configured budget, elapsed observation, last known phase, termination/reaping result, and capture limitations. Do not infer a deadlock or completed cleanup. |
| `T003` | Case process ended without a valid harness result | Case ID, native status/signal, last known phase, and available streams; exit zero alone is not a pass.                                                       |
| `T004` | Case output limit exceeded                        | Combined byte limit, per-stream observed counts, retained prefix, truncation state, and termination outcome.                                                 |

If an observed assertion is followed by blocked cleanup and watchdog expiry,
the terminal case result is `T002` with the assertion retained as related evidence.
Count one failed case. A mismatched expected panic instead keeps its actual
panic code with an expectation note. `P008`, module initialization/teardown
failures, and supervisor failures never become a matching expected panic.

Invalid descriptors use `E218`; invalid test manifest fields use `E505`.
Unavailable selected target/runtime inputs use `E507`, and disallowed empty
selection uses `E509`. Selection failures may have no executable/case artifact;
their diagnostics must describe what was selected and which phases ran.

## Runtime panics and compiler faults

See [panic behavior](diagnostics.md#panics). A known invalid operation can be
rejected statically; a runtime-dependent violation uses its corresponding panic
record. The same bounds and arithmetic rules apply in debug and release builds.

| Code   | Failure                                   | Evidence                                                                                     |
| ------ | ----------------------------------------- | -------------------------------------------------------------------------------------------- |
| `P001` | Dynamic collection index is out of bounds | Requested position, actual initialized length, and access site                               |
| `P002` | Checked arithmetic failed                 | Operation, operands when available, and numeric range                                        |
| `P003` | Bounded list is full                      | Capacity, initialized length, and failing append                                             |
| `P004` | Task group is full                        | Group identity, capacity, and submitted slots                                                |
| `P005` | Assertion failed                          | Assertion site, condition, and explicitly supplied message                                   |
| `P006` | Explicit `debug.panic`                    | Call site and supplied message                                                               |
| `P007` | Invalid dynamic shift count               | Operand width, count, and bit-operation site                                                 |
| `P008` | Panic during cleanup                      | Original unwind cause, cleanup operation, and second panic; fatal outcome                    |
| `F001` | Compiler invariant failed                 | Failing compiler stage/pass, internal assertion, toolchain identity, and preserved inputs    |
| `F002` | Compiler process crashed unexpectedly     | Stage if known, signal/process outcome, available stack information, and replay completeness |

`E101` and `P001`, `E103` and `P003`, and `E408` and `P004` describe static and
dynamic violations of the same underlying bounds rules. `E107` maps to `P002` and
`E108` to `P007` when the operands are known only at runtime.

The [`testing` assertions](stdlib/testing.md#assertions-borrow-their-evidence)
use `P005`, including exact actual/expected comparison evidence where available.
A deliberately false assertion stays an executable panic path even when constant;
it does not make an otherwise invalid bounds/arithmetic operation executable.

A child panic keeps its panic code inside `tasks.Panicked` after cleanup. Merely
receiving that outcome does not print an error or terminate the parent. An
uncaught root panic creates a runtime diagnostic occurrence and a replay capsule
when capture succeeds. Allocation failures, rejected sends, drained channels,
and task deadlines keep their documented result types rather than becoming panic
codes just to obtain a diagnostic number.

Compiler faults are reportable tool failures, not a statement that the source
violates a language rule. Preserve the compiler and failing inputs, disable
speculative repairs, and offer the [report workflow](../cli/README.md#report-a-compiler-bug).
Externally terminating the compiler is not automatically a compiler bug; a known
user interrupt or resource limit is reported with that observed cause.

## Worked ownership diagnostic

This **invalid** function returns a view of storage that is released when the
function completes:

```meowy
bytes <uint8[]> : () {
    local <uint8[3]> : [1, 2, 3]
    -> local.slice()
}
```

```text
$ meowy check main.mwy

meowy v0.0.1
checking main.mwy

1 - error[E303]: borrow outlives its storage
  --> main.mwy:3:8
   |
 1 | bytes <uint8[]> : () {
   |       --------- result is a borrowed slice
 2 |     local <uint8[3]> : [1, 2, 3]
   |     ----- storage belongs to this call
 3 |     -> local.slice()
   |        ^^^^^^^^^^^^^ returned view borrows local
 4 | }
   | - local is released here
   |
   = the result would outlive its owner
   = return an owning value, or write into caller-owned storage
   . no automatic repair: the function's result contract must change

1 error - 0 fixes - 1 replay capsule

inspect the ownership proof:
   `meowy err reproduce 1 --verbose --trace ownership --trace layout`
```

The absence of a patch is useful information. The compiler can show the lifetime
conflict precisely without deciding whether the API should return an inline
owner, use allocated storage, or accept a caller-provided destination.
