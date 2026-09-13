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
its actual integer signature. Floating-point/text scratch and comparisons,
shifts, mutable scratch and helper calls remain separate capabilities.

## Boolean scratch

Required type blocks can bind immutable booleans from lexical literals, previously
checked eligible locals, record fields, named module exports and module primaries.
Bindings may use a `boolean` annotation; aliases and type queries retain their kind.

```meowy
settings : {
    -> enabled : { -> false; unused : 2 }
    -> capacity <uint8> : 4
}
<Flags> : {
    enabled <boolean> : settings.enabled
    copy : enabled
    capacity : settings.capacity
    -> <(copy<>)[capacity]>
}
flags <Flags> : [true, false]
debug : @"debug"
debug.print(flags[2])
```

This prints `false`. The scratch binding holds the checked boolean value, and its type
query supplies the list element type. Boolean values cannot become integer extents or
be emitted as types. Scratch leaves no runtime storage and does not escape its scope.

Every source read charges retained initializer and ancestor work, including unused
bindings after the type emission. Reading a scratch alias charges a local read; it
does not reevaluate the source already materialized in that root. Independent roots
reset their budgets. Errors retain their original source span, and nested groups share
the active depth limit. A folded runtime constant alone does not establish eligibility.

A boolean-only module can be read by name. A module with named fields needs a boolean
annotation in required scratch, for example `enabled <boolean> : flags`. Unannotated
module aliases and type queries preserve the complete module identity. Eligible reads
work across function scopes and through function-local imports without permitting
ordinary runtime captures. Checking/building never execute module initialization.

Evaluated runtime parameters and effectful inputs remain unavailable. Mutable scratch
and helper calls remain separate. Names such as `true` and `false` follow ordinary
lexical lookup and may be shadowed.

### Boolean operators

Required boolean scratch supports `!`, `&&`, `||`, `==` and `!=` with boolean operands:

```meowy
settings : { -> enabled : false }
<Flags> : {
    enabled : !settings.enabled
    same : enabled == (settings.enabled || true)
    skipped : false && settings.enabled
    -> <(same<>)[2]>
}
flags <Flags> : [false, true]
debug : @"debug"
debug.print(flags[2])
```

This prints `true`. All supported operand names, field paths and types are checked
before evaluation, including skipped operands. Unknown/private names and incompatible
operand kinds still fail. Logical operators evaluate left-to-right and skip unnecessary
right operands. Skipped inputs need no initializer evidence and contribute no evaluation
work or retained errors. An evaluated runtime, mutable or effectful source remains
unavailable. Module initialization still runs normally, even when a required read is skipped.

Equality evaluates both operands and charges both reads, including repeated reads of
the same cached source. A true/false left value does not skip the right operand. The
first evaluated failure stops evaluation and retains its original source span.
Mixed-module primaries project when compared with a boolean; comparing two complete
module records remains outside this slice. Use `!module == !module` for explicit
primary comparisons. Float/text comparisons inside required blocks remain separate.

Operand-tree checking has its own 4096-node/64-level bounds, including skipped syntax,
and consumes frontend checking work. Evaluation retains the shared required-root
work/depth bounds; skipped evaluation is not charged. Independent roots reset their
budgets. These bootstrap limits report B001 and do not implement full-language E220.
Direct calls, inline boolean blocks and unsupported operators remain unavailable even
in skipped syntax. Conditional type selection and required record scratch remain separate.

### Integer comparisons

Required booleans support integer `==`, `!=`, `<`, `>`, `<=` and `>=`. Operands can use
eligible locals, record fields, module primaries and the supported integer arithmetic:

```meowy
limits : {
    -> minimum <int8> : -3
    -> capacity <uint8> : 4
}
<Flags> : {
    enough : limits.capacity >= 4
    ordered : limits.minimum < 0
    same : limits.capacity + 1 == 5
    ready : enough && ordered && same
    -> <(ready<>)[2]>
}
flags <Flags> : [false, true]
debug : @"debug"
debug.print(flags[2])
```

This prints `true`. Declared widths and signedness remain exact; compatible operand
contexts give literals their width without converting named values. For example,
`1 + 2 < limits.capacity` checks the literals as `uint8`. Mixed integer widths report
E213, unrepresentable literals E216 and invalid evaluated arithmetic E107.
Unsigned negation remains invalid. Comparisons produce booleans, not integer extents.

Operand types and literal ranges are checked before evaluation, even when a logical
operator skips a comparison. Skipped comparisons read no initializer evidence, charge
no evaluation work and perform no arithmetic. Evaluated operands use the existing
required integer checks from left to right; a failing left operand stops the right
read. Each source read retains its original error span and transitive work.

