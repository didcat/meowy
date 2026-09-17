# Memory and resource ownership

[Documentation index](../README.md)

meowy uses value storage, explicit allocation, and deterministic cleanup. Safe
code must not read uninitialized storage, use a released allocation, violate
reference exclusivity, or race on ordinary memory. There is no tracing garbage
collector or automatic heap promotion of escaping locals.

[Memory and binary optimization](optimization.md) follows these owners through
frames, heap allocations, static storage, and channel queues. It covers peak
memory budgets, constrained hosts, and the code/data retained in an executable.

## Storage and layout

Scalars, records, closed unions, and bounded lists live inline in their owner.
An owner may be a local frame, another record, static storage, or an explicitly
allocated object. Escape analysis may remove or relocate storage while preserving
its observable lifetime and identity; it cannot introduce an observable allocator
call to make an invalid borrow legal.

| Value                    | Representation contract                                             |
| ------------------------ | ------------------------------------------------------------------- |
| Fixed-width integer      | Stated bit width; signed integers use two's complement              |
| Boolean                  | One byte in ordinary addressable storage, valid values 0 and 1      |
| Null                     | No payload bytes; its enclosing union may need a tag                |
| Reference or raw pointer | Target-width address; safe references are non-null                  |
| String or slice          | Data address and `usize` length                                     |
| Bounded list `T[N]`      | Runtime length and inline capacity for `N` elements                 |
| Record                   | Primary storage, if any, and storage for each field                 |
| Closed union             | Active alternative plus enough aligned storage for that alternative |
| Non-capturing function   | Code pointer                                                        |
| Capturing function       | Code plus an inline capture environment                             |

Only initialized list elements are valid `T` values and are cleaned up. Padding
bytes are unspecified and cannot be inspected as initialized data. Native layout,
field order, tag placement, and calling convention require explicit FFI declarations;
ordinary records and unions do not promise a stable binary ABI.

`memory.size_of<T>()` and `memory.align_of<T>()` are compile-time target queries.
The size includes padding, capacity, and tags as applicable. Large inline values
can exhaust a stack; choosing inline storage is not a guarantee of cheap copies.

## Copies, moves, and borrows

Every resource has one owner. Initialization, assignment, parameter passing, and
emission copy a `memory.Copy` value and move any other value. A move invalidates
the source until it is assigned a new value.

Scalars, shared references, raw pointers, non-capturing function items/pointers, strings,
and immutable slices are copyable. Records, unions, arrays, and bounded lists are
copyable exactly when every constituent is copyable. Exclusive references,
allocation owners, channel endpoints, and task handles are not copyable. Copying a
borrowed view never extends the referenced storage's lifetime.

