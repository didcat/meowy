# Borrow and dereference syntax migration

The approved syntax reads selection and borrowing from left to right. This is a
direct grammar change: there are no external users to support and no compatibility
mode or legacy spelling warning is required. Ownership rules and capability gates
remain unchanged. All migration stages completed on 2026-09-09;
[STATUS.md](../STATUS.md) records the passing compiler/native/editor evidence.

## Grammar

| Expression | Structure and intent |
| --- | --- |
| `&object.field` | `(&object).field`: borrow the receiver, then select |
| `object.&field` | `&(object.field)`: borrow the selected field |
| `object.&!field` | `&!(object.field)`: exclusively borrow the selected field |
| `&items[i]` | `(&items)[i]`: borrow the collection, then index |
| `&(items[i])` | Borrow the selected element |
| `object.inner.&field` | Borrow the nested field |
| `object.&inner.field` | Borrow inner, then select field through that reference |
| `object.&items[i]` | Borrow items, then index; element borrowing requires grouping |
| `*pointer.field` | `(*pointer).field`: dereference the receiver, then select |
| `object.*field` | `*(object.field)`: dereference the selected reference field |
| `*items[i]` | `(*items)[i]`: dereference the receiver, then index |
| `*(items[i])` | Dereference the selected reference element, subject to supported types |
| `*f()` | `(*f)()`: call the dereferenced value, subject to callable support |
| `*(f())` | Dereference the call result |
| `&f()` | `(&f)()`: call the borrowed value, subject to callable support |
| `&(f())` | Borrow the call result, subject to temporary lifetime rules |
| `&value<>` | Query the reference's type |
| `&value~<T>` | Ascribe the resulting reference, using an explicit type target |
| `&object.&!field` | `(&object).&!field`; exclusive access through a shared receiver stays restricted |
| `&(object.&!field)` | Borrow the produced exclusive reference; existing nested-reference gates apply |

Prefix `&`, `&!` and `*` consume a prefix operand before subsequent postfix operations.
That boundary applies through nested prefix operators: `&*pointer.field` groups as
`(&(*pointer)).field`, and `&!*pointer` remains an exclusive reborrow. Bare
`*pointer.field` now means `(*pointer).field`; `*(pointer.field)` or `pointer.*field`
dereferences the selected field. Parentheses reset the prefix boundary.

Postfix `.&`, `.&!` and `.*` require one field name and then resume the ordinary postfix
chain. They do not introduce indexed or dispatch borrowing syntax. Field names
retain ordinary lookup; no intrinsic-name or whitespace exceptions are added.
Newlines/comments retain existing continuation rules. Types `<&T>`/`<&!T>`,
boolean operators, task groups and all other syntax keep their roles.
Different expression structure does not promise pointer inequality or new supported
types: receiver selection can copy a Copy field/element in a value context.

## Completed implementation and migration sequence

1. Record this grammar and inventory affected sources. Preserve the old parser
   for migration inspection. Use the existing public AST and exact source spans
   where sources can be parsed; manually review generated/incomplete fragments.
2. Extend `parser/expressions.rs` to lower dotted pointer operations into
   existing `Unary(Field)` nodes. Give prefix borrowing/dereferencing a postfix
   boundary inherited by its nested prefixes, with grouping resetting it. Keep the existing AST/HIR, ownership
   passes, backend and runtime. Test AST distinctions, nesting bounds, malformed
   syntax, Unicode spans, interpolation and newline continuation.
3. Preserve old program intent before qualifying the switch. Old full-place
   borrows and selected-value dereferences become explicit groups (`&!(rows[index].field)`) or selected-field forms
   (`rows[index].&!field`, `holder.*field`). Do not turn those into borrows of
   the containing list.
   Migrate Rust source strings/fragments, standalone examples, reference fixtures,
   guide/reference/design prose and documentation examples. Update source-span
   expectations from actual new text. Preserve type references and task-group forms.
4. Test native shared/exclusive field borrowing, selected-field dereferencing,
   grouped indexed borrowing/dereferencing, receiver-borrow selection, nested paths, sibling disjointness, conflicts,
   reference expiry and once-only index/call effects in debug and release. Keep
   existing unsupported features unsupported. Add editor token coverage without
   expanding editor infrastructure.
5. Run focused tests, `tools/verify.py --compiler`, Vim/Neovim checks and final local
   links/whitespace checks. Review the entire syntax migration and commit cohesive
   changes. The separate runtime/sanitizer gate is only needed if runtime changes.

## Preservation and completion

Use old/new AST comparison with groups/spans normalized wherever practical;
existing output and primary-diagnostic checks remain the behavior authority.
Never use a successful parse or B001 gate as native correctness evidence. No
reference fixture expectation changes may conceal ownership regressions.

Completion means all repository sources use their intended new interpretation,
the grammar and editor agree, and the compiler gate passes with unchanged existing
execution/diagnostic outcomes plus explicit coverage for the new distinctions.
Full language conformance and release qualification remain separate.

Rollback is a revert of the cohesive grammar/corpus migration commit(s), retaining
this design decision for review. There is no persisted data or runtime ABI migration.
Do not keep two production parser modes or create a public migration CLI for this
one-time repository transition.
