# gatostyle

[Documentation index](../README.md) · [Project configuration](mod.md) · [CLI guide](../cli/README.md)

**Many ways to write it. Your project gets a say.**

gatostyle is meowy's configurable code-quality and coding-style tool. Layout is
one part of that job. It also understands declarations, types, data flow, effects,
ownership, and the different ways a meowy program can express the same operation.
A project can prefer pipelines, intermediate bindings, explicit annotations, or
almost no visual ceremony, without turning those preferences into language rules.

Run `meowy style check` to inspect the project's policy, `meowy style fix --diff`
to review its proposed changes, and `meowy style fix` to apply eligible repairs.
`meowy fmt` uses just gatostyle's layout engine. This guide defines those commands,
the configuration schema, and the guarantees behind a rewrite.

## More than indentation

These expressions can describe the same calculation:

```meowy
answer : double(increment(20))
```

```meowy
answer : 20.(increment).(double)
```

```meowy
increased : increment(20)
answer : double(increased)
```

A call-oriented project might prefer the first, a pipeline-oriented project the
second, and a teaching project the third. Gatostyle can enforce those choices
through independent rules. They are not interchangeable merely because the last
value looks equal: moving a binding can change a borrow's lifetime, when a panic
happens, or when a resource is released.

A rule therefore has two jobs: explain the desired form, and establish whether a
particular change is safe. When it cannot establish equivalence, it reports the
preference without an automatic edit. No rule turns valid meowy into invalid
meowy, suppresses a compiler error, or gives a word keyword status.

## Start with a policy

Place an optional `gatostyle` block in the project's `mod.mwy`:

```meowy
-> gatostyle : {
    -> preset : "structured"
    -> fail_on : "warning"

    -> format : {
        -> indent_size : 4
        -> line_width : 100
    }

    -> rules : {
        -> call_form : {
            -> level : "warning"
            -> prefer : "dispatch"
            -> fix : "safe"
        }
        -> bindings : {
            -> level : "hint"
            -> prefer : "named"
            -> fix : "manual"
        }
    }

    -> exclude : ["./generated", "./vendor"]
}
```

This project prefers dispatch and named intermediate steps, with four-space
indentation. Naming an intermediate result remains a human decision here; safe
call-to-dispatch changes can be applied automatically. Omitting `gatostyle` selects
the structured preset. `preset : "none"` starts with formatting and every rule
disabled, so a project can enable exactly the checks it wants.

All configuration entries are ordinary values emitted by a restricted
compile-time block. Pure local bindings and helpers can construct and reuse
records, just as they can elsewhere in a manifest. Policy evaluation cannot
fetch dependencies, initialize application modules, inspect environment variables,
or execute shell commands. Unknown fields, rule names, enum values, and invalid
option combinations are configuration errors (`E505`). Nothing silently falls
back to a different preference.

### Presets are starting values

| Preset         | Layout defaults                                                                    | Quality defaults         |
| -------------- | ---------------------------------------------------------------------------------- | ------------------------ |
| `"structured"` | Four spaces, spaced punctuation, 100 columns, short single-statement blocks inline | Recommended checks below |
| `"compact"`    | Four spaces, spaced punctuation, 100 columns, up to two simple statements inline   | Same recommended checks  |
| `"minimal"`    | No indentation or horizontal spacing, no width target, short blocks inline         | Same recommended checks  |
| `"none"`       | Formatting disabled                                                                | Every rule off           |

Recommended checks are `immutable_bindings`, `unused_binding`, and
`discarded_primary` at `warning`, plus `redundant_ascription` and `shadowing` at
`hint`. All other rules start off with manual fixes. Recommended repairable checks
use `fix : "safe"`; the others use `"manual"`. Choosing compact or minimal presentation does
not enable binding elimination or another expression rewrite.

For all presets, `fail_on` defaults to `"warning"`, and exclusions, overrides, and
custom rules start empty. A disabled formatter still has structured option values
available when explicitly enabled. The tables below supply defaults for options
not changed by a preset.

## Configure the layout

Every layout option lives under `gatostyle.format`:

| Field                   | Structured default    | Accepted values and meaning                                               |
| ----------------------- | --------------------- | ------------------------------------------------------------------------- |
| `enabled`               | `true`                | Boolean; participate in style checks and layout commands                  |
| `indent`                | `"spaces"`            | `"spaces"`, `"tabs"`, or `"none"`                                         |
| `indent_size`           | `4`                   | Positive integer; indentation width or tab-stop width                     |
| `line_width`            | `100`                 | Positive integer column target, or `null` for no automatic width wrapping |
| `spacing`               | Spaced baseline below | Boolean or category record, described below                               |
| `semicolons`            | `"needed"`            | `"needed"`, `"always"`, or `"preserve"` at optional statement boundaries  |
| `blocks`                | `"single"`            | `"expanded"`, `"single"`, `"small"`, or `"preserve"`                      |
| `max_inline_statements` | `2`                   | Positive integer limit when `blocks` is `"small"`                         |
| `braces`                | `"same_line"`         | `"same_line"` or `"next_line"` for multiline blocks                       |
| `chains`                | `"fit"`               | `"fit"`, `"vertical"`, or `"preserve"` for dispatch chains                |
| `blank_lines`           | `1`                   | Nonnegative maximum consecutive blank lines between statement groups      |
| `line_endings`          | `"lf"`                | `"lf"`, `"crlf"`, or `"preserve"` outside protected text                  |
| `final_newline`         | `true`                | Whether a nonempty file ends with a newline                               |
| `trailing_whitespace`   | `"remove"`            | `"remove"` or `"preserve"` outside protected text                         |

Compact changes `blocks` to `"small"`. Minimal changes `indent` to `"none"`,
`spacing` to `false`, `line_width` to `null`, `blocks` to `"small"`, and
`blank_lines` to `0`; it keeps statement-ending newlines. Other defaults remain
as shown. Integer options must fit `<uint32>`; zero is allowed only where stated.
With tabs, a nesting level is one tab and remaining alignment uses spaces.

`spacing : false` removes horizontal padding outside protected text;
`spacing : true` enables it. A record can override these individual categories:

| Category         | Sites controlled when `true`                                                       |
| ---------------- | ---------------------------------------------------------------------------------- |
| `bindings`       | Around `:`, `:=`, and assignment `=` in value declarations/statements              |
| `annotations`    | Before a declaration's type annotation                                             |
| `type_tests`     | Before the type suffix in a matcher condition                                      |
| `ascriptions`    | Before the type suffix in a value expression                                       |
| `type_arguments` | Before a generic argument list                                                     |
| `binary`         | Around binary operators, including capability constraints and function-type arrows |
| `emissions`      | After `->` and between a scope label and its emission arrow                        |
| `tasks`          | Between `>>`/`<<` and their operands, and around group submission `>>`             |
| `commas`         | After commas                                                                       |
| `matchers`       | Inside matcher pipes and between the closing pipe and its body                     |
| `blocks`         | Before a block opener and inside a nonempty inline block                           |
| `delimiters`     | Padding inside nonempty call/grouping parentheses and list brackets                |

The structured baseline sets `ascriptions`, `type_arguments`, and `delimiters`
to `false`; all other categories are `true`. A category record overlays the
inherited categories, while a boolean replaces them all. An operator internal to
a type suffix, such as union adjacency or subtraction, stays attached. Unary
operators stay attached except where `tasks` applies; field selection, dispatch,
indexing, imports, and scope member access stay attached. Inside annotations,
generic binders and named list entries use the `bindings` spacing category.

Spaces never distinguish a matcher type test from an ascription. Both of these
mean exactly the same thing:

```meowy
| value <int32> | debug.print("{value}")
```

```meowy
|value<int32>|debug.print("{value}")
```

