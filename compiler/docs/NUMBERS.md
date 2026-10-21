# Numeric names and literal syntax

The bootstrap implements [numeric value names](../../docs/reference/syntax.md#numeric-names-and-intrinsic-literals)
and the [`core.literal` intrinsic](../../docs/reference/stdlib/core.md#intrinsic-numeric-literals).
Names can start with digits in bindings, parameters, functions, fields, types and
scope labels. This includes words such as `2_or_more_cats` and complete undotted
numeric spellings such as `10`, `0x1` and `1e+2`. Ordinary undotted numeric
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
Types, values and scope labels keep separate namespaces with the same name syntax:

```meowy
<2Count> : <uint32>
2_or_more_cats <2Count> : 3
'2_scope { '2_scope.leave() }
```

Words such as `1__0`, `0x_ff` and `12cat` are names. An unbound value name reports
E201; `core.literal(1__0)` reports E207 because that spelling is not a numeric token.
`2name+4` uses ordinary addition, while the complete exponent `1e+2` is one token.

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
In type positions `<10.4>` and `<10.1e+2>` select exported types from the module
named `10`; there is no decimal fallback. Type-alias declarations require one name.

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
The numeric escape accepts one numeric token, optionally directly negated. Expressions,
groups and other name operands are `E207`; wrong arity is `E212`. Original lexical,
range and operator diagnostics remain in force. Callable lookup is ordinary:
aliases preserve the intrinsic, and unrelated values named `literal` do not.

## Boolean escape

The same intrinsic accepts the exact spellings `true` and `false`, even when local
bindings shadow those names:

```meowy
literal : @"core".literal
true : "shadowed"
false : 7

enabled <boolean> : literal(true)
disabled <boolean> : literal(false)
```

The operands are syntax and are not read as variables. Required evaluation and contextual
list inference retain the boolean type and ordinary literal work. `literal(!false)`,
`literal(-true)` and `literal((true))` are `E207`; unrelated functions named `literal`
continue to evaluate ordinary arguments. See the
[boolean contract](../../docs/reference/stdlib/core.md#intrinsic-boolean-literals) and
[required source case](../../docs/conformance/sources/boolean_literal_values.mwy).

## Null escape and complete operands

`literal(null)` constructs intrinsic null even when a local variable or parameter named
`null` has another type. Immutable intrinsic aliases preserve this behavior:

```meowy
literal : @"core".literal
null := 7

absent <int32><null> : literal(null)
<T> : literal(null)<>
value <T> : literal(null)
```

Type queries and contextual lists preserve the null type; ordinary union/type rules apply.
`literal((null))`, `literal(-null)` and `literal("null")` are `E207`. Null cannot serve as
an integer list capacity. Broader required scalar-binding limits remain unchanged.

The [complete operand contract](../../docs/reference/stdlib/core.md#literal-operand-contract)
is numeric tokens, optionally directly negated, plus the exact names `true`, `false` and
`null`. Strings, lists, blocks and type expressions already have distinct syntax and are
outside this lookup escape. `<never>` has no values. The
[required null source case](../../docs/conformance/sources/null_literal_values.mwy) covers
shadowing, aliases, queries, nullable values, lists and ordinary callable identity.

## Integration

Numeric storage uses ordinary mutation, borrowing, narrowing and scope rules.
Required expressions, list capacities and list inference retain binding types and
input eligibility. Escaped numbers receive ordinary literal context. Fields and
file exports preserve exact undotted names. Checked documentation supports
`[[2cats]]`, `[[1e0]]`, `[[10.4]]` and `[[<10.4>]]`; value and type links keep their
namespaces, require declarations and never resolve to decimal literals. Vim/Neovim
highlight digit-leading declarations, calls, types and labels.
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
Required cases cover [digit-leading words](../../docs/conformance/sources/digit_leading_names.mwy),
[types and labels](../../docs/conformance/sources/digit_leading_namespaces.mwy),
[checked links](../../docs/conformance/sources/digit_leading_doc_links.mwy) and
[numeric module namespaces](../../docs/conformance/sources/modules/digit_leading_names/main.mwy).
[STATUS](../STATUS.md#numeric-shadowing-implementation) records final validation.
