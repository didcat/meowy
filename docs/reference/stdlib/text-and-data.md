# Text, data, and numeric utilities

[Library index](README.md) · [Collections](../collections.md) · [I/O](io-and-system.md)

These modules turn raw input into useful values without hiding storage decisions.
Borrowed operations return views; growing operations take an allocator; streaming
operations take an explicit write callable. No conversion silently boxes a value
or treats arbitrary bytes as valid text.

## Bytes and UTF-8 strings

All slice/string positions are one-based byte positions. A `(start, length)` span
may begin at `size() + 1` only when its length is zero. Raw wire offsets remain
zero-based and must be translated explicitly. Arithmetic validating a span cannot
wrap before the bounds check.

| API                                                             | Result                                        | Contract                                                                                      |
| --------------------------------------------------------------- | --------------------------------------------- | --------------------------------------------------------------------------------------------- |
| `bytes.filled<N>(value <uint8>)`                                | `uint8[N]`                                    | Inline list of length/capacity N, with every byte initialized                                 |
| `bytes.slice(data, start, length)`                              | `uint8[]` or `collections.Bounds`             | Borrow a checked byte span                                                                    |
| `bytes.copy(destination, source)`                               | `usize`                                       | Copy up to the shorter length between disjoint spans                                          |
| `bytes.copy_within(buffer, destination, source, length)`        | `null` or `collections.Bounds`                | Checked one-based spans within one exclusive initialized slice; overlap has memmove semantics |
| `bytes.find(data, needle)`                                      | `usize` or `null`                             | First matching byte position; an empty needle matches at 1                                    |
| `bytes.equal(a, b)`                                             | `boolean`                                     | Compare lengths and bytes                                                                     |
| `strings.from_utf8(data <uint8[]>)`                             | `string` or `strings.InvalidUtf8`             | Validate once and return a view of the same storage                                           |
| `strings.slice(text, start, length)`                            | `string` or `strings.SliceError`              | Validate bounds and both UTF-8 boundaries                                                     |
| `strings.contains(text, part)`                                  | `boolean`                                     | Exact, case-sensitive UTF-8 substring search                                                  |
| `strings.starts_with(text, prefix)`, `.ends_with(text, suffix)` | `boolean`                                     | Exact prefix/suffix test                                                                      |
| `strings.trim_ascii(text)`                                      | `string`                                      | Borrow after trimming ASCII space, tab, CR, LF, VT, and FF at both ends                       |
| `strings.trim(text)`                                            | `string`                                      | Borrow after trimming Unicode White_Space scalars                                             |
| `strings.split(text, separator)`                                | `strings.Split` or `strings.InvalidSeparator` | Borrowed cursor over non-overlapping exact separators; empty separator is an error            |
| `strings.lines(text)`                                           | `strings.Lines`                               | Borrowed cursor splitting LF and CRLF, omitting the terminator                                |

`Split.next()` returns `<string><iter.End>`. It preserves empty segments, including
trailing ones. `Lines.next()` returns the same union; a final newline does not add
a spurious extra line, and a lone CR remains text. Cursors store their position
inline, require exclusive access to advance, and cannot outlive their source.
`iter.End` is an ordinary end marker, distinct from `null`, an empty string, and
an error. Reading a cursor again after End still returns End.

`bytes.copy` returns the number written and requires exclusive destination access.
Its source borrow cannot overlap that destination. Use copy_within to describe an
overlapping move through a single exclusive borrow; invalid bounds leave the slice
unchanged. Other APIs borrow immutable data unless stated otherwise. Substring search does
not normalize Unicode or fold case. Invalid UTF-8 errors carry the first invalid
byte position, never an automatically repaired replacement string.

## Unicode operations

`unicode.scalars(text)` returns an inline cursor whose `next()` returns a record
with `value <unicode.Scalar>`, `byte_position <usize>`, and `byte_length <usize>`,
or `iter.End`. `unicode.graphemes(text)` returns borrowed string segments or End.
Both inspect the original UTF-8 without allocating; a grapheme may contain several
scalars and several bytes.

