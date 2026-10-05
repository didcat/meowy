# Numeric names and literal syntax

The bootstrap implements [numeric value names](../../docs/reference/syntax.md#numeric-names-and-intrinsic-literals)
and the [`core.literal` intrinsic](../../docs/reference/stdlib/core.md#intrinsic-numeric-literals).
Integer and floating-point spellings can name bindings, parameters, functions and
fields. Ordinary numeric expressions first look for that exact value name; an
unbound spelling retains its normal literal behavior.

```meowy
1 : @"core".literal(1)
2 : @"core".literal(2)
3 : @"core".literal(3)
```

These declarations create ordinary `int32` bindings. An annotation can choose a
different supported type at declaration; later reads retain that fixed type.
`1`, `01`, `0x1`, `1.0` and `1e0` remain independent spellings. Signs are operators.
Type names and scope labels continue to use identifier syntax.

## Shadowing and escape

```meowy
literal : @"core".literal

{
    1 : literal(2)
    doubled : 1 + 1
    original <uint8> : literal(1)
    minimum <int8> : literal(-128)
}
```

`doubled` is `4`; `original` is the intrinsic `1` with a contextual `uint8` type.
The escape accepts one numeric token, optionally directly negated. Expressions,
groups and identifier operands are `E207`; wrong arity is `E212`. Original lexical,
range and operator diagnostics remain in force. Callable lookup is ordinary:
aliases preserve the intrinsic, and unrelated values named `literal` do not.

## Integration

Numeric storage uses ordinary mutation, borrowing, narrowing and scope rules.
Required expressions, list capacities and list inference retain binding types and
input eligibility. Escaped numbers receive ordinary literal context. Fields and
file exports preserve exact names; `row.1.0` selects the single `1.0` field.
Use `(1).field` for a numeric receiver because malformed-number token rules remain.
Checked documentation supports `[[1.0]]` and `[[(1).field]]`. Vim/Neovim highlight
numeric declarations and calls, while other occurrences keep lexical number colors.

The AST retains spelling plus an intrinsic-literal marker. The checker adapts
bound numeric leaves to ordinary name reads; explicit literal operands bypass
that adaptation across required evaluation and isolated list probes. No numeric
lookup or literal-intrinsic call is added to the generated runtime. Optional AST
adapters are boxed to preserve the existing deep-probe stack bound.

Existing bootstrap restrictions on captures, callable storage, required
floating-point evaluation and proof outcomes still apply. This feature does not
qualify the complete language or release.

## Evidence

[Native tests](../tests/native/numbers.rs) exercise both profiles.
Required conformance sources cover [lookup and escape](../../docs/conformance/sources/numeric_names.mwy),
[storage](../../docs/conformance/sources/numeric_storage.mwy),
[required values](../../docs/conformance/sources/numeric_required.mwy),
[checked links](../../docs/conformance/sources/numeric_doc_links.mwy) and
[file facades](../../docs/conformance/sources/modules/numeric_exports/main.mwy).
The catalog also pins duplicate, type, write, borrow and intrinsic-argument errors.
[STATUS](../STATUS.md#numeric-shadowing-implementation) records final validation.
