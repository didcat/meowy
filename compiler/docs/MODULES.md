# Relative file modules

The driver supports a bounded executable slice of
[the module contract](../../docs/reference/modules-and-ffi.md). Literal imports are
discovered throughout the parsed AST. An immutable, unannotated binding can name
an exact relative `.mwy` module:

```meowy
settings : @"./settings.mwy"
d : @"debug"
d.print(settings.port)
```

```meowy
private : 7
-> port : 8080
-> label : "local"
```

The second file exports only `port` and `label`. Ordinary bindings remain private.
A primary emission exports the module's primary value. Exported data may be
immutable reference-free scalars, records or bounded lists; no mutable descendants,
references, unions, owning values or foundational resource values are enabled.
A module alias is a compile-time identity, so aliasing or importing it again does
not rerun initialization or create another module-storage binding. Reading an
exported Copy value retains normal value-copy semantics.

## Annotated function exports

Named top-level function emissions now export the existing checked function identity:

```meowy
-> increment <int32> : (n <int32>) { -> n + 1 }
```

Parameters and results must be annotated. Missing result/signature annotations
report E214. Definitions can call private helpers and recurse using ordinary
function rules. Imported calls and immutable aliases reuse the original function ID;
no function pointers, closures or callable record storage are introduced.

A facade can re-export an existing function under a fresh name with its complete
function-type annotation:

```meowy
ops : @"./ops.mwy"
-> increase <(int32) -> int32> : ops.increment
```

The annotation must match all parameter and result types exactly (E207 otherwise).
An explicitly annotated export can expose a private function whose result was
inferred internally. An existing local name still cannot be redeclared (E203).
Data and function exports share the value namespace; repeated exports and conflicts
introduced by unnamed record emissions report E205. Unreachable data emissions
retain their existing behavior. Function equality remains E222; canonical identity
is preserved internally through imports and typed re-exports.

Only unconditional top-level function exports are supported. Nested or matcher-arm
exports remain gated. Private helper names are not visible through an import.
Calls retain existing argument checking, exclusive permissions, all-input returned
reference bounds and local-storage escape checks. A function may return a borrow
of its input; exporting a function does not export a borrow of module storage.
Runtime module-data captures and package policy remain separate.

The [facade example](../examples/function-modules/main.mwy) mixes data exports,
recursive functions and shared/exclusive calls. It prints ops, api, main, 1, 7, 4,
10, demonstrating dependency initialization before calls. Imported function panics
retain the callee's file and local span, including through facade aliases.

## Exported type aliases

`-> <Name> :` declares a top-level exported alias in the separate type namespace:

```meowy
-> <Point> : <{ x <int32>; y <int32> }>
```

Importers use `<geometry.Point>` in type positions. A facade can re-export the
same type with `-> <Point> : <geometry.Point>`. Private aliases can define the underlying
type without making their private names visible. Missing or private imported type
names report E202; type declarations do not create values in the value namespace.
A type and a data/function export may intentionally use the same name.

Aliases remain transparent: record field order is normalized, permissions remain
part of the type, and aliases of one primitive/record type remain interchangeable.
Existing nominal foundation types retain their original identity; exporting an alias
does not make them constructible as lookalike records. No wrapper or layout change
is introduced. Supported callable-signature aliases can annotate function re-exports;
this does not enable stored function pointers.

Existing computed type values still work, for example
`token : <geometry.Point>; -> <PublicPoint> : token`. Qualified type references use the
angle-bracket type context. [Straight-line computed type blocks](COMPUTED_TYPES.md)
can also construct an exported alias using local type bindings and a primary emission.
General helper evaluation remains unsupported. Type lookup and copied signatures
charge the existing proof budget.

Duplicate type bindings, including attempts to redeclare a private alias under the
same name, report E203. Exported names must be unqualified. Nested, conditional and
labeled type exports remain gated. Types may be used in function signatures/bodies
without granting runtime access to module data. Reference, mutability and ownership
rules remain those of the underlying type; borrowed module values and unsupported
owning storage remain unavailable.