Mixed-module primaries work in comparisons and arithmetic; two complete records still
cannot be compared in required blocks. Required reads inside functions do not grant
runtime captures. Checking/building remain silent and preserve module initialization.
Float/text comparisons, direct calls, inline blocks and conditional type selection
remain separate capabilities.

## Block initializers

A supported integer block uses immutable eligible integer, boolean and bounded record
bindings and selects one integer primary emission targeting that block. Matcher branches
reuse eligible predicate evidence, including boolean inputs from enclosing scopes and
integer comparisons.
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

Evaluated string/float scratch, unsupported record shapes, mutation, named/outer emissions,
helper calls, selected standalone expression blocks and restarts remain unavailable. Checking/building
never execute initialization; ordinary runtime values, effects and source evaluation
order remain unchanged.

### Boolean block initializers

A boolean initializer block can bind immutable eligible integers, booleans and bounded
records and select one boolean primary through eligible matcher branches. Nested
boolean/integer blocks, comparisons, aliases and imported integer leaves reuse their
existing evidence. Successful evaluation inspects every statement after the emission too; an emission does not return early.

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

Selected standalone expression blocks, named/outer emissions, mutation and evaluated
helper calls remain unavailable inside boolean blocks. Integer and boolean blocks
retain independently typed primaries. Required type blocks can read the resulting
booleans as immutable scratch; inline boolean block evaluation there remains separate.

### Record scratch in scalar initializers

