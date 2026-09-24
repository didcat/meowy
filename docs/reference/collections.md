# Collections and text

[Documentation index](../README.md)

meowy separates capacity, length, ownership, and lookup. Choosing a storage type
is part of choosing a program's memory behavior.

## Storage choices

| Type                        | Length                         | Storage                     | Can grow?                      |
| --------------------------- | ------------------------------ | --------------------------- | ------------------------------ |
| `<T[N]>`                    | Runtime, from zero through `N` | Inline, bounded list        | Within capacity                |
| `<collections.Array<T, N>>` | Exactly `N`                    | Inline, contiguous elements | No                             |
| `<T[]>`                     | Runtime                        | Borrowed immutable slice    | No                             |
| `<collections.MutSlice<T>>` | Runtime                        | Exclusive borrowed slice    | No                             |
| `<collections.Vector<T>>`   | Runtime                        | Owned allocation            | Explicitly, with its allocator |
| `<collections.Map<K, V>>`   | Runtime                        | Owned table                 | Explicitly, with its allocator |

All collection positions are **one-based**. A valid position is in
`1..=size()`. Raw byte offsets, as used in a native interface or a packet format,
remain zero-based offsets and must be translated deliberately.

## Bounded lists

```meowy
names <string[4]> := ["Ada", "Dev"]
names = names.add("Lin")

names.size()  # 3 #
names[1]      # "Ada" #
```

`<string[4]>` reserves room for at most four string views. It does not allocate or
promise exactly four initialized elements. An unannotated nonempty literal has
capacity equal to its element count and a common inferred element type. Use an
annotation when you want extra capacity or a union element type.

For an unannotated literal, all already typed elements must have one identical
normalized type. Check untyped scalar literals against that type; if there are
no typed elements, apply the ordinary scalar defaults and require the resulting
types to agree. No new union, numeric promotion, or primary projection is invented
to reconcile elements. Thus `[1,2]` is `int32[2]`, a typed `uint8` beside literal
`2` produces `uint8[2]`, and `[1,"two"]` requires an explicit union element
annotation (`E207` otherwise). If the existing common type is already a union,
ordinary assignment of a member into that union is allowed. An expected list
type checks every element against its declared element type and capacity instead.

`.add(value)` consumes a list and returns the updated list of the same type. A
full list panics; if fullness is known at compile time, the operation is rejected.
For recoverable capacity checks, `.try_add(value)` returns the updated list or
`collections.Full<T, N>`, which owns both the unchanged list and the rejected
element. An error must not silently consume the caller's resource.

An inline capacity cannot become dynamic after an assignment. Construct a vector
with an allocator when runtime growth is required.

## Indexing, mutation, and removal

`list[index]` performs a bounds check and panics if the position is invalid. A
constant invalid position is a static error. It copies copyable elements and
provides a borrowed place for non-copyable elements; it never implicitly moves an
owner out of a list. Use an explicit take/remove operation to transfer ownership.

`list.get(index)` borrows the list and returns `<&T><collections.Bounds>`. A
checked copyable read can use `list.get_copy(index)`, returning
`<T><collections.Bounds>`. Bounds errors contain the requested position and length.

```meowy
values <uint8[3]> := [10, 20, 30]
values[2] = 25
```

Replacing an element requires a mutable containing list slot, exclusive access
and an existing position. A list stored in an object's `:=` field can have its
elements replaced even when the object's binding uses `:`. An immutable list
slot prevents element replacement, but a record element may expose its own mutable
fields; changing such a field does not replace the element or the list.
Assignment to position `size() + 1` never appends. Removing an element compacts the
remaining positions, preserving their order. Borrowed slices and references must
expire before an operation that moves elements or changes length.

`list.remove(index)` consumes the list and returns a record with `list` and
`value` fields; an invalid position panics. Its checked counterpart
`try_remove` returns that record or `collections.Missing<T, N>`, retaining the
unchanged list in the error. Indexing a moved list is a static error.

## Slices and arrays

`.slice()` borrows all initialized elements as `<T[]>`; `.slice_mut()` requires
exclusive access and returns `<collections.MutSlice<T>>`. A slice's lifetime
cannot exceed its owner, and it never frees the underlying storage.

```meowy
sum <uint32> : (values <uint8[]>) {
    numbers : @"numbers"
    index <usize> := 1
    total <uint32> := 0

    'loop {
        | index > values.size() | 'loop.leave()
        widened : numbers.convert<uint32>(values[index])
        # uint8 to uint32 is always representable; use the proven conversion #
        total = total + widened~<uint32>
        index = index + 1
        'loop.restart()
    }

    -> total
}
```

A checked conversion with a statically proven representable source range has no
error alternative. It remains an explicit conversion. The sum itself still has
checked overflow; callers needing arbitrarily large totals must choose a larger
accumulator or a checked addition API.

`collections.array<T, N>(literal)` requires exactly `N` elements. Arrays have no
runtime length field and are suitable for fixed binary storage. A bounded list's
length header makes it unsuitable as a direct native `T[N]` argument. Both forms
can produce slices without allocating.

## Named positions

Literal names remain a convenience for bounded lists:

```meowy
fruit <string[3]> : [
    "apple" : "red",
    "yellow",
    "grape" : "purple"
]

fruit[1]        # "red" #
fruit["apple"]  # "red" #
```

Names must be unique string literals. They are aliases for element identities,
not a replacement for numeric positions. After removal, surviving aliases follow
their elements as positions shift. Removing an element clears its alias's runtime
presence; a replacement element does not inherit it. The type retains the same
set of permitted alias names, so removal does not change the list's static type.
Duplicate aliases are static errors.

This form uses bounded inline alias metadata; it does not create a hash table.
A named-list literal carries that metadata in its concrete type. A `<T[N]>`
annotation supplies element type and capacity while preserving those literal
aliases; it does not erase them. Permitted alias sets must match on whole-value
assignment.
Only literal-key lookup is allowed. Missing or removed aliases panic on indexing;
checked `get("name")` returns a bounds error instead. A slice drops access to
aliases and exposes only contiguous elements.

For runtime keys, use `collections.Map<K, V>` with an explicit allocator and
explicit hash/equality functions for custom keys. A map is a separate data
structure; its [API contract](stdlib/collections.md#runtime-key-maps) defines
failed-insertion ownership, iteration, and capacity. Insertion never overloads
an out-of-bounds list assignment, and maps do not promise numeric positions or
stable iteration order.

## Text

Strings are immutable borrowed UTF-8 slices. `.size()` counts bytes and `.bytes()`
returns `<uint8[]>`. String slicing must preserve UTF-8 boundaries; arbitrary
binary data belongs in a byte slice. Unicode scalar iteration and grapheme
iteration are separate library operations and must not be confused with byte
indexing.

Owned text uses an explicit builder and allocator. Formatting can stream to a
writer without constructing an intermediate string. See [memory](memory.md#strings-and-formatting)
for interpolation and lifetime rules.

The [text library](stdlib/text-and-data.md) defines byte spans, Unicode cursors,
owned builders, numeric parsing, and streaming output.
