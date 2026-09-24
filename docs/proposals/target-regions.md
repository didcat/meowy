# Target-qualified unsafe regions

Status: exploratory proposal for meowy. This is not part of the language contract
and does not describe implemented compiler behavior. The syntax, selector registry
and manifest additions below are proposed.

## The idea

An unsafe region permits operations with caller-proven safety conditions. A
**target-qualified unsafe region** restricts those operations to a compilation
target through a **target selector**:

```meowy
!{
    !`linux` {
        use_linux_interface()
    }

    !`windows` {
        use_windows_interface()
    }
}
```

Every target-qualified unsafe region must be lexically inside an explicit
`!{ ... }` unsafe region in the same function or initializer. An unsafe function signature or an unsafe caller
does not replace that enclosing region. The target selector supplies a target
condition; it does not prove pointer validity, ownership, foreign preconditions,
or any other safety obligation.

Within this proposal, "region" refers to a target-qualified unsafe region unless
an ordinary unsafe region is explicitly named. "Selector" refers to its target
selector, also used in manifest target declarations. Function and FFI signatures
retain their usual meaning.

## Selector syntax

The proposed grammar is:

```text
!`<os>[.<architecture>][:<environment>]` { ... }
```

Angle brackets name grammar components; square brackets mark optional components.
Neither appears in a selector. The target selector is literal source syntax, not a
string expression, interpolation, user binding or arbitrary compile-time predicate.
Names are lowercase and case-sensitive. Whitespace inside the selector, empty
components, repeated components and unknown names are errors.

| Selector              | Matches                                                               |
| --------------------- | --------------------------------------------------------------------- |
| `linux`               | Every recognized Linux target.                                        |
| `linux.x86_64`        | Linux on x86-64, in any recognized Linux environment.                 |
| `linux:gnu`           | Linux with the GNU environment, on any recognized Linux architecture. |
| `linux.x86_64:musl`   | Linux on x86-64 with the musl environment.                            |
| `windows`             | Every recognized Windows target.                                      |
| `windows:msvc`        | Windows with the MSVC environment.                                    |
| `windows.x86_64:msvc` | Windows on x86-64 with the MSVC environment.                          |
| `macos.aarch64`       | macOS on AArch64.                                                     |

Every specified component must match. Omitted architecture and environment
components impose no restriction; they do not select a default. The OS component
is required by design. Architecture-only selectors such as `x86_64` are invalid.
The grammar also excludes wildcards, negation, alternatives and version comparisons;
these exclusions are not a roadmap for adding them.

## Why the OS comes first

Target selectors should always answer **"which platform is this source for?"**
An architecture alone tells ordinary source code too little about what it can
safely assume. `x86_64` leaves calling conventions, object formats, system interfaces
and runtime requirements unresolved. Requiring an OS makes the platform context
explicit, preserving meowy's "what you write is what you get" principle.

A sequence of register operations can be independent of the OS:

```asm
mov rax, rbx
add rax, 1
```

Its interaction with surrounding code still needs a contract: how values enter and
leave, which registers may change, and which calling convention applies. Architecture
selection alone cannot supply that platform contract. OS-first selection is an
intentional language constraint, not a temporary limitation.

`linux` deliberately leaves architecture and environment unrestricted. A
`` !`linux` `` region, for example, semantically covers every supported Linux target,
but each compilation checks its body against only the one concrete target being
compiled. It does not trigger checking across every Linux target, enumerate toolchains
or require a universal proof of portability. Checking other declared targets requires
separate compilations; a successful build establishes validity only for its selected
target.

`linux.x86_64` adds an architecture constraint, and `linux.x86_64:musl` adds an
environment constraint. None of these selectors proves the safety of a foreign
operation.

### A separate possibility for CPU intrinsics

Architecture-specific operations could instead belong to an explicit library API.
This sketch illustrates a possibility, not a proposed module contract or a planned
feature; the intrinsic name and arguments are placeholders:

```meowy
cpu : @"cpu"

!{
    cpu.x86_64.some_intrinsic(...)
}
```

Such an API would need to define target availability, instruction requirements,
inputs, outputs and safety conditions. An unsupported call would be rejected rather
than silently omitted. It would not introduce architecture-only target selectors or
remove the platform context governing the surrounding source.

## Built-in names

Start with a small, fixed registry understood without project configuration:

| Component    | Built-in names              |
| ------------ | --------------------------- |
| OS           | `linux`, `windows`, `macos` |
| Architecture | `x86_64`, `aarch64`         |
| Environment  | `gnu`, `musl`, `msvc`       |