A capturing callable is copyable exactly when all of its captures are copyable
and it supports shared, repeatable calls. A callable requiring exclusive access
or consuming its environment is not copyable, even when its stored fields would
otherwise be copyable. Transfer between tasks follows the separate
[`Send` and `Sync` rules](tasks-and-channels.md#capability-rules).

```meowy
<Point> : <{
    x <int32>
    y <int32>
}>

point <Point> : { -> x : 2; -> y : 3 }
same : point       # an inline copy #
view : &point      # a shared borrow #
```

`&value` creates a shared reference. `&!value` creates an exclusive reference
and requires a mutable location. Shared references permit concurrent reads;
exclusive references exclude all other access to the same storage until their
last use. Field access through a reference projects a reference or copies a
copyable field. It never moves a resource out of borrowed storage.

`object.&field` borrows the selected field; `object.&!field` borrows it
exclusively. These mean `&(object.field)` and `&!(object.field)`, respectively.
For an element, write `&(items[index])` or `&!(items[index])`. The complete
selected place retains the collection's existing evaluation, bounds and loan
rules. A selected-field borrow evaluates its receiver once.

An owned field's replacement permission is independent of the containing binding:
`object : { -> field := 7 }` permits `object.&!field` while preventing replacement
of `object`. Nested owned field selection uses the final field's permission;
indexing inherits the list slot's permission. Shared-reference access remains
read-only and live overlapping loans still prevent mutation. See
[binding and field mutability](values-and-blocks.md#mutability).

Prefix borrowing happens before following field selection or indexing:
`&object.field` selects a field through `&object`, and `&items[index]` indexes
through `&items`. Selecting a copyable value this way may copy it; an explicit
borrow of the selected storage produces a reference. `object.&inner.field`
borrows `inner` before selecting `field`; `object.inner.&field` borrows the final
field. These groupings do not relax storage, mutability or lifetime requirements,
or enable an otherwise unsupported compiler capability. See
[operator precedence](syntax.md#operators-and-evaluation-order).

`*reference` accesses a safe reference's referent. In a copyable value context it
copies the value; in a predicate, field access, borrow, or formatting context it
inspects the borrowed place. It cannot move a non-copyable owner out of a reference.
Raw pointers use the explicitly unsafe memory operations instead.

Dereferencing follows the same prefix boundary as borrowing: `*object.field`
means `(*object).field`, while `object.*field` means `*(object.field)`.
`*items[index]` indexes a dereferenced list; `*(items[index])` dereferences the
selected element. To dereference a returned reference, write `*(get())`.
An ascription of the reference belongs inside that grouping, as in
`*(value<&int32>)`; `*value<&int32>` ascribes the dereferenced result instead.
The field modifier evaluates its receiver once and preserves ordinary reference
permissions, storage lifetimes and compiler capability checks.

Borrowing a field can be disjoint from borrowing another field when the checker
can prove their storage does not overlap. A dynamic index is conservatively
treated as potentially overlapping another index. No safe operation can grow,
move, or destroy a collection while outstanding references depend on its storage.

## Lifetimes

Reference lifetimes follow the borrowed owner. Local uses are inferred through
the last use of a borrow. A returned reference must borrow from an input or static
storage, never a function local.

Returned views use an inferred lifetime contract. The body must prove that every
returned reference points into borrowed input storage, a borrowed receiver or
capture, or static storage. For a public signature, the result is conservatively
bounded by all borrow-carrying inputs and captures; it cannot outlive any of them.
With no such inputs or captures, a returned borrowed view must be static. This
rule is available to callers without inspecting the body or parsing an extra
declaration modifier.

Opaque foundational-library operations can have a narrower, explicitly documented
borrow-origin contract checked by the compiler against their intrinsic behavior.
Only the named origins constrain their result: an owning copy constructor can
copy all source bytes while retaining only its allocator, and a lookup can return
a receiver-bounded view without retaining its search key. These are declarations
of verified library behavior, not inferred exceptions for an ordinary function.
User-defined wrappers retain the conservative all-input rule above, including
when they call such an operation internally.

```meowy
bytes <uint8[]> : (text <string>) {
    -> text.bytes()
}
```

Here the returned slice cannot outlive `text`. With multiple borrowed inputs,
the conservative contract may shorten a view's usable lifetime; separate the
operation into smaller functions when an independent view should live longer.
This rule applies to records containing references too. A static string literal
can be returned freely. A string built into a local buffer cannot be returned as
a borrowed view. Return the owning buffer instead, or write into caller-owned
storage and borrow that.

Non-copyable field moves mark that field uninitialized. Remaining fields are
still cleaned up, but the whole record cannot be used until all fields are valid.
Safe field moves are forbidden for opaque resources with a custom cleanup
contract. References cannot outlive a task join, loop iteration, or allocation
that they borrow from.

### Temporary owners

An expression result that has not moved into a binding, emission slot, argument,
or other owner is a temporary. Its storage lasts until the end of the innermost
complete statement that evaluated it. This includes an entire call or dispatch
chain and, for a matcher, its condition and controlled statement. Temporaries
remaining at that boundary are released in reverse initialization order. An
unwind or scoped control transfer releases them before leaving their statement.
Moving a temporary into another owner transfers cleanup to that owner normally.

A borrow of a temporary can be used during that statement. Storing the borrow
does not extend the temporary's lifetime into later statements. There is no
initializer-specific lifetime extension or automatic promotion to static storage;
constant folding cannot turn an otherwise invalid borrow into valid source.
`&(make())` explicitly borrows a call result; `&make()` instead calls through a
borrow of `make`. Borrowing the result does not extend its temporary lifetime.

```meowy
bytes : @"bytes"
debug : @"debug"

# Accepted: the temporary list survives the complete print statement. #
debug.print(bytes.filled<4>(0).slice().size())

# Accepted: explicit owner storage survives the subsequent view use. #
buffer : bytes.filled<4>(0)
view : buffer.slice()
debug.print(view.size())
```

**Rejected — the temporary owner has expired at the later use:**

```meowy
view : bytes.filled<4>(0).slice()
debug.print(view.size())
```

Task submission does not extend a borrowed temporary either. Moving an owned
temporary into a child's capture is valid; capturing its borrowed view requires
the child to have joined before the temporary's statement ends. A bound task
that will be joined in a later statement therefore needs an explicit surviving
owner for any such borrow.

## Explicit allocation

Dynamic storage takes an allocator:

```meowy
memory : @"memory"
collections : @"collections"

'work {
    buffer : collections.vector<uint8>(memory.heap, 256)
    | buffer <memory.AllocationFailure> | 'work.leave()

    # buffer owns its allocation; leaving this scope releases it #
}
```

`memory.heap` is an explicitly selected system allocator. Its type is
`memory.Allocator`, a copyable borrowed handle supporting `tasks.Send` and
`tasks.Sync`; this particular handle has static lifetime. Every safe allocator
handle supports allocation and release on any runtime thread, synchronizing its
own state as needed. A thread-affine allocator cannot expose this safe handle.
Library operations that grow storage accept or retain the allocator chosen at
construction. Allocation failure is a value, not a null pointer or an automatic
process abort.

An allocator handle is borrowed by its allocations and must remain valid until
they are released. An arena can use caller-provided storage; resetting it requires
that all values and references backed by that arena have expired. An arena reset
cannot silently skip the cleanup of live resource owners.

Allocation returns initialized owners through safe constructors. Uninitialized
raw storage is confined to unsafe code until every required byte and field has
been initialized. There is no general safe `zeroed<T>()`: zero bits are not a valid
value for every type.

## Strings and formatting

`<string>` is a borrowed immutable UTF-8 view. Its length counts bytes. It cannot
be used as a NUL-terminated C string, and byte positions are not character indices.
Owned text uses a library buffer with an allocator; `.view()` borrows it.

An interpolation at a formatting call, such as `debug.print("value: {x}")`,
streams its pieces to the writer and needs no intermediate owned string. In a
stored string initializer, interpolation must be constant-evaluable or use an
explicit text builder and allocator. String concatenation follows the same rule;
`+` cannot silently allocate a runtime string.

The message arguments of [`testing` assertions](stdlib/testing.md#assertions-borrow-their-evidence)
are also formatting boundaries. They evaluate message expressions once and can
stream failing-test context into bounded diagnostic storage without an owned
intermediate string; this does not change the rules for stored strings.

For example, `"{prefix}-{id}"` is a constant when both inputs are constant values.
For runtime output, use a writer or builder. Plain string literal views live for
the whole program.

## Cleanup

Owners are released in reverse initialization order on normal completion,
`leave()`, `restart()`, and panic unwinding. Emitted owners belong to the result
and survive normal completion. If construction fails, only the slots already
initialized are released.

[Deferred actions](values-and-blocks.md#deferred-actions) and automatic owner
release share one cleanup sequence. Successful owner initialization and executed
`<-` registration add entries; scope exit processes those entries in reverse
order, after required child-task joins. An action can therefore access owners
initialized before its registration, while later owners have already been released.

For example, initializing `first`, registering `<- inspect(first)`, then
initializing `second` releases `second`, runs `inspect(first)`, then releases
`first`. If an action consumes an owner, its later automatic release is skipped.
Explicit closure of an opaque resource follows that resource's existing contract;
it must not produce a second release during automatic cleanup.

Registration creates no persistent implicit borrow. The checker includes delayed
reads, writes and moves in cleanup order on every exit path. Moving an owner into
an emission does not move a registered action with it: an action that still needs
the old local makes that transfer invalid. Cleanup cannot use a released owner,
an invalid reference or a value consumed by an action that runs earlier.

Cleanup is compiler-generated from the value's ownership structure. Opaque
resource types supply an intrinsic release operation. Cleanup cannot emit into an
enclosing result, restart an enclosing scope, or throw a recoverable error.
Deferred actions may handle fallible operations internally. Otherwise, an operation such as flushing
a file must be called explicitly before cleanup. Releasing an already explicitly
closed endpoint has no further effect.

A scope first requests cancellation for unfinished children and joins **all**
children before releasing storage they could borrow. It then releases remaining
locals in reverse order. A deadline does not permit destroying a running child's
stack or borrowed data. This may delay scope exit if a child cannot cooperate.

A recoverable panic unwinds and releases owners. A panic during cleanup is fatal.
Process termination, a fatal trap, and an explicit abort do not promise cleanup.

## Raw pointers and unsafe operations

Raw pointers describe addresses without proving validity. They may be null or
dangling, but reading or writing through them requires a `!{ ... }` block and a
proof of allocation lifetime, bounds, alignment, initialization, and access rights.
A raw pointer alone establishes none of the safe-reference rules.

### Raw pointer values

The following operations construct or inspect raw **data** pointers. None exposes
a function's code address, allocates storage, reads pointed-to bytes, or extends
a borrowed owner's lifetime:

- `memory.address<T>(value <&T>)` returns `<*T>` with the referent's address and
  allocation provenance. `memory.address_mut<T>(value <&!T>)` returns `<*!T>`.
  These safe operations borrow only for the call. Afterward the pointer can
  become dangling; using it still requires proving the original owner is live
  and the requested access is permitted at that use.
- `memory.null<T>()` and `memory.null_mut<T>()` return a null `<*T>` or `<*!T>`.
  They do not produce a safe reference or a `T` value. A literal `null` does not
  implicitly convert to either pointer type.
- `memory.is_null<T>(pointer)` returns `boolean` and accepts either `<*T>` or
  `<*!T>`. It tests only the address, so it can inspect a dangling raw pointer
  without dereferencing it. A false result proves neither validity nor alignment.
- `memory.readonly<T>(pointer <*!T>)` returns `<*T>` with unchanged address and
  provenance. It grants no new access rights and does not release an owner.
- `memory.cast<T, U>(pointer <*T>)` returns `<*U>`; its writable counterpart
  `memory.cast_mut<T, U>(pointer <*!T>)` returns `<*!U>`. Both require `!{ ... }`
  and preserve address and allocation provenance. They establish no bounds,
  alignment, initialization, lifetime, or permission for `U`. Null remains null;
  a resulting misaligned pointer can exist but cannot be dereferenced as `U`.
  A cast cannot turn a read-only pointer into a writable one.

These are the complete pointer-construction conversions in this profile. There
is no integer-to-pointer conversion, pointer-to-integer conversion, implicit
mutability conversion, or raw-pointer-to-safe-reference constructor. Use an
explicit safe wrapper with an existing borrow when safe access must be returned;
unsafe native results can instead be copied through `memory.read` once its
preconditions hold. The absence of a safe-reference constructor does not permit
an ascription to perform that conversion.

`T` and `U` must be concrete runtime data types; a function or callable type is
not a data-pointer target for these constructors. `null` may be used as the
target of a raw void pointer at the native boundary, but cannot be dereferenced
as a live zero-sized object. Casting an address into or out of `<*null>` retains
the source allocation's provenance and all obligations for its actual contents.

### Reading pointed-to storage

```meowy
memory : @"memory"

read_word <uint32> : (address <*uint32>) !{
    -> memory.read<uint32>(address)
}
```

The function's `!{ ... }` body preserves the caller's precondition: `address` must
point to a live, aligned, initialized `uint32` that can be read without racing a writer.
Its item has signature `!(*uint32) -> uint32` and may coerce to that unsafe
function-pointer type; calling it requires a `!{ ... }` block even
though it has a meowy body.
Prefer a safe slice parameter when bounds and ownership can be expressed in the type.

Inside an ordinary function, an inner `!{ ... }` block marks a locally justified
operation. It does not grant permission to a separately declared function or a
spawned task: each must establish its own boundary. An intrinsic such as
`memory.read` keeps its calling requirements when assigned another name. A local
binding named `unsafe` has no special meaning.

Pointer arithmetic uses byte offsets (`memory.offset_bytes`); collection indexing
remains one-based. Integer/pointer conversions are unavailable in this profile;
an integer containing an address is not a valid pointer proof. `!{ ... }` never
disables integer checks, type checking, cleanup, or task ownership rules.

For memory shared between tasks, use channels, locks, or atomics. Ordinary shared
mutable storage and raw pointers are not automatically transferable. Volatile
access expresses an observable memory access; it is not a synchronization primitive.
