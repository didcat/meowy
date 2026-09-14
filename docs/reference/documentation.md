# Checked documentation comments

[Documentation index](../README.md) - [Syntax](syntax.md) - [Language server](lsp.md)

Documentation is source metadata attached to declarations, not a second type
system. Signatures, type parameters, result alternatives, visibility and ownership
facts come from the same analyzed program that the compiler checks. Prose explains
intent, constraints and examples; it does not override those facts.

This chapter specifies the language/tool contract. The standalone bootstrap now
implements attachment, checked value/type links, compiler-derived signatures,
documentation commands and checked/explicitly run examples. Unclosed fences use
E002; documentation failures use E801-E805 in the shared catalog. Full LSP/rename,
package documentation and a public serialized index remain separate capabilities.

## Implemented bootstrap profile

Use an explicit `.mwy` source file. Ancestor manifests remain unsupported unless
`--standalone` deliberately selects an isolated file; imports remain limited to
the compiler's supported foundational modules. Unsupported language/library code
in an example remains B001, not a passed rejection or an invented API.

Links use the actual checker scopes and resolved types. Function documentation
can resolve its parameters; module documentation resolves after the file's
declarations are checked. Other declaration links follow available bindings at
that declaration. Builtin targets can be checked without having a local page.
The standalone public-coverage policy treats top-level declarations and their
exposed record members as the API, not as a substitute for future facade exports.

`doc build` writes one owned `index.html` with CommonMark rendering and a basic
responsive layout. It refuses unrelated output files and unowned indexes, stages
replacement, escapes raw HTML and neutralizes unsafe link schemes. Images render
as alt text; `--assets` is not implemented in this profile. No source values,
application initialization, external assets or browser launch are inferred.

`doc check` and `doc build` check examples without executing them. `doc check
--run-examples` runs only explicitly runnable examples in temporary working
directories with closed stdin, an empty inherited environment, a default 5000 ms
execution limit and 1 MiB per captured output stream. `--example-timeout-ms` accepts
1..60000. Compilation uses the ordinary compiler budgets. These process controls
are not a security sandbox. Nested documentation examples inside an example are
not recursively scheduled.

Example failures identify the original fence and example-local line/byte offsets;
declared links retain exact original source spans, including CRLF/interpolation.
Automatic example-code rename is deferred rather than claiming complete virtual
source maps. The renderer labels unexecuted examples as not executed. The pinned
Markdown dependency is compiler tooling and is not linked into generated programs.

## Fences and source bytes

`#| ... |#` documents the following declaration. `#!| ... |!#` documents the
containing source module and must precede its first declaration. A file has at
most one module documentation block. Neither form adds a keyword.

An opener contains a maximal run of one or more `|` characters. Its closer has
exactly the same number of bars followed by `#`, or by `!#` for module docs.
The closing run must itself be maximal. Different-length runs are content.
Use a longer fence when the prose or an example contains a shorter closer:

````text
#||
An example with its own documentation:

```meowy
#| A constant count. |#
count<int32>:7
```
||#
````

The body is opaque to the source lexer: quoted strings, Markdown code fences and
other comment openers inside it do not nest or suppress its matching closer.
There is no interpolation or escape processing. A longer outer fence preserves
literal example bytes without backslash tricks. An unterminated fence is a lexical
error with the opener span and the expected closer, even if its declaration is unused.

Outside strings/comments, these openers take precedence over ordinary `# ... #`
comments. Ordinary comments still do not nest; `##` remains an empty ordinary
comment. An old ordinary comment starting immediately with `#|` or `#!|` must
insert a space after its opening `#` if documentation was not intended.
The first matching closer ends the block; quotes inside it are not source strings.

Documentation remains a token separator. Newlines inside it do not terminate a
meowy statement. UTF-8 and source offsets follow the ordinary syntax contract.
The source snapshot retains exact bytes, including CRLF, delimiters and indentation.
Rendering may normalize line endings and strip one common indentation prefix from
nonblank payload lines, but every transformed byte range must retain a source map.
Tabs are not silently expanded into guessed source columns.

