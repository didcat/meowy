# Function obligations

Status: exploratory proposal for meowy. This is not part of the language contract
and does not describe implemented compiler behavior. The syntax below is proposed.

## The idea

A function can state a requirement that every caller must prove before the call is
accepted. The proposed spelling is `-| predicate |`, called an **obligation**.

```meowy
div : (n <uint128>, by <uint128>) {
    -> n / by
} -| by > 0 |
```

The caller must establish that `by > 0`. The function body can rely on that fact at
entry, regardless of which caller invokes it.

This puts the requirement alongside the operation that needs it. Callers do not
have to remember an informal rule or repeat a check that their existing context
already establishes.

## What must be proven

Acceptance requires the predicate to be **proven true**. Merely knowing whether it
is true or false is insufficient: a concretely false obligation is still violated.

Assume `input()` returns an arbitrary `uint128`:

| Call               | Evidence                   | Proposed result                |
| ------------------ | -------------------------- | ------------------------------ |
| `div(10, 2)`       | The divisor is positive.   | Accept.                        |
| `div(10, 0)`       | The divisor is zero.       | Reject: obligation violated.   |
| `div(10, input())` | The divisor might be zero. | Reject: obligation unresolved. |

The argument itself does not need to be a compile-time constant. Ordinary
control-flow facts can prove the predicate for a runtime value:

```meowy
value : input()

| value > 0 | {
    div(10, value)
}
```

The compiler need not know which positive integer `value` contains. It only needs
enough evidence to establish the obligation at the call.

An obligation does not insert a runtime check, assume its own truth, or execute
`input()` during compilation. The explicit guard supplies the runtime check in
this example.

## Proposed first version

Restrict obligations to function declarations and predicates over their parameters.
They are entry preconditions; they are not promises about a function's result.

Under this proposal, attaching an obligation to an ordinary value binding would
be invalid:

```meowy
value : input() -| self > 0 |
```

Result guarantees would need their own rules about who establishes them and how
failure is represented.

A small initial predicate language could allow parameter names, literals,
comparisons, and boolean combinations. Arbitrary helper calls, effects, and
unrestricted compile-time execution should remain outside the initial scope.
The exact supported operators and proof rules are still to be decided.

Evidence should come from ordinary types, constants, enclosing function
obligations, and control flow. Mutation or effects that invalidate a fact must
prevent that fact from satisfying a later call. Likewise, a function's entry
obligation does not make its parameters permanently immutable.

## Relationship to existing proof observations

This proposal concerns requirements used when checking ordinary code. It does not
grant permission to use `@"proof"` answers as evidence for accepting that code.
The existing [proof dependency rules](../reference/stdlib/proof.md) prohibit
backward dependencies on observation results. Any eventual obligation design must
preserve that separation and avoid circular justification.

## Open questions

- Which predicates and proof rules must every conforming compiler support?
- How are obligations carried through function types, aliases, imports, and
  higher-order calls so a requirement cannot disappear?
- How can a caller forward its own obligation to another function, including
  through recursion, without circular reasoning?
- Which mutations, borrows, and intervening calls invalidate available evidence?
- How are violated and unresolved obligations explained, including the relevant
  declaration, argument, and missing fact?
- What happens when proof analysis exhausts its budget? Resource exhaustion must
  remain distinct from a proven violation and must never silently accept a call.

The next design step is to define a small, predictable proof model and examples
for these boundaries before proposing reference text or implementation work.
