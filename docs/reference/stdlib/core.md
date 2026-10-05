# Core values and diagnostics

[Library index](README.md)

## Predefined values

`@"core"` exposes the constants `true`, `false`, and `null`, along with the
fundamental type values listed in the type reference. These bindings are also
available through the program's enclosing prelude scope. Source declarations
can shadow them normally; `core.true` and `<core.boolean>`, for example, still
refer to their intrinsic values when local names differ.

An intrinsic's behavior follows its resolved identity, including through aliases.
No API name is a keyword, and defining a new binding with an intrinsic's spelling
does not give it that intrinsic's compiler privileges or representation.

`core.Type` is the compile-time-only type of type values. Public type-producing
helpers use it in ordinary parameter/result annotations; see
[compile-time evaluation](../compile-time.md) for admissible effects and limits.
`core.Call<S>`, `core.CallMut<S>`, and `core.CallOnce<S>` constrain shared,
exclusive, and consuming calls of a concrete environment with signature `S`.
`core.Pure` requires a callable with no external effects. These are structural,
compiler-checked capabilities, not keywords, runtime interfaces, or opt-in flags.
See [callable environments](../types.md#callable-environments) for source examples.

## Intrinsic numeric literals

`core.literal(number)` constructs the intrinsic number even when its spelling
has a local binding. Core exposes this intrinsic instead of an enumerated set of
numeric members such as `core.1`. Ordinary unshadowed numeric tokens already
construct their intrinsic values; no prelude declarations are needed.

```meowy
core : @"core"
literal : core.literal

1 : literal(2)
original <uint8> : literal(1)
minimum <int8> : literal(-128)
hex <uint16> : literal(0xff)
```

The single argument must be a valid integer or floating-point token, optionally
preceded directly by unary `-`; intervening whitespace and comments are allowed.
The operand is syntax: it is neither looked up nor evaluated as an expression.
`literal(name)`, `literal(1 + 2)` and `literal((1))` are invalid (`E207`). Missing
or extra arguments are `E212`. Malformed numeric tokens remain lexical errors.

The result follows ordinary literal defaults, contextual widths, range checks
and floating-point rounding. A value outside its expected range is `E216`.
`literal(-128)` can construct `int8`; `-literal(128)` must first construct its
positive operand and does not extend the signed-minimum literal rule.
The intrinsic is pure and available in required evaluation and list capacities.
It has no runtime call or allocation. It is not a conversion or string parser.

Call the intrinsic directly, including through an immutable alias. Its syntax
operand is not a runtime function parameter or dispatch receiver. Ordinary
lexical resolution selects the callable: shadowing `core` or an alias affects
that lookup, and an unrelated function named `literal` keeps ordinary arguments.
Numeric bindings created from its result have fixed types like other bindings.
See [numeric names](../syntax.md#numeric-names-and-intrinsic-literals).

## Output and text

| API                               | Result                        | Contract                                                                |
| --------------------------------- | ----------------------------- | ----------------------------------------------------------------------- |
| `debug.print(value)`              | `<null>`                      | Prints a formattable value and a newline; output failure panics         |
| `debug.panic(value)`              | `<never>`                     | Raises a recoverable panic with formatted diagnostic context            |
| `strings.to_uint8(text <string>)` | `<uint8><strings.ParseError>` | Parses decimal digits with an optional leading `+`, range 0 through 255 |
| `string.size()`                   | `<usize>`                     | UTF-8 byte length                                                       |
| `string.bytes()`                  | `<uint8[]>`                   | Borrows the string's bytes                                              |

The parser rejects empty input, whitespace, separators, signs other than a single
leading `+`, fractional numbers, trailing characters, and out-of-range values.
`strings.ParseError` is an allocation-free concrete `<error>` with a static code
and message; formatting can add the original input without storing an owned copy.
It is descriptor-compatible and may be passed as `<error>` without boxing.
`strings.to_uint8` and the immutable `string.size`/`string.bytes` accessors are
`core.Pure`; they may be used in pure validators and required evaluation with
compile-time inputs. Parsing failure remains a value during evaluation.
The [`errors` module](errors.md) defines how to read error codes and messages,
construct custom failures, and inspect typed payloads while preserving ownership.
These values remain application results; creating or printing one does not
publish a compiler diagnostic or a saved replay session.

`debug.print` borrows its argument and supports primitives, errors, library values
with a documented display, and aggregates whose fields are formattable. An aggregate's default display uses its primary
value. Runtime string interpolation at this call streams directly to output.
Production I/O that needs recoverable write errors uses a supplied writer.

[`testing`](testing.md) provides assertions using `P005`, borrowed equality and
byte/text checks, and suite descriptors for `meowy test`. Assertions are ordinary
calls and may also be used outside a suite; they do not establish caller-side
type narrowing. `testing.fail` has result `<never>` for explicit failure branches.

See [text and data](text-and-data.md) for owned strings, Unicode, numeric parsing,
and `fmt` writer functions with recoverable output errors.