## Attach descriptions, not duplicated signatures

```meowy
#| Adds an amount to a count. Arithmetic overflow panics. |#
add <int32> : (
    #| Starting count. |#
    count <int32>,
    #| Amount to add. |#
    amount <int32>
) {
    -> count + amount
}
```

A leading block can attach to a named binding/function, type declaration, named
record field, named emission, parameter or generic binder. It belongs to the
next eligible declaration in the same containing declaration/parameter/field list.
Whitespace and ordinary comments may intervene; another documentation block may
not. Multiple blocks for one declaration, a closing container, or an expression
instead of a declaration produce attachment diagnostics, not guessed associations.

There is no implicit trailing-documentation form. A block after an already parsed
declaration but before its statement separator cannot migrate onto the next one.
An unnamed primary emission has no named member to document; describe its result
on the enclosing declaration. A declaration that introduces several names receives
one declaration-level description, not an arbitrary first-name attachment.

Module docs belong to the owning source module. A package facade uses its own
module documentation; auxiliary files do not silently replace the package overview.
An alias can provide its own prose. Otherwise tooling may show the original
declaration's documentation with source attribution; aliases do not copy or alter
the underlying signature, intrinsic identity or lifetime contract.

There are no `@param`, `@type` or `@returns` declarations. Parameter/field descriptions
attach to their syntax nodes. The compiler supplies the parameter name and type,
result union, capabilities and proven ownership information. Ordinary Markdown
sections such as "Result", "Failures" and "Ownership" remain prose, not executable
preconditions or new type annotations.

## Semantic links

`[[name]]` links a value declaration; `[[<Type>]]` links a type declaration.
Qualified identifier paths are allowed, such as `[[json.decode]]` and
`[[<json.Document>]]`. `[[target|label]]` supplies display text without changing
the target. Labels are text, not nested expressions or links.

Targets are identifier/member paths, optionally enclosed in type brackets, not
arbitrary expressions, calls or type-helper evaluation. A generic declaration is
linked without instantiating it. Resolution uses the documented declaration's
lexical environment and its declared parameters/generic binders, never function-body
locals or whichever unrelated name happens to match in the project.

Unknown, ambiguous or inaccessible targets are diagnostics. Dependency links use
the selected, locked module graph. Checking never fetches a package or URL just to
repair a link. Inline code without `[[...]]` is presentation, not a semantic link.
External Markdown links remain external and are not advertised as availability-checked.

Structured links carry symbol identities and source spans. A permitted binding-aware
rename can update those references and analyzed example code, but never prose or
ordinary code spans by spelling. Existing visibility and complete-reference-search
restrictions still apply. Incomplete example/source maps withhold the affected
rename rather than producing a partly updated documentation graph.

## Markdown and examples

