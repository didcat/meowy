# Computed type blocks

The bootstrap supports required type blocks with immutable bindings and bounded
boolean matchers, using existing type construction and lexical scopes:

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

Local immutable, unannotated bindings hold supported type values, integers, booleans
and records.
Immutable integer bindings may have an explicit integer annotation, such as
`base <uint8> : 2`. Local type aliases such as `<Count> : <int32>` use the separate
type namespace. Nested blocks may produce types for these bindings or for computed annotations. Parenthesized expressions and
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

## Type subtraction

The suffix `!<U>` removes supported concrete alternatives from a compile-time type:

```meowy
<Remove> : <null><boolean>
kind <Type> : <int32><null><boolean>
element <Type> : kind !<Remove>
<Items> : {
    reduced : ({ -> <int32><null> }) !<null>
    same : reduced == element
    | same | -> <(element)[4]>
}
items <Items> : [3, 7]
debug : @"debug"
debug.print(items[2])
```

This prints `7`. Subtraction works in annotations, type aliases, metatype bindings
and required type blocks. Supported operands include literals, lexical/imported type
values, static type queries and bare type-producing blocks. For example,
`<int32><null> !<null>` and `value<> !<null>` construct the remaining type set.
An empty result is `never`; removing an absent alternative preserves the source set.
Normalized record identity, mutability, widths, list capacities, reference modes and
nominal foundation identities determine which supported alternatives are removed.

Each suffix takes one explicit bracket. A named union such as `<Remove>` above, or
`!<(<null><boolean>)>`, removes several alternatives. Repeated suffixes evaluate from
left to right. Adjacent union suffixes after subtraction, such as
`<int32> !<null><boolean>`, remain B001 in this bootstrap; use an explicit union
operand. Annotation chains allow at most 64 subtraction operations, while existing
syntax and evaluator depth limits may be reached earlier.

Both operands resolve in source order, including when the source is already `never`.
The first failure retains its original diagnostic and source span through facades.
Each operand and resulting type charges the shared node budget; constructor inputs
retain their work on every read. Independent roots reset budgets. An enclosing
short-circuit comparison skips constructor work while preserving supported form and
outer-name checks. Imports inside either operand retain normal graph discovery and
runtime initialization order.

Known non-type operands report E222; an evaluated bare block producing a scalar
reports E207. Broad bases such as `core.error`, literal subtypes and unrepresentable
literal exclusions remain outside this compiler. The supported subset does not
implement the full E209 representability rules. Type-producing helpers and runtime
type-value storage remain gated.

Subtraction changes a type set only. Annotating a nullable runtime value with
`<(value<> !<null>)>` does not validate that value; an initializer which may still be
null is rejected with E207. Ordinary predicates must establish any runtime narrowing.

## Type-value equality

Inside required evaluation, `==` and `!=` compare normalized concrete type identity:

```meowy
element <Type> : <int32>
<Items> : {
    same : element == <int32>
    union : <int32><null><never> == <null><int32>
    shape : <{ first <int32>; ready <boolean> }> == <{ ready <boolean>; first <int32> }>
    | same && union && shape | -> <(element)[4]>
}
values <Items> : [3, 7]
debug : @"debug"
debug.print(values[2])
```

This prints `7`. Type literals, supported type queries, lexical type-value aliases,
core/foundation type members and explicitly exported type values retain identity.
Union order, repeated members and `never` do not change the normalized set. Record
field order does not matter; names, types, mutability and the primary do. Integer
width/sign, list capacity, reference mode and nominal foundation identities remain
part of the type. Equality compares identity, not assignability.

Both operands resolve once in source order through the bounded type evaluator.
Each resolved payload charges its type nodes; each eligible constructor input charges
its retained work again, even when both operands read the same input. A failing left
operand stops the right; a successful left operand always evaluates the right.
Failures keep their original source span through imports and facades. Independent
required roots reset budgets; nested comparisons share the enclosing counters.

Short-circuit `&&`/`||` checks operand forms and outer lexical/member lookup, but skips
constructor evaluation, type-name resolution inside literals and their input/node
work. Thus `false && (<int32[1 / 0]> == <Missing>)` does not construct either type.
Mixed known type/scalar operands and ordered type comparisons report E222.

Type-of-type queries and helpers remain unsupported. Equality is available only
inside required evaluation; it does not create runtime type values or enable
ordinary runtime type equality.

### Type-producing block operands

Required equality also accepts bare blocks that produce types, on either side of
another block or an existing type value:

```meowy
<Items> : {
    same : ({ base <uint8> : 2; -> <int32[base * 2]> }) == <int32[4]>
    different : ({ -> <uint8> }) != ({ -> <int32> })
    | same && different | -> <int32[4]>
}
items <Items> : [3, 7]
debug : @"debug"
debug.print(items[2])
```

