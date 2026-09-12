# Computed type blocks

The bootstrap supports straight-line blocks in required type expressions, using
existing type construction and lexical scopes from the checker:

```meowy
settings : { -> limits : { -> base <uint8> : { offset <uint8> : 1; -> offset + 1 } } }
<Counts> : {
    element : <int32>
    base : settings.limits.base
    capacity : base * 2
    -> <(element)[capacity]>
}
values <Counts> : [3, 7]
```

Local immutable, unannotated bindings hold supported type values. Immutable integer
bindings may have an explicit integer annotation, such as `base <uint8> : 2`.
Local type aliases
such as `<Count> : <int32>` use the separate type namespace. Nested blocks may produce
types for these bindings or for computed annotations. Parenthesized expressions and
supported type queries retain their existing behavior. Symbolic type members such as
`core.int32`, foundation type aliases and imported types preserve their identities.

Exactly one unlabelled, unnamed, unannotated primary emission supplies the result.
An emission does not exit the block: subsequent declarations and errors are checked
in source order. Duplicate primary emissions report E205. A missing type result or
a runtime value used as a type reports E211; duplicate local declarations use E203.
The block's local names leave scope when it finishes. Constructed records, lists and
references retain ordinary storage, layout, mutability and ownership rules.

No HIR statements, runtime locals or functions are created for the type block itself.
Exported aliases can use computed blocks, and importers consume the same resulting
types. Application initializers still run only during program execution. Existing
runtime-parameter type queries inspect the parameter type, without treating its
value as a compile-time input. Documentation derives the resulting signatures and
checks local declaration links while the construction scope is active.

Known `debug.print` and `debug.panic` calls in evaluated positions report E219,
including calls through resolved aliases and calls after a primary emission.
The checker does not execute them. This is a narrow effect check; source helper
purity and transitive call analysis are not implemented by this slice.

## Integer calculations

Local integer bindings retain their checked width and signedness through aliases,
subsequent calculations and type queries. Supported expressions are integer literals,
eligible names, parentheses, unary `-`/`~` and binary `+`, `-`, `*`, `/`, `%`, `&`, `|`,
`^`. They reuse the scalar checker and constant evaluator. Incompatible widths use
E213, literal overflow E216 and invalid arithmetic E107. Negative or unrepresentable
list capacities retain E104. Required arithmetic is checked even inside an unreachable
runtime branch, and its temporary checking state is restored afterwards.

