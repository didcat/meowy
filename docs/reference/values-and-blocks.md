# Values, blocks, and control flow

[Documentation index](../README.md)

## Evaluation

A block evaluates its statements in source order. `{ ... }` evaluates immediately;
`(parameters) { ... }` creates a function whose body evaluates on a call. A file
is a module body, evaluated once during module initialization.

```meowy
value : { -> 10 }
make_value : () { -> 10 }

value         # an integer #
make_value    # a function value #
make_value()  # an integer #
```

Selecting a field is a data operation. It never invokes a function, follows a
getter, or reruns the block that created the field. A function-valued field must
be called explicitly.

## The shape of a block value

A completed block contains a primary value and statically named fields. A missing
primary is `null`. A block with neither primary nor fields is just `null`.

```meowy
reading : {
    -> 24
    -> unit : "celsius"
    -> sensor : 7
}
```

The full type includes both the primary and the fields:

```meowy
<Reading> : <{
    -> <int32>
    unit <string>
    sensor <int32>
}>
```

For an ordinary record, omit the primary member of its type. Such a record has a
`null` primary but is not the `<null>` type. `record <null>` does not match merely
because the record has no primary emission.

A block value is an aggregate, with no mandatory heap allocation. See
[memory](memory.md) for moves, references, and layout.

## Emission is construction

`-> expression` initializes the primary slot. `-> name : expression` initializes
a field and introduces that name into the current scope after the expression
finishes. Local bindings without `->` remain private.

```meowy
account : {
    prefix : "user"
    -> id : 42
    -> label : "{prefix}-{id}"
    -> true
}
```

An emission does **not** stop execution or publish a partly initialized value.
Callers receive the result only after the block finishes, including its cleanup
and required task joins. An emitted owned value moves into its slot immediately;
the old binding cannot subsequently use the moved value.

Each slot can be initialized at most once on every execution path. A second
primary emission is an error, not an overwrite. The checker must prove this; if
conditions are too complex to establish exclusivity, express the selection using
`leave()`.

**Invalid — both arms can emit into the same slot:**

```meowy
select <int32> : (x <int32>) 'result {
    | x > 0 | 'result -> 1
    | x > 10 | 'result -> 2
}
```

A path without a primary emission contributes `<null>` to the inferred result
type. A declared non-null result must emit on every normally completing path.
Every declared non-null field must also be initialized on every such path. A
nullable field omitted on a path is initialized to `null`.

Without an expected record type, the block's shape contains every named slot
emitted on any reachable normally completing path. Infer each slot's type as the
normalized union of its emitted expression types, adding `null` when a completing
path omits it. Thus `{ | flag | -> name : "hello" }` has `name<string><null>` and a null
primary; the field exists on both paths. The same join rule applies to the primary.
There is no numeric widening between different already typed values. A slot's
mutability must agree at all of its emissions (`E206` otherwise).

With an expected type, check each emission against that slot's declared type;
do not first infer a wider union. Non-completing paths (`never`, leave to an outer
scope, or an iteration discarded by restart) contribute no completed value.
A block with no normally completing path has type `never`. Flow analysis uses
the control-flow graph, boolean constants, stable type predicates and their
negations, and short-circuit edges; it need not prove arbitrary arithmetic
identities to accept mutually exclusive emissions. When that proof is absent,
use an explicit leaving arm as above. A same-slot second emission remains `E205`.

An emission into an unnamed nested block stays in that block. It does not become
an emission into the parent by proximity. Use a label to target the parent, or
emit the completed nested value explicitly.

## Primary composition

When a block emits another block value as its primary, the composition retains
the inner primary and its named fields:

```meowy
connection : {
    -> {
        -> 42
        -> token : "abc"
    }
    -> ready : true
}

connection        # primary: 42 #
connection.token  # "abc" #
connection.ready  # true #
```

This is static composition. The resulting shape can be flattened into one
aggregate. Duplicate names anywhere along the primary chain are rejected; there
is no runtime search or implicit shadowing. A regular local binding in an inner
block is never exposed by this rule.

Numeric operators, comparisons against a scalar, and formatting inspect the
primary value. Equality between two aggregates compares their entire compatible
shape, including fields. Type predicates and `value<>` inspect the full type.

In a context expecting a copyable scalar, a compatible primary may be copied out
without consuming the aggregate. A context expecting a record checks the complete
record shape. An owned, non-copyable primary is never implicitly moved out while
discarding its fields: `values.take_primary(aggregate)` explicitly consumes the
aggregate and releases the named fields. This distinction keeps resources from
disappearing as an incidental consequence of a pipeline.