Standalone docs attach to the exported alias as a type declaration, keep its checked
signature and enforce the existing required-doc policy. Ordinary graph compilation
also checks documented aliases in their file scopes. The
[typed geometry facade](../examples/type-modules/main.mwy) demonstrates exported records,
data and functions and prints geometry, 10, 0.

## Documentation in source graphs

Ordinary `check`, `build` and `run` validate documentation in every loaded file,
including dependencies imported only by unused functions or inactive branches.
Each model uses its own source snapshot and lexical scope. Module links resolve
at the end of that file; function docs can link parameters but not body locals.
Attachment, normalized Unicode/CRLF maps, checked signatures and E801-E803 retain
file-local diagnostics. Documentation analysis keeps the existing per-file budgets.

Links such as `[[counter.inc]]`, `[[counter.defaults.start]]` and
`[[<counter.Count>]]` follow the actual exported value/type namespaces, including
facade aliases. Private or missing members fail with E802. For multi-file checking,
explicit exports and their exposed members/parameters are public; ordinary bindings
and type aliases remain private. Public docs cannot expose private local targets.
Cross-file link checking does not create public index identities or rendered URLs.

The [documented facade example](../examples/documented-modules/main.mwy) prints
`counter initialized` once, then `8`, in both profiles. Documentation does not run
initializers during checking/building or execute embedded examples. Ordinary compile
checks example metadata; standalone `doc check`/`doc build` retain their separate
example checking and explicit execution policy.

`doc check` and `doc build` still reject relative file imports with B001. Multi-file
site generation, public coverage/index publication and example graph resolution
remain separate work. The existing single-file public-coverage policy is unchanged.

## Resolution and initialization

Relative paths resolve from the importing file, with exact extensions and `/`
separators. Canonical filesystem paths, including symlinks, identify each module
within this standalone context. Repeated imports and diamonds share one snapshot
and initializer. Each source is parsed separately; source text is not concatenated.
The graph's dependency order assembles isolated AST blocks and internal bindings,
then passes the complete program through the existing checker, ownership passes
and backend. Private lexical scopes remain separate.

Discovery visits statement, expression and type operands, including function bodies,
matcher branches, list/index expressions, annotations and string interpolation.
Comments and ordinary string text are not imports. Import sites are ordered by
source position, independent of AST traversal order. Inactive branches and unused
functions still contribute dependencies: this is a static graph, not lazy loading.

Dependencies initialize before their importer, with siblings visited in source
order, even when an import appears after an ordinary statement. All dependencies
initialize before the entry body. Cycles report E502 with the ordered path chain
and closing import site; nested or inactive imports cannot hide a cycle. Missing
files, directories and malformed relative paths report E501, including imports in
unused code. No working-directory fallback, extension guessing, network access or
package resolution is added.

A function-local import binds a compile-time module identity. Exported functions
and type names are available there without capturing the module's runtime storage.
Private members, runtime module-data captures and references to module storage keep
their existing gates. Inline imports can select supported members directly, such
as `(@"./ops.mwy").increment(7)`. Discovery does not make unsupported type evaluation
or mutable/annotated module-identity bindings valid.

The [scoped-import example](../examples/scoped-imports/main.mwy) prints side, ops,
entry, 8, again 9. It demonstrates initialization from an unused function and an
inactive branch, function-local type/call use and repeated inline imports.

Initializer failure stops startup before dependent/entry effects. Export restrictions
avoid claiming module resource cleanup or borrowed static exports. The
[diamond example](../examples/modules/main.mwy) prints shared, left, right, entry, 17;
the shared module initializes once in both profiles.

## Required initializer inputs

Eligible immutable named data exports can supply computed-type scratch and extents.
Public field lookup retains the original checked initializer identity, exact integer
widths and complete record ancestor evidence through copies, projections and re-exports.
Private dependencies stay private. Unrelated module effects do not disqualify a pure
export, but effects within its initializer do. Check/build never execute initialization;
ordinary startup ordering, definite initialization and runtime capture gates remain.
Scalar integer modules additionally expose eligible direct primary emissions as
required inputs through the module name. Aliases, arithmetic copies and primary/named
re-exports retain their original emission evidence. Runtime HIR and constant folding
remain unchanged. Modules with named fields also supply direct integer primaries
through arithmetic and integer-annotated required scratch. Ordinary aliases and type
queries preserve their complete record type; named initializer effects remain separate
from primary eligibility. Direct top-level compositions such as `-> source` preserve
each eligible runtime export's original evidence through facade chains. Each hop
adds retained work; projected records keep complete ancestor evidence. Function/type
exports still require explicit re-exports. Whole-module record inputs and conditional
export inputs remain separate.

