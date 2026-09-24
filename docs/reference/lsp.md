# Language server

[Documentation index](../README.md) · [Project manifest](../guide/mod.md) · [Command line](../cli/README.md)

`meowy lsp` connects an editor to the same parser, resolver, type checker,
ownership analysis, and gatostyle policy used by the command line. It analyzes
the editor's current buffers, including unsaved changes in other modules and
`mod.mwy`. This chapter defines the server contract, configuration schema 1,
capability requirements, and behavior when analysis cannot finish.

Use it for completion, navigation, source diagnostics, type and storage
explanations, formatting, and reviewed source changes. Use `meowy check` for a
saved check session and `meowy run` for execution and runtime replay evidence.
Starting an editor session does not build, link, run, or initialize the application.

## Connect an editor

1. Select the project's meowy distribution; `meowy --version` identifies it.
2. Open the project containing `mod.mwy`, or a parent workspace containing several
   independent projects. The [sample manifest](../guide/mod.sample.mwy) includes
   a minimal `lsp` block.
3. Configure the editor's LSP client to start the executable with arguments
   `["lsp"]`, use standard input/output, and attach to language ID `meowy` for
   `.mwy` files. `mod.mwy` has the same language ID.
4. Initialize the connection with workspace folders and the client's actual
   capabilities, then synchronize documents before asking about their contents.