This prints `7`. Selected blocks infer their result kind and preserve normalized
type identity, including local aliases, nested type blocks, supported type queries
and imported payloads. The explicit `<(expression)>` spelling remains supported.
Bindings inside an operand leave scope before the next operand is evaluated.

Known scalar operands supply their existing integer/boolean context. Otherwise the
left operand is inferred, and both selected results must have the same value kind.
An evaluated type/scalar mismatch reports E207. A type emitted in a scalar result
block also reports E207 after its constructor is checked; constructor errors keep
their own diagnostics. A type-valued result does not enable ordered comparisons.

Each operand executes once from left to right. Primary emissions do not skip tails;
a failing left initializer or tail stops the right operand. Blocks and their nested
type payloads share the enclosing work/depth/node limits. Eligible input reads retain
their full cost, including repeated reads of the same source. Independent roots reset
those budgets, and checking creates no runtime locals for either operand.

Short-circuiting checks supported block statement forms and outer operand names
without resolving block-local initializers, annotations or result kinds. Thus
`false && (({ -> <int32> }) == ({ -> true }))` skips the mismatched results.
Mutable scratch, named/labeled emissions and helpers retain their existing gates.
Module initialization still runs in dependency order at program startup.

## Explicit type-value bindings

Inside a required block, an immutable binding can declare `core.Type` as its kind:

```meowy
core : @"core"
<Kind> : <core.Type>
<Items> : {
    element <Kind> : <int32>
    result <core.Type> : {
        count <uint8> : 4
        -> <(element)[count]>
    }
    -> result
}
items <Items> : [3, 7]
debug : @"debug"
debug.print(items[2])
```

This prints `7`. The initializer must produce an existing supported type value.
Type literals, type-value aliases, supported type queries and type-producing required
blocks retain their concrete identity. A supported scalar/record result instead reports
E207; source failures keep their own diagnostics and original spans.

`Type` is also available in the prelude type namespace. Core module aliases, local type
aliases and explicitly exported/imported metatype aliases preserve the same kind.
Names remain shadowable: `<Type> : <uint8>` makes a local `Type` annotation an ordinary
integer annotation. A file exporting its own type named `Type` does not become core.
Facades must explicitly re-export type aliases; `-> module` does not forward them.

The checker keeps this kind outside runtime HIR. No runtime locals or native layouts
are created for typed required bindings. Data types containing `core.Type`, including
record fields, list elements, references and unions, report E211 when resolved through
the data-type path. Type-producing function signatures remain explicitly unsupported.

Within required evaluation, each selected annotation charges one type node in addition
to existing initializer/materialization work. Aliases, nested blocks and imported
identities share the enclosing budgets. Skipped required bindings do not resolve their
annotations or initializers; skipped documented declarations retain their existing gate. Local names leave scope
normally, and checking/building preserve silent staging and normal module startup.

Checked documentation displays `core.Type` for an explicitly annotated type-value
binding, while a constructed type alias displays the actual resulting data type.

Named metatype annotations also start required evaluation in ordinary statements,
as described below. Mutable type bindings, first-class metatype expressions
(`core.Type` or `<core.Type>`), type-of-type queries, computed metatype annotations
and compile-time helper execution remain separate.
The type namespace can name `core.Type`; that does not make it a native data type.

### Ordinary type-value bindings

An immutable named `core.Type` annotation starts required evaluation even at ordinary
module or function scope:

```meowy
core : @"core"
element <core.Type> : <int32>
items <Type> : { -> <(element)[4]> }
values <(items)> : [3, 7]

copy <int32> : (value <int32>) {
    local <Type> : value<>
    result <(local)> : value
    -> result
}

debug : @"debug"
debug.print(copy(values[2]))
```

This prints `7`. The type-valued bindings create no runtime locals or statements.
The parameter type query is static; using the parameter's runtime value as the
initializer of `local` would remain E211. Concrete type payloads retain identity
through both typed and ordinary unannotated identity aliases.

A binding starts a fresh budget only when no required root is active. Bindings nested
inside its initializer share that root's remaining work and type-node counters.
Finishing or failing the root restores the previous checking state. Imported metatype
aliases use the same rules; unrelated or shadowed types named `Type` remain ordinary
data types. Existing unannotated identity and runtime data bindings are unchanged.

Runtime branch reachability does not defer a metatype initializer: the compiler checks
it even in a statically unreachable runtime arm. A skipped branch *inside required
evaluation* still skips its initializer under the existing matcher rules. Initializer
errors preserve their original source spans, and known forbidden effects remain E219.