Grapheme boundaries follow the default extended rules in
[Unicode Text Segmentation](https://www.unicode.org/reports/tr29/), using the
version named by `unicode.Version`. Word segmentation is a different operation;
reading one grapheme cluster never promises one word, glyph, or terminal column.

| API                                                                         | Result                                      | Contract                                                            |
| --------------------------------------------------------------------------- | ------------------------------------------- | ------------------------------------------------------------------- |
| `unicode.scalar(value <uint32>)`                                            | `unicode.Scalar` or `unicode.InvalidScalar` | Reject surrogate values and values above the Unicode range          |
| `scalar.value()`                                                            | `uint32`                                    | Read the validated code point                                       |
| `unicode.is_letter(scalar)`, `.is_number(scalar)`, `.is_whitespace(scalar)` | `boolean`                                   | Query versioned Unicode properties                                  |
| `unicode.normalize_into(text, form, &!buffer)`                              | `string` or `strings.BufferTooSmall`        | NFC, NFD, NFKC, or NFKD into caller-owned bytes                     |
| `unicode.casefold_into(text, &!buffer)`                                     | `string` or `strings.BufferTooSmall`        | Full default Unicode case folding, not locale-sensitive lowercasing |

`unicode.NFC`, `NFD`, `NFKC`, and `NFKD` are ordinary normalization-form values.
Scalar and normalization-form values are `memory.Copy`, `tasks.Send`, and
`tasks.Sync`; scalar equality compares code points and form equality compares
the selected form. Both support their diagnostic display (a scalar as `U+`
followed by uppercase hexadecimal, at least four digits; a form as its name).
`is_letter` uses general category L, `is_number` category N, and `is_whitespace`
the White_Space property. Normalization and folding may change byte length;
they never promise in-place fixed-width character replacement. Their `_into`
operations measure and validate first, leave the buffer unchanged on failure,
and return a view borrowing the successful output. Locale-sensitive spelling and
collation require an explicitly selected locale and are not guessed from the OS.

## Owned strings and formatting

| API                                         | Result                                          | Contract                                                           |
| ------------------------------------------- | ----------------------------------------------- | ------------------------------------------------------------------ |
| `strings.copy(text, allocator)`             | `strings.Owned` or `memory.AllocationFailure`   | Copy UTF-8 into an owner                                           |
| `strings.builder(allocator, capacity)`      | `strings.Builder` or `memory.AllocationFailure` | Allocate an initially empty text builder                           |
| `builder.append(text)`                      | `null` or `memory.AllocationFailure`            | Append checked UTF-8, retaining old contents if growth fails       |
| `builder.append_scalar(scalar)`             | `null` or `memory.AllocationFailure`            | Append one encoded Unicode scalar                                  |
| `builder.view()`                            | `string`                                        | Borrow current contents; mutation must wait for the view to expire |
| `builder.finish()`                          | `strings.Owned`                                 | Consume the builder and transfer its allocation                    |
| `owned.view()`                              | `string`                                        | Borrow owned text                                                  |
| `strings.join(parts, separator, allocator)` | `strings.Owned` or `memory.AllocationFailure`   | Measure and allocate the concatenation explicitly                  |
| `fmt.write(write, value)`                   | `null` or `io.Error`                            | Stream the value's diagnostic form to an explicit writer           |
| `fmt.write_line(write, value)`              | `null` or `io.Error`                            | Stream the value followed by LF                                    |

Interpolation at `fmt.write` and `fmt.write_line` streams as at `debug.print`.
Stored runtime interpolation still requires a builder. Formatters do not silently
serialize private state, follow getters, or load a locale. Date/time and calendar
values have the display contracts in their respective chapters.

Owners and builders are move-only; allocators outlive them. A failed append is
atomic with respect to builder contents. A failed streamed write may already have
emitted a prefix; it cannot retract external output.

## Numbers, mathematics, and randomness

`strings.to_integer<T>(text)` supports all fixed-width integer types and returns
`T` or `strings.ParseError`. Its decimal grammar accepts an optional leading `+`,
and `-` for signed targets; no whitespace, separators, trailing text, or overflow.
`strings.to_uint8` is the existing specialization. `strings.to_float<T>` supports
float32/float64 decimal and exponent notation, rounded to the nearest representable
value with ties to even; textual NaN, infinities, and overflow are parse errors.

| API                                                                         | Result                                                     | Contract                                                                         |
| --------------------------------------------------------------------------- | ---------------------------------------------------------- | -------------------------------------------------------------------------------- |
| `numbers.min(a, b)`, `.max(a, b)`                                           | Same numeric type                                          | Numeric extrema with the floating rules below; no implicit widening              |
| `numbers.clamp(value, lower, upper)`                                        | Same type or `numbers.RangeError`                          | Require ordered bounds; reject NaN inputs                                        |
| `numbers.checked_sub(a, b)`, `.checked_mul(a, b)`                           | Same integer type or `numbers.RangeError`                  | Recoverable arithmetic overflow                                                  |
| `bits.count_ones(value)`, `.leading_zeros(value)`, `.trailing_zeros(value)` | `usize`                                                    | Bit-width-defined counts; zero has width leading/trailing zeros                  |
| `bits.rotate_left(value, count)`                                            | Same unsigned type                                         | Rotate by count modulo bit width                                                 |
| `math.sqrt(value)`, `.log(value)`, `.sin(value)`, `.cos(value)`             | Same float type or `math.DomainError` or `math.RangeError` | Reject nonfinite inputs and invalid real domains; report unrepresentable results |
| `math.floor(value)`, `.ceil(value)`, `.round(value)`                        | Same float type                                            | Integral-valued floats; round ties to even; preserve nonfinite inputs            |
| `random.seeded(seed <uint64>)`                                              | `random.Generator`                                         | Inline, reproducible pseudorandom state for simulations and tests                |
| `generator.next_uint64()`                                                   | `uint64`                                                   | Advance the exclusively borrowed generator                                       |
| `generator.below(upper <uint64>)`                                           | `uint64` or `random.InvalidBound`                          | Unbiased value in `0..upper`, upper exclusive and nonzero                        |
| `random.secure_fill(buffer)`                                                | `null` or `random.EntropyError`                            | Fill an exclusive initialized byte slice from the OS cryptographic source        |

Floating extrema propagate NaN if supplied and choose negative zero for a minimum,
positive zero for a maximum. Transcendental results follow the selected target's
versioned math contract; no cross-target bit identity is implied. Numeric utilities
do not introduce overloaded operators or a target-sized default integer.

A Generator is a mutable inline value that can move between tasks; sharing its
state requires synchronization outside these calls. Its algorithm identity is
`random.Algorithm`, a static `<string>` identifying the algorithm and its version
within the foundational library. The same seed and
call sequence reproduce results under that identity. It is distinct from the
OS cryptographic source: secure_fill never falls back to a deterministic seed.
On entropy failure it clears the requested output span and reports failure.
The separation follows the purpose of APIs such as [Go's crypto/rand](https://pkg.go.dev/crypto/rand).

## Binary encodings and digests

`encoding.hex_encode_into`, `hex_decode_into`, `base64_encode_into`, and
`base64_decode_into` accept an input byte slice and an exclusive bounded output
list. Base64 additionally takes `encoding.Base64Standard` or
`encoding.Base64URL`, both padded encodings with strict alphabet/padding checks.
Hex output is lowercase; decoding accepts either case. Whitespace and malformed
padding are rejected. Success returns a byte view; encoding text can also be
viewed as UTF-8 through `strings.from_utf8`.

These operations return an output view, `encoding.InvalidData`, or
`encoding.BufferTooSmall`. They measure and validate before writing, preserve the
buffer on failure, and never overlap borrowed input with mutable output. Error
positions are one-based input byte positions. No decoder silently drops bytes.

`hash.sha256(bytes)` returns an inline `<uint8[32]>` digest. `hash.sha256_state()`
returns an inline state with `update(bytes)` and consuming `finish()`; update
returns `null` or `hash.LimitError` for an algorithm-length overflow and leaves
the state unchanged on error. Stable digests hash the specified bytes, never an
ordinary record's unspecified padding. Hashing an application value requires an
explicit byte encoding.

## JSON with bounded work and explicit storage

`json.decode<T>(text, allocator, limits)` returns `<json.Document<T>>`,
`json.ParseError`, or `memory.AllocationFailure`. Typed schemas and dynamic
`json.Value` trees share the same strict syntax, document ownership, and explicit
limits. `json.write` streams through an explicit writer and can fail after a
committed prefix. Dynamic numbers preserve their decimal source exactly;
converting them to an integer or finite float is an explicit checked operation.

The [JSON contract](json.md) defines supported schema types, dynamic accessors,
numeric conversion/encoding, exact byte/node/depth counting, and borrowed output.
It includes a dynamic-member example and a source suite covering the extension
points. No JSON operation silently constructs foreign pointers, errors, dates,
executable values, or arbitrary resource owners.

## Numeric conversions and shifts

The numeric conversion/arithmetic and bit-operation items in this section satisfy
`core.Pure` and allocate nothing. Required evaluation uses the same result and
failure rules with compile-time operands; runtime calls retain their checked
behavior. This grant does not cover entropy, I/O, or mutation of caller storage.

| API                                  | Result                    | Contract                                                                      |
| ------------------------------------ | ------------------------- | ----------------------------------------------------------------------------- |
| `numbers.convert<T>(value)`          | `<T><numbers.RangeError>` | Exact, range-checked numeric conversion                                       |
| `numbers.checked_add(a <T>, b <T>)`  | `<T><numbers.RangeError>` | Reports integer overflow as a value                                           |
| `numbers.wrapping_add(a <T>, b <T>)` | `<T>`                     | Integer addition modulo the width                                             |
| `numbers.truncate<T>(value)`         | `<T><numbers.RangeError>` | Discards a floating fractional part, then checks range                        |
| `numbers.wrapping<T>(value)`         | `<T>`                     | Explicit integer low-bit conversion, interpreted in destination signedness    |
| `bits.and(a <T>, b <T>)`            | `<T>`                     | Bitwise conjunction of two integers of the same type                         |
| `bits.or(a <T>, b <T>)`             | `<T>`                     | Bitwise inclusive disjunction of two integers of the same type               |
| `bits.xor(a <T>, b <T>)`            | `<T>`                     | Bitwise exclusive disjunction of two integers of the same type               |
| `bits.not(value <T>)`               | `<T>`                     | Complement every bit within the integer width                               |
| `bits.shl(value <T>, count <usize>)` | `<T>`                     | Shift left, discarding high bits                                              |
| `bits.shr(value <T>, count <usize>)` | `<T>`                     | Logical right shift for unsigned integers; sign-extending for signed integers |

Import `@"bits"` to use these operations. `and`, `or`, `xor`, and `not` accept
all fixed-width signed and unsigned integer types, plus `isize` and `usize`.
They preserve the operand width and signedness; signed values use their
two's-complement bit pattern. Binary operands must have the same type, with
untyped literals taking the expected operand type. These operations do not
overflow or allocate. Boolean logic continues to use `&&`, `||`, and `!`;
there are no integer bitwise operators in expression syntax.

Both shift functions panic when `count` is at least the bit width. A constant
invalid shift is a static error. Conversions and arithmetic are compiler-known
operations: when the input type's full range proves conversion cannot fail, the
result excludes `RangeError`. This permits explicit widening without a redundant
runtime failure branch. `truncate` still rejects NaN, infinities, and values whose
truncated result is out of range.

Worked projects: [Unicode text lab](../../programs/text-lab/README.md),
[typed JSON report](../../programs/json-report/README.md), and
[seeded rolls](../../programs/seeded-rolls/README.md).
