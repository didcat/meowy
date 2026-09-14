# Define and inspect custom errors

[All programs](../README.md) · [Manifest](mod.mwy) · [Entry](main.mwy) · [Validation](validation.mwy)

A signup policy returns an age or one of two concrete errors. Its caller prints
common metadata, reads a typed payload, and inspects a preserved parser failure.
The public result is the closed union `<uint8><TooYoung><InvalidAge>`.

From the repository root:

```sh
cd docs/programs/custom-errors
meowy check
meowy run
```

Expected output:

```text
Input: 24
Accepted age: 24
Input: 16
signup.too_young: Age must be at least 18
Age 16; minimum 18
Input: twenty
signup.invalid_age: Age must be a decimal integer from 0 to 255
Could not parse: twenty
Cause: strings.ParseError (the decimal parser rejected the input)
```

| Operation                                           | What this program demonstrates                                         |
| --------------------------------------------------- | ---------------------------------------------------------------------- |
| `errors.define<AgeRequirement>(...)`                | Defines a distinct error type with a static code and message           |
| `too_young.make(...)`                               | Constructs that type with the supplied age and minimum stored inline   |
| `age <error>` in a matcher                          | Selects both failure alternatives without erasing their concrete types |
| `errors.code(&failure)`, `errors.message(&failure)` | Reads metadata through a shared borrow of the closed error union       |
| `failure.payload()`                                 | Borrows the `TooYoung` payload so its fields can be inspected          |
| `failure.take_payload()`                            | Consumes the `InvalidAge` error and yields its payload                 |
| `*cause <strings.ParseError>` in a matcher          | Inspects the concrete parser failure retained in that payload          |

`AgeRequirement` and `InvalidInput` are ordinary records. They do not match
`<error>` by themselves: `errors.define` supplies the nominal identity, and its
`make` constructor creates the error value. The module exports aliases for the
generated types while keeping its two definition values private.

`InvalidInput.cause` is an ordinary typed field, not an implicit exception chain.
The wrapper preserves the actual `strings.ParseError`; it does not replace it with
formatted text. Callers can also read its metadata with `errors.code(cause)` and
`errors.message(cause)` while that borrow is alive. The demo prints a fixed
explanation after matching its type, so the transcript does not depend on the
parser's diagnostic wording.

No error construction here allocates. The payload records and result union are
inline, and the input strings borrow static literals. A returned `InvalidAge`
borrows its input text, so an error produced from a caller-owned text buffer must
not outlive that buffer. Reading a payload borrows its error; taking a payload
ends that error value and transfers the fields instead. Neither operation
extends the lifetime of borrowed text.

Rejections are expected values, so all three calls complete successfully. Each
failure path emits to its named scope and leaves it; emitting alone would not
stop execution. `debug.print` streams the output and panics on output failure.

See the [errors package](../../reference/stdlib/errors.md) for definition rules,
ownership, metadata, and explicit erasure, and [union narrowing](../../reference/types.md#unions-and-narrowing)
for the distinction between a predicate and a conversion.
