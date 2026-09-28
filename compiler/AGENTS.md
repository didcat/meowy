# Compiler working rules

These instructions apply to this directory and all descendants.

## Start and handoff

- Read `STATUS.md` before changing code. It records implementation state, known gaps, evidence, and next steps.
- Read `README.md` for setup and commands, then the relevant language references in `../docs/reference/`.
- Keep compiler documentation in `docs/`; `README.md`, `AGENTS.md` and `STATUS.md` remain at the compiler root.
- Treat `../COMPILER.md` as the implementation plan and the language reference as the behavior contract.
- Inspect the current Git changes before working. Preserve existing work and coordinate edits to shared interfaces.
- After every compiler work step (investigation, edit, validation, or decision), update `STATUS.md` before starting the next step. Do not defer updates until the end of a task or session.
- Keep its `Next steps` section current and ordered. Each immediate action must identify the relevant files or rule, the intended result, and how to verify it. Replace completed or superseded actions; preserve unfinished work and blockers.
- Keep `STATUS.md` focused on the current handoff, gaps, actual validation and next steps. Replace superseded notes; Git history preserves prior work. Do not create STEP logs.
- Record outstanding failures and blockers in STATUS until resolved, then retain the current result rather than a chronological transcript.
- When delegating, assign one STATUS writer. Workers must report each completed step and its next actions so the writer can update the handoff without conflicting edits.
- Before stopping, leave enough detail for another developer to resume without the conversation. Incomplete files and failing tests must be explicitly identified.
- Never describe the bootstrap as a complete v0.0.1 release.

## Coding style

- Keep explanations short.
- Use short, clear variable names.
- Do not prefix private fields or methods with `_` unless existing surrounding code does.
- Declare visibility explicitly wherever the language permits. In Rust, use the narrowest appropriate explicit visibility, such as `pub(crate)`; trait implementations and enum variants follow Rust's rules.
- Even when public visibility is implicit, declare it where permitted.
- Use `const`, `final`, or immutable bindings for values that do not change.
- Use `static` for members that belong to a class rather than an instance.
- Do not add code comments unless the logic is extremely complex or cannot otherwise be made clear.
- Prefer small, cohesive modules and existing representations over duplicate infrastructure or unnecessary dependencies.
- Consider extracting focused modules and grouping tests by behavior when a file
  becomes hard to navigate or spans several responsibilities. This is a
  recommendation, not a hard line-count limit. Prefer separately reviewable
  structural changes that preserve interfaces, diagnostics and existing checks.

## Architecture and semantics

- Keep the frontend, semantic analysis, diagnostics, and CLI in Rust. Contain LLVM interactions behind the C++20 backend boundary.
- Keep the native runtime separate from the compiler bridge. Generated programs must not link the Rust compiler or LLVM libraries.
- Keep original source bytes and half-open byte spans. Parser context, never whitespace or name spelling, determines punctuation meaning.
- Resolve intrinsics through ordinary lexical bindings and aliases. Do not give special behavior to a familiar spelling after lookup.
- An emission initializes a block component; it does not return. Preserve evaluation order, short circuiting, and named-scope behavior.
- Preserve checked numeric behavior in every profile. Never replace a defined language failure with native undefined behavior.
- Reject unavailable features explicitly. Do not add temporary syntax, approximate ownership, implicit allocations, or permissive type conversions.
- Use assigned diagnostic codes only for their documented rules. Bootstrap capability/infrastructure diagnostics are separate and must not count as language-conformance passes.
- Treat public artifact schemas, canonical formats, and replay claims as separate implementation work. A convenient internal JSON format is not a release-compatible artifact.
- Do not silently ignore manifests, package locks, unsupported flags, or requested build policies.
- Keep output replacement safe: protect inputs, stage successful builds, and never run an executable left over from a failed build.

## Validation

- Add meaningful behavior tests for compiler changes: accepted execution, relevant rejection, and boundary cases. Avoid tests that only reproduce implementation details.
- Run `cargo test --manifest-path compiler/Cargo.toml` from the repository root after relevant changes.
- Run `cargo clippy --manifest-path compiler/Cargo.toml --all-targets -- -D warnings` and `cargo fmt --manifest-path compiler/Cargo.toml --check` before handing off a completed change.
- Run the native integration suite and `python3 compiler/tests/conformance.py` when changing supported language behavior or code generation.
- Run `python3 -m unittest discover -s compiler/tests -p 'test_*.py'` when changing the conformance harness.
- Run conformance cases independently. Compare stdout bytes and primary diagnostic codes. Missing tools, crashes, and unsupported diagnostics never satisfy an expected language rejection.
- Exercise debug and release when runtime behavior changes. Inspect actual ELF artifacts when claiming linkage properties.
- Never edit reference fixtures to hide compiler failures.
- Add source-level conformance cases for documented behavior changes alongside
  focused internal tests. Link each case to its owning reference and preserve the
  distinction between seeded metadata checks and end-to-end language behavior.
- New cases are required unless their exact B001 blocker is recorded in
  `tests/conformance_support.json`. A changed blocker or newly passing exception
  fails the gate and requires review; promote passing cases by removing the exception.
- Update the documentation/proof-obligation maps under `../docs/conformance/` when
  their contracts or evidence change. Report required passes and capability gaps
  separately. `--strict` rejects known gaps; a successful bootstrap gate does not
  establish full reference coverage or release qualification.
- Report precisely what ran and what remains unverified. Host execution does not qualify the minimum kernel/glibc baseline or a bundled distribution.
- Do not broaden checks repeatedly after they pass unless a new change or unresolved concern warrants it.

## Scope and permissions

- Keep this implementation in `compiler/` unless an integration change outside it is necessary.
- Follow the root AGENTS.md commit-slicing rules, including its size review threshold.
  A complete compiler feature is not a single commit by default. Record the ordered
  slices in STATUS before implementation and commit them as they become validated.
- Separate independently reviewable source/AST changes, analysis or graph machinery,
  checker behavior and driver/backend integration. Include the focused tests with
  each slice; keep every intermediate commit buildable and meaningful. Do not
  postpone all tests or documentation until a final oversized commit.
- Inspect staged hunks against the planned purpose before each commit. Stage only
  that slice's files/hunks and preserve unrelated changes. Run the compiler gate
  across the completed series and list its commits in the handoff.
- Do not push, publish, deploy, or contact other people without explicit authorization.
- Do not add dependencies before the owning component needs them. Keep the dependency graph locked and bootstrap inputs explicit.
- Keep permission requests limited to actual environment restrictions or actions outside the authorized task.
