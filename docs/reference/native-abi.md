# Native ABI profile

[Documentation index](../README.md)

This profile defines the C boundary for the initial
[`x86_64-unknown-linux-gnu` target](target-profile.md). It uses the System V AMD64
C ABI and the target descriptor's LP64 data model. Ordinary meowy records,
unions, and function calls still have no public native ABI. An unsupported
convention, type, or target combination is a static error, not a request for
best-effort marshalling.

The external layout and call rules are pinned to the upstream
[x86-64 psABI source, revision `ab2062ad`](https://gitlab.com/x86-psABIs/x86-64-ABI/-/blob/ab2062ad5653913c39124548943b1177330e34c8/x86-64-ABI/low-level-sys-info.tex),
particularly **Fundamental Types**, **Aggregates and Unions**, and **Function
Calling Sequence**. The admitted meowy subset and adapter rules below are this
language profile's choices; the psABI supports additional C types that it omits.

## Scalar aliases

These are the complete C scalar aliases exposed by `@"ffi"` in this profile.
They are ordinary aliases of the stated meowy types. Using the underlying type
has the same admissibility; an alias does not create a new nominal scalar type.

| Alias                                 | meowy type           | C type in this target                         |
| ------------------------------------- | -------------------- | --------------------------------------------- |
| `ffi.c_bool`                          | `boolean`            | `_Bool`                                       |
| `ffi.c_char`, `ffi.c_schar`           | `int8`               | `char`, `signed char`; plain `char` is signed |
| `ffi.c_uchar`                         | `uint8`              | `unsigned char`                               |
| `ffi.c_short`, `ffi.c_ushort`         | `int16`, `uint16`    | `short`, `unsigned short`                     |
| `ffi.c_int`, `ffi.c_uint`             | `int32`, `uint32`    | `int`, `unsigned int`                         |
| `ffi.c_long`, `ffi.c_ulong`           | `int64`, `uint64`    | `long`, `unsigned long`                       |
| `ffi.c_long_long`, `ffi.c_ulong_long` | `int64`, `uint64`    | `long long`, `unsigned long long`             |
| `ffi.c_size`, `ffi.c_ptrdiff`         | `usize`, `isize`     | `size_t`, `ptrdiff_t`                         |
| `ffi.c_float`, `ffi.c_double`         | `float32`, `float64` | `float`, `double`                             |

Signedness, width, alignment, argument classification, and result classification
follow this target's ABI. Its `long` and `long long` aliases share a meowy type
and ABI representation; the native binding must nevertheless declare the actual
foreign function's compatible C contract. There is no C alias for `long double`,
`__int128`, a C enum, or an implementation-specific packed/vector type in this
profile. Lower an enum through the explicit integer type used by a fixed native
wrapper. A `boolean` crossing the boundary has only the valid values false and
true; native storage presented as that type must contain a valid C `_Bool`.

In bytes, the native size/alignment pairs are 1/1 for boolean and character
types, 2/2 for short integers, 4/4 for `int` and `float`, and 8/8 for LP64
`long`, `long long`, `double`, and data pointers. Native aggregate alignment is
the greatest field alignment, with padding between fields and at the end as
required. A native array field uses its element's alignment. The psABI's separate
16-byte minimum for sufficiently large standalone C array variables does not
increase the alignment of an array member inside a native record.

## Admissible signatures and fields

This table is closed. Each recursively contained field and array element must
also be admissible in its position. A read-only raw pointer expresses the
binding's access promise; C `const` does not create a separate ABI representation.

| meowy type                                                               | Parameter                         | Result           | Field of `ffi.record` |
| ------------------------------------------------------------------------ | --------------------------------- | ---------------- | --------------------- |
| Scalar types in the alias table, including their underlying primitives   | Yes                               | Yes              | Yes                   |
| Raw data pointer `*T` or `*!T`, for an admissible native object type `T` | Yes                               | Yes              | Yes                   |
| `*null`, `*!null`                                                        | Yes, as `const void *` / `void *` | Yes              | Yes                   |
| Type constructed by `ffi.record("C", fields)`                            | Yes, by value                     | Yes, by value    | Yes, inline           |
| `collections.Array<T, N>` with native element type and positive `N`      | No                                | No               | Yes, inline C array   |
| `null`                                                                   | No                                | Yes, as C `void` | No                    |
| Every other type                                                         | No                                | No               | No                    |

In particular, `int128`, `uint128`, `never`, strings, safe references, slices,
bounded lists, ordinary records/unions, literal-subtype unions, erased values,
resource owners, and meowy function pointers are not native parameter, result,
or field types. A primitive literal subtype must be widened to its primitive
type in a native signature. A C function with no parameters has `()` in its
meowy signature; `null` does not stand for an omitted or void parameter.

Raw pointers to fixed arrays or native records are admitted. `*null` is a raw
void-pointer type, not a safe reference or a `null` literal coercion. The
[raw-pointer operations](memory.md#raw-pointer-values) explicitly construct null
pointers, expose borrowed data addresses, test for null, and reinterpret raw
pointer types. The admissibility table does not manufacture a valid address.
meowy code cannot treat a `*null` pointer as an initialized zero-sized object.
Native function-pointer types and C callbacks are outside this profile.

`ffi.record` requires at least one uniquely named field. The ordered field list
sets field order; the target ABI sets offsets, padding, alignment, and total
size. Field access follows ordinary meowy mutability rules. A native record has
no primary emission, expansion, bitfields, flexible array member, packed-layout
override, or custom cleanup. Use a native wrapper when such a foreign structure
must be translated. Native record identity is the tuple of target ABI,
convention, and ordered field names/type identities. Repeating the same
constructor inputs yields the same native type; a matching ordinary structural
record remains incompatible.

These declarations are accepted:

```meowy
ffi : @"ffi"
collections : @"collections"

<Position> : ffi.record("C", ["x" : <float32>, "y" : <float32>])
<Packet> : ffi.record("C", [
    "count" : <ffi.c_uint>,
    "bytes" : <collections.Array<uint8, 16>>
])
```

These declarations are rejected before native linking:

```meowy
bad_text : ffi.extern<(string) -> null>("C", "consume_text")
bad_array : ffi.extern<(collections.Array<uint8, 16>) -> null>("C", "consume_array")
bad_void_parameter : ffi.extern<(null) -> null>("C", "flush")
```

## Function adapters retain the calling convention

`ffi.extern<Signature>("C", symbol)` takes an ordinary, fixed-arity function
signature and a compile-time symbol literal. Its result always has the unsafe
meowy function-pointer type `<!(Parameters) -> Result>`, even though the supplied
signature omits `!`. The compiler creates a non-capturing meowy-callable adapter
whose body calls the resolved C symbol using the target C ABI. A signature
already marked unsafe denotes the same requested parameter/result signature.
Variadic signatures and unchecked implicit promotions are rejected.

Calls through the returned pointer always use the meowy calling convention.
The adapter performs only the specified ABI lowering/raising; it does not allocate,
box arguments, convert a string, repair invalid pointers, catch a foreign
exception, or validate a symbol's real declaration. Its identity is determined by
the resolved native symbol, parameter/result type identities, convention, and
target ABI. Repeated declarations of the same identity and ordinary aliases
preserve that identity. Different checked signatures do not become compatible
merely because they resolve the same native symbol.

Consequently this assignment is valid and does not erase an ABI distinction:

```meowy
ffi : @"ffi"

local <ffi.c_uint> : (value <ffi.c_uint>) !{ -> value }
native : ffi.extern<(ffi.c_uint) -> ffi.c_uint>("C", "native_identity")
selected <!(ffi.c_uint) -> ffi.c_uint> := local
selected = native

!{ selected(7) }
```

Both values stored in `selected` are meowy-callable pointers; only `native` has
an adapter body that crosses into C. Assigning either to a safe function-pointer
type is rejected. The native C address cannot be extracted from the adapter or
passed as a native callback. Function pointers inside C records/parameters are
therefore rejected rather than reusing this meowy pointer representation.

## A complete fixed-signature call

Suppose an explicitly selected native artifact exports these C definitions:

```c
#include <stdint.h>
#include <stddef.h>

uint32_t native_add(uint32_t left, uint32_t right) {
    return left + right;
}

void native_ping(void) { }

void native_fill(uint8_t *output, size_t length) {
    for (size_t i = 0; i < length; ++i) output[i] = (uint8_t)i;
}
```

The caller declares matching signatures:

```meowy
ffi : @"ffi"
debug : @"debug"
bytes : @"bytes"
memory : @"memory"

add : ffi.extern<(uint32, uint32) -> uint32>("C", "native_add")
ping : ffi.extern<() -> null>("C", "native_ping")
fill : ffi.extern<(*!uint8, usize) -> null>("C", "native_fill")

answer : !{ -> add(2, 3) }
!{ ping() }
debug.print(answer)  # 5 #

buffer := bytes.filled<4>(0)
!{ fill(memory.address_mut(&!(buffer[1])), buffer.size()) }
debug.print(buffer[4])  # 3 #
-> 0
```

The `void` adapter returns meowy `null` after the C call returns. Native
artifacts must be declared through the project's exact `build.native` inputs;
the symbol example does not authorize ambient library search. C arithmetic and
other native effects follow the declared foreign contract, not the meowy
arithmetic rules. A safe wrapper may call this boundary after establishing its
preconditions, but `!{ ... }` alone proves nothing about a native implementation.

The `fill` call exposes the first initialized element, not the bounded list's
length header. Its native contract writes exactly `length` bytes during the
call and retains no address. The exclusive element borrow and the live buffer
identify the first data byte; the caller additionally establishes that all four
initialized bytes belong to that allocation and that no overlapping borrow is
live for the call. Raw C offsets start at zero while the corresponding meowy
positions start at one.

Unwinding across the adapter's foreign frame is forbidden. A C function must not
throw, long-jump through a meowy frame, or call back into meowy under this profile.
Foreign data/lifetime/thread preconditions remain binding, and a foreign call may
block its worker until it returns. Native exports, native callbacks, host runtime
embedding, dynamic loading, and foreign exception translation require separate
contracts and are not accepted by this profile.