Outside the condition, both `copy:value<int32>` and `copy : value <int32>` are
ascriptions. A compact computed annotation is `other<(value<>)>:value`.
The [syntax reference](../reference/syntax.md#angle-brackets-in-context) defines
context, including generic calls, parentheses, and call arguments.

The same distinction composes through a dispatched block:

```meowy
| t.{ -> $<MyCoolType> } <MyCoolType> | matched()
```

Inside the block, `$<MyCoolType>` is a proven ascription. Outside it, the
matcher tests the emitted value's type. The ascription needs a proof before it
executes; the outer test does not establish that proof retroactively. Gatostyle
must understand these nested contexts when changing spacing or proposing a
simplification, including the dispatch's ownership and cleanup behavior.

### Use no spaces

This policy asks for zero horizontal whitespace outside strings and comments:

```meowy
-> gatostyle : {
    -> preset : "minimal"
    -> format : {
        -> semicolons : "always"
        -> final_newline : false
    }
}
```

An example result is:

```meowy
debug:@"debug";
value<int32><null>:7;
|value<int32>|debug.print("{value}");
```

Those statements can also occupy one line. Newlines and semicolons are structural
separators; dropping all whitespace by search-and-replace is not formatting.
Gatostyle prints parsed tokens and retains a newline or uses grouping parentheses
where joining tokens would change the parse. A setting can remove padding, but
cannot split `&!` into another operation, turn a generic closer into task `>>`,
or remove a required statement separator. Strings and comments retain their
contents even under minimal style.

### Wrap structure, not bytes

Expanded blocks put each statement on a separate line. Single mode permits one
simple statement inline; small mode permits up to `max_inline_statements`.
A simple statement is a binding, assignment, expression, or emission with no
nested block, function, matcher, or label. Inline blocks must fit the width target
when one exists and contain no comments or intentional blank lines. Preserve
mode keeps the existing inline/multiline choice. These settings change layout,
never the existence of a block or an emission.

Wrapped calls, lists, and type arguments use one item per line, with their closing
delimiter aligned to the construct. No trailing comma is introduced. A wrapped
chain keeps the receiver first and then one indented dispatch per line:

```meowy
answer : input
    .(decode)
    .(validate)
    .(summarize)
```

Binary expressions continue after an operator; grouping and operand order stay
unchanged. Long indivisible tokens, strings, and comments may exceed `line_width`.
`braces : "next_line"` cannot terminate an expression before its block: required
continuation parentheses are inserted when necessary, and the output is reparsed.
Blank padding just inside an expanded block is removed; existing statement groups
retain up to `blank_lines` empty lines. An empty file stays empty.

With `line_endings : "preserve"`, existing separators retain their form; newly
inserted lines use the first unprotected line ending in the file, or LF if there
is none. Whitespace settings cannot alter the contents of comments, strings, or
an explicitly protected region.

With `semicolons : "needed"`, remove only optional separators. With `"always"`,
terminate every complete statement, including the last statement in an inline
block; with `"preserve"`, keep optional separators already present. Record-type
fields use their own separators: insert `;` between fields if they share a line,
without treating the type's parentheses or generic arguments as statements.

## Choose how the code is written

Each built-in entry under `gatostyle.rules` accepts `level : "off"`, `"hint"`,
`"warning"`, or `"error"`, and `fix : "manual"` or `"safe"`. These override the
preset. Options in the last column below use their stated default when omitted.
`fix : "safe"` permits an edit only when that rule supplies an equivalence proof;
it does not create a repair for every finding. Report-only rules reject a `safe`
setting rather than pretending to apply it.

| Rule / code                     | What it checks                                                         | Additional options and fix availability                                                                                                    |
| ------------------------------- | ---------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| `call_form` / `G101`            | Ordinary calls versus first-argument dispatch                          | `prefer : "call"` or `"dispatch"`, default `"call"`; safe rewrites of resolved stable callees                                              |
| `bindings` / `G102`             | Intermediate names versus inline expressions                           | `prefer : "named"` or `"inline"`, default `"named"`; `min_depth : 2` counts nested calls for named form; inline form can have a safe fix   |
| `type_annotations` / `G103`     | Explicit local types versus inference                                  | `prefer : "explicit"` or `"inferred"`, default `"explicit"`; safe only when inferred types and public contracts remain identical           |
| `redundant_ascription` / `G104` | Ascriptions that add no proof or type information                      | No extra options; safe removal when generic selection and flow types stay identical                                                        |
| `immutable_bindings` / `G105`   | Mutable bindings and fields never mutated or exclusively borrowed      | No extra options; safe local tightening when no exposed type changes                                                                       |
| `naming` / `G106`               | Value, field, type, and type-parameter spellings                       | `values` and `fields` default `"snake_case"`; `types` defaults `"UpperCamelCase"`; `parameters` defaults `"UPPER_SNAKE_CASE"`; report only |
| `unused_binding` / `G201`       | Local bindings never subsequently read                                 | `ignore_prefix : "_"`; report only because an initializer or cleanup can have effects                                                      |
| `discarded_primary` / `G202`    | A non-null primary result discarded by an expression statement         | No extra options; report only; an explicitly named result documents intentional disposal                                                   |
| `shadowing` / `G203`            | A binding hides another binding in its namespace                       | `allow : ["item"]`, an exact name list; report only                                                                                        |
| `scope_depth` / `G204`          | Lexical nesting makes a path hard to follow                            | `max : 4`, a positive integer; report only                                                                                                 |
| `unchecked_reason` / `G205`     | An unchecked boundary lacks a nearby explanation                       | `require_comment : true`; report only; a comment documents a proof, never supplies it                                                      |
| `allocation` / `G206`           | Calls may allocate where the project excludes heap use                 | `allow : []`, a list of resolved callee selectors; report only                                                                             |
| `task_capture` / `G207`         | A call-form task submission captures arguments with observable effects | No extra options; report parent-side capture and child-side execution; no automatic task-block conversion                                  |

Naming options accept `"snake_case"`, `"lowerCamelCase"`, `"UpperCamelCase"`,
`"UPPER_SNAKE_CASE"`, or `"any"`. Its optional `allow : []` exempts exact spellings.
No rule automatically renames an API, changes a serialized field, or rewrites
string contents. A leading underscore remains an ordinary identifier, never a
special discard keyword.

For `bindings`, an inline candidate is a single-use immutable local with its use
in the same block. Named form reports nested calls meeting `min_depth`, counted
from the outer call as depth one; a useful name is left to the author. Scope depth
counts nested lexical block/function bodies below the file, starting at one.
Numeric rule options must fit `<uint32>`; `min_depth` and `max` must be positive.
An unchecked reason is a nonempty comment immediately preceding or first inside
the boundary. Neither comments nor a gatostyle exemption relax caller obligations.

Allocation checks inspect reachable callee effects. An unknown effect is reported
as unproven, not assumed allocation-free. An allow entry uses the same
`{ -> module : "..."; -> export : "..." }` selector as custom rules below and
exempts calls to that resolved operation only. Task capture checks report calls,
mutable reads, or other effects in captured arguments, with their actual evaluation
location; they do not complain merely because a pure value is passed to a task.

`G001` reports layout drift. Its severity follows `fail_on`, so an enabled layout
check always gates CI; disable `format.enabled` to opt out. Code quality findings
remain distinct from compiler `E...` errors and runtime `P...` panics. Turning a
style rule off never permits a borrow escape, invalid type, or missing task join.

## Make project rules

For conventions the built-in rules cannot name, `gatostyle.custom` contains named,
report-only rules over resolved source facts. For example:

```meowy
-> custom : {
    -> project_logging : {
        -> level : "warning"
        -> select : {
            -> node : "call"
            -> callee : {
                -> module : "debug"
                -> export : "print"
            }
        }
        -> message : "Route application messages through the project's logger."
    }
}
```

The finding ID is `G:project:project_logging`; renaming that key changes its
identity. Its `level` defaults to `"warning"`. `select` and a nonempty `message`
are required. Custom rules do not accept a `fix` field or replacement text.
Textual substitution cannot establish that two composable expressions mean the
same thing.

A selector is either a leaf with `node`, or exactly one combinator: `all`, `any`,
or `not`. `all` and `any` contain nonempty lists of selectors; `not` contains one
selector. All operands inspect the same candidate node. Leaves accept these
node-specific predicates, combined with AND:

| `node`      | Optional predicates                                                       |
| ----------- | ------------------------------------------------------------------------- |
| `"call"`    | `callee : { module, export }`; `allocates : true` or `false`              |
| `"binding"` | `mutable : true` or `false`; `exported : true` or `false`                 |
| `"block"`   | `unchecked : true` or `false`; `depth_above : N`                          |
| `"matcher"` | `type_test : true` or `false` for a condition containing a type predicate |

`module` is a literal import operand resolved in the declaring project's import
map; `export` is a named field path in that module. Matching uses resolved symbol
identity, including imports through aliases and ordinary function aliases, not
source spelling. Calls through an unknown function pointer are unresolved.
For allocation predicates, an unknown effect is also unresolved. Negating an
unresolved fact leaves it unresolved. A selector must be provably true to emit its
message; unresolved candidates produce `G002` at the rule's severity, with the
missing fact and candidate location. This is a policy finding governed by
`fail_on`, distinct from missing dependencies or another failed analysis input.
An enabled prohibition therefore reports uncertainty instead of silently passing.

These selectors are data: local pure functions may construct or combine them.
There is no dependency-provided executable linter or implicit plugin download.
Use built-in rules for transformations with defined proofs, and custom rules for
project-specific questions and explanations.

## Share policy and make exceptions

Configuration is resolved in this order for each source file:

1. The nearest enclosing `mod.mwy` selects a project and its preset; an optional
   CLI `--preset NAME` replaces that base preset.
2. That manifest's explicit `format`, `rules`, `custom`, and `fail_on` settings
   overlay the preset.
3. Matching `overrides` apply in list order; a later explicit leaf wins.

A CLI preset does not discard explicit project settings or overrides.

There is no parent-manifest inheritance or ambient personal configuration. Lists
replace inherited lists; records merge by field; a `spacing` boolean replaces
all spacing categories. To see the effective settings and each value's origin:

```sh
meowy style config --resolved ./cmd/cli/help/main.mwy
```

Paths need not exist for this query. It emits JSON containing the resolved policy,
origin paths, and rule identities, and never initializes or builds the program.
Version-lock the meowy toolchain in CI: the same source, dependency graph, toolchain,
and effective policy must give the same findings and fixes.

An override contains `paths` plus any of `format`, `rules`, `custom`, and `fail_on`:

```meowy
-> overrides : [
    {
        -> paths : ["./tests"]
        -> rules : {
            -> naming : { -> level : "off" }
            -> scope_depth : { -> level : "warning"; -> max : 6 }
        }
    }
]
```

`paths` and top-level `exclude` contain literal relative file/directory paths,
using `/`, with no globs or import-alias expansion. A directory includes its
descendants. Normalize `.` and `..` and require every path to stay within the
project root, including symlinks. Exclusions affect directory discovery; an
explicitly named file can still be checked. Overrides never cross project roots.
The [fully commented manifest](mod.full.mwy) includes every exclusion and override
setting, with examples for adapting policy to selected source directories.

### Preserve layout or suppress one finding

To protect hand-arranged source, bracket complete sibling statements with
standalone comments:

```meowy
# gatostyle: off #
mask <uint8[4]> : [0x00, 0x01,
                  0x80, 0xff]
# gatostyle: on #
```

The region retains its bytes and receives no edits, including semantic edits.
Analysis still reports quality findings there. The pair cannot nest or cross a
scope boundary, and the whole file must still parse. Unmatched or misplaced
controls are policy errors, not changes to the language grammar.

To explain one intentional exception, put a rule-specific comment immediately
before the statement it governs:

```meowy
# gatostyle: allow unused_binding -- owner deliberately lives until scope exit #
lease : acquire_lease()
```

The allowance suppresses that rule for that statement and its expression subtree,
including a nested body; it never extends to later siblings. It requires a known
rule key (or `G:project:NAME`) and a nonempty reason after `--`. Only one rule is
named per comment; several adjacent allowance comments can cover the same
statement. The allowance also prevents that rule from rewriting the subtree.

Only a whole standalone comment whose trimmed body matches a directive has this
meaning. Strings, trailing comments, and longer prose mentioning a directive do
not activate it. Unknown directives and unused allowances are reported as policy
errors rather than becoming permanent unexplained exceptions.

## Understand automatic changes

Layout reparses its output and requires the same syntax structure, allowing only
layout, optional separator changes, and grouping parentheses that preserve the
same expression tree. It preserves comments and quoted expressions byte-for-byte,
including interpolations, escapes, raw newlines, and indentation within strings.
This prevents a formatter from changing a diagnostic message or a multiline query.

Style fixes may deliberately change the syntax tree. Each safe fix must preserve:

- Resolved names and callees, exact inferred types, representation, and public API.
- Evaluation count and order, short-circuiting, observable effects, and failure paths.
- Scope targets, emissions, initialization, moves, borrows, and cleanup order.
- Task capture, scheduling boundaries, submission, joining, and cancellation.

For a direct immutable function binding, `f(value, other)` may become
`value.(f, other)` when the same callee and operands are evaluated exactly once in
the same order. A computed or mutable callee can invalidate that proof. Likewise,
`temporary : 24; -> temporary` may reduce to `-> 24` when no typing or scope fact
depends on the binding. An owner with a destructor is not that example.

These two task submissions have different behavior:

```meowy
job : >> work(next_id())
```

```meowy
job : >> { -> work(next_id()) }
```

The first calls `next_id()` in the parent before submission; the second calls it
in the child. Choosing small blocks or inline expressions cannot equate them.
Imports and matcher arms also have an evaluation order. Gatostyle never assumes
that sorting imports, combining arms, merging scopes, or removing an unused
initializer is harmless.

`fix : "manual"` is the escape hatch for a preference that needs judgment. There
is no `unsafe` switch that upgrades an unproven rewrite to an automatic one.
Changing behavior deliberately belongs in an ordinary reviewed source edit.

### Apply a coherent patch

A fix run analyzes a source snapshot and resolves the selected rules before
writing. Compatible safe edits form one patch; overlapping or incompatible edits
are reported for selection, never applied in arbitrary rule order. Semantic fixes
run before layout. The result is rechecked for language validity and the promised
proofs against the affected module graph, then analyzed for remaining findings.
A further run must not propose a cycle or reverse an accepted fix under the same
policy; conflicting policies are reported before any write.

Every selected source/configuration/dependency input is hashed. A changed input
aborts the write rather than applying a stale plan. All output files are prepared
before replacement; originals and permissions are retained in a recovery journal.
A failed batch restores earlier replacements, and interrupted recovery must finish
before a later batch writes. Files without edits keep their modification times.
A failed parse, incomplete required analysis, invalid policy, conflicting repair,
or failed proof validation writes nothing. A candidate for which no proof can be
established is simply report-only; it is not a failed patch validation.
Remaining report-only findings do not prevent a
valid safe patch from being applied, but still affect the command's exit status.

A diff preview writes nothing and is never an approval cached for a later changed
source. A subsequent write recomputes and validates its plan.

## Commands for an editing loop and CI

| Intent                              | Command                                  | Writes                       |
| ----------------------------------- | ---------------------------------------- | ---------------------------- |
| Inspect layout and quality          | `meowy style check`                      | None                         |
| Preview eligible repairs and layout | `meowy style fix --diff`                 | None; unified diff on stdout |
| Apply eligible repairs and layout   | `meowy style fix`                        | Selected source files        |
| Restrict analysis to source paths   | `meowy style check main.mwy cmd/`        | None                         |
| Learn a rule                        | `meowy style explain call_form`          | None                         |
| Inspect effective configuration     | `meowy style config --resolved main.mwy` | None; JSON on stdout         |
| Apply only layout                   | `meowy fmt`                              | Selected source files        |
| Check only layout                   | `meowy fmt --check`                      | None                         |
| Preview only layout                 | `meowy fmt --diff`                       | None; unified diff on stdout |
| Format one file into a pipe         | `meowy fmt --stdout main.mwy`            | None; source on stdout       |

`meowy style` without a subcommand means `check`. These commands accept `--help`;
`style check`, `style fix`, and `fmt` accept `--preset`. `--check`, `--diff`, and
`--stdout` are mutually exclusive modes of `fmt`; `--stdout` requires one file.
If formatting is disabled, `fmt` leaves source unchanged and reports that setting.
`style explain` accepts a rule key, built-in G code, or project custom-rule ID.

Without paths, discover `.mwy` files under the nearest project root, or the working
directory when there is no manifest. Explicit paths are relative to the working
directory. Visit directories in stable path order, include `mod.mwy`, skip VCS
metadata, root `build/`, and exclusions, and stop at nested project roots.
Explicitly selecting a nested project uses its own policy. Do not follow symlinked
directories, import aliases, or dependencies for file discovery; canonical duplicate
files are processed once. Use `--` for filenames beginning with `-`.

Quality checks resolve and type-check the module graphs needed for selected files,
using the manifest and existing lock. Dependency files supply facts but receive
no project-local style findings or edits unless explicitly selected with their own
policy. Analysis can obtain content pinned by the lock; `--offline` on `style
check` or `style fix` requires it locally. These commands never update `mod.lock`
or execute application initialization. Missing or invalid analysis inputs fail
the operation; they cannot silently skip enabled semantic rules. Layout-only
`fmt` needs syntax and local policy, with no dependency resolution or execution.

An illustrative finding shows the preference, evidence, and exact candidate:

```text
$ meowy style check main.mwy

meowy v0.0.1

warning[G101]: prefer first-argument dispatch
  --> main.mwy:4:10
   |
 4 | answer : double(increment(20))
   |          ^^^^^^^^^^^^^^^^^^^^^ call form differs from project policy
   |
   = rule: call_form; configured at mod.mwy:12
   = both callees are immutable function bindings
   = evaluation order and inferred types are preserved

safe fix:
   |
 4 | answer : 20.(increment).(double)
   |
   = meowy style fix --diff main.mwy

1 warning; 1 safe fix; no files changed
```

Use `meowy style check --offline --quiet --color never` in CI. `fail_on` accepts
`"hint"`, `"warning"`, or `"error"`; findings at or above that level fail the check.
`--quiet` hides banners and conversational hints, while keeping findings and
results. Human output goes to stderr; source, JSON, and diffs go to stdout.

| Outcome                                                                                    | Exit status                             |
| ------------------------------------------------------------------------------------------ | --------------------------------------- |
| Style check passes, or style fix leaves no findings at/above `fail_on`                     | `0`                                     |
| Style check fails, or applied fixes leave findings at/above `fail_on`                      | `1`                                     |
| Successful style-fix diff preview                                                          | `0`, even with remaining findings shown |
| Successful fmt write, source output, or diff preview                                       | `0`                                     |
| Fmt check finds layout drift                                                               | `1`; otherwise `0`                      |
| Invalid source/policy/usage, incomplete analysis, conflicting edits, failed proof or write | `2`                                     |

An empty selection succeeds with zero files. Configuration and suppressed-finding
checks still run. Layout/quality commands do not replace the last `check`, `build`,
or `run` diagnostic session, apply its error repairs, or mutate a replay capsule.
Compiler errors encountered during style analysis are printed without publishing
a new application session. Source edits may make a saved error patch stale; run
a fresh `meowy check` before applying it. Saved failures still replay the original
source and binaries. See [repair contracts](../reference/diagnostics.md#repair-contracts).

## Work with an editor buffer

For continuous editor analysis, use the [language server](../reference/lsp.md).
Its formatting and style actions use this same policy from the current `mod.mwy`
buffer, including path overrides and proof requirements. `lsp` controls feature
availability and scheduling; it cannot redefine indentation or rule severities.
`lsp.format_on_save` enables layout through the client's before-save exchange.
Semantic style changes remain explicit code actions. The commands below also
support editors that use a process per request.

For layout, send the complete buffer through stdin:

```sh
meowy fmt --stdin --stdin-filepath ./cmd/cli/help/main.mwy
```

Stdin defaults to source on stdout and accepts `--check` or `--diff` instead.
It accepts no file operands and never writes a source file. The filepath need
not exist; it selects configuration and diagnostic locations. With no path,
use defaults and `--preset`, identify the input as `<stdin>`, and do not search
for a manifest. A buffered `mod.mwy` supplies its own local policy instead of the
on-disk version. Invalid buffered configuration fails without output source.

`meowy style check --stdin --stdin-filepath PATH` analyzes a buffer as an overlay
on that file's module graph. A path is required for quality analysis; other files
come from disk. Findings include the analyzed buffer's content hash so the editor
can discard stale results. Stdin never applies semantic fixes.

Replace formatted buffer contents only on exit `0` and only if the buffer has not
changed since the request. Keep the cursor and undo history. The
[Vim/Neovim bundle](../../editor/nvim/README.md) provides syntax highlighting and
four-space editing defaults; its installation does not enable automatic fixes.