The binding's name follows ordinary lexical scope. It is not implicitly exported and
cannot be printed, borrowed, or placed in runtime storage. Explicit type-namespace
exports can publish the concrete result; named value exports use the form below.
An annotation on a function declaration remains its result annotation, so this does
not enable type-producing functions. Module initialization effects still run normally
at execution; checking/building neither run nor remove them.

### Named type-value exports

An unconditional top-level named emission with `core.Type`, prelude `Type` or a
metatype alias exports a concrete type value. For example, `types.mwy`:

```meowy
core : @"core"
private <core.Type> : <int32>
-> element <core.Type> : private
-> items <Type> : { -> <(element)[4]> }
```

A facade explicitly forwards each value in `facade.mwy`:

```meowy
types : @"./types.mwy"
-> items <Type> : types.items
```

The importer uses the value expression in a computed type annotation:

```meowy
types : @"./facade.mwy"
values <(types.items)> : [3, 7]
debug : @"debug"
debug.print(values[2])
```

This prints `7`. Each export retains the concrete payload in the value namespace;
`<types.items>` would instead look for a type-namespace alias. Ordinary local aliases
remain private. A facade can also publish `-> <Items> : types.items` in the type
namespace. `-> types` forwards runtime fields and does not forward type values.

Exports require an explicit metatype annotation in this bootstrap. Mutable, nested,
labeled and conditional type-value exports remain unsupported, as do type-producing
functions. Type-value names collide with function/data exports, while the separate
type namespace may reuse the name. Metatype annotations follow lexical lookup.

Each export starts or joins the same bounded required evaluation as an ordinary
metatype binding. Nested bindings share work; independent exports start fresh roots.
Eligible input failures preserve their original source spans, including initializer
tails after the type emission. Checked documentation shows `core.Type` for the value
export and the concrete result for a type alias.

Type exports create no runtime locals, fields or emissions. Imported payloads remain
available inside function type construction. Checking/building stay silent, while
module initialization effects still execute once in dependency order. An initializer
panic prevents dependent modules and the entry file from running even when the module
is imported only for its types.

## Conditional type selection

Independent boolean matchers can select a type inside a required block:

```meowy
capacity <uint8> : 4
<Items> : {
    | capacity >= 4 | -> {
        <Element> : <{ n <int32> }>
        -> <Element[capacity]>
    }
    | capacity < 4 | -> <int32[2]>
}
items <Items> : [{ -> n : 3 }, { -> n : 7 }]
debug : @"debug"
debug.print(items[2].n)
```

This prints `7`. Only the selected type is constructed; different branches need not
produce the same type or a union. Reached conditions must be boolean (E215 otherwise)
and use the supported required boolean/integer operators and eligible inputs.
Every matcher is independent: an emission does not suppress later conditions or tail
statements. Two selected primary emissions report E205; no selected type emission
reports E211. The first evaluated condition/body failure retains its original span.

Each selected matcher body gets a lexical scope. Its declarations cannot escape or
replace outer bindings. Use `| condition | -> { ... }` to emit a scoped nested type
construction, as above. A standalone `| condition | { ... }` is not a transparent
parent emission and remains unsupported in this slice.

Skipped bodies are checked for supported statement forms, but their initializer and
type expressions are not resolved or evaluated. Consequently, a skipped type name
need not resolve, and a skipped nested matcher condition is not read. Supported body
forms are immutable bindings, local type aliases, primary type emissions and nested
matchers. In type/scalar result blocks, labeled/named/annotated emissions remain gated.
Assignment, mutable bindings, standalone expression statements and fallback arms remain
gated in all required blocks, including skipped bodies.

Reached conditions retain their work even when false. Selected statements and nested
type construction share the enclosing work/depth/node budgets; skipped bodies add no
evaluation work. Structural matcher checks have bounded depth/work and consume frontend
checking work. Independent roots reset their evaluation budgets. These bootstrap
limits report B001, not full-language E220.

Selected declarations retain checked documentation and derived signatures. Documented
declarations in skipped bodies remain unsupported with B001 for an unanalyzed declaration;
the compiler does not claim their signatures or links were checked.

Selected types can be exported and used through module facades or in function-local
required blocks. Runtime capture rules, file graph discovery and eager module
initialization remain unchanged. Checking/building never run application initializers,
and a skipped required branch does not remove a runtime startup failure.

## Annotated scalar blocks

An explicit integer or `boolean` annotation makes a block binding inside required
evaluation produce that scalar kind:

```meowy
<Items> : {
    ready <boolean> : { -> true }
    count <uint8> : {
        base <uint8> : { -> 2 }
        | ready | -> { -> base * 2 }
        | !ready | -> 2
        unused : 7
    }
    -> <int32[count]>
}
items <Items> : [3, 7]
debug : @"debug"
debug.print(items[2])
```

This prints `7`. The annotation supplies the result kind and integer width. Emitted
literals use that context; already typed values keep their own width and cannot be
silently converted. A missing scalar primary reports E204, a second selected primary
E205, an incompatible scalar kind/width E207, literal overflow E216 and invalid
arithmetic E107. Boolean results retain true/false values without integer encoding.

Unannotated blocks infer their selected scalar, type or record result, as described
below. Directly emitted nested blocks inside an annotated scalar block inherit that
block's expected kind and width. A separately bound unannotated block infers its own
result. Local type aliases and immutable scratch remain available inside scalar blocks.

Scalar and type blocks share lexical scopes, matcher selection and root budgets.
An emission does not exit the block: selected tail statements still contribute work
and failures. Skipped matcher expressions are not evaluated, with the same structural
statement restrictions as conditional type selection. Standalone branch blocks do not
emit into their parent. Bindings and local type aliases do not escape the block.

Each evaluated initializer source retains transitive work and original errors, including
reads in unused tails and nested blocks. Independent roots reset their budgets. Results
are compile-time values with no runtime locals, HIR statements or functions. Selected
documentation retains the declared widths and resulting constructed type signatures.

Eligible local/field/module inputs work in function-scoped required blocks without
allowing runtime captures. Checking/building remain silent and preserve eager module
initialization, including failures behind skipped required reads. Float/text results,
mutable scratch and non-scalar operand results remain unsupported.
Scalar result blocks do not accept named field emissions.

## Inferred scalar blocks

An unannotated immutable block binding in required evaluation can produce an integer
or boolean from its selected primary:

```meowy
<Items> : {
    ready : { -> true }
    count : {
        base <uint8> : 2
        | ready | -> { -> base * 2 }
        | !ready | -> 2
    }
    -> <int32[count]>
}
items <Items> : [3, 7]
debug : @"debug"
debug.print(items[2])
```

This prints `7`. Here `ready` is boolean and `count` retains the selected `uint8`
value. A bare integer literal without an expected type uses its ordinary default width.
Already typed inputs keep their widths; assigning the result to another width does not
silently convert it. Nested primary blocks and groups retain the same inference rules.

Exactly one selected primary supplies the result. Two selected primaries report E205,
including conflicting kinds. Skipped expressions do not supply a result or charge their
evaluation work. A block with no selected result remains unsupported and reports E211;
explicitly annotated scalar blocks retain their E204 missing-result check. Literal range
errors remain E216 and evaluated arithmetic failures retain E107 and original spans.

Type-producing blocks retain type values, and type aliases still require a type result.
Named fields can infer a unit-primary record; records with integer/boolean primaries
remain gated. A nested unannotated block may also supply an inferred scalar record field.
Float/text/null results, mutable scratch, fallback arms, skipped documented declarations
and non-scalar operand results remain separate.

Inference shares the existing required scopes, source eligibility and work/depth/node
budgets. Original source reads retain their work and errors, while aliases of materialized
results reuse checked values. Independent roots reset budgets. No runtime locals are
created; function capture restrictions and ordinary module startup are preserved.

## Integer block operands

Integer blocks can be operands of required arithmetic and bitwise expressions:

```meowy
<Items> : {
    count <uint8> : ({
        base <uint8> : 2
        -> base
    }) + { -> 2 }
    -> <int32[count + ({ -> 0 })]>
}
items <Items> : [3, 7]
debug : @"debug"
debug.print(items[2])
```

This prints `7`. Supported operators are unary `-`/`~` and binary `+`, `-`, `*`, `/`,
`%`, `&`, `|`, `^`. Operand blocks use the surrounding expected integer width when
available; otherwise their result is inferred. Already typed operands retain their
widths. Context follows ordinary operand hints and left-to-right evaluation; a block
is never evaluated early to discover its type.

Each selected block executes once during required evaluation, including its local
bindings, matcher conditions and tail statements. Its scope ends before the next
operand is evaluated. A failure in the left operand prevents evaluating the right;
arithmetic and ancestor initializer failures retain their original spans. Checked
operator rules, literal range checks and unsigned-negation restrictions are shared
with ordinary scalar checking.

Blocks and operators share the existing required work/depth/node budgets. Repeated
reads of an original source retain its work; materialized aliases reuse values.
Skipped required matcher bodies remain unevaluated, with existing structural and
skipped-documentation gates. No runtime locals or statements are introduced.