Inputs may be static integers in the construction scope or immutable integer bindings
with recorded initializer eligibility. The checker tracks separate evidence over
checked literals, aliases, supported unary/arithmetic expressions and eligible
integer blocks. Nested immutable integer records also carry complete initializer evidence,
including the [eligible predicates](#conditional-record-initializers) below.
Every local value dependency must already have that evidence; a folded constant alone does not
establish eligibility. Exact widths and source declaration identities are preserved.

Required reads can use eligible lexical inputs across function scopes. This does not
enable runtime captures or expose private names from another file. Original runtime
bindings and application effects remain in the program; checking/building does not
execute them. Runtime parameters, mutable state and effectful results remain unavailable
(E211). Evaluated effects, mutation, unsupported control flow and helper calls remain
unproven; unsupported folded inputs retain B001. Named imported inputs follow the
[export eligibility rules](#imported-immutable-inputs) below.

Evidence retains integer failures from unreachable runtime paths. A required read
reports E107 at the original failing expression, even through aliases. Dependency
work is charged transitively on each read, including cached values and repeated
references, against the shared bootstrap bound. Independent required roots reset it.
Extents within computed-type roots use the same eligibility checks as scratch bindings.
Direct extents outside those roots retain their existing supported profile.

The [example](../examples/computed-types.mwy) calculates a capacity of four from an
eligible immutable `uint8` record field. Scalar scratch produces no runtime locals, and documentation preserves
its actual integer signature. Floating-point, boolean and text scratch, comparisons,
shifts, mutable scratch and helper calls remain separate capabilities.

## Block initializers

A supported integer block uses immutable eligible integer and boolean bindings and
selects one integer primary emission targeting that block. Matcher branches reuse
eligible predicate evidence, including boolean inputs from enclosing scopes and integer comparisons.
Selected branches retain local scope; every visited condition and tail statement
contributes work. Skipped bodies contribute none. Nested eligible blocks are supported.

```meowy
debug : @"debug"
capacity <uint8> : {
    base <uint8> : 4
    pick : base == 4
    | pick | -> base
    | !pick | -> 2
    unused : false
}
<Items> : { -> <int32[capacity]> }
items <Items> : [3, 7]
debug.print(items[2])
```

This prints `7`. Selected values keep their exact checked widths, including through
aliases, named/module-primary exports and record fields. Ordinary type, primary and
scope checks still apply: branches cannot silently widen a typed value, emit twice,
omit a required result or expose branch-local names.

Successful evaluation includes unused bindings after the primary; an emission does
not return early. The first evaluated integer failure retains E107 at its original
span and stops evaluation, even before a primary or a later unavailable statement.
Error-only `never` HIR retains no invented integer result. Capture and scratch checks
preserve declared kinds so known boolean/string/record values cannot become integer
inputs merely because their initializer failed. Existing inferred error-only inputs
retain their supported behavior.

Every required read charges retained work again, including conditions whose bodies
are skipped, selected tails and cached module forwarding. Independent roots start
fresh. Blocks and predicates share the 64-level/4096-visit evaluator bounds; other
frontend and ownership limits remain independent.

Boolean scratch retains its own value, source error and work, including aliases and
nested boolean blocks. Its work and errors contribute to the integer initializer;
its boolean value never replaces the integer primary. Unused boolean tails are checked
even after an emission, and branch-local bindings do not escape their scope.

Evaluated record/string/float scratch, mutation, named/outer emissions, helper calls,
selected standalone expression blocks and restarts remain unavailable. Checking/building
never execute initialization; ordinary runtime values, effects and source evaluation
order remain unchanged.

### Boolean block initializers

A boolean initializer block can bind immutable eligible integers and booleans and
select one boolean primary through eligible matcher branches. Nested boolean/integer
blocks, comparisons, aliases and imported integer leaves reuse their existing evidence. Successful evaluation
inspects every statement after the emission too; an emission does not return early.

```meowy
debug : @"debug"
ready : {
    count <uint8> : 4
    valid : count == 4
    | valid | -> true
    | !valid | -> false
    unused : count + 1
}
settings : {
    | ready | -> width : 4
    | !ready | -> width : 2
}
<Items> : { -> <int32[settings.width]> }
items <Items> : [3, 7]
debug.print(items[2])
```

This prints `7`. Unused bindings retain work and failures, including tail statements.
The first evaluated failure retains its original diagnostic and stops evaluation;
no boolean value is invented. A declared boolean binding can preserve error-only
`never` HIR on unreachable paths. Nested unreachable bindings need their own declared
boolean type when inference loses the result kind.

Matcher conditions reuse eligible predicate evidence. Selected statement lists share
one primary result and retain their visited tail work; branch-local bindings do not
escape their scope. Duplicate and missing primary diagnostics remain E205/E204.
Skipped branches contribute no evaluation work or effects, while their conditions
still contribute work even when false. A failed condition stops selection and keeps
its original source error. Ordinary source and flow checks still apply.

Blocks also work directly as record predicates or short-circuit operands. Skipped
operands do not contribute evaluation work or effects. Runtime blocks remain in HIR,
and checking/building never execute initialization. Every required record-field read
charges retained boolean-block work again, including work forwarded through aliases
and modules. Branches, nested blocks and predicate operands share the existing
64-level/4096-visit evaluation bounds. Frontend nesting limits can be reached before
those evaluator limits; these are not full-language E220 counters.

Selected standalone expression blocks, named/outer emissions, record-valued scratch,
mutation and evaluated helper calls remain unavailable inside boolean blocks. Integer
and boolean blocks retain independently typed primaries. Boolean scratch inside required
type blocks and boolean field/export inputs remain separate capabilities.

## Record-field inputs

Named immutable record bindings and their aliases can supply integer leaves to copied
integer bindings, computed scratch and list extents. Every record has a unit primary
and nonempty immutable fields containing integers or records of the same kind. A shape
is bounded to 256 total fields across all descendants and 32 record levels; unused
local records must also have supported shapes. Checked field-index paths retain exact
widths independently of source declaration order.

Nested record construction shares scoped scalar/record evidence and work. Initializers
may use earlier emitted records, record aliases, integer fields and eligible scalar
blocks/locals. Every initializer statement remains checked. A selected leaf cannot hide
an evaluated effect, mutable descendant, invalid sibling computation or unused tail work.

Ordinary immutable subrecord aliases retain the complete original ancestor's errors
and transitive work. A subsequent leaf read cannot escape that evidence by shortening
the path. Hidden failures remain E107 at the originating expression, even when the
failing sibling lies outside the projected subrecord. No partial record is admitted.
Unreachable inline record emissions whose field identity was erased from HIR remain
unavailable; declared record aliases can retain known error evidence.

Direct required paths have a named local record or file-module root, optionally grouped.
Nested paths and copied subrecords preserve checked type/member identity. Eligible lexical leaves
may be used inside functions without enabling ordinary runtime captures. Qualified
type identities such as `core.int32` retain their separate behavior.

Reference projections, inline roots, non-integer leaves, empty records and non-unit
primaries remain outside this record slice. Checking never runs initializers; ordinary
record reads and runtime initialization remain unchanged.

### Conditional record initializers

Record evidence can select matcher branches using eligible immutable boolean locals,
checked boolean literals, `!`, `&&`, `||` and integer comparisons (`==`, `!=`, `<`,
`<=`, `>` and `>=`). Comparison operands retain their checked integer widths and
complete initializer evidence, including integer fields imported through modules.
Logical operands retain short-circuit order. Optimizer folding alone does not prove
eligibility; evaluated mutable/runtime inputs and helper calls remain unavailable.

```meowy
debug : @"debug"
limit <uint8> : 4
enabled : limit >= 4
settings : {
    selected : enabled && !false
    | selected | -> width <uint8> : limit
    | !selected | -> width <uint8> : 2
}
copy : { -> settings }
<Items> : { -> <int32[copy.width]> }
items <Items> : [3, 7]
debug.print(items[2])
```

This prints `7`. Ordinary boolean bindings, aliases and immutable boolean scratch
inside record initializers retain their value, source error and work. Branch-local
shadowing does not change outer bindings. Selected branches otherwise use the same
eligible integer, record and composition statements as straight-line initializers.
Every visited condition and statement contributes work, including selected unused bindings
and statements after emissions. An effect or invalid sibling in a selected branch
cannot be hidden by reading another field. Proven skipped branches and short-circuited
operands contribute no evaluation work or effects; ordinary source checks still apply.

Failed evaluated comparisons or boolean aliases retain the original E107 span,
including through a later subrecord projection. No branch value is invented after a
predicate failure. Evaluated operands are inspected in order; skipped operands do
not contribute failures. Ordinary flow checks still decide whether fields are fully
initialized. Repeated field comparisons are not necessarily stable flow atoms;
storing an eligible comparison in an immutable boolean gives complementary matcher
arms an ordinary shared binding to inspect.

Aliases, projections and module forwarding retain the selected evidence. Every
required read charges it again; independent required roots start fresh. Checking and
building remain silent, and ordinary runtime conditions and effects are preserved.

Selected branch traversal shares the existing 32-level record-evidence recursion
bound; predicate traversal shares the 64-level/4096-visit limits. Nested records and
branches consume depth together. These bootstrap bounds do not implement E220.
Boolean/float/text comparisons, boolean fields in eligible
records, boolean module exports as predicate inputs, and boolean scratch inside required
type blocks remain unavailable. Standalone expression statements in selected branches,
loops and conditional module exports are also separate.
A top-level unconditional export may still forward an eligible record whose own
initializer contains branches.

## Imported immutable inputs

Direct immutable named file exports retain the checked local identity of their eligible
integer or record initializer. Required reads such as `m.width` and `m.row.nested.n`
resolve the public field to that identity. Module aliases, copied integers, projected
subrecords and named re-exports preserve the original integer widths and complete
ancestor evidence. Private dependencies can supply an exported initializer without
making their names public. Synthetic module bindings do not receive record evidence.

Eligibility applies to each exported initializer. Unrelated file initialization effects
may appear before or after a pure export; they still execute once in dependency order
at runtime. Effects anywhere inside the selected initializer or its ancestor prevent
eligibility, including effects after emissions and outside projected subrecords.
Checking and building never execute these effects. Known failures retain original
source spans, and every required read charges retained transitive work again.

Eligible imported leaves can supply required types inside functions without enabling
ordinary runtime module-data captures. Existing module export shape/initialization/
privacy checks still apply; mutable or borrowed exports, package resolution and helper
purity remain separate capabilities.

### Scalar primary imports

A file whose complete runtime value is an integer can also supply its eligible direct
primary emission. For example, a file containing `base <uint8> : 2; -> base * 2` can be imported
as `capacity : @"./capacity.mwy"` and read by `<Items> : { -> <int32[capacity]> }`. Module aliases,
arithmetic copies and primary/named re-exports preserve its exact integer width,
initializer failures and transitive work. Required scratch may read the module name
itself, including inside functions or after a function-local import.

The checker retains the original primary emission identity and evidence without
rewriting its runtime HIR or changing constant folding. Each required read charges
cached initializer work again, including unused statements after the primary inside
an initializer block. Independent module effects still run once at startup, and
initializer failure still stops dependent/entry execution. Checking never runs them.

### Integer primaries with named exports

A module with named fields can supply its eligible direct integer primary through
arithmetic or integer-annotated required scratch. For example, `capacity.mwy`:

```meowy
base <uint8> : 4
-> base
-> label : "ready"
```

Its importer can construct a list type while retaining the module's named fields:

```meowy
capacity : @"./capacity.mwy"
<Items> : {
    count <uint8> : capacity
    -> <int32[count]>
}
items <Items> : [3, 7]
debug : @"debug"
debug.print(items[2])
debug.print(capacity.label)
```

Required arithmetic such as `capacity + 0`, copied integer inputs and scalar
re-exports retain exact widths, failures and transitive work. Required reads also
work inside functions and through function-local module identities. Each read
charges the original initializer work again; independent required roots reset it.

Unannotated ordinary aliases and `capacity<>` preserve the complete record type.
A mixed module is not itself integer scratch: use an integer annotation or arithmetic.
Likewise, use `<int32[capacity + 0]>` inside a computed root instead of a bare
mixed-module extent. Whole-record computed scratch remains unavailable.

Named initializer effects do not disqualify a separate pure primary. An effect inside
the primary initializer does, even through copies or re-exports. Checking/building
never run either initializer. At runtime, every module still initializes once in
source dependency order, and failures stop dependent and entry execution.

### Module composition

A direct top-level composition can forward a file module's eligible integer primary
and named integer/record inputs. For example, `facade.mwy` can compose the
`capacity.mwy` above:

```meowy
source : @"./capacity.mwy"
-> source
```

The importer above can load `@"./facade.mwy"` instead. Its required integer read and
runtime `label` lookup keep the same results. Module aliases and parentheses also
preserve this forwarding through chains of facades.

Each eligible named export retains its original checked source identity. Integer
primaries retain their value, width and failure evidence. Every composition adds
copy/projection work, charged on each required read along with all retained initializer
work. Projecting a forwarded record or re-exporting its subrecord keeps complete
ancestor evidence, including unused siblings and tail work.

Eligibility remains separate for each original module export. An unrelated effectful
export does not disqualify a pure one, while a selected record cannot hide effects in
its ancestors. The complete module never becomes an eligible whole-record input.
Private locals stay private; compile-time function/type exports need their existing
explicit re-export forms because they are not runtime record fields.

Composition retains ordinary field/primary collision checks, record identity,
startup order and runtime capture gates. Checking/building never execute source or
facade initialization. Runtime failures still stop dependent and entry execution.

### Local record composition

Eligible unit-primary records can also be composed into local records or directly
into a file's exports. Named sources, inline initializers and projected subrecords
retain all ancestor work and failures. For example, `settings.mwy` can contain:

```meowy
settings : {
    -> limits : { -> width <uint8> : 4 }
    unused : 2
}
copy : { -> settings.limits }
-> copy
```

Its importer can use the composed field in a required type:

```meowy
settings : @"./settings.mwy"
<Items> : { -> <int32[settings.width]> }
items <Items> : [3, 7]
debug : @"debug"
debug.print(items[2])
```

This prints `7`. Each exported field retains a path into the complete eligible
source record. Further module compositions and subrecord copies preserve that path
and its evidence. Evaluated effects, mutation or unsupported values in the source
initializer, including unread siblings and unused tail statements, prevent
eligibility for every composed field. Unrelated file initialization remains allowed.
Composing a module namespace into a local record does not make the namespace eligible
as a whole record; constructing a record from individually eligible exports works.

The existing unit-primary, immutable-field, depth and total-field bounds still apply.
Required reads charge retained work again, including copies and projections. Large
valid shapes may encounter other bootstrap work or ownership-analysis limits before
native execution. Declared shapes retain source errors on unreachable paths;
unannotated unreachable shapes can still lose field identity.

Conditional module exports and inline required import roots remain outside this slice.
Annotated or mutable ordinary module-identity aliases retain their existing restrictions. Ordinary runtime captures remain unavailable; a type-only
use does not grant runtime access.

## Explicit limits

One outer type-value resolution shares these bootstrap bounds with nested resolutions:

| Bound | Maximum |
| --- | --- |
| Resolver/scalar-validation expressions and block-statement visits | 4096 |
| Active resolver depth | 64 |
| Concrete type nodes traversed across resolved values/local aliases | 16384 |

Exhaustion reports B001. Nested blocks share the counters; independent roots reset
them. Existing parser, type/layout and proof limits still apply. These limits qualify
bootstrap support only; they do not implement the language's logical E220 counters.

Non-integer scalar scratch, mutable scratch, branches/restarts, labeled blocks,
annotated or named emissions, general expression statements and source/helper calls
remain unsupported in type blocks. `core.Type` parameter/result annotations, generic
specialization, type equality, full purity analysis, intrinsic descriptions and the
complete required evaluator remain separate. Runtime type-value storage remains gated.

See [the runnable example](../examples/computed-types.mwy), [STATUS.md](../STATUS.md),
the [language contract](../../docs/reference/compile-time.md), and the
[implementation pipeline](../../COMPILER.md#the-pipeline).