## Mutability

Bindings and fields are separate replacement boundaries. `:` prevents replacing
that slot; `:=` permits replacement. An immutable binding can own mutable fields:

```meowy
counter : {
    -> count <uint32> := 0
    -> label : "requests"
}

counter.count = counter.count + 1
```

Writing a field requires that selected field to be mutable and no conflicting
borrow. The containing owned binding and enclosing record fields need not be
mutable. Thus `counter.count` can change while `counter` and `counter.label`
cannot be replaced. Likewise, an immutable `object.inner` can contain a mutable
`field`: `object.inner.field = value` replaces only that final field.

List indexing inherits the containing list slot's replacement permission. A list
bound with `:` cannot have its elements replaced, but mutable fields of record
elements can still change. A field declared `-> items := ...` permits element
replacement through that list field even when the object binding uses `:`. Bounds and borrow rules
still apply. A mutable binding can replace its whole value with another value of
the same type, releasing the previous owned value.

In a record type, `count <uint32> :=` declares a mutable field. Mutability is part
of a record's shape. Shared `&value` access permits reads only, regardless of field
flags. `object.&!field` requires a mutable selected field; whole-value `&!object`
requires a replaceable object slot and a supported pointee type. An immutable
binding holding an exclusive reference can still mutate its pointee through that
reference; replacing the reference itself requires a mutable binding.

## Dispatch

`value.(f)` means `f(value)`. `value.(f, other)` means `f(value, other)`; arguments
are evaluated once, left-to-right. The ordinary parameter types determine whether
the receiver is copied, moved, or borrowed. Use `&value.(f)`, equivalently
`(&value).(f)`, for an explicit borrow when `f` takes a reference.

`value.{ ... }` evaluates the receiver once and binds it as `self` in the block.
It follows ordinary ownership rules: a move-only receiver moves into that binding.
The `self` binding cannot be replaced. An owned receiver retains its field
permissions; a shared-reference receiver remains read-only, and an exclusive
reference retains its pointee permissions.

```meowy
increment <int32> : (value <int32>) { -> value + 1 }
double <int32> : (value <int32>) { -> value * 2 }

answer : 20
    .(increment)
    .(double)
```

`value.name` selects a field. `value.&name` and `value.&!name` borrow that
selected field, shared or exclusively, and mean `&(value.name)` and
`&!(value.name)`. The modifier applies to the immediately named field:
`value.&inner.name` borrows `inner` before selecting `name`, while
`value.inner.&name` borrows `name` itself. Prefix `&value.name` instead means
`(&value).name`. Likewise, `value.*name` means `*(value.name)` and dereferences
the selected field, while `*value.name` means `(*value).name`. In
`value.*inner.name`, dereferencing applies to `inner` before selecting `name`.
These forms share a left-to-right surface syntax, but member lookup never implies
arbitrary execution. Ordinary mutability, ownership, lifetime and compiler
capability checks still apply.

## Matchers and flow analysis

`| condition | statement` requires a boolean condition. There is no truthiness:
`null`, zero, empty strings, and errors cannot stand in for booleans. Every matcher
is independent and evaluated when execution reaches it.