5. Set project preferences in `mod.mwy`; the server reloads them as it receives
   edits. Use the [checks below](#inspect-configuration-and-connection-problems)
   if a feature is unavailable.

For example, the editor launches this process and owns its streams:

```sh
meowy lsp
```

`meowy lsp --stdio` is equivalent. There is no TCP listener or separate server
binary to configure. Running this command in a terminal waits for protocol
messages; it is not an interactive shell. The process serves one editor client
and can contain several project contexts. Separate editors start separate
processes and keep independent unsaved buffers.

The editor supplies only the executable, transport, file association, workspace
folders, protocol capabilities, and user gestures such as requesting a format.
All meowy analysis, feature, save-action, and tracing settings come from `mod.mwy`.
There is no `.meowy-lsp.json`, environment-variable policy, alternate config path,
or project setting in an editor's `settings` or `initializationOptions` object.
Theme, key bindings, and whether the editor displays a panel remain editor UI.

The [Vim/Neovim runtime](../../editor/nvim/README.md) supplies file detection and
lexical highlighting. An LSP client starts the server separately; installing the
runtime does not launch a process or enable automatic source edits.

## Versions that must agree

| Version or identity            | Contract                                                                                                                                                                                        |
| ------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| meowy distribution             | Server, compiler, foundational library, and gatostyle ship as one identified distribution. The server uses its bundled components, never a second compiler found through `PATH`.                |
| `lsp.toolchain`                | Optional exact distribution version string, including any prerelease/build suffix. `null` accepts the launched distribution. No ranges, `latest`, downloads, or automatic executable switching. |
| `lsp.version`                  | Integer manifest schema version. This chapter defines `1`; it is independent of the distribution's release number.                                                                              |
| LSP feature revision           | The wire contract uses the stable LSP 3.17 feature set, with the fallbacks below. Support is negotiated through capabilities, not a numeric protocol-version handshake.                         |
| meowy extensions               | Optional extension protocol `1`, negotiated separately; ordinary LSP clients need no extensions.                                                                                                |
| Input identities               | `mod.lock`, target, source hashes, native declarations, and bundled Unicode/calendar/zone data identify analysis inputs. A matching server version does not make different inputs equivalent.   |
| Diagnostic and capsule schemas | Retain their own versions. An editor upgrade neither rewrites a saved capsule nor changes the rule meaning of an existing diagnostic code.                                                      |

An exact toolchain pin is useful when a team wants identical editor analysis:

```meowy
-> lsp : {
    -> version : 1
    -> toolchain : "0.0.1"
}
```

The release string is illustrative; select the distribution the project uses.
The pin compares that exact string; builds sharing a release label are not thereby
byte-identical. Custom builds should use distinct build suffixes. Analysis and
capture always record the actual executable/build digest as well as its version.
A mismatch produces `E507` at `lsp.toolchain`, showing required and running
identities. Semantic answers and edits for that project stop. Manifest recovery
and source syntax remain available, explicitly marked as limited. Other roots
using the running toolchain continue to work. Use separate server processes when
workspace projects pin different distributions.

An unsupported schema, missing `version` inside an explicit block, unknown field,
or invalid option is `E505`; the server never guesses a previous schema. A future
schema changes the version integer. Within schema 1, existing field meanings and
defaults remain stable; additive options may require a newer distribution, and
older distributions reject them rather than silently ignore them. Removing the
whole block selects schema 1 defaults. `lsp.toolchain` gates editor analysis; it
does not replace dependency locks or select a compiler for CLI invocations.

These choices use the protocol's [initialization and capability negotiation](https://raw.githubusercontent.com/microsoft/language-server-protocol/gh-pages/_specifications/lsp/3.17/general/initialize.md).

The initial server uses the [initial target profile](target-profile.md), the
root-authoritative [package resolver](packages-and-builds.md), and the
[artifact schemas](artifact-formats.md). Its distribution digest identifies the
entire bundled component inventory; the actual compiler digest is additionally
recorded for captures. A root lock pinned to another distribution is `E507` even
when `lsp.toolchain` is null. Missing executor requirements use `E404`, without
starting an executor or invoking module initialization during analysis.

## Configure everything in mod.mwy

This is a complete example; omitted fields take the defaults in the tables:

```meowy
-> lsp : {
    -> version : 1
    -> enabled : true
    -> toolchain : null

    -> check : {
        -> trigger : "change"
        -> scope : "project"
        -> debounce_ms : 150
    }

    -> diagnostics : {
        -> style : true
        -> max_per_file : 200
    }

    -> index : {
        -> exclude : ["./generated", "./vendor"]
        -> max_file_bytes : 4_194_304
    }

    -> features : {
        -> inlay_hints : true
        -> capture : true
    }

    -> format_on_save : false
    -> trace : "off"
}
```

The block uses ordinary well-known values, never reserved words. The
[manifest's pure evaluation rules](modules-and-ffi.md#the-project-manifest) apply.
Nested records merge with schema defaults; lists replace their default list.
No setting is inherited from an ancestor or dependency project's `lsp` block.
The complete recognized fields are listed here and in the feature table below.

| Field                      | Default                                  | Accepted values and meaning                                                                                                                                         |
| -------------------------- | ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `version`                  | `1` only when the entire block is absent | Required integer `1` in an explicit block.                                                                                                                          |
| `enabled`                  | `true`                                   | Boolean. `false` disables project language features, retaining document synchronization and manifest diagnostics needed to re-enable it.                            |
| `toolchain`                | `null`                                   | `null` or a nonempty exact distribution version string, as defined above.                                                                                           |
| `check.trigger`            | `"change"`                               | `"change"`, `"save"`, or `"manual"`; controls automatic semantic diagnostic work.                                                                                   |
| `check.scope`              | `"project"`                              | `"project"` or `"open"`; selects the graphs and diagnostic publication scope described below.                                                                       |
| `check.debounce_ms`        | `150`                                    | Integer `0..5000`. Quiet interval before a change-triggered check; `0` schedules immediately. Does not delay saves, Check project, or interactive feature requests. |
| `diagnostics.style`        | `true`                                   | Boolean. Include gatostyle findings in the selected checking scope. Does not change the policy or compiler severities.                                              |
| `diagnostics.max_per_file` | `200`                                    | Integer `1..10000`. Presentation limit after analysis, never a limit on which language rules are checked.                                                           |
| `index.exclude`            | `[]`                                     | List of relative file/directory paths excluded from background symbol discovery. No globs or import prefixes.                                                       |
| `index.max_file_bytes`     | `4_194_304`                              | Positive `<uint32>` byte limit per source buffer/file admitted to analysis. Oversized required inputs make the affected analysis incomplete.                        |
| `features`                 | Per-feature defaults below               | Record of boolean feature switches. Unknown names are errors.                                                                                                       |
| `format_on_save`           | `false`                                  | Boolean. Request layout during the protocol's before-save exchange, subject to formatting gates below.                                                              |
| `trace`                    | `"off"`                                  | `"off"`, `"messages"`, or `"verbose"`; local stderr tracing as defined below.                                                                                       |

An omitted `check`, `diagnostics`, `index`, or `features` record uses all its
defaults. Integer fields reject fractions, negative values, and overflow.
Duplicate configuration fields and wrong value types are `E505`, even inside
`enabled : false`. Setting `format_on_save : true` with `features.formatting :
false` is also `E505`; a gatostyle path policy that disables layout simply makes
that path ineligible for formatting.

`build.entry`, `build.profile`, `build.target`, `build.native`, `build.executor`,
and the [optimization settings](optimization.md#select-build-policy-in-modmwy)
remain manifest inputs. The server validates `optimize`, `cpu`, `jobs`,
`debug_info`, and `link` against the selected toolchain and current manifest
snapshot. Expanded CPU requirements and effective build policy contribute to
input identity. Editor checking does not invoke the linker or run LTO, and it
cannot promise final section sizes or retention decisions. Use
[`meowy build --report`](../cli/README.md#explain-a-builds-size-and-dependencies)
for that evidence. `build.jobs` controls builds, not editor analysis scheduling.

`import` and `mod.lock` remain the resolver inputs.
The manifest's [`test` policy](stdlib/testing.md#configure-a-run-in-modmwy)
supplies suite discovery roots and harness input identity. The server validates
its schema and descriptors; process limits, seeds, output capture, and watchdogs
remain runner settings. They neither start test processes nor change LSP scheduling.
`gatostyle` remains the only source of layout, rule severities, preferred forms,
exceptions, and proof requirements. There is no competing target, indentation,
import map, or list of disabled compiler errors inside `lsp`.

For a slower project, set `check.trigger : "save"` and `check.scope : "open"`.
Navigation requests still compute the facts they need. For fully explicit
diagnostic work, use `"manual"` and request **meowy: Check project**. These choices
control scheduling; they never allow outdated proofs to authorize an edit.

## Project ownership and file discovery

For a `file:` URI, canonicalize the file location and search its directory and
ancestors for the nearest `mod.mwy`, matching
[CLI project discovery](../cli/README.md#projects-and-entry-selection). An opened,
unsaved `mod.mwy` counts as a manifest even before its first disk save. Paths
inside it remain relative to its directory, regardless of the server's working
directory. A malformed nearest manifest still owns its files; do not fall back
to an ancestor because the current configuration is invalid.

Workspace folders choose where background discovery starts. Scan `.mwy` files
in stable canonical path order, include `mod.mwy`, skip VCS metadata and each
root's `build/`, and do not follow symlinked directories. A nested manifest starts
an independent context; its files never inherit the outer root's configuration.
Adding/removing a workspace folder or creating/deleting/moving a manifest
recomputes ownership and clears diagnostics from the old context. Two URI
spellings of the same canonical file must not create conflicting overlays;
the client must use one document identity or resolve the duplicate first.

`index.exclude` paths are normalized relative to that manifest, must remain
inside it, and cannot exclude `mod.mwy`. A directory excludes its descendants.
Exclusions skip background discovery, not imports required to understand a
program. Opening an excluded source explicitly admits that document for analysis;
it does not recursively admit its directory. Opening a file is explicit selection
for gatostyle too, bypassing its directory-discovery exclusions. Path overrides,
protected regions, and rule allowances still apply. Project style diagnostics and
edits target only selected project-owned files; imported dependencies supply facts.

Dependency graphs, alias directories outside the root, and pinned library sources
can supply semantic facts without becoming workspace search or rewrite targets.
Navigation may point into them. Cross-project references are reported with their
owning context; rename cannot silently edit them. Select a dependency as its own
workspace project to work on its source under its own manifest.

| Document situation            | Available context                                                                                                                                                   |
| ----------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| File under a manifest         | That project's aliases, lock, target, executor contract, style, and LSP policy.                                                                                     |
| Named file without a manifest | Its directory is a standalone root, with schema defaults and foundational/relative imports. The explicitly opened file is an entry, as for CLI standalone checking. |
| `untitled:` document          | Syntax, local declarations, local type facts, and default layout only. No filesystem imports, project graph, or capture; save to a path to acquire a project.       |
| Source above the size limit   | Synchronize its identity, report the limit, and withhold answers needing its contents; never parse only a prefix.                                                   |

Saving an untitled document closes that identity and opens a file URI. Loose-file
mode has no personal configuration layer: create `mod.mwy` to change its defaults.

## When analysis runs

The server keeps a source overlay for every opened document. An analysis snapshot
contains those buffers, disk contents of unopened inputs, the effective manifest,
lockfile, selected target and bundled toolchain identities. Other open modules'
unsaved changes participate in the same graph. `didClose` drops that overlay and
returns to disk contents; it does not write the buffer.

| Event                                                             | Required work                                                                                                                                                                                                    |
| ----------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Initialize/open                                                   | Discover the root, validate configuration, parse the buffer, and index declarations. Start semantic diagnostics unless the trigger is `"manual"`.                                                                |
| Source change                                                     | Parse immediately; invalidate affected semantic facts and diagnostic caches. With `"change"`, check after the quiet interval. Otherwise wait for save/Check project; client display behavior is described below. |
| Save                                                              | Check immediately for `"change"` or `"save"`; `"manual"` still waits for Check project.                                                                                                                          |
| Hover, completion, rename, code action                            | Analyze the current snapshot as needed for that request, independent of the automatic diagnostic trigger.                                                                                                        |
| Diagnostic pull                                                   | Return a completed current diagnostic snapshot, or follow the scheduling rules below. A client pull never overrides `check.trigger`.                                                                             |
| Check project                                                     | Run the configured checking scope immediately, including in `"manual"` mode.                                                                                                                                     |
| Manifest, lock, required source, or declared target input changes | Invalidate dependent contexts, requests, proofs, and caches; reload configuration immediately and reschedule according to the trigger.                                                                           |

In `"project"` scope, check the declared entry, public export graphs, configured
test suite graphs, and explicitly open source modules. Publish diagnostics
throughout those graphs. Discover test files under `test.paths` with the
[runner's path rules](stdlib/testing.md#discovery-identity-and-selection), including
canonical identity and nested-project boundaries. An absent default `./tests`
directory adds no graph and does not emit `E509` in the editor. Missing explicit
test paths still produce their configuration/resolution diagnostics.
In `"open"` scope, check the graphs needed by open sources, publishing primary
diagnostics only for open files and the manifest. Dependency failures still
appear at their importing edge, with related locations. A closed source may be
indexed for navigation without having been checked. Neither scope treats each
helper as a program entry or checks unreachable files merely because they exist.
A library with no `build.entry` is checked through its exports, discovered test
suites, and open modules. Test roots use the existing `module` coverage mode;
they are not executable application entries. Index exclusions do not remove
required suite graphs from project checking.

Checking tests validates static Suite/Case descriptors and their callback bodies,
including skipped cases. It never invokes callbacks, evaluates runtime assertions,
initializes fixtures, links harnesses, or runs watchdogs. A literal false assertion
is an intentional runtime `P005` path, not a static failing-test result in an
editor. No run-test command or test-result transport is added to extension 1;
use `meowy test` to execute the suite.

The server uses a bounded background work queue and prioritizes requests for
open documents. Required dependency content must already be present and match
the lock: editor analysis never fetches, resolves new revisions, updates
`mod.lock`, generates bindings, invokes linkers, or starts an executor. Missing
locks/content produce `E503`/`E501`; unavailable target inputs produce `E507`.
Use explicit [dependency commands](../cli/README.md#profiles-targets-and-dependencies)
to supply inputs, then recheck. Save dependency declaration changes in `mod.mwy`
before those disk-based commands. Open edits to locked dependency sources cannot
replace verified content in a consumer graph: report `E504` there. To develop the
dependency, select a local path dependency or its own project context explicitly.
Module initialization and application I/O remain outside analysis even for
compile-time configuration expressions.

Watch disk inputs as well as client document notifications. When client file
watching is unavailable, the server watches locally. If host watching also fails,
report degraded watching, rescan required disk inputs before explicit requests,
and, when allowed by the trigger, recheck on save. Do not pretend external changes
are observed immediately or override manual checking because a watcher failed.
Disk edits never overwrite an open overlay. A dirty `mod.lock` buffer is not a
dependency selection: lockfiles are consumed from disk after an explicit resolve
or update, and a detected unsaved lock buffer is reported as divergent.

Every edit invalidates the server's obsolete diagnostics and proof-dependent answers. Canceled
work still completes its protocol request with a cancellation response. Superseded
background work cannot overwrite a newer diagnostic set. Independent, established
facts may remain usable after a syntax/type error; dependent facts are unavailable,
not guessed. A partial graph or resource limit must be identified as incomplete,
never shown as a successful project check with zero errors.

## Features and their conditions

These are all boolean fields under `lsp.features`. Availability requires the
project switch, the corresponding client capability where applicable, and enough
current analysis. Disabling a feature does not disable facts needed by another.

| Field               | Default | Methods and behavior                                                                                                                                                                                  |
| ------------------- | ------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `completion`        | `true`  | `textDocument/completion`, `completionItem/resolve`: visible bindings, callable signatures, fields, explicit types, and literal import paths; offer only declared aliases and available dependencies. |
| `hover`             | `true`  | `textDocument/hover`: declared/inferred type, narrowing proof, ownership mode, definition, and documentation. Storage size/alignment are shown only when established for the selected target.         |
| `signature_help`    | `true`  | `textDocument/signatureHelp`: generic binders, parameters, result alternatives, and dispatch's supplied first argument.                                                                               |
| `definition`        | `true`  | `textDocument/definition`, `textDocument/typeDefinition`: resolved binding/type; imports point to the exact source or package facade and can show the alias declaration.                              |
| `references`        | `true`  | `textDocument/references`: resolved uses, respecting shadowing and declaration inclusion; results are limited to the available workspace graph, never all downstream consumers.                       |
| `rename`            | `true`  | `textDocument/prepareRename`, `textDocument/rename`: binding-aware private/local rename, with collision and affected-graph checks. Conditions below apply.                                            |
| `document_symbols`  | `true`  | `textDocument/documentSymbol`: bindings, exported fields, types, functions, and named scopes; syntax recovery may provide partial outlines.                                                           |
| `workspace_symbols` | `true`  | `workspace/symbol`: query indexed declarations in selected workspace roots; excluded/unloaded sources are outside its coverage.                                                                       |
| `semantic_tokens`   | `true`  | `textDocument/semanticTokens/full`, `/full/delta`, `/range`: classify resolved names; use full results when delta support is absent.                                                                  |
| `inlay_hints`       | `false` | `textDocument/inlayHint`: inferred binding types and argument parameter names where omitted; no text edits or speculative runtime values.                                                             |
| `folding`           | `true`  | `textDocument/foldingRange`: blocks, record types, and delimited comments from syntax.                                                                                                                |
| `selection_ranges`  | `true`  | `textDocument/selectionRange`: enclosing parsed expressions and statements.                                                                                                                           |
| `formatting`        | `true`  | `textDocument/formatting`, `textDocument/rangeFormatting`: gatostyle layout only, using the current buffer and path policy.                                                                           |
| `code_actions`      | `true`  | `textDocument/codeAction`, `codeAction/resolve`: compiler repair alternatives and proven gatostyle changes.                                                                                           |
| `capture`           | `false` | Explicit **Capture diagnostic** command for a current static failure; requires extension 1 and the snapshot conditions below.                                                                         |

Completion distinguishes type predicates in matcher conditions from ascriptions
in value expressions without inspecting surrounding spaces. Generic closers and
task punctuation use their parsed contexts. `true`, `false`, `null` and scope
operations remain values/bindings, never a keyword vocabulary. `$` denotes the
dispatch receiver and is classified as a readonly variable; `self` is ordinary.
Semantic tokens distinguish an actual foundational value from a shadowing local binding.

The token legend uses the supported intersection of `namespace`, `type`,
`typeParameter`, `parameter`, `variable`, `property`, `function`, `operator`,
`comment`, `string`, and `number`, with `declaration`, `definition`, `readonly`,
`modification`, and `defaultLibrary` modifiers. Unsupported classifications are
omitted. The legend is fixed for a connection; tokens are non-overlapping and
split at line boundaries, including multiline literals. See the
[semantic token wire format](https://raw.githubusercontent.com/microsoft/language-server-protocol/gh-pages/_specifications/lsp/3.17/language/semanticTokens.md).

For an expression such as `| t.{ -> $<MyCoolType> } <MyCoolType> | matched()`, hover
must identify the inner proven ascription and the outer predicate separately.
The outer predicate cannot supply an earlier proof. Ownership explanations can
show why a borrow ends at a scope, where an owner moves, or which task remains
unjoined. They describe static facts, not live addresses or a runtime trace.

Rename preserves symbol identity and must check every affected use in the owning
project. It refuses an incomplete reference search, a conflicting new binding,
foundational definitions, public facade/API changes, manifest schema field names,
and import alias/path/file renames. Those changes need an explicit source review.
A local name that happens to be spelled `true` may be renamed when it resolves to
that local binding. Strings and comments are never changed by spelling alone.

[Documentation comments](documentation.md) supply structurally attached prose,
resolved symbol links and separately classified example results from the same
analysis snapshot. Hover and signature help use those facts, not a second annotation
parser. Permitted rename can update bound documentation links and analyzed example
code only with complete source maps and reference coverage; ordinary prose is not
rewritten. Indexing, hover and completion never execute documentation examples.

## Diagnostics in a changing buffer

Compiler diagnostics use `source : "meowy"`, stable codes such as `E103`, and
error severity. Gatostyle uses `source : "gatostyle"`, its `G...` codes, and the
configured rule level; layout drift `G001` takes its severity from
`gatostyle.fail_on`, as specified by gatostyle. For other findings, `fail_on`
sets the CLI failure threshold without changing their editor severity.
Related spans carry declarations, constraints, or ownership paths;
clients without related-location support receive that evidence in the message.
Optional diagnostic data identifies the analysis snapshot and candidate set.

Publish source diagnostics from completed work in stable source order. The
per-file cap retains errors before warnings before hints, then orders by source
position/code. If capped, report both shown and total counts as project status;
do not invent another language error code for truncation. All checks and candidate
validation still see the uncapped graph. Separate failures sharing `E103` stay
separate diagnostics. A later edit can remove one without removing the others.

The connection selects one delivery model:

- With `textDocument.diagnostic` support, advertise `diagnosticProvider` with
  identifier `"meowy"`, `interFileDependencies : true`, and
  `workspaceDiagnostics : true`. Clients may request document or workspace
  reports; `workspace.diagnostics.refreshSupport` controls refresh requests,
  not the availability of workspace reports. Pulls use snapshot-specific result
  IDs; a changed input invalidates them. Without refresh support, the next pull
  observes the change.
- Otherwise use `textDocument/publishDiagnostics`, including the open document
  version when the client supports it. Publish empty lists when errors disappear,
  a document leaves the publication scope, or a context is removed.

Do not publish the same diagnostics through both models. A pull returns the
completed current snapshot if available. With `"change"`, it waits for the
scheduled check; with `"save"` or `"manual"`, a pending check returns
`ServerCancelled` with `data : { "retriggerRequest": false }` and a status reason
to save or use **Check project**. Do not return an empty successful report to
mean “not checked.” Saving when the trigger permits it, or explicitly checking,
enables a new current result
and requests refresh when supported. Thus automatic client pulls cannot override
the project's manifest scheduling. Facts computed privately for hover or a
repair do not silently publish a full semantic check in manual mode.

For push clients, invalidation publishes a replacement set containing only current
syntax/configuration findings, possibly empty, and logs that semantic checking is
pending. Pull cancellation does not clear a client's previously displayed report;
that client may retain older diagnostics until a successful fresh pull. Its UI
must distinguish retained results from a current check. Scope removal/disable
returns an empty full report on the next pull, with the disabled/removed status.
The [pull diagnostic contract](https://raw.githubusercontent.com/microsoft/language-server-protocol/gh-pages/_specifications/lsp/3.17/language/pullDiagnostics.md)
defines result reuse and workspace/document report coordination.

Language diagnostics are ephemeral until explicitly captured. They never borrow
CLI occurrence IDs such as `1` or patch IDs such as `1.2`, advance the project's
last-run pointer, or overwrite a replay capsule. Use a rule code with
`meowy err explain E103`; `meowy err explain 1` instead explains a saved session's
first occurrence. The [diagnostic reference](diagnostics.md#labels-and-identities)
defines those identities and the terminal's display columns.

## Formatting and source changes

Formatting requires a parsable current document, a valid current manifest,
`lsp.features.formatting`, and the effective `gatostyle.format.enabled` setting.
Imports and semantic analysis are not required for layout. A malformed buffered
manifest blocks formatting rather than selecting its on-disk policy. The client
still sends the protocol's formatting options, but indentation, line endings,
spacing, and final-newline choices come exclusively from gatostyle.

Whole-document formatting returns edits for that buffer. Range formatting selects
complete enclosing statements; if a safe enclosing selection would require edits
outside the requested range, return no edits with an explanation. Neither method
rewrites expression form, normalizes literal contents, sorts imports, or changes
resource lifetimes. Protected gatostyle regions remain byte-for-byte intact.

`format_on_save` uses `textDocument/willSaveWaitUntil` only if the client announces
support. It formats the synchronized buffer before the save and never writes a
file itself. If the client cannot wait, report automatic formatting unavailable;
explicit formatting remains usable. No background substitute runs after save.
Even a supporting client may discard before-save edits after a timeout or
failure. Report that formatting was skipped when observable; saving remains an
editor operation and format-on-save is best effort.
There are no automatic compiler repairs or semantic style fixes on save in schema 1.

Code actions use `quickfix` for compiler alternatives and individual style fixes,
and `source.fixAll.meowy.gatostyle` for compatible proven style changes. The
server advertises no generic compiler fix-all or organize-imports action.
An action explains its preconditions and behavioral/storage effects; “preferred”
is ranking, not proof of intent. A capacity increase can change layout and a
public type. A text value never becomes a guessed integer just to remove `E207`.
Set `isPreferred` only when the client advertises support for that property;
otherwise keep the recommendation in the action's title and explanation.

Code actions require `textDocument.codeAction.codeActionLiteralSupport`; clients
without it receive explanations through diagnostics and no action provider.
Defer an action's `edit` to `codeAction/resolve` only when both
`textDocument.codeAction.dataSupport` and `resolveSupport.properties` containing
`"edit"` are advertised. Otherwise compute the complete action eagerly.

Before returning or resolving a compiler repair, semantic style fix, or rename,
hash the selected source, configuration, and dependency inputs, construct the
combined patch in memory, and recheck the affected graph. Compiler fixes must
remove their target without introducing new
errors. Gatostyle fixes additionally require its
[equivalence proofs](../guide/gatostyle.md#understand-automatic-changes). Overlapping
alternatives, incomplete required facts, or changed inputs disable the action
with a reason; clients without disabled-action support omit it and can read the
reason in the diagnostic. No operation silently upgrades an unproven preference.

Rename and code-action edits require versioned `WorkspaceEdit.documentChanges`.
Every edited file must be open and synchronized; otherwise ask the user to open
the affected files and request the action again. This keeps closed-file `null`
versions out of source-changing actions. A multi-file edit additionally requires
the client to advertise `transactional` or `textOnlyTransactional` failure handling.
Without these capabilities, explanation/navigation remains available and the CLI
provides the [journaled repair workflow](diagnostics.md#repair-contracts).

The client must reject mismatched document versions and preserve exact replacement
bytes. If it declares line-ending normalization that could alter protected text,
the server withholds that edit. Formatting and completion return protocol edits
without embedded document versions, so the client must discard their response if
the originating buffer changed. Completion edits are confined to that buffer;
there are no hidden cross-file import insertions.

The server validates a snapshot before responding. Standard LSP cannot lock every
other analysis input while an editor applies the response; a later input change
requires fresh analysis. A client's in-memory transaction is not the CLI's
filesystem recovery journal. The client surfaces application failures and
synchronizes actual buffers; returned rename/action edits have no standard
application-result acknowledgement. Reanalyze those buffers before offering more
edits. The server never writes source files on behalf of formatting, rename,
completion, or code actions. These transport choices
use [workspace edits](https://raw.githubusercontent.com/microsoft/language-server-protocol/gh-pages/_specifications/lsp/3.17/types/workspaceEdit.md)
and [code-action capabilities](https://raw.githubusercontent.com/microsoft/language-server-protocol/gh-pages/_specifications/lsp/3.17/language/codeAction.md).

## Inspect facts and capture a failure

Every client can request **meowy: Check project** through `workspace/executeCommand`
with command `"meowy.checkProject"` and arguments `[{ "uri": "file:///PROJECT/main.mwy" }]`.
The URI selects a context; the configured scope and current overlays select its
graphs. It produces current diagnostics and returns a status record, without
creating a CLI session. Invalid or disabled contexts return a reason.
Advertise `meowy.checkProject` in `executeCommandProvider.commands`.

Optional meowy extension 1 adds inspectable status, diagnostic evidence, and
explicit capture. A client opts in with
`capabilities.experimental.meowy : { "version": 1 }`; the server acknowledges the
same record in its experimental capabilities. An unsupported extension revision
leaves standard features intact and enables no custom requests. Extension records
are protocol declarations, not project settings.
Also advertise `meowy.captureDiagnostic` as an execute command when extension 1
and diagnostic data support are negotiated; its project and snapshot gates still
apply at invocation. Unknown commands are rejected without invoking a shell.

| Request or command                        | Parameters                                                                          | Result                                                                                                       |
| ----------------------------------------- | ----------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| `meowy/projectStatus` request             | `{ "uri": "file:///PROJECT/main.mwy" }`                                             | The status record below, for the live buffered project.                                                      |
| `meowy/diagnosticEvidence` request        | `{ "uri": "file:///PROJECT/main.mwy", "snapshot": "TOKEN", "diagnostic": "TOKEN" }` | `{ "snapshot": "TOKEN", "code": "E103", "facts": [...] }`; each fact has `area`, `availability`, and `text`. |
| `meowy.captureDiagnostic` execute command | One argument containing the same URI/snapshot/diagnostic record                     | `{ "session": "ID", "entry": "PATH", "occurrence": 1, "capsule": "URI" }` after successful capture.          |

`diagnosticEvidence` accepts the opaque tokens from diagnostic `data.meowy`:
`{ "version": 1, "snapshot": "TOKEN", "diagnostic": "TOKEN" }`. Return them only
when extension 1 and diagnostic data support are negotiated. Tokens expire on an
input change or restart; a stale token returns `RequestFailed` with a refresh
instruction, never the nearest diagnostic at that location. Facts use areas
`types`, `ownership`, or `layout`, with availability `established` or `unavailable`.
For example, E103 evidence can show initialized length 3, required length 4,
capacity 3, and a target-specific storage consequence. Unavailable layout does
not become a guessed host layout. There are no fabricated task/clock events.

Capture additionally requires `lsp.features.capture`, negotiated diagnostic
data, a current static compiler failure in a selected entry's checking graph,
and all inputs needed for a closed checking capsule. A manifest-backed project
must have `build.entry` and an already saved `mod.mwy` whose path owns that entry
under disk discovery. Its contents and sources may still have unsaved changes.
Loose files must acquire such a manifest to enable capture. Export-only libraries and helper graphs
outside the entry's check need a concrete entry before capture; the command must
not fabricate an executable entry or a CLI selector for them.

A diagnostic confined to a test graph likewise cannot use this entry-capture
extension. Save the files and use `meowy test --no-run` to preserve a failing
test check/build session, then select it with `meowy err ... --test`. Shared
helpers that also belong to the selected application entry remain subject to
the ordinary entry-capture rules above.

Gatostyle findings, toolchain/configuration blocks, missing dependencies,
truncated analysis, and untitled sources are not capture candidates. The command
must be an explicit user action; merely opening, editing, saving, requesting
diagnostics, or viewing evidence never captures.

Capture freezes the entire required overlay graph, preserving unsaved bytes,
original URI/path mappings, source hashes, manifest, lock, target, and exact
compiler. It invokes the checking phase against that immutable snapshot and
verifies the selected failure, then packages the
[executable replay capsule](diagnostics.md#replay-capsules). The replay runner
repeats that checking failure; it does not claim an application executable or
runtime recording. If verification or packaging fails, return a failure with no
successful capture result or partial session publication.

The completed editor capture is stored as an explicitly selected session under
the owning project, with its returned entry path relative to that project's root
identifying the checked graph. The entry need not be saved, but its on-disk path
ancestry must resolve to the same owning root. Capture does not advance the CLI
last-run pointer. Its numbered occurrences are assigned
only after capture, in saved source order. Use the returned identifiers:

```sh
meowy err inspect OCCURRENCE --entry ENTRY --session SESSION --verbose
meowy err reproduce OCCURRENCE --entry ENTRY --session SESSION --verbose
meowy err export OCCURRENCE --entry ENTRY --session SESSION --output failure.replay
```

Run these from the owning project root. Saved repairs targeting unsaved source
remain subject to the CLI's disk-hash checks; saving or editing later does not
change what the capsule reproduces. Runtime tasks, channels, clocks, and native
execution evidence come from a separately recorded `meowy run`, not editor analysis.

## Wire lifecycle and compatibility

Messages use JSON-RPC 2.0 and LSP `Content-Length` framing with UTF-8 JSON on
stdin/stdout. Standard output contains only protocol traffic, with no version
banner, diagnostic transcript, or debug print. Logs go to stderr. Follow the
[base protocol](https://raw.githubusercontent.com/microsoft/language-server-protocol/gh-pages/_specifications/lsp/3.17/specification.md)
for framing, initialization ordering, cancellation, and shutdown.

| Client capability                                    | meowy behavior when absent                                                                                                                    |
| ---------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| Workspace folders                                    | Use `rootUri`; if absent, discover contexts as named documents open. Never guess a project from the process working directory.                |
| Position encoding negotiation                        | Use UTF-16. Otherwise prefer offered UTF-8, UTF-16, then UTF-32; mandatory UTF-16 remains a fallback even when omitted from the offered list. |
| Pull diagnostics                                     | Use publish diagnostics. Missing workspace pulls limit transport coverage, not analysis validity.                                             |
| Markdown hover content                               | Return plain text.                                                                                                                            |
| Hierarchical document symbols / location links       | Return flat symbols / ordinary locations.                                                                                                     |
| Semantic tokens or inlay hints                       | Omit those providers; lexical highlighting still works.                                                                                       |
| Snippet completion                                   | Return plain insertion text.                                                                                                                  |
| Literal code actions                                 | Omit code actions; diagnostics retain repair explanations.                                                                                    |
| Code-action data and resolve support for `edit`      | Compute complete actions eagerly; still validate the current snapshot.                                                                        |
| Versioned workspace edits / transactional text edits | Withhold rename and unsupported source actions, as defined above.                                                                             |
| Before-save wait                                     | Withhold format-on-save; explicit format remains available.                                                                                   |
| Save notifications                                   | Report save-triggered checking unavailable; use Check project, or change the manifest trigger. Do not guess saves from document changes.      |
| Dynamic registration                                 | Use static providers; project switches are enforced per request.                                                                              |
| meowy extension 1                                    | Keep standard language features; omit custom status/evidence requests and capture.                                                            |

Initialization announces supported providers for the connection. That is an
ability to handle requests, not a claim that every root enables them. Static
providers remain registered when a manifest disables a feature; return an empty
result for a disabled read-only feature, or `RequestFailed` explaining a disabled
edit. Clients receive project status through `window/logMessage`; negotiated
refresh requests invalidate their cached tokens, hints, and diagnostics.
Without refresh support, the next explicit request observes the new policy.

The server requests open/close and incremental change synchronization, and
accepts full replacement changes too. When the client advertises
`textDocument.synchronization.didSave`, request saves with
`save : { "includeText": false }`; otherwise omit save notification support and
report the limitation for `check.trigger : "save"`. For format-on-save it
advertises before-save wait support at initialization when the client can use it,
then gates each request by the current manifest.
Monotonically increasing document versions identify overlays. Invalid edit ranges
or out-of-order versions leave that document unsynchronized; log the problem and
require a complete replacement or close/open before semantic requests continue.

All protocol lines and character offsets are zero-based and ranges are end
exclusive. Convert compiler UTF-8 byte spans using the negotiated character
encoding; never copy one-based terminal display columns. An emoji has four UTF-8
bytes and two UTF-16 units, and tabs occupy one encoded character regardless of
screen width. Newline boundaries must preserve CRLF buffers without shifting spans.

`initialize` precedes ordinary requests; `initialized` starts project work.
`shutdown` cancels work and returns `null`, then `exit` terminates with status `0`.
An `exit` without shutdown is status `1`. A lost transport ends the process and
drops unsaved overlays; the server never saves them automatically. On restart,
the client opens documents with their current full text. Cached semantic state
is reusable only when all input identities match; live diagnostic tokens and
pending edits never survive a connection restart.

Protocol `initializationOptions` must be absent, `null`, or `{}`; nonempty values
fail initialization with `InvalidParams` and point to `mod.mwy`. The server never
requests project preferences through `workspace/configuration`.
`workspace/didChangeConfiguration`, initialize `trace`, and `$/setTrace` cannot
override manifest policy; unsupported setting attempts receive a log explanation.
Protocol capabilities cannot fabricate support for a disabled project feature.

## Inspect configuration and connection problems

```sh
meowy --version
meowy lsp config --resolved docs/programs/packet/main.mwy
meowy lsp doctor docs/programs/packet/main.mwy
```

`config --resolved PATH` accepts one source path or project directory and prints
JSON containing schema defaults, effective `lsp`, `build`, and path-specific
gatostyle configuration, manifest path, and the origin of each setting. With no
path, use the working directory's nearest project. Paths need not exist when
their containing root can be determined. `doctor [PATH]` uses the same selection
and checks configuration, the toolchain pin, entry/export resolution, required
locked content, target inputs, file limits, and local watcher availability.
It reports conditions and remedies without running or fetching anything. It
checks readiness, not every source rule; `meowy check` remains the validation command.

Both commands read disk contents. They cannot inspect another process's unsaved
buffers or negotiate an editor's capabilities. The live **Check project** command
and optional `meowy/projectStatus` report those differences. The status record
has these fields (unknown/unowned URIs return `InvalidParams`):

| Field               | Value                                                                                                                                                                                                                                                                                                                                                                      |
| ------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `root`, `manifest`  | Canonical root directory and manifest URI strings; `manifest` is `null` for loose files, and both are `null` for untitled buffers.                                                                                                                                                                                                                                         |
| `snapshot`          | Opaque string identifying the current inputs; it changes when those inputs change.                                                                                                                                                                                                                                                                                         |
| `toolchain`         | `{ "version": "STRING", "build": "DIGEST" }` for the running distribution.                                                                                                                                                                                                                                                                                                 |
| `target`, `profile` | Effective target triple and profile strings, or `null` if configuration cannot establish them.                                                                                                                                                                                                                                                                             |
| `state`             | `ready`, `limited`, `disabled`, or `invalid`. Ready means current analysis coverage is complete, not that source has no errors.                                                                                                                                                                                                                                            |
| `reasons`           | List of strings explaining pending work, unavailable inputs, or disabled features.                                                                                                                                                                                                                                                                                         |
| `coverage`          | `{ "scope": "project", "checked": true, "roots": [], "skipped": [], "shown": 0, "total": 0 }`; scope is the effective `project`/`open` selection. Roots contain `{ "uri": "URI", "mode": "entry" }`, with modes `entry`, `module`, or `facade`. Skipped entries contain `uri` and `reason`. Counts are nonnegative integers, or `null` until a diagnostic check completes. |
| `features`          | Map of the feature names above to `{ "available": true, "reason": null }` or `{ "available": false, "reason": "STRING" }`, describing the current root/document and negotiated capabilities.                                                                                                                                                                               |

While checking is pending or partial, set `coverage.checked : false` and state
`limited`; never present a ready zero-error summary for unchecked inputs.
For an invalid configuration, unresolved coverage fields are `null` and list
fields stay empty. A disabled context reports `disabled` without starting work.

`trace : "messages"` logs method names, request IDs, timing, root selection, and
state transitions. `"verbose"` adds invalidation and analysis summaries; it does
not dump buffer contents, string literals, environment variables, or whole
protocol payloads. `"off"` retains essential startup/failure messages only.
The server writes no trace file, sends no telemetry, and offers no shell-command
hooks. In a multi-root process, tag logs by root and apply that root's trace level;
connection-wide traffic uses the least verbose active policy (off before discovery).

| Symptom                                            | Check and remedy                                                                                                                                   |
| -------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| Process waits silently in a terminal               | Connect an LSP client; stdout is the protocol stream.                                                                                              |
| Syntax works but project features stop             | Read the nearest buffered `mod.mwy` diagnostic, especially `E505` or a pinned-toolchain `E507`.                                                    |
| Editor and CLI disagree                            | Compare saved vs unsaved source/manifest, actual executable, target/profile, entry graphs, lock content, and checking scope.                       |
| Dependency completion is missing                   | Resolve the declared dependency explicitly and provide its locked content; an alias does not fetch a package.                                      |
| A second project has the wrong policy              | Inspect its nearest manifest and canonical URI; sibling/nested roots do not inherit settings.                                                      |
| No format on save                                  | Enable it in `lsp`, enable gatostyle layout for that path, and use a client with before-save wait support.                                         |
| Indentation differs from editor options            | Inspect `gatostyle`; LSP formatting deliberately follows the manifest.                                                                             |
| Rename or fix is unavailable                       | Check complete reference/proof coverage, open affected files, versioned-edit support, and multi-file transaction support.                          |
| Errors disappear or remain marked old while typing | Save/manual checking invalidates semantic diagnostics; push clears them, while pull clients may retain old reports until the next completed check. |
| Missing workspace errors                           | Check `check.scope`, selected entry/export graphs, pull coverage, file limits, and diagnostic truncation counts.                                   |
| Replay does not show current editor text           | Ordinary saved CLI sessions use their original source. Explicitly capture the current diagnostic or save and run a new check.                      |
| Highlighting shifts after Unicode text             | Verify negotiated position encoding and client buffer versions; display columns are not protocol offsets.                                          |

`lsp config` exits `0` after valid JSON output or `2` for invalid configuration or
usage; it can display a valid but mismatched toolchain pin. `lsp doctor` exits `0`
when ready, `1` for unavailable/mismatched inputs or degraded operation, and `2`
for invalid configuration/usage. A deliberately disabled root is reported as such
and exits `0`. Human doctor output and errors go to stderr; config JSON goes to
stdout. Neither command publishes a diagnostic session or changes source.
`meowy lsp --help`, `meowy help lsp config`, and `meowy help lsp doctor` describe
these invocations. `--profile`, `--target`, `--preset`, `--config`, logging, and
editor-setting overrides are not accepted by the LSP command family.
The CLI's global `--color` and `--quiet` options apply only to human output from
`config`/`doctor`; they are rejected when serving a protocol connection, whose
diagnostic presentation belongs to the client and tracing belongs to `lsp.trace`.
