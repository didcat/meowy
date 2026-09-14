# Storage and owned values

[Library index](README.md)

## Storage and values

| API                                                    | Result                         | Contract                                                               |
| ------------------------------------------------------ | ------------------------------ | ---------------------------------------------------------------------- |
| `memory.heap`                                          | `memory.Allocator`             | Static handle selecting system-managed dynamic storage                 |
| `memory.size_of<T>()`                                  | `<usize>`                      | Compile-time storage size for the target                               |
| `memory.align_of<T>()`                                 | `<usize>`                      | Compile-time storage alignment for the target                          |
| `memory.address<T>(value <&T>)`                        | `<*T>`                         | Obtain a raw data address while retaining provenance                   |
| `memory.address_mut<T>(value <&!T>)`                   | `<*!T>`                        | Obtain a writable raw data address through an exclusive borrow         |
| `memory.null<T>()`, `memory.null_mut<T>()`             | `<*T>`, `<*!T>`                | Construct a typed null data pointer                                    |
| `memory.is_null<T>(pointer)`                           | `<boolean>`                    | Test either raw-pointer mutability without dereferencing               |
| `memory.readonly<T>(pointer <*!T>)`                    | `<*T>`                         | Remove write permission without changing the address                   |
| `memory.cast<T, U>(pointer <*T>)`                      | `<*U>`                         | Unsafe data-pointer reinterpretation preserving address/provenance     |
| `memory.cast_mut<T, U>(pointer <*!T>)`                 | `<*!U>`                        | Unsafe writable data-pointer reinterpretation preserving provenance    |
| `memory.read<T>(pointer <*T>)`                         | `<T>`                          | Unsafe read of initialized aligned storage; requires `T : memory.Copy` |
| `memory.offset_bytes<T>(pointer <*T>, offset <isize>)` | `<*T>`                         | Unsafe address calculation within the same allocation or one past it   |
| `values.take_primary(value)`                           | Primary type of the input      | Consumes the aggregate, releases its fields, and transfers its primary |
| `dynamic.box(value, allocator)`                        | `<any><dynamic.BoxFailure<T>>` | Erases a non-null owner; failure retains the original `value`          |
| `dynamic.take<T>(value <any>)`                         | `<T>`                          | Consumes and extracts a box after a proven matching type test          |

`memory.AllocationFailure` describes a failed allocation without requiring a new
allocation to report it. `dynamic.BoxFailure<T>` is a concrete error that also
retains the input owner. An allocator handle retained by a constructor must
outlive the constructed owner. `memory.Copy` is a structural capability, not an
opt-in promise that can override ownership restrictions.

The safe `memory.Allocator` handle is `memory.Copy`, `tasks.Send`, and `tasks.Sync`;
allocation and release must be usable from any runtime thread. Copying its handle
does not extend its retained lifetime. `memory.heap` is the program-lifetime
instance. Thread-affine allocators are outside this safe contract. This keeps
owner capabilities determined by their contained types rather than by the runtime
identity of an allocator; see the [allocator contract](../memory.md#explicit-allocation).

Address construction, null construction/testing, and conversion to readonly are
safe operations; they do not establish a later dereference's lifetime, alignment,
initialization, or access proof. Casts require an unchecked boundary and preserve
data-pointer provenance. None of these operations accepts a function pointer or
an integer address, reveals a native function address, or extends a referent's
lifetime. The [raw-pointer rules](../memory.md#raw-pointers-and-unsafe-operations)
define the obligations of reads, foreign calls, and unsafe casts.

For error-specific erasure, [errors.box](errors.md#choose-inline-storage-or-explicit-erasure)
preserves an error's nominal tag and returns either an owned descriptor in a
success record or `errors.BoxFailure<E>` retaining the original error. Common
metadata inspection and concrete error construction need no such allocation.

The [memory reference](../memory.md) defines ownership, borrowing, raw pointer
preconditions, string lifetimes, and deterministic cleanup. These APIs do not
relax those rules.

Type capability queries and immediate place-operation probes are specified by
[`@"proof"`](proof.md#type-capabilities-and-ownership-probes). A query reports the
canonical checker's knowledge; it performs no copy, move, borrow, or reservation.
Its result cannot replace any of the storage, lifetime, or authority checks above.
