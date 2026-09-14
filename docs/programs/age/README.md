# Validate an age

[All programs](../README.md) · [Manifest](mod.mwy) · [Entry](main.mwy) · [Policy](policy.mwy)

A small module keeps parsing and policy separate from presentation. The entry
chooses an inclusive age range and tries four inputs; `policy.describe` narrows
a parsed byte, checks the range, and returns static text through named exits.

From the repository root:

```sh
cd docs/programs/age
meowy check
meowy run
```

Expected output:

```text
Welcome
Below the minimum age
Above the maximum age
Enter a whole number from 0 to 255
```

| Input                             | Result                                 |
| --------------------------------- | -------------------------------------- |
| `"18"`, `"24"`, `"100"`           | Accepted; both endpoints are inclusive |
| `"17"`                            | Below the minimum                      |
| `"101"`                           | Above the maximum                      |
| `"256"`, `"-1"`, `"twenty"`, `""` | Parse failure                          |

The supplied Options satisfies `minimum <= maximum`. Change the arguments in
main to explore the table. A rejected age is an expected result, so this demo
prints it and completes successfully. Diagnostic output failure panics under
`debug.print`'s contract.

There are no heap allocations: input and returned messages borrow static storage,
and the options record is inline. Each early path emits to the function's named
scope and leaves it. An emission alone would not prevent later checks from running.
See [union narrowing](../../reference/types.md#unions-and-narrowing).