Payloads use [CommonMark 0.31.2](https://spec.commonmark.org/0.31.2/) with the
semantic links and code-fence attributes defined here. Attributes are documentation
syntax, not meowy keywords. Unknown attributes are errors, not silently ignored tags.
Raw HTML is escaped; generated pages permit no embedded script or active content.
Authored assets are not copied by default. `doc build --assets DIR` explicitly
permits PNG, JPEG, GIF and WebP assets from that directory; resolved paths must stay
inside it. SVG, source/credential files and arbitrary HTML are not copied as assets.
Builds do not download remote images, include remote source or execute Markdown extensions.

| Fence info                         | Check behavior                                                               |
| ---------------------------------- | ---------------------------------------------------------------------------- |
| `meowy` or `meowy check`           | Parse and semantically check a complete example without execution            |
| `meowy run`                        | Check an example; build/run it only when the user requests example execution |
| `meowy reject=CODE`                | Require the specified primary language diagnostic during checking            |
| `output`                           | Expected UTF-8 stdout for the immediately preceding run example              |
| `text` or another display language | Render only; make no compiler-validation claim                               |

Every meowy example is a complete source unit with explicit imports and setup.
There are no hidden `main` wrappers, implicit enclosing locals or erased setup
lines. Imports use the selected project's dependency graph without exposing
otherwise private declarations. Library helpers in a project can reduce repeated
setup through ordinary explicit imports.

An `output` block contains the expected stdout bytes after documented line-ending
normalization, including its final newline when present. Run examples require
exit status zero and empty stderr unless the ordinary testing API explicitly
asserts a failure inside the example. Tests needing binary output, panic metadata
or richer assertions should use that API rather than inventing more doc tags.
At most one output block belongs to a run example; an orphan output is an error.

Reject examples check the primary assigned code, not an arbitrary failure or a
matching substring. Unsupported features, missing tools, budget/infrastructure
failures and crashes never satisfy an expected language rejection. A skipped run
is not a passed run. Results retain example identity, source snapshot, target,
profile and whether checking, compilation or execution actually occurred.

Runtime examples reuse the [testing runner](stdlib/testing.md), process budgets
and failure reporting. They are code execution, not a security sandbox merely
because they came from comments. Execution requires an explicit user command;
hover, indexing, ordinary builds and documentation rendering never run examples.
HTTP examples use controlled fixture transports or explicit test services, not
unannounced public endpoints or inherited production credentials.

## Commands and publication

These commands extend the full language's [CLI contract](../cli/README.md).
The bootstrap implements their standalone forms under the profile above:

| Command                                 | Work performed                                                                                 |
| --------------------------------------- | ---------------------------------------------------------------------------------------------- |
| `meowy doc check PATH`                  | Check source, attachments, links and all meowy example check expectations                      |
| `meowy doc check PATH --require-public` | Also require nonempty documentation on exported declarations and named exported record members |
| `meowy doc check PATH --run-examples`   | Additionally build/run explicitly runnable examples using the test runner                      |
| `meowy doc build PATH --output DIR`     | Check documentation and render a local API site without executing examples                     |

PATH/project discovery and `--profile`, `--target` and `--offline` use ordinary CLI
rules. An incompatible host may check examples but cannot claim their execution.
Checking returns nonzero on failed or unsupported required checks. Missing public
prose is permitted without `--require-public`; inferred signatures are still shown.
Parameter descriptions are optional even under that flag and remain individually
attached when provided.

Normal source checking validates fence syntax, attachment and semantic links for
analyzed declarations. Strict documentation checking adds public-coverage policy
and example units. Well-formed documentation changes no executable effects,
layout, type identity, capability or ownership rule.

The renderer and LSP consume one analyzed documentation model: declaration identity,
signature facts, payload/source map, resolved links and separately classified
example results. They do not parse competing annotation languages. Stale or absent
results are shown as unverified, never as a green check for a different snapshot.
Prose cannot be proved truthful; enforceable guarantees belong in types and tests.

Publication stages output under DIR and replaces only the tool's owned output
after success. It protects source, manifests, locks and unrelated existing files;
it never uploads or opens a browser implicitly. Names and links are deterministic
for the selected graph, target and profile, with collision checking. Private items
and internal diagnostic details do not leak into a public site through backlinks.

This logical model is not a promised JSON interchange schema. A serialized public
index must be registered in the artifact/schema contract before compatibility is
claimed. E801-E805 are assigned to documentation in the shared code catalog;
unrelated numeric codes are not repurposed.

## Implementation and qualification

1. Preserve documentation trivia and exact matching fences in the lexer/lossless
   tree; diagnose missing closers and retain source maps. Update both editors.
2. Attach nodes during parsing and resolve links using the compiler's analyzed
   graph. Pin alias, parameter, field, orphan and rename behavior with regressions.
3. Build one documentation model for LSP and rendering. Add assigned diagnostics
   and any public artifact schemas before advertising their interfaces.
4. Reuse test-process machinery for checked/run/reject examples; test effect
   boundaries, stale snapshots, output bytes, unsupported cases and safe publication.
5. Qualify nested quoted fences, hashes inside Markdown, CRLF, UTF-8 spans, module
   placement, malformed attributes, missing public docs and inaccessible links.

Documentation builds and link checks do not qualify an HTTP implementation,
compiler release, executable example or minimum supported host.