The same expressions can supply list extents inside an active required root, as in
the computed type block above. Negative extents remain E104 and bootstrap capacity
limits still apply. This does not enable block extents in a direct type annotation
outside such a root. Noninteger arithmetic block results, helpers, mutable scratch
and records with scalar primaries remain separate.

## Annotated record construction

A record annotation supplies the complete immutable shape for a required constructor:

```meowy
<Settings> : <{ enabled <boolean>; part <{ capacity <uint8> }> }>
<Items> : {
    settings <Settings> : {
        -> part : { -> capacity : 4 }
        | part.capacity >= 4 | -> enabled : true
        | part.capacity < 4 | -> enabled : false
    }
    | settings.enabled | -> <int32[settings.part.capacity]>
    | !settings.enabled | -> <string>
}
items <Items> : [3, 7]
debug : @"debug"
debug.print(items[2])
```

This prints `7`. Supported shapes are nonempty, have an implicit unit primary, and
contain only immutable integer, boolean or nested record fields. The existing
256-total-field and 32-record-level bounds apply. Empty, mutable, nullable, float,
text, list and reference-bearing shapes remain outside this slice.

Selected named emissions must match expected fields. Unknown fields or incompatible
field annotations/values report E207; literals inherit the field width, preserving
E216 for unrepresentable values and E107 for evaluated arithmetic failures. A second
selected emission into a field reports E205. Every expected field must be initialized
when the block finishes (E204 otherwise). The unit primary is implicit unless an
eligible record is composed through the primary slot, as described below.

A field name becomes a local binding after its initializer completes, so later fields
can read it. Ordinary lexical scope still applies: a name emitted inside a matcher
stays in that matcher's scope. Constructor fields remain recorded across selected
branches, and completed records expose all fields through normal field selection.
Nested record/scalar field blocks inherit their field's expected type. An existing
eligible record or required record alias can initialize a nested record field when
its complete type matches.

Emissions do not exit the block. Selected tail statements retain their work/errors;
skipped matcher initializers and annotations are not resolved or evaluated. Skipped
emissions do not initialize fields. Structural matcher guards still reject mutable,
labeled and unsupported statement forms. Unannotated block bindings remain type-producing.

Construction produces only a compile-time record, without runtime locals,
statements or functions. Every record construction/copy charges its type shape to the
shared node budget; evaluated fields and source copies retain required work/error rules.
Documentation derives completed field widths and constructed type signatures. Required
function-scoped reads keep runtime capture gates, and checking/building never run or
remove module initialization, including failures behind skipped required reads.

### Primary record composition

An annotated required constructor can compose an existing eligible record by name or
field path, including aliases and exported subrecords:

```meowy
base : { -> width <uint8> : 4 }
<Settings> : <{ enabled <boolean>; width <uint8> }>
<Items> : {
    copy : base
    settings <Settings> : {
        -> copy
        -> enabled : true
    }
    | settings.enabled | -> <int32[settings.width]>
    | !settings.enabled | -> <string>
}
items <Items> : [3, 7]
debug : @"debug"
debug.print(items[2])
```

This prints `7`. Source fields map by name into the expected shape, with exact types
and widths. A source may initialize some or all fields; remaining fields still need
named emissions. Unknown or incompatible fields report E207, duplicate fields E205
and fields missing at completion E204. Nested record fields retain their complete shape.

Composition also initializes the unit primary. A second selected composition reports
E205 even if its fields are disjoint. Complementary matcher branches can select one
source; nested constructors have independent primary slots. Emission still does not
exit the block, so selected tails retain their work and errors.

Forwarded field names do not become local bindings. Inside the constructor, read
`copy.width` in this example; after construction, read `settings.width`. Explicit named
emissions still introduce their own local bindings after initialization.

The source's complete evidence is checked and materialized once before forwarding.
Ancestor failures cannot be hidden by partial composition. Each forwarded field adds
one evaluation step; copied nested records also charge their type shape. The source
and final record retain the existing materialization budgets. Reusing a required record
alias does not reevaluate its original initializer; independent roots reset budgets.

Composition creates no runtime locals and works in function-scoped required evaluation.
Checking/building preserve silent staging and normal module initialization. Annotated
primary emissions, nonrecord primaries and whole-module namespaces remain unsupported;
use eligible named record exports instead.

### Inline partial composition

An inline block can supply part of an annotated required record. Its fields inherit
the surrounding expected types, including integer widths:

```meowy
<Settings> : <{ enabled <boolean>; width <uint8> }>
<Items> : {
    settings <Settings> : {
        -> {
            base <uint8> : 2
            -> width : base * 2
        }
        -> enabled : true
    }
    | settings.enabled | -> <int32[settings.width]>
    | !settings.enabled | -> <string>
}
items <Items> : [3, 7]
debug : @"debug"
debug.print(items[2])
```

