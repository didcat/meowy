# Syntax

[Documentation index](../README.md)

Source files use UTF-8 and the `.mwy` extension. Names are case-sensitive. The
portable identifier set is ASCII letters, digits, and `_`, with a letter or `_`
first. This revision accepts exactly that identifier set; non-ASCII text remains
valid in strings and comments. Type aliases conventionally start with an uppercase letter.

Outside strings/comments, whitespace is ASCII space, tab, LF, or CRLF. CRLF is
one statement-ending newline; a bare CR, a leading UTF-8 BOM, and other Unicode
spacing characters are invalid tokens (`E001`). Source offsets still count the
original bytes. Inside strings, CRLF is retained as two bytes. Comments act as
token separators; newlines inside a comment do not terminate a statement.

## No keywords

No identifier spelling is reserved by the grammar. Punctuation introduces
bindings, functions, blocks, types, matchers, and control targets. Words resolve
to values in a scope, including the well-known names supplied by the language.

`true`, `false`, and `null` are predefined constant values. Primitive types such
as `<boolean>` and `<uint8>` are predefined values in the type namespace. These
names follow ordinary name resolution and can be shadowed in an inner scope;
shadowing cannot change their intrinsic identity, representation, or implicit
language behavior. `$` is punctuation denoting the nearest dispatch receiver,
not an identifier. `self` is an ordinary name with no implicit binding.

The foundational module `@"core"` exposes the predefined constants and types when
a local name shadows one of them. A missing primary emission, for example, still
produces the intrinsic null value regardless of a local binding named `null`.

```meowy
core : @"core"

{
    true : "an ordinary local name"
    enabled <core.boolean> : core.true
}
```

`leave`, `restart`, `Copy`, `Send`, and library operation names are not keywords.
Their behavior belongs to the value found through lookup. Aliasing an intrinsic
preserves its rules; giving an unrelated value the same name grants no special
behavior. For example, a scoped control operation can be called through an alias:

```meowy
'work {
    finish : 'work.leave
    finish()
}
```

The alias must stay inside its target's function, task, and lexical lifetime.
An ordinary record field named `leave` has no scope-control behavior unless it
actually contains that intrinsic value.

## Literals, values, and comments

| Form                             | Meaning                                        |
| -------------------------------- | ---------------------------------------------- |
| `null`, `true`, `false`          | Predefined null and boolean values             |
| `42`, `1_024`, `0xff`, `0b1010`  | Integer literals                               |
| `3.5`, `1.0e-3`                  | Floating-point literals                        |
| `"hello"`                        | UTF-8 string literal                           |
| `"value: {expression}"`          | Interpolated string                            |
| `[1, 2, 3]`                      | Bounded list literal                           |
| `{ -> x : 1 }`                   | Block with a named emission                    |
| `# comment #`                    | Delimited comment; may span lines              |
| `#\| documentation \|#`          | Documentation for the following declaration    |
| `#!\| module documentation \|!#` | Documentation for the containing source module |

Ordinary comments do not nest. Documentation fences use matching bar counts and
their own closers; see [checked documentation comments](documentation.md) for
quoting, attachment, semantic links and implementation status. The openers `#|`
and `#!|` take precedence over an ordinary comment; insert a space after `#` when
that documentation opener was not intended. `##` remains an empty ordinary comment.

`#` inside a string is ordinary text. Strings accept `\n`,
`\r`, `\t`, `\0`, `\"`, `\\`, `\{`, and `\}` escapes. A raw newline is allowed
inside a string and is retained. There is no implicit indentation stripping.
String literals contain UTF-8 bytes and need not be NUL-terminated.

Integer literals are checked against their expected type; unconstrained integers
default to `<int32>` and unconstrained decimals to `<float64>`. A default that
cannot represent a literal is a diagnostic, not an automatic promotion. Negative
numbers use unary `-`.

Digit separators are single `_` characters between two digits of the literal's
base. Decimal leading zeroes are decimal, not octal. Hexadecimal prefixes are
`0x`/`0X`, binary prefixes `0b`/`0B`; each needs a digit. Decimal floats have
digits on both sides of `.` and/or an `e`/`E` exponent with optional sign and at
least one decimal digit. `1.`, `.5`, `1__0`, and `0x_ff` are not numeric literals;
there are no numeric suffixes. A digit-starting malformed number such as `12cat`
is one invalid token, not a number followed by a name. `1.(f)` is instead an
integer followed by dispatch. Float literals round once to the expected binary
precision, ties to even; overflow to infinity is `E216`, and underflow to a
subnormal or signed zero is allowed.