Eligible named boolean exports, boolean record leaves and direct boolean primaries
can supply predicates in ordinary scalar/record initializers. Boolean operations
project a mixed module's primary; identity aliases and type queries keep its named
fields. Copies and facade forwarding retain values, failures and evaluation work.
Required boolean scratch and ordinary runtime module-data captures remain unavailable.
See [the supported input rules](COMPUTED_TYPES.md#imported-immutable-inputs).

## Source identity and output protection

Every file retains its original bytes and a disjoint range of internal span offsets.
Lexer/parser errors and semantic errors are mapped to the originating canonical
file, local byte range, line and column. Interpolation and Unicode spans retain the
same mapping. Existing one-file library compilation remains available without a
filesystem context. The driver snapshots inputs before checking; build/IR outputs
cannot replace any loaded source, including a canonicalized alias.

Multi-file native P001/P002/P003/P006 failures use the canonical source file and
local half-open byte range, including failures in module initializers or the entry:

```text
panic[P001]: index 2 is outside initialized length 1 at "/src/value.mwy" bytes 20..29
```

File labels are quoted; quotes, backslashes and control bytes are escaped while
UTF-8 names remain readable. Labels are embedded in the executable, deduplicated
per source, and require no runtime filesystem access. One-file programs retain the
existing `at bytes START..END` format. Initializer failure still prevents dependent
and entry effects. Nested failures retain the actual failing site, and copied panic
evidence carries the label into the existing cleanup-failure report.

The backend accepts an optional source-range table. It requires ordered disjoint
ranges, at most 64 labels and 1 MiB of total label bytes; every mapped runtime site
must fit within one range. Invalid tables/sites fail lowering instead of guessing
a file. Compiler diagnostics keep their existing file/line/column mapping. This
is a private bootstrap diagnostic format, not public replay/source-map artifacts
or complete native stack diagnostics.

## Limits and remaining gates

Graphs are bounded to 64 files, 32 active dependency levels, 4096 import edges,
4 MiB per source and 16 MiB of snapshot span space. Discovery additionally allows
at most 262144 queued AST nodes per file and rejects duplicate import source sites.
A budget failure returns no partial discovery result. Export shapes reuse the bounded
256-part/32-level reference-free shape check. Existing parser, type and ownership
budgets still apply. Exhaustion reports B001; it never admits an incomplete graph.

The entry's existing manifest refusal remains. `--standalone` explicitly bypasses
that entry policy without loading configuration. Relative imports cannot enter a
different nearest manifest context or import `mod.mwy` as executable source. This
is not a package identity or manifest implementation. Bare package names, path
aliases and remote dependencies remain gated; foundational lookup is unchanged.

Mutable or annotated module-identity bindings, references to module storage,
runtime module-data values in function bodies remain B001.
Copying an exported value into a local uses ordinary local borrowing/mutation rules.

Graph/checker and native groups cover canonical diamonds/symlinks, relative
resolution, snapshots and limits, compiler error spans, privacy/export boundaries,
initialization order/failure, and source-output protection. Function-export groups
also cover exact signatures, canonical call IDs, namespace collisions, recursion,
all-input borrow bounds, exclusive arguments and callee panic attribution. Type-export
groups cover separate namespaces, transparent/nominal identity, callable aliases,
privacy, permissions, documentation roles and the typed facade. Native
execution runs in debug and release. Runtime-site groups additionally cover all
four panic codes, legacy output, escaped names, retained causes, nested failures,
source-range validation and evaluation order. The full compiler-gate result is recorded in
[STATUS.md](../STATUS.md); this is not complete language or distribution qualification.
