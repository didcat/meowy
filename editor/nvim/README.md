# Vim and Neovim support

This directory is a standalone runtime bundle for `.mwy` files. It works in both
Vim and Neovim without an external plugin dependency.

- [File detection](ftdetect/meowy.vim) handles new and existing files, replaces
  generic fallback guesses, and respects an earlier explicit filetype detection.
- [Syntax highlighting](syntax/meowy.vim) follows the
  [punctuation-based language reference](../../docs/reference/syntax.md).
- [Buffer settings](ftplugin/meowy.vim) use four-space indentation, expand tabs,
  provide `# %s #` for comment commands, and add `.mwy` to filename suffix lookup.

## Setup

Add this directory to the runtime path before enabling filetype plugins.

For Neovim, in `init.lua`:

```lua
vim.opt.runtimepath:prepend("/absolute/path/to/meowy/editor/nvim")
vim.cmd("filetype plugin on")
vim.cmd("syntax enable")
```

For Vim, or Neovim with an `init.vim`:

```vim
execute 'set runtimepath^=' . fnameescape('/absolute/path/to/meowy/editor/nvim')
filetype plugin on
syntax enable
```

Replace the path with the checkout location. Reopen a `.mwy` file after changing
setup; `:setlocal filetype?` should report `meowy`. Only buffer-local editing
options change, and the filetype plugin provides an undo command when switching
filetypes. It installs no key mappings.

The defaults retain the current indentation on a new line. They disable C-style
smart indentation and automatic comment-leader insertion: a second `#` closes a
meowy comment. Automatic hard wrapping of source and literal strings is disabled.

[Gatostyle](../../docs/guide/gatostyle.md) defines configurable layout and semantic
style checks. Its stdin commands support editor buffers; this runtime keeps
formatting and automatic fixes opt-in. The four-space buffer defaults can be
overridden independently to suit a project's policy.

## Language server

The [language-server reference](../../docs/reference/lsp.md) defines the full
editor contract: launch `meowy lsp` over stdio with language ID `meowy`, provide
workspace folders, and synchronize `.mwy` buffers. Use a client supporting the
documented capabilities; the runtime bundle and LSP client have separate setup.
There is no fixed Vim/Neovim version requirement for the protocol itself: feature
availability follows the actual client capabilities exchanged at initialization.

Put project analysis and feature settings in `mod.mwy`, including automatic
formatting through `lsp.format_on_save`. All formatting policy comes from
`gatostyle`; buffer options such as `shiftwidth` do not override it. An LSP client
without the before-save wait capability can still request formatting explicitly.
Do not duplicate meowy configuration in editor settings or automatic shell hooks.

Use `meowy lsp config --resolved PATH` to inspect saved configuration and
`meowy lsp doctor PATH` to inspect prerequisites. These commands read disk, while
the live server includes unsaved buffers. Semantic tokens can distinguish local
bindings from foundational values; the lexical highlighter below remains useful
when the client omits that capability.

## Highlighted syntax

The highlighter handles:

- Nested generic types, record types, function signatures, constrained binders,
  multiple parameters and arguments, union alternatives, type subtraction, and
  type queries, ordinary type predicates (`value<T>`) and explicit proven
  ascriptions (`value~<T>`).
- The `$` dispatch receiver, including field access, borrows and interpolation;
  `self` remains an ordinary identifier.
- References and pointers, including `<&!T>`, `<*!T>`, and `&!value`, plus `!{ ... }`
  safety boundaries.
- Task submission and joins (`>>`, `<<`), `%group` names, labeled scopes,
  emissions, matchers, field selection, and dispatch. Simple and qualified callables
  in `value.(function)`, `value.(module.function)` and `value.(function, argument)`
  use function highlighting,
  including a line break after the opening parenthesis.
- Decimal, hexadecimal, and binary integers with separators, floating-point
  literals and exponents, and arithmetic and logical operators.
