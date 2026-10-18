# Repository verification

Run from any working directory by giving the path to the script:

```sh
python3 tools/verify.py
python3 tools/verify.py --editor both
python3 tools/verify.py --compiler
python3 tools/verify.py --runtime
python3 tools/verify.py --all --list
```

The default checks tooling regressions, local documentation links, the existing
[conformance catalog validator](../docs/conformance/README.md), and the existing
[artifact schema validator](../docs/schemas/README.md). It does not compile or
execute meowy source. Python 3.10 or newer, `jsonschema`, and `referencing` are
required. The runner never installs dependencies or fetches documentation links.

`--editor vim`, `--editor nvim`, or `--editor both` runs the existing
[editor regression script](../editor/nvim/README.md#checks) in the selected
installed editors. Missing executables and dependencies fail verification.

`--compiler` runs Cargo formatting, Clippy, Rust and native tests, compiler-harness
tests, a compiler build, and the existing
[compiler conformance harness](../compiler/tests/conformance.py). It requires the
[compiler toolchain](../compiler/README.md#build-and-run). Unsupported cases remain
visible and fail if they were already required by the bootstrap harness.
Cargo receives explicit `x86_64-unknown-linux-gnu` and `compiler/target` paths;
the harness executes that build's binary even when Cargo environment defaults
select a different target or directory.
`--runtime` runs the [native runtime prototypes](../runtime/README.md), their Python
regressions, and debug/release/ASan/UBSan checks, including fatal cleanup, guard-page
and context-switch checks, plus fixed-capacity scheduling and parent-owned child
joins that yield the worker. Stack allocation is bounded and contexts remain on
their worker thread. Explicit capture/result ownership checks cover relocation,
failed admission, panic cleanup and join retries. Explicit scope-close checks cover
nested marks, owned child-result release and retry-safe failure reports. Caller
buffers collect failure details in bounded batches; full buffers leave the next
failed child pending, and retries preserve ticket identity and cumulative counts.
Panic messages own bounded snapshots; checks cover callback/capture destruction,
copy independence, UTF-8 truncation, exact P008 evidence and per-profile storage size.
Automatic cancellation, scope-exit joins and DWARF unwinding are pending.
LeakSanitizer needs an execution environment without ptrace supervision; a blocked
sanitizer check fails visibly. This protocol does not qualify native stack unwinding.
`--all` includes both editors, compiler verification and native runtime checks.

`--strict` with `--compiler` or `--all` requires every reference catalog case to
pass. That gate is expected to fail while language features remain unavailable.
Passing it would still cover only that catalog, not the full v0.0.1 release.

`--list` prints the exact selected commands without executing them. Checks run
from the repository root and stop at the first failure. `--timeout SECONDS`
limits each command (default: 300); timed-out commands and their process group
are terminated. The runner targets the current Linux development host.

`--keep-going` runs every selected check and still exits unsuccessfully if any fail.
The final table shows passed, failed and unrun checks with elapsed times.
`--log-dir build/verification` keeps detailed command output in separate files and
shows concise progress. Relative log paths use the repository root. Failed checks
show their log paths; the summary preserves conformance pass/gap/failure counts.
Use `--all --keep-going --log-dir build/verification` for the complete logged run.

## Documentation links

```sh
python3 tools/check_links.py
python3 -B -m unittest discover -s tools/tests -p 'test_*.py'
```

The link check covers root Markdown files and Markdown under `docs`, `editor`,
`tools`, `compiler`, and `runtime`, excluding generated `target`, `build`, and `__pycache__`
directories. It validates inline links and images, relative or repository-root
file paths, percent-encoded paths, and Markdown ATX heading fragments, including
duplicate headings. Code fences, inline code, and HTML comments are ignored.
Failures include the source file and line.

This is a check for the repository's current Markdown style. Reference-style
links, Setext headings, raw HTML links or anchors, and fragments in non-Markdown
files are outside its scope. External URLs are skipped without network requests.