The initial recognized combinations are:

| OS        | Architectures       | Environments              |
| --------- | ------------------- | ------------------------- |
| `linux`   | `x86_64`, `aarch64` | `gnu`, `musl`             |
| `windows` | `x86_64`, `aarch64` | `msvc`                    |
| `macos`   | `x86_64`, `aarch64` | No environment component. |

These tables define proposed selector vocabulary, not a promise that a distribution
ships every combination. Windows GNU, mobile OSes, WebAssembly and other targets
need explicit registry extensions. There are no implicit aliases such as `amd64`,
`arm64`, `darwin` or `win32` in source selectors.

An environment identifies the target's ABI/runtime family. It does not mean a
shell environment, optimization profile, dynamic/static linkage choice or arbitrary
compiler flag. The initial macOS entries have no selectable environment suffix;
`macos:gnu` and `linux:msvc` are invalid combinations, even in inactive regions.

Recognized but unavailable targets can occur in source selectors. Selecting such
a target for an actual build still fails capability validation. The existing
[initial distribution contract](../reference/target-profile.md) supports only
`x86_64-unknown-linux-gnu`; this proposal does not expand that commitment.

## Where target facts come from

Resolve one effective output target before checking target-dependent source. The
compiler uses a validated target descriptor containing canonical OS, architecture
and optional environment fields. Selector matching uses those fields, not substring
searches over a triple or guesses from the build host.

For example, the existing `x86_64-unknown-linux-gnu` target has selector facts
`linux`, `x86_64` and `gnu`. The triple's vendor field is not a selector component.
Additional output targets need explicit descriptor mappings and validation.

Cross-compilation uses the output target. CPU tuning, enabled instructions and
static linking do not change its OS, architecture or environment identity. A
project cannot define a new selector name or claim `gnu` while using a descriptor
and native artifacts for `musl`.

## Selection and checking

Parse every region and validate every selector, its required unsafe enclosure and
nesting consistency. Then select regions before name resolution, type checking, ownership analysis,
source import resolution, FFI registration and code generation of their bodies.
Inactive bodies must be syntactically well-formed. They do not require their names,
imports, target-specific types or native symbols to exist for the selected target.
They contribute no effects, loans, initialization, cleanup actions or link demands.

A matching region remains ordinary code: it runs only if execution reaches it.
Target selection does not execute its body during compilation. Ordinary static
checking still applies to all active code, including runtime branches.

Regions are independent, not an implicit `if`/`else` chain. On Linux x86-64,
`linux`, `linux.x86_64` and a matching environment-qualified region all participate.
Reached sibling regions execute in source order. More specific selectors do not
replace broader ones. Nested selectors combine with all enclosing target
selectors. Diagnose nesting against that accumulated constraint:

- Contradictory nesting is an error: no target in the selector registry can satisfy
  the combined constraints.
- Redundant nesting is a warning: every target satisfying the enclosing constraints
  already satisfies the inner selector, so it adds no restriction.
- A compatible inner selector that narrows the enclosing constraints is valid.

| Enclosing constraint | Inner selector  | Result                                               |
| -------------------- | --------------- | ---------------------------------------------------- |
| `linux`              | `windows`       | Error: contradictory OS requirements.                |
| `linux:gnu`          | `linux:musl`    | Error: contradictory environments.                   |
| `linux.x86_64`       | `linux.aarch64` | Error: contradictory architectures.                  |
| `linux`              | `linux`         | Warning: repeated constraint.                        |
| `linux.x86_64`       | `linux`         | Warning: the outer constraint already implies Linux. |
| `linux`              | `linux.x86_64`  | Valid: narrows the architecture.                     |
| `linux.x86_64`       | `linux:musl`    | Valid: combines to `linux.x86_64:musl`.              |

These diagnostics use selector relationships, not the current concrete target,
installed toolchains or the project's support list. They apply even inside inactive
regions and do not require type checking their bodies or compiling other targets.
A selector is not redundant merely because it matches the current build. Diagnose
at the inner selector and identify the enclosing selectors responsible. A warning
does not remove the inner block's ordinary scope or change its execution semantics.

No matching region is permitted and performs no work. It does not implicitly
initialize a value or prove that every supported target has an implementation.
Malformed or unknown selectors are errors rather than silently inactive code.

## Scope, values and borrowing