- Delimited multiline comments, multiline strings, supported escapes, and
  interpolation containing nested expressions, blocks, or quoted strings.
- Declaration/module documentation fences with exact matching bar counts, including
  quoted shorter fences, hashes and braces inside the payload. Their lexical
  highlighting does not claim declaration attachment or semantic-link checking.
- Literal module imports and ordinary calls, including generic calls.

Type regions nest, so `>>` inside a generic type closes two type arguments while
`>>` in an expression denotes task submission. A function type's `->` does not
close its angle brackets. A comparison such as `count < limit` does not start a
multiline type region. Unrecognized string escapes are highlighted as errors.
Comma-separated specializations such as `make_d<string, uint32, boolean, string>(...)`
also retain type highlighting; `call(x < limit, other > 0)` keeps its comparison
operators.

Spaced and unspaced types share highlighting. `value<T>` is a type predicate in
any expression; `value~<T>` is a proven ascription. The `~` consumes one bracketed
type, so `value~<Choice><int32>` highlights the ascription followed by a predicate.
A generic target such as `value~<D<string, uint32>>` nests normally. Union targets
use a named alias. Generic calls retain `function<T>(arguments)` syntax.
Computed annotations such as `other <(value<>)> : value` retain explicit type
delimiters too.

Bitwise operations use ordinary `@"bits"` module calls, including `bits.and`,
`bits.or`, `bits.xor`, and `bits.not`. Prefix `~` and `^` have no operator
highlighting; `~` is highlighted only before an ascription type. Borrows,
matcher bars, capability intersections and boolean operators retain their groups.

Blocks, record types, and multiline comments also expose syntax folds. Enable
those explicitly with `:setlocal foldmethod=syntax` if desired.

Documentation uses `#| ... |#` or `#!| ... |!#`, with longer matching bar runs when
needed. Wrong-length or wrong-family closers remain documentation text; an unclosed
fence highlights to EOF. Ordinary comment commands still produce `# %s #`, not
documentation. The compiler now checks supported attachment and semantic links;
the lexical highlighter does not replace those checks. See the
[documentation contract](../../docs/reference/documentation.md).

## Values are not keywords

Names such as `self`, `leave`, `restart`, `unsafe`, and `where` receive ordinary
identifier or call highlighting. Type names receive type highlighting inside
explicit type syntax, rather than from a reserved list of type words.

Reads spelled `true`, `false`, or `null` receive optional constant highlighting;
plain binding declarations such as `true : "local name"` remain identifiers.
This is lexical emphasis: later uses of a shadowed name cannot be distinguished
without resolving bindings. To disable that emphasis, set this before loading
syntax:

```lua
vim.g.meowy_highlight_builtin_values = false
```

Or in Vimscript:

```vim
let g:meowy_highlight_builtin_values = 0
```

The highlighter recognizes `%name` as a task group in operand positions, while
binary `%` remains an operator and `&name` remains a borrow. It does not type-check
expressions or resolve aliases. Incomplete or ambiguous
annotations may stay plain until enough punctuation has been entered. The
[language reference](../../docs/reference/syntax.md) defines their meaning.

## Checks

From the repository root, run either command:

```sh
nvim --headless -u NONE -i NONE -n -S editor/nvim/tests/run.vim
vim -Nu NONE -i NONE -n -es -S editor/nvim/tests/run.vim
```

The [regression script](tests/run.vim) checks actual syntax groups against a
[focused fixture](tests/fixtures/syntax.mwy), file detection, buffer settings,
reloading, option cleanup, and recursively loading each worked project's sources,
helper modules, and manifests, plus the guide's sample
[manifest](../../docs/guide/mod.sample.mwy). It exits unsuccessfully on failed
assertions or editor errors.

The bundle uses the standard [syntax runtime](https://neovim.io/doc/user/syntax/)
and [filetype runtime](https://neovim.io/doc/user/filetype/) conventions.