This prints `7`. The inline source contains only its emitted fields; `enabled` is
required when the outer constructor completes. Nested named record fields must still
initialize their complete expected shape. Unknown fields, incompatible annotations,
range errors and duplicate fields retain the same checks as direct named emissions.

Groups and nested composition are supported, including an inline source that composes
an existing record. Each block has its own primary slot; two selected compositions in
one block still report E205. Local bindings and emitted field names are visible inside
their source scope, but forwarding does not declare them in the receiving scope.

Selected matcher bodies contribute their emitted fields. Skipped sources and field
initializers do not run or charge evaluation work; existing structural and skipped
documentation gates still apply. A source with no selected named fields remains gated,
even if the outer record could otherwise be completed.

Source construction and forwarding share the enclosing evaluation budget. Temporary
records charge only their actual field shape, while the completed outer record charges
its full shape. Nested sources share depth limits; partial construction cannot bypass
the expected record's field-count or shape limits.

## Inferred record construction

An unannotated immutable binding inside required evaluation can construct a record
from named emissions or compose an eligible record source:

```meowy
<Items> : {
    kind : { -> <uint8> }
    base : { -> width <(kind)> : 4 }
    settings : {
        -> base
        -> enabled : true
    }
    | settings.enabled | -> <int32[settings.width]>
    | !settings.enabled | -> <string>
}
items <Items> : [3, 7]
debug : @"debug"
debug.print(items[2])
```

This prints `7`. `kind` remains a type value, used through the computed annotation
`<(kind)>`. `base` and `settings` are records with implicit unit primaries. Type aliases
such as `<Items>` retain their type-only result contract. Mixing a type-valued primary
with record fields reports E211; a type value cannot become a record field either.

Fields keep their inferred integer widths, boolean kinds and nested record shapes.
A field annotation supplies the expected type, including for nested scalar/record
blocks. Without one, integer literals use their ordinary default type; inference does
not widen already typed values. Field order does not change record identity or values.
Explicit named emissions declare local names after initialization; composed fields
retain their source names without declaring them in the receiving scope.

Only selected required matcher paths contribute fields. Skipped initializers and nested
sources are not evaluated. A field present only in a skipped branch is absent from
this required result, and reading it reports E201. Duplicate selected fields or primary
compositions report E205. Empty results, records with scalar primaries, mutable fields,
unsupported field kinds, fallback arms and skipped documented declarations remain gated.

Nested records, grouped blocks, existing record sources and inline composition share
the same scope and evaluation budgets. Materialized aliases reuse checked values while
new reads of an original source retain its complete work and errors. The existing
256-field/32-level shape limits still apply. No runtime storage is introduced, and
checking/building preserve module startup behavior and function capture restrictions.

## Required record scratch

Required scopes can bind already eligible immutable records and exported subrecords,
then copy or project them without runtime storage:

```meowy
settings : {
    -> enabled : true
    -> limits : { -> capacity <uint8> : 4 }
}
<Items> : {
    copy : settings
    limits : copy.limits
    alias : limits
    capacity <uint8> : {
        record : alias
        -> record.capacity
    }
    | copy.enabled | -> <int32[capacity]>
    | !copy.enabled | -> <string>
}
items <Items> : [3, 7]
debug : @"debug"
debug.print(items[2])
```

This prints `7`. Integer widths, boolean values and nested field shapes are preserved.
Optional annotations must match the record's complete type (E207 otherwise). Type
queries inspect the retained shape. A record value itself is neither a type nor an
integer extent, and whole-record equality remains outside required evaluation.

Binding an original source checks its complete initializer evidence, including ancestor
work/errors outside a projected subrecord. Mutable descendants, unsupported siblings
and effectful tails prevent eligibility; a valid leaf cannot hide them. Original
failures retain their source spans. Unrelated module initialization effects do not
disqualify a separately eligible exported record.

A successful binding materializes the record in the required scope. Later aliases and
field reads charge their own read/path work without reevaluating that initializer.
Binding the original source again charges its retained work again. Every record binding
also charges its retained type shape to the existing 16384-node budget, including copies;
independent roots reset their budgets. Existing 256-total-field and 32-record-level
limits remain. These bootstrap checks do not qualify the full-language E220 counters.

Record scratch works inside nested type/scalar blocks, matcher bodies and function-scoped
required reads. Normal lexical shadowing and scope exit apply. Documentation retains the
record shape and scalar widths. Required aliases do not enable ordinary runtime captures,
and checking/building never execute initialization or remove runtime startup failures.