Unary `-` immediately applied to an integer literal, with only whitespace or
comments between them, checks the **negated mathematical value** against the
expected signed type. Thus `x <int8> : -128` is valid. `128` alone as `int8` and
`x <int8> : -(128)` are `E216`; parentheses end this literal rule. Negating an
already typed signed minimum is checked arithmetic (`E107` or runtime panic).
Unsigned negation is not defined, including a negated literal with an unsigned
expected type. A leading sign remains an operator, never part of the token.

## Statements and delimiters

A newline terminates a complete statement. `;` explicitly separates statements,
including multiple statements on one line. Newlines inside `(...)`, `[...]`, and
type expressions do not terminate an incomplete expression. A line beginning
with a dispatch `.` continues the preceding expression. Place a binary operator
at the end of a line to continue that expression on the next line.

Braces contain a sequence of statements. An expression statement evaluates and
discards its result; it does not implicitly emit that result.

No grammar production requires a space or tab. Indentation is presentation, and
spaces never select between a type test and an ascription. Use `;` when removing
a statement-ending newline. Record-type fields and type expansions can likewise
be separated with `;`, so a record type can be written `<{x<int32>;y<int32>}>`.
An optional final separator is permitted before the closing brace.

This is a complete program with no whitespace outside its string:

```meowy
debug:@"debug";value<int32><null>:7;|value<int32>|debug.print("{value}");
```

Compact source still has tokens. Identifiers and numbers cannot be joined into
different tokens, and multi-character operators such as `&&`, `->`, and `>>`
must stay intact. Use punctuation or a newline to separate tokens when needed;
removing whitespace with a text substitution is not a minifier. Comments and
literal contents retain their own bytes regardless of the surrounding layout.

## Forms at a glance

| Form                                          | Meaning                                                          |
| --------------------------------------------- | ---------------------------------------------------------------- |
| `name : value`                                | Immutable binding                                                |
| `name := value`                               | Mutable binding                                                  |
| `name <T> : value`                            | Explicit binding type                                            |
| `name = value`                                | Reassign a mutable binding                                       |
| `<Name> : <T>`                                | Type alias                                                       |
| `-> value`                                    | Primary emission                                                 |
| `-> name : value`                             | Immutable named emission                                         |
| `-> name := value`                            | Mutable named emission                                           |
| `<- expression`                               | Register an expression for cleanup of the current scope          |
| `(x <T>) { ... }`                             | Function value                                                   |
| `f <R> : (x <T>) { ... }`                     | Function declaration with result type `R`                        |
| `f <(T) -> R>;`                               | Forward signature, completed by the following definition group   |
| `f(value)`                                    | Function call                                                    |
| `value.name`                                  | Field selection                                                  |
| `value.&name`, `value.&!name`                 | Shared or exclusive borrow of the selected field                 |
| `value.*name`                                 | Dereference the selected field                                   |
| `value.(f)`                                   | Call `f` with `value` as its first argument                      |
| `value.{ ... }`                               | Evaluate block with `$` bound to `value`                      |
| `\| condition \| statement`                   | Conditional matcher arm                                          |
| `'scope { ... }`                              | Named, immediately evaluated block                               |
| `'scope -> value`                             | Primary emission into a named enclosing block                    |
| `'scope <- expression`                        | Register cleanup in a named enclosing block                      |
| `'scope.leave()`                              | Finish that named block                                          |
| `'scope.restart()`                            | Clean up and restart that named block                            |
| `\| value <T> \| statement`                   | Type predicate in a matcher condition                            |
| `value<>`                                     | Compile-time type query                                          |
| `value<T>`                                    | Boolean type predicate in any expression                         |
| `value~<T>`                                   | Proven type ascription; no conversion                             |
| `name <(expression)> : value`                 | Binding annotated by a computed type                             |
| `@"name"`                                     | Module import                                                    |
| `&value`, `&!value`                           | Shared or exclusive borrow                                       |
| `&(value[index])`, `&!(value[index])`         | Shared or exclusive borrow of the selected element               |
| `*reference`                                  | Access a safe reference's referent                               |
| `>> expression`, `<< task`                    | Start or join a task                                             |
| `%group<T[N]>`                                | Declare a bounded task group                                     |
| `%group >> expression`                        | Submit a task to a group                                         |
| `<< %group`                                  | Seal and join a group, consuming its result storage               |
| `!{ ... }`                                    | Block permitting operations with caller-proven safety conditions |
| `(x <T>) !{ ... }`                            | Function whose callers must establish those conditions           |
| `<:T : memory.Copy>`                          | Generic type binder constrained by a capability value            |
| `<D<:K, :V, :Y, :Z>> : <{ ... }>`             | Generic type alias with four independent type parameters         |
| `f<:K, :V><V> : (key <K>, value <V>) { ... }` | Generic function with an explicit result type                    |