Integer and boolean initializers can bind eligible unit-primary records, copy them
and project nested integer or boolean fields. Record scratch reuses the complete
[record-field evidence](#record-field-inputs), including unread siblings and ancestor
tail work. Its retained work and errors enter the scalar result; its fields never
replace the scalar primary.

```meowy
debug : @"debug"
capacity <uint8> : {
    settings : {
        -> limits : { -> width <uint8> : 4 }
        unused : 1
    }
    limits : settings.limits
    -> limits.width
}
<Items> : { -> <int32[capacity]> }
items <Items> : [3, 7]
debug.print(items[2])
```

This prints `7`. Every evaluated record binding must qualify as a whole, even when
unused or read through one projected leaf. Unsupported or effectful siblings cannot
be hidden by copying a subrecord. Declared record aliases preserve retained errors
on unreachable paths; inline emissions whose HIR loses field identity remain limited.

The existing 256-total-field and 32-record-level bounds apply to scratch too, sharing
caller evaluation counters rather than starting fresh. Selected branches restore local
record scope. Each required scalar read charges retained record work again. These
input bounds do not qualify larger native shapes past other frontend/ownership limits.

Eligible named record exports can supply scratch records. Module aliases and inline
import identities produce no runtime binding and may read eligible named exports.
Constructing a whole-module record still does not grant whole-record input eligibility.
Checking/building never run initialization, and ordinary runtime copies remain intact.
Non-unit primaries, mutation, references and helper-produced records remain outside
this input slice.

## Record-field inputs

Named immutable record bindings and their aliases supply integer leaves to integer
bindings, computed scratch and list extents, and boolean leaves to predicate inputs
and boolean aliases. Every record has a unit primary and nonempty immutable fields
containing integers, booleans or records of the same kind. A shape is bounded to 256
total fields across all descendants and 32 record levels; unused
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

Reference projections, inline roots, other leaf kinds, empty records and non-unit
primaries remain outside this record slice. Checking never runs initializers; ordinary
record reads and runtime initialization remain unchanged.

### Typed boolean record fields

Boolean fields retain their own typed leaf values, including error-only leaves.
Integer lookup cannot read them as zero or one. Predicate lookup carries the complete
ancestor error and work through copies, subrecord projections and compositions.

```meowy
debug : @"debug"
settings : {
    -> enabled : true
    -> limits : { -> width <uint8> : 4 }
}
capacity <uint8> : {
    limits : settings.limits
    ready : settings.enabled
    | ready | -> limits.width
    | !ready | -> 2
}
<Items> : { -> <int32[capacity]> }
items <Items> : [3, 7]
debug.print(items[2])
```

This prints `7`. Boolean reads cannot hide a failing or effectful sibling, an unused
tail, or a mutable descendant. Each read charges retained ancestor work again.
The existing 256-total-field and 32-record-level bounds include boolean fields.

Named exported records and record-derived composed exports retain boolean leaf paths
through facades. Direct named boolean exports retain scalar source identities and work.
Required boolean scratch can read these leaves without making them integer extents.
Ordinary runtime captures remain unavailable.

### Conditional record initializers

Record evidence can select matcher branches using eligible immutable boolean locals,
checked boolean literals, `!`, `&&`, `||`, boolean equality/inequality (`==`, `!=`)
and integer comparisons (`==`, `!=`, `<`, `<=`, `>` and `>=`). Integer operands retain
their checked widths. Both forms retain complete initializer evidence, including
integer fields imported through modules.
Logical operands retain short-circuit order. Optimizer folding alone does not prove
eligibility; evaluated mutable/runtime inputs and helper calls remain unavailable.

```meowy
debug : @"debug"
limit <uint8> : 4
enabled : limit >= 4
settings : {
    selected : (enabled == true) && (false != true)
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

Boolean equality evaluates both operands from left to right and charges both reads,
including repeated reads of the same cached value. A true or false left value still
requires eligible right-operand evidence. Evaluation stops at the first error, retaining
its original span. Logical `&&`/`||` keep their existing short-circuit behavior. This
applies to ordinary boolean bindings, scalar initializer scratch and record predicates;
required boolean scratch uses the same evaluation order for boolean equality.

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
Float/text comparisons remain unavailable. Required blocks support the integer comparisons above.
Standalone expression statements in selected branches, loops and conditional module exports are also separate.
A top-level unconditional export may still forward an eligible record whose own
initializer contains branches.

## Imported immutable inputs

Direct immutable named file exports retain the checked local identity of their eligible
integer, boolean or record initializer. Required integer reads such as `m.width` and
`m.row.nested.n`, and predicate reads such as `m.enabled`, resolve the public field
to that identity. Module aliases, copied scalars, projected
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

### Named boolean exports

A direct immutable named boolean export can supply an eligible predicate. For example,
`flags.mwy` can use a private dependency without exposing it:

```meowy
debug : @"debug"
debug.print("flags")
ready : true
-> enabled : ready
```

Its importer can choose an integer capacity in an ordinary scalar initializer:

```meowy
flags : @"./flags.mwy"
capacity <uint8> : {
    enabled : flags.enabled
    | enabled | -> 4
    | !enabled | -> 2
}
<Items> : { -> <int32[capacity]> }
items <Items> : [3, 7]
debug : @"debug"
debug.print(items[2])
```

Running this prints `flags` and then `7`. Checking and building remain silent.
Aliases, named re-exports and module compositions retain true/false values, original
initializer errors and transitive work. Every evaluated read charges that work again;
short-circuited operands contribute none. An unrelated effectful export does not
invalidate an independently eligible boolean export.

Whole-module record inputs and conditional module exports remain unavailable. Ordinary runtime
module-data captures and private-field access remain rejected.

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

### Boolean primary imports

A file can supply an eligible boolean primary, including when it also has named exports.
For example, `flags.mwy`:

```meowy
debug : @"debug"
debug.print("flags")
ready : true
-> ready
-> label : "ready"
```

Boolean operations project the primary while ordinary aliases preserve the module's
complete identity. Its importer can choose an integer capacity:

```meowy
flags : @"./flags.mwy"
alias : flags
capacity : {
    enabled : alias && true
    | enabled | -> 4
    | !enabled | -> 2
}
<Items> : { -> <int32[capacity]> }
items <Items> : [3, 7]
debug : @"debug"
debug.print(items[2])
debug.print(alias.label)
```

Running this prints `flags`, `7` and `ready`. Checking/building remain silent.
Negation, short-circuit logic and boolean equality use the retained primary evidence.
Copies, primary/named re-exports and direct module compositions preserve true/false
values, original initializer errors and complete work. Every evaluated read charges
that work again; independent required roots reset their budgets.

Unrelated file effects or named initializer effects do not invalidate an eligible
primary. Effects inside the primary initializer prevent eligibility, including unused
tails. Required evaluation neither runs nor removes runtime initialization; a startup
failure still stops dependent and entry execution.

Ordinary module aliases and type queries retain named fields. Annotated ordinary
module-identity bindings remain unavailable; use a boolean operation to make a scalar
copy. Comparing two complete module records does not project their primaries;
use explicit boolean operands when a primary comparison is intended. Required boolean
scratch can read a mixed primary with a `boolean` annotation. Ordinary runtime
module-data captures remain unavailable, including after a successful required read.

### Module composition

A direct top-level composition can forward a file module's eligible integer or boolean
primary and eligible named inputs, including direct booleans and boolean fields derived from
record composition.
For example, `facade.mwy` can compose the `capacity.mwy` above:

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

Scalar scratch beyond integers/booleans, mutable scratch, branches/restarts, labeled blocks,
annotated or named emissions, general expression statements and source/helper calls
remain unsupported in type blocks. `core.Type` parameter/result annotations, generic
specialization, type equality, full purity analysis, intrinsic descriptions and the
complete required evaluator remain separate. Runtime type-value storage remains gated.

See [the runnable example](../examples/computed-types.mwy), [STATUS.md](../STATUS.md),
the [language contract](../../docs/reference/compile-time.md), and the
[implementation pipeline](../../COMPILER.md#the-pipeline).
