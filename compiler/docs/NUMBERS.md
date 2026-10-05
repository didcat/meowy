# Numeric names and literal syntax

The bootstrap implements [numeric value names](../../docs/reference/syntax.md#numeric-names-and-intrinsic-literals)
and the [`core.literal` intrinsic](../../docs/reference/stdlib/core.md#intrinsic-numeric-literals).
Undotted numeric spellings can name bindings, parameters, functions and fields.
This includes integers and exponents such as `1e2`. Ordinary undotted numeric
expressions first look for that exact value name; an unbound spelling retains its
normal literal behavior. Dotted declaration names are syntax errors (`E004`).

```meowy
1 : @"core".literal(1)
2 : @"core".literal(2)
3 : @"core".literal(3)
```

These declarations create ordinary `int32` bindings. An annotation can choose a
different supported type at declaration; later reads retain that fixed type.
`1`, `01`, `0x1` and `1e0` remain independent spellings. Signs are operators.
Type names and scope labels continue to use identifier syntax.

## Numeric members

```meowy
10 : {
    -> 4 : {
        -> "4"
    }
}

@"debug".print(10.4)
@"debug".print(11.4)
@"debug".print(@"core".literal(10.4))
```

These print `4`, `11.4` and `10.4`. When `10` is bound, `10.4` selects its field
`4`. A missing field, private field or scalar receiver is an ordinary member
error; lookup never retries the decimal interpretation. Only an unbound root
permits decimal fallback. `core.literal` always requests the intrinsic number.

`10.name`, `10.4.5` and `row.1.0` use ordinary member paths; the last expression
selects field `1`, then field `0`. Parentheses remain ordinary grouping, so
`(10).4` is also valid. Named fields still require `->` to be visible externally.

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
file exports preserve exact undotted names. Checked documentation supports
`[[1e0]]` and `[[10.4]]`; member links require a declaration and never resolve to
a decimal literal. Vim/Neovim highlight undotted numeric declarations and calls.
Other occurrences keep lexical number colors: `10.4` stays Float even when scope
resolution makes it a field access.

The AST retains spelling plus an intrinsic-literal marker. The checker adapts
bound numeric leaves to ordinary name reads and bound decimal roots to field
accesses; explicit literal operands bypass that adaptation across required
evaluation and isolated list probes. No numeric lookup or literal-intrinsic call
is added to the generated runtime. Optional AST
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