In the condition, `value<T>` is a type predicate regardless of spacing.
`|value<T>|statement` and `| value <T> | statement` mean the same thing. The
matcher body resumes ordinary expression syntax, where `value<T>` is a proven
ascription. [Contextual angle-bracket rules](syntax.md#angle-brackets-in-context)
also cover nested parentheses, generic calls, and arguments.

A type predicate refines a stable binding inside the matching arm. `&&` carries
the left side's refinement into its right side. A failed arm refines later code
only if its successful path cannot continue, for example after `leave()` or a
call with result `<never>`.

```meowy
is_positive <boolean> : (value <int32><null>) 'result {
    | value <null> | {
        'result -> false
        'result.leave()
    }
    -> value > 0
}
```

Assignment or a call that can mutate a tested location invalidates its refinement.
Matching a borrowed field cannot promise the same value after an intervening
exclusive mutation. The type chapter explains union subtraction in this model.

## Named scopes and cleanup

`'name { ... }` evaluates once unless restarted. A label is a control target, not
a value, closure, or reference that can escape.

Its `leave` and `restart` members are well-known scoped operation values accessed
through ordinary member lookup. They can be aliased within that scope, but an alias
retains the same function, task, and lifetime restrictions. A similarly named
member on an ordinary record has only the behavior of the value stored there.

- `'name -> value` initializes that scope's primary slot and keeps executing.
- `'name.leave()` cleans up inner scopes, then completes the named scope with its
  emissions so far. A required uninitialized result is a static error.
- `'name.restart()` cleans up inner scopes and the current iteration's locals and
  emitted values, then starts a fresh iteration. Bindings outside the named scope
  retain their current values.

Restarting after an emission discards and releases that iteration's partial
result; a later iteration has fresh emission slots. No reference to an iteration
local may survive the restart. Labels cannot be targeted across functions or
tasks. A label on a function body provides an explicit early-result pattern.

Cleanup also runs during a recoverable task panic. Already emitted values are
released if the block fails rather than completing normally. See
[memory](memory.md#cleanup) and [tasks](tasks-and-channels.md#scope-exit).

## Deferred actions

`<- expression` registers an action for cleanup of the innermost executing
lexical scope. Registration does not evaluate the expression, its receiver,
arguments or block body. The action runs once when that scope exits, after its
required child-task joins. The call and block forms have identical timing:

```meowy
{
    acquire()
    <- release()
    work()
}
```

`release()` runs after `work()` on normal completion and during recoverable
unwinding after registration. If `acquire()` fails before registration is
reached, that action does not run. The acquisition API's ordinary success/error
handling still applies; registration does not test whether acquisition succeeded.

Actions run in reverse registration order:

```meowy
debug : @"debug"

{
    <- { debug.print("first") }
    <- { debug.print("second") }
    <- { debug.print("third") }
    debug.print("body")
}
```

Output:

```text
body
third
second
first
```

Only executed registrations participate. A matcher with a direct `<-` body
conditionally registers in the containing scope; a braced matcher body has its
own scope and runs its actions when that block exits. Inner scopes finish their
cleanup before outer scopes. There is no labeled registration into another scope.

Normal completion, `leave()`, `restart()` and recoverable panic all run the
actions of scopes they exit. Each restart cleans up the current iteration before
creating fresh bindings and registrations:

```meowy
debug : @"debug"
count := 0

'loop {
    <- { debug.print("iteration finished") }
    debug.print("iteration")
    count = count + 1
    | count == 3 | 'loop.leave()
    'loop.restart()
}
```

This prints `iteration` then `iteration finished` three times, including on the
final `leave()`. Registrations never accumulate across restarts.

### Delayed reads and ownership

Names resolve at the registration site, using bindings already visible there.
Later shadowing does not retarget an action. Values are read at execution time:

```meowy
debug : @"debug"
x <int32> := 1
<- debug.print(x)
x = 2
```

The action prints `2`. To retain the earlier value, bind an explicit snapshot:

```meowy
debug : @"debug"
x <int32> := 1
saved : x
<- debug.print(saved)
x = 2
```

This prints `1`. A snapshot follows normal copy/move rules; it does not clone a
non-copyable owner. A later reassignment changes what a deferred read observes,
including which resource a deferred operation uses.

An action is a cleanup control-flow edge, not an escaping closure. Registering
it does not immediately copy, move or borrow its referenced bindings. Ordinary
mutation and temporary borrows may precede cleanup, provided the action's accesses
are valid when it runs. Explicit references stored in bindings retain their
ordinary lifetime and exclusivity restrictions.

The checker validates every applicable exit path, including recoverable unwind
paths and the effects of earlier actions in cleanup order. A deferred read cannot
rely on a refinement invalidated by intervening mutation. A consumed or partially
uninitialized value required by an action is a static ownership error; moving it
and hoping to restore it later is insufficient if unwinding can occur in between.

**Invalid — emission moves the owner needed by cleanup:**

```meowy
{
    connection : open()
    <- connection.close()
    -> connection
}
```

Here `open()` and consuming `close()` stand for a resource API. For a non-copyable
connection, emission invalidates the local immediately. The action cannot use
that moved local, even though the completed result has not yet been published.
Remove the local close action when transferring ownership to the result.

### Action boundaries

An action runs in the exiting task; it does not start a task or install a listener.
Its expression result is discarded with ordinary temporary cleanup. Recoverable
errors must be handled inside the action rather than silently discarded or
propagated from cleanup. A panic during cleanup is fatal under the existing
[cleanup contract](memory.md#cleanup).

An action cannot emit into an enclosing result, or call `leave()` or `restart()`
on a scope outside that action. Blocks and labels created inside it retain their
ordinary local emission and control rules. A nested deferred action belongs to
its own executing block and finishes before that block returns. Fatal termination
and abort do not promise execution of registered actions.

This syntax is a language contract; bootstrap parsing, ownership analysis and
execution of deferred actions remain unimplemented.