Unannotated record construction, mutable scratch, borrowed fields and whole-module
namespace records remain separate. Use an eligible named record
export such as `module.settings`; copying the module identity itself does not materialize
its namespace as a record.

## Integer calculations

Local integer bindings retain their checked width and signedness through aliases,
subsequent calculations and type queries. Supported expressions are integer literals,
eligible names, parentheses, unary `-`/`~` and binary `+`, `-`, `*`, `/`, `%`, `&`, `|`,
`^`, plus [integer block operands](#integer-block-operands) inside required evaluation.
They reuse the scalar checker and constant evaluator. Incompatible widths use
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

This prints `true`. Outer operand names, field paths and known types are checked
before evaluation, including skipped operands. Operand-block initializers follow the
deferred rules below. Unknown/private outer names and known incompatible operand kinds
still fail. Logical operators evaluate left-to-right and skip unnecessary
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
Direct calls and unsupported operators remain unavailable as outer operands even
in skipped expression syntax. Supported operand blocks defer their local initializers.

### Logical block operands

Required `!`, `&&`, `||` and matcher conditions accept boolean-result blocks:

```meowy
<Items> : {
    ready : !({ -> false }) && ({ -> true })
    skipped : false && ({ unused : 1 / 0; -> true })
    | ({ -> ready }) | -> <int32[4]>
    | !ready | -> <string>
}
items <Items> : [3, 7]
debug : @"debug"
debug.print(items[2])
```

This prints `7`; the skipped block is not evaluated. Use `!({ ... })` to negate
a block result. `!{ ... }` retains the language's unchecked-block meaning.

Structural validation checks supported statement forms and labels without resolving
block-local values or annotations. Selected blocks execute once with boolean context,
including nested primaries and tails. Empty selected results report E204, duplicate
primaries E205 and wrong scalar kinds E207. Skipped initializers add no evaluation
work or constructed type nodes. Local names do not escape the block.

Logical short circuits, source failures, shared budgets and module startup retain
their existing behavior. Named/outer emissions, mutation, fallback arms and skipped
documented declarations retain their gates. A boolean block in a matcher condition
uses this same checking and execution path.

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

For comparisons without inline block operands, operand types and literal ranges are
checked before evaluation, even when a logical operator skips a comparison. Skipped
comparisons read no initializer evidence, charge no evaluation work and perform no
arithmetic. Evaluated operands use the existing
required integer checks from left to right; a failing left operand stops the right
read. Each source read retains its original error span and transitive work.

Mixed-module primaries work in comparisons and arithmetic; two complete records still
cannot be compared in required blocks. Required reads inside functions do not grant
runtime captures. Checking/building remain silent and preserve module initialization.
Float/text comparisons and direct calls remain separate capabilities. Integer block
operands use the deferred checks described below.

### Integer block comparisons

Required integer comparisons accept inline blocks and arithmetic containing them:

```meowy
<Items> : {
    ready : ({
        base <uint8> : 4
        -> base
    }) >= 4
    skipped : false && (({ -> 1 / 0 }) == 0)
    | ready | -> <int32[4]>
    | !ready | -> <string>
}
items <Items> : [3, 7]
debug : @"debug"
debug.print(items[2])
```

This prints `7`; the skipped division is not evaluated. All six integer relations
(`==`, `!=`, `<`, `>`, `<=`, `>=`) retain boolean results. Each selected operand is
materialized once, left to right. A failing left operand prevents evaluating the right,
and block-local bindings leave scope before the next operand.

Before evaluation, bounded structural checks validate known outer operands and the
supported statement forms of each inline block. Block-local initializers, annotations
and emitted values are resolved only when the comparison runs. If a block's width is
not yet known, dependent literal range checks also wait. A known outer name or type
error is still diagnosed even when the comparison is short-circuited.

For example, skipping a comparison against a block that computes a `uint16` result
also skips a literal range check that depends on discovering that width. Evaluating
that comparison against `65536` reports E216. Already known incompatible widths remain
E213; an evaluated block that cannot satisfy its expected width reports E207. No value
is widened to make a comparison fit.

Skipped blocks contribute no evaluation visits or constructed type nodes. Selected
blocks retain source work, tail errors, shared budgets and original diagnostic spans.
Unsupported statement forms, fallback arms and skipped documented declarations retain
their gates. Ordered comparisons require integers. Equality can instead select
boolean operands as described below; whole-record comparisons and helper evaluation
remain separate.

### Scalar block equality

Required `==` and `!=` accept integer or boolean block results, including a block
on either side of an ordinary scalar operand:

```meowy
<Items> : {
    enabled : { -> true }
    same : ({ -> enabled }) == ({ local : false; -> !local })
    different : ({ -> false }) != true
    | same && different | -> <int32[4]>
    | !(same && different) | -> <string>
}
items <Items> : [3, 7]
debug : @"debug"
debug.print(items[2])
```

This prints `7`. Known outer types provide context; otherwise the left operand's
selected result determines the scalar kind and the right operand must match it.
Integer widths remain exact. The checker never executes a skipped block to guess
its kind, and booleans are never encoded as integers for equality.

Equality evaluates both operands once, left to right. A `false` left value does not
skip the right side; only a failure stops evaluation. An enclosing `&&` or `||` can
skip the entire comparison, in which case block-local initializers and unresolved
kinds remain unevaluated. Ordinary known outer name/type checks still apply.

Known incompatible scalar kinds report E222, known integer-width mismatches retain
E213, and an evaluated operand that does not satisfy its scalar context reports E207.
Ordered comparisons remain integer-only. [Type-producing blocks](#type-producing-block-operands)
use normalized type identity. Empty results, record-valued operands, float/text
comparisons, helpers and borrowed storage remain separate capabilities.
Source work/errors, aliases, lexical scope, documentation and module/function staging
follow the same rules as the existing required operand paths.

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
booleans as immutable scratch and evaluate explicitly annotated boolean block bindings.
Required operand-block execution follows the rules above; it does not broaden
eligibility for arbitrary runtime initializers.

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
mixed-module extent. Whole-module record scratch remains unavailable; eligible named
record exports can be materialized separately.

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

A separate logical ledger now charges required statements, scalar evaluation,
record reads/copies and type-expression dispatch. Type literals, type-value reads,
queries and subtraction each charge an expression step; member reads also charge
their evaluated ancestors. Blocks charge at entry. Parentheses and synthetic
subtraction wrappers add no logical steps or type nodes. Type-query operands and
skipped branches remain unevaluated. Existing bootstrap traversal guards still apply.

For example, `<int32> !<null>` and `(<int32>) !<null>` each currently charge six
logical steps and three type nodes. Repeated reads and constructors charge again;
logical exhaustion retains the outer required root.

Within required evaluation, type literals, aliases and binding/field annotations
now charge source construction before normalization. `<int32><int32>` charges
three type nodes (the union and both inputs), even though its result is `int32`.
Composite nodes charge before their children; implicit record primaries and each
substitution of a named payload also count. Construction stops at the first error,
retaining the work already performed. Array extents evaluate once per occurrence;
constructing a list type does not construct its elements as values.

Lookup and form checks do not enter this source-construction path. Completed
literal/alias results retain bootstrap traversal without charging their logical
construction twice.

Ordinary aliases and data binding/emission annotations create a logical root at
their type expression. Their nested constructors, substitutions and list extents
share that root. Computed type operands temporarily enter required input mode and
restore ordinary mode afterward, including on failure. Independent declarations
reset their counters. Runtime initializer evaluation is outside the annotation root.

Ordinary list extents without an enclosing constructor create their own root at
the extent expression. Extents inside required evaluation share its counters.
The ordinary path retains its existing syntax and constant-input rules; adding a
budget does not enable helper calls, inline blocks, member projections or retained
initializer proofs there. Reachability and required-evaluation state restore after
success or failure. An extent root charges integer evaluation, not the surrounding
list type or its elements.

Function definition headers now have one root for their written annotations,
ending before body checking. Result annotations precede parameter annotations;
forward reservations and explicit function re-export signatures have separate
roots at their function-type expressions. A reservation and its written definition
each charge their own source annotations. Omitted result annotations reuse the
reserved or inferred result without inventing construction work.

Body locals reuse the parameter types checked at declaration time. Computed
annotations do not run again after parameter names enter scope, so a parameter
shadowing an outer constant cannot change another parameter's checked type.
Computed operands share the signature budget and restore ordinary input mode.

Other type-use execution boundaries, text/helper counters and pending-query budget
retention still need integration.
This ledger does not yet qualify the full required-evaluation contract or execute
proof queries.

Scalar scratch beyond integers/booleans, mutable scratch, restarts, labeled blocks,
annotated or named emissions, general expression statements and source/helper calls
remain unsupported in type blocks. `core.Type` parameter/result annotations, generic
specialization, full purity analysis, intrinsic descriptions and the
complete required evaluator remain separate. Runtime type-value storage remains gated.

See [the runnable example](../examples/computed-types.mwy), [STATUS.md](../STATUS.md),
the [language contract](../../docs/reference/compile-time.md), and the
[implementation pipeline](../../COMPILER.md#the-pipeline).