Target-qualified unsafe regions have ordinary block scope. Declarations stay local; there is no
conditional injection into an enclosing scope. Declare shared state outside the
regions and update it through ordinary assignments.

This FFI sketch uses placeholders for platform-specific signatures, arguments and
normalization. It is not a complete program or an ABI declaration to copy:

```meowy
normal <Normalized><null> := null

!{
    !`linux` {
        open : ffi.extern<...>("C", "open")
        normal = normalize(open(...))
    }

    !`windows` {
        create_file : ffi.extern<...>("system", "CreateFileW")
        normal = normalize(create_file(...))
    }
}
```

`Normalized` stands for an application type. Its union with `null` permits the
initial value; target selection grants no nullable conversion or implicit narrowing.
The application must handle any remaining `null` case before requiring a result.
The Windows `"system"` convention illustrates future target-specific FFI support;
it is not added to the current [native ABI contract](../reference/native-abi.md)
by recognizing a selector.

Active assignments, moves, borrows, dereferences and cleanup follow the ordinary
rules. An owned result may leave a region through a valid assignment. A borrowed
result may escape only when its backing storage outlives every use. Assignment to
an outer binding never extends a local owner's lifetime. Overlapping active regions
are checked together, including conflicts between their loans and writes. Inactive
regions provide no evidence for safety or definite initialization.

## Rework mod.mwy around validated target selection

Source selectors require a shared target model in the compiler, manifest evaluator,
build graph and editor. This proposal reforms the current
[manifest contract](../guide/mod.md): `build.target` uses the same selector grammar
as target-qualified unsafe regions, with a stricter completeness requirement. Concrete backend
identifiers such as `x86_64-unknown-linux-gnu` remain descriptor/build metadata;
they are not a second user-facing target syntax.

The short form selects one fully explicit platform:

```meowy
-> build : {
    -> target : "linux.x86_64:gnu"
}
```

The expanded form uses an ordinary block's primary value for the selected target
and its named `supported` field for the project's support declaration:

```meowy
-> build : {
    -> target : {
        -> "linux.x86_64:gnu"
        -> supported : ["linux", "windows"]
    }
}
```

There is no separate `build.targets` field. Both forms select the same target;
the short form omits an explicit support declaration. The block's primary value
is required and follows the same rules as the short-form string.

### Complete selection, broader support predicates

`target` must have the maximum explicitness defined for its platform in the
registry. This means every applicable component, not an invented suffix for a
platform without an environment component:

| Platform | Required selection components | Example               |
| -------- | ----------------------------- | --------------------- |
| Linux    | OS, architecture, environment | `linux.x86_64:gnu`    |
| Windows  | OS, architecture, environment | `windows.x86_64:msvc` |
| macOS    | OS, architecture              | `macos.aarch64`       |

`target : "linux"` and `target : "linux.x86_64"` are errors. The compiler must not
fill missing components from the host, distribution defaults or installed
toolchains. A build needs an explicit complete target from the root manifest or
an invocation override; omission from both is an error. Invocation overrides use
the same grammar and completeness rules. This replaces the current implicit host
default rather than preserving a second route to inferred target selection.

Target-qualified unsafe regions and `supported` entries may use broader predicates such as `linux`
or `windows:msvc`. Entries in `supported` combine with OR; components within an
entry combine with AND. The selected complete target must match at least one entry
when the list is present. An empty list is an error. An absent list makes no
additional portability claim; it does not establish support for other targets.

The support list is a declaration, not an automatic build matrix, proof of
portability or request to download toolchains. Each compilation checks only its
selected concrete target. No matching source region is inferred to mean that a
package supports a target.

### Validation and build identity

The proposed resolution order is:

1. Read the root's target choice and support declarations without evaluating any
   target-dependent configuration. Apply invocation-override precedence to the
   target value while retaining the support declaration. Require a complete
   selector and resolve it through the registry to one concrete output descriptor.
2. Validate the descriptor against the selected distribution, sysroot, runtime and
   ABI capabilities. Reject unavailable targets and inconsistent inputs. An explicit
   selector does not guarantee that its toolchain is installed or supported.
3. Match the resolved target against the root's declared support set and every
   selected dependency's declared support set. A command-line override cannot bypass
   these declarations. Dependencies cannot change the consumer's target.
4. Resolve target-specific imports and exact native inputs using that same descriptor,
   then check selected source. Reject incompatible native inputs before linking;
   a selector is not evidence that an artifact has the claimed ABI.
5. Record the complete selector, concrete backend target, descriptor identity,
   selector-registry version and selected inputs in the build identity. Caches,
   reports, replay and editor analysis must agree on this selection.