The escaped pipes in the table stand for literal `|` characters. Spaces around
angle brackets do not change their role: `value<T>` and `value <T>` have the same
meaning everywhere. `~` explicitly introduces an ascription; see the rules below.

`<T><U>` is a union in a type position, and `!<U>` subtracts members from a type.
Generic arguments name types without an extra pair of angle brackets:
`<task<int32>>` or `<D<string, uint32, boolean, string>>`. Use a type alias for a
union inside a generic argument. Declaration lists introduce each type binder
with `:`, as in `<:K, :V>`; argument lists omit those markers, as in `<K, V>`.
Parameters are positional, and an explicit list supplies every argument.
See [multiple type parameters](types.md#multiple-type-parameters) for complete
type declarations, functions, inference, and constraints.

A function's leading binder list is separate from its result annotations.
`f<:K, :V><V>` declares two parameters and result `<V>`; the binders are not union
alternatives. The existing `f<:T>` shorthand declares `T` and result `<T>` when
no separate result is written. `f<:T><T><null>` explicitly returns their union.
These forms retain the same meaning without spaces.

## Angle brackets in context

Declarations establish an annotation position: `name <T> : value` annotates the
binding. After an expression, these forms keep their meaning in every expression
position, independently of spacing:

1. Empty `<>` is a type query.
2. Type arguments followed by call parentheses specialize that call:
   `accepts<T>(value)`.
3. A nonempty type suffix is a boolean type predicate, with comparison precedence.
   `value<T><U>` tests membership in the union `<T><U>`.
4. `~<T>` is a proven ascription, with postfix precedence. `copy : value~<T>`
   requests no conversion or runtime check; the current flow type must already
   satisfy `T` (`E208` otherwise).

An ascription consumes exactly one bracketed type. A following `<U>` is a predicate
on the ascribed value: `value~<T><U>`. To ascribe a union, name the union first:

```meowy
<Choice> : <A><B>
copy : value~<Choice>
```

Generic type arguments remain inside the target type: `obj~<D<S, K, T, V>>`.
`obj~<S, K, T, V>` does not name one target type and is invalid. Generic function
calls retain `f<S, K, T, V>(args)`; `value<>` retains its type-query meaning.

| Form                                   | Interpretation                                  |
| -------------------------------------- | ----------------------------------------------- |
| `\| value <T> \| use(value)`           | Test `value` and refine it in the arm            |
| `matches : value<T>`                    | Store the boolean test result                   |
| `accepts(value<T>)`                     | Pass the boolean test result                    |
| `accepts(value~<T>)`                    | Pass the proven ascribed value                  |
| `\| accepts<T>(value) \| use(value)`   | Call a specialized boolean function             |
| `copy : value ~ <T>`                    | Ascribe, with optional spaces                   |

A nullable boolean makes the distinction useful:

```meowy
debug : @"debug"

show <null> : (enabled <boolean><null>) {
    | enabled<boolean> && enabled~<boolean> | {
        debug.print("Enabled")
    }
}
```

The first operand establishes that `enabled` is boolean. Short-circuiting carries
that proof into the second operand, which reads its boolean value. Both `null`
and `false` skip the body. A stored boolean test result does not itself carry a
narrowing proof into a later matcher; use the predicate where the proof is needed.
Because unary operators bind more tightly than predicates, write `!(value<T>)`
to negate a test; `!value<T>` tests the result of `!value`.

A complete type form wins over a relational interpretation, without consulting
whether a name resolves to a type. `age < 18` is a comparison: it has no closing
type delimiter. `left < limit && other > 0` is two comparisons joined by `&&`; the
intervening operator cannot belong to the putative type form. Chained relational
comparisons are invalid; parentheses must express the intended grouping.

`<(expression)>` evaluates a compile-time expression that produces a type. It
allows computed annotations such as `other <(name<>)> : value` without using a space
to separate two identifiers. A function type instead contains an arrow after its
parameter list: `<(T) -> R>`. Bare computed annotations such as `other name<> : value`
are not part of the grammar. See [type queries](types.md#type-queries).

A computed type atom accepts ordinary extent suffixes before the final type
delimiter: `<(element)[capacity]>` and `<(element)[]>`. The expression must
produce `core.Type`; see [compile-time evaluation](compile-time.md). This does
not change the meaning of a type suffix after a runtime expression.

## Operators and evaluation order

From highest to lowest precedence:

| Level | Operators/forms                                                                      |
| ----- | ------------------------------------------------------------------------------------ |
| 1     | Prefix borrow `&`, `&!` and dereference `*`                                          |
| 2     | Calls, field selection/borrow/dereference, indexing, dispatch, type query/ascription |
| 3     | Unary `!`, `-`, task start `>>`, join `<<`                                      |
| 4     | `*`, `/`, `%`                                                                        |
| 5     | `+`, `-`                                                                             |
| 6     | `<`, `<=`, `>`, `>=`, type predicates                                                |
| 7     | `==`, `!=`                                                                           |
| 8     | `&&`                                                                                 |
| 9     | `\|\|`                                                                               |

`%name` is a task-group primary expression, not a borrow or a general unary
operator. Its following calls and field selections use the usual postfix rules.
Binary `%` remains the remainder operator: `a % b`. The parser distinguishes these
forms by operand position, not whitespace.

Binary arithmetic operators associate left-to-right; comparisons cannot be
chained. Assignment, emissions, and matchers are statement forms. Parentheses
override precedence. Integer bit operations belong to `@"bits"`: use `bits.and`,
`bits.or`, `bits.xor`, and `bits.not`. There are no binary `&`, `|`, `^` or unary
`~` bitwise operators. `>>` and `<<` are never bit shifts; use `bits.shl` and
`bits.shr`. Shared borrowing still uses prefix `&`; capability conjunction still
uses `&` in type binders. Boolean `&&`, `||`, and `!` are unchanged.

`<- expression` is also a statement form, not a binary operator or a binding.
`<-` is one token. Its operand is required on the same statement and can be a
call or a block: `<- release()` and `<- { release() }` both delay evaluation
until scope exit. It has no value and cannot appear as a call argument or
initializer. See [deferred actions](values-and-blocks.md#deferred-actions).
The labeled statement `'scope <- expression` selects an enclosing cleanup target,
symmetrically with `'scope -> value`; the expression's names still resolve at
the registration site, and all delayed accesses must survive until target exit.

Prefix borrowing and dereferencing consume the following prefix operators and
one primary expression, before any unparenthesized postfix forms. A parenthesized
expression is one primary and may contain a complete selection, call or other
expression. Prefix operators nest right-to-left. Consequently, `&*p.field`
means `(&(*p)).field`, and `*p.field` means `(*p).field`. Other unary operators
retain their usual precedence: `-value.field` means `-(value.field)`.

| Expression            | Grouping and meaning                                                |
| --------------------- | ------------------------------------------------------------------- |
| `&object.field`       | `(&object).field`: borrow the object, then select its field         |
| `object.&field`       | `&(object.field)`: borrow the selected field                        |
| `object.&!field`      | `&!(object.field)`: exclusively borrow the selected field           |
| `object.inner.&field` | `&(object.inner.field)`: borrow the final field                     |
| `object.&inner.field` | `(&(object.inner)).field`: borrow `inner`, then select `field`      |
| `&items[i]`           | `(&items)[i]`: borrow the list, then index it                       |
| `&(items[i])`         | Select and borrow the element                                       |
| `object.&items[i]`    | `(&(object.items))[i]`: borrow `items`, then index it               |
| `&f()`                | `(&f)()`: borrow the callable, then call through that reference     |
| `&(f())`              | Call `f`, then borrow its result                                    |
| `&*p`                 | `&(*p)`: reborrow the referent                                      |
| `*object.field`       | `(*object).field`: dereference the object, then select its field    |
| `object.*field`       | `*(object.field)`: dereference the selected field                   |
| `object.*inner.field` | `(*(object.inner)).field`: dereference `inner`, then select `field` |
| `*items[i]`           | `(*items)[i]`: dereference the list reference, then index it        |
| `*(items[i])`         | Select the element, then dereference it                             |
| `*f()`                | `(*f)()`: dereference the callable reference, then call it          |
| `*(f())`              | Call `f`, then dereference its result                               |

The exclusive prefix `&!` follows the same grouping rules. A dotted `&`, `&!` or
`*` must be followed by a field name and applies only to that immediately selected
field. Later postfix forms operate on the result. Use grouping for an indexed
place or a call result; `object.&[i]`, `object.*[i]`, `object.&(f)` and
`object.*(f)` are not forms.
Grouping establishes the operation, not its availability: callable references,
reference nesting, mutability, lifetimes and compiler capability limits still
apply. Field access or indexing through a reference may copy a copyable value;
it does not itself request another borrow.

`&!` is the exclusive-borrow operator; `&(!value)` instead borrows a negated
boolean. In type expressions, `<&!T>` is an exclusive reference and `<*!T>` is a
writable raw pointer. A `!` followed by a block opener marks an unchecked block,
with or without intervening whitespace. To negate a block's boolean primary,
write `!({ ... })`. These forms have distinct punctuation, not contextual words.

Operands and call arguments evaluate left-to-right. `&&` and `||` short-circuit.
Dispatch evaluates its receiver once. Task start captures its inputs at submission
and evaluates its operand in the child task; the concurrency reference specifies
that boundary.

## Names and scopes

Bindings are lexically scoped and visible after their declaration. Function
declarations can refer to themselves; mutual recursion requires explicit function
type declarations. Shadowing is allowed in a nested block, not by redeclaring a
name in the same block. `$` cannot be declared or rebound as a user name. A nested
dispatch supplies its own receiver; ordinary nested blocks use the enclosing `$`.
Outside a dispatch receiver's lexical scope, `$` is invalid.

Types, labels, values, and task groups use distinct namespaces. `%name` selects a
task group; `&name` borrows an ordinary value. A group and a value may therefore
share a name without changing either lookup. Label operations can only target a
lexically enclosing scope in the current function and task.

The names of intrinsic values never introduce extra parser productions. Function
constraints use punctuation in their type binders; returned borrows follow the
[lifetime rules](memory.md#lifetimes); native layout is selected by a normal
compile-time call to `ffi.record`. `!{ ... }` marks a boundary for operations
requiring a safety proof; it does not disable type checking.

### Forward function groups

An annotation-only statement `name <(Parameters) -> Result>;` reserves an immutable,
non-capturing function in the current value scope. It is recognized at statement
start when an identifier and a complete function-type annotation reach a statement
terminator without a binding operator. It is not a type-predicate expression statement;
parenthesize such an expression to discard a predicate result instead. An explicit
`f~<(T) -> R>` ascribes a function value.

One or more consecutive forward signatures must be followed immediately by exactly
one function definition for each reserved name, in any order. Comments and blank
lines may intervene; no other statement may interrupt this group. Each definition
must have the exact parameter/result types and safety requirement promised by its
signature. The whole group is initialized together before subsequent statements.
Bodies see all signatures in the group; no partially initialized pointer can be
called or escape. A definition may use `->` to export its completed function.
This fulfills the reservation rather than redeclaring the name.

```meowy
even <(uint32) -> boolean>;
odd <(uint32) -> boolean>;
even <boolean> : (n <uint32>) 'answer {
    | n == 0 | { 'answer -> true; 'answer.leave() }
    -> odd(n - 1)
}
odd <boolean> : (n <uint32>) 'answer {
    | n == 0 | { 'answer -> false; 'answer.leave() }
    -> even(n - 1)
}
```

The first revision permits concrete, non-capturing groups only. Generic or
capturing forward declarations, unfinished groups, and mismatched definitions
use `E221`. Ordinary single-function recursion still uses its own declared
signature without a forward group. Nonfunction bindings require initializers;
this syntax does not introduce general uninitialized storage.