Target-dependent native-input configuration needs its own explicit manifest schema;
its spelling is deferred. It must reuse selector matching, retain exact artifact
paths/digests, and define overlap handling without implicit specificity overrides.
Target choice and support declarations must not depend on regions selected by that
same choice. Manifest evaluation remains pure: target selection grants no permission
to execute FFI, inspect ambient variables or run the application during configuration.

### Dependency compatibility and changes in support

Every build validates its concrete target against the support declarations of all
selected dependencies, including transitive dependencies. Each declaration must
match that target; there is no requirement to enumerate or compile the intersection
of every package's advertised target families.

Suppose A declares `supported : ["linux"]` and depends on B. B initially also
supports `["linux"]`, but a new revision narrows its declaration to `["linux:gnu"]`:

| A's selected target | Result after updating B                                            |
| ------------------- | ------------------------------------------------------------------ |
| `linux.x86_64:gnu`  | Support declarations match; ordinary checking proceeds.            |
| `linux.x86_64:musl` | Dependency compatibility error before compiling B for this target. |

Narrowing supported targets is a compatibility change for consumers of the removed
targets. A successful GNU build does not validate A's broader Linux support claim.
Checking its advertised support matrix requires separate builds; ordinary compilation
checks only the selected concrete target. Disjoint support declarations likewise
produce an error when a selected dependency does not support that target.

A dependency's `target` value selects its standalone build target. When B is consumed
by A, A's concrete target governs B's source compilation; B's standalone choice does
not override it or require equality. B's `supported` declaration still constrains that
compilation. Without a support declaration, B's standalone choice is not an implicit
support whitelist or a guarantee of portability: its source and artifacts must still
pass ordinary checks for A's target. Precompiled dependencies and native inputs must
be compatible with the actual selected target, regardless of their support claims.

An incompatibility must be an explicit diagnostic identifying the dependency chain,
selected target and conflicting support declaration, with the declaration's source
location. For example:

```text
Dependency B does not support linux.x86_64:musl.
Required through: A -> B
B declares support for: linux:gnu
```

Dependency selection follows the [manifest guide](../guide/mod.md#put-package-dependencies-beside-aliases)
and [package lock rules](../reference/packages-and-builds.md#one-lock-authority):

- `fetch` with `hash` pins an immutable commit. Selecting another commit requires
  changing the manifest selector and explicitly updating the lock; an update does
  not rewrite that selector.
- `fetch` with `tag` or `ref` resolves to a commit recorded in the root's `mod.lock`.
  Ordinary builds retain that commit and do not advance the selection when the
  upstream tag or branch moves. An explicit dependency update can select a new commit.
- `path` uses the current local package contents. Local packages have no source lock
  or frozen working-tree digest. Changing B's local support declaration can therefore
  make A's next build fail immediately, without a dependency update. Any remote
  requirements of B still use the root's lock.

A newer upstream support declaration does not retroactively change a selected remote
revision. When that revision changes, validate its support declaration again. For
local dependencies, validate the current declarations on each build. Neither case
permits silently falling back to older contents, changing targets or ignoring a
narrowed support declaration to obtain a successful build.

Cached artifacts cannot bypass compatibility validation. Dependency identities,
manifest inputs and the concrete target participate in cache validity; artifacts
from an earlier dependency revision or an incompatible target cannot satisfy the new
build merely because their package names match.

## Acceptance and follow-up

Before adoption, specify parser integration, diagnostics and the manifest schema,
then update the language reference, CLI, editor and artifact contracts together.
Implementation should cover:

- All selector forms above, omitted components and invalid names/combinations.
- Missing unsafe enclosures, malformed inactive bodies and unresolved inactive names.
- Host/output disagreement, overlapping selectors and no matching region.
- Contradictory nesting errors, redundant nesting warnings and compatible refinements,
  including accumulated constraints across multiple levels and inactive regions.
- Ordinary scope, nullable results, escaping borrows, active loan conflicts and cleanup.
- Both manifest forms, missing/incomplete target choices, invocation overrides,
  dependency support sets, unavailable distributions, incompatible native artifacts
  and target-specific cache identity.
- Dependency support narrowing, disjoint support declarations, different standalone
  targets, transitive conflict diagnostics, lockfile updates and cache invalidation.

This proposal is the first documentation step. Compiler implementation and the
`mod.mwy` rework remain follow-up work; existing reference contracts are unchanged.
