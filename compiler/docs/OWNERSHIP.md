# Storage and borrow implementation

The language contract is `../../docs/reference/memory.md`. This file describes the
implementation boundary; it does not change language rules.

## Shared references and loan liveness

- A physical place names a local ID plus zero or more record field indices.
  Its type is the declared storage type, independent of flow narrowing.
- Borrowing uses that storage's address. It must not copy the referent into a new
  temporary. Distinct live locals retain distinct identities in both profiles.
- Shared references are non-null target-width pointers and copy by value.
  Equality compares addresses; dereference copies the supported copyable referent.
- Eligible roots include ordinary locals, by-value parameters and dispatch receiver
  copies. Reference-bearing referents carry transitive value summaries as described
  below. Parameter/receiver addresses refer to their
  local storage, not an original caller value. Emitted storage uses the slot model below.
  A narrowed union payload is not an addressable record projection yet.
- Each HIR emission has a unique ID, including generated record components and
  unreachable writes. Private tables associate those IDs and block IDs with the
  existing guard arena; lowering does not reinterpret source spans as identities.
- The borrow-origin pass preserves every possible `(component, place, guard)`
  alternative through immutable references, records and unions carrying references. It checks
  the intersection of origin, write and target completion guards. A retained root
  must live strictly outside the receiving block; otherwise the escape is E303.
- Completion proofs include named leave and exclude discarded restart, panic and
  enclosing-leave paths. Discarded emissions retain operand evaluation and effects.
  Constant-unreachable results do not manufacture lifetime errors.
- Emission assumptions enter the value proof before it is masked by the retained
  emission guard. Disjoint arms therefore contribute conditional proofs; merging
  their results cannot globally conjoin incompatible arm assumptions and erase loans.
- Conditional results may choose different surviving roots in each component.
  Missing per-component origin coverage or more than 4,096 alternatives/components
  in one result reports B001. Guard budget
  exhaustion takes precedence over tentative lifetime diagnostics. Assigning a
  predicate invalidates its old facts; correlated safe transfers after reassignment
  can still be conservatively rejected until stronger dataflow is implemented.
- List-bearing mutable carriers, broader emitted-alias writes, wider exclusive shapes
  and owned-value temporary borrows remain B001. Direct signatures and dispatch
  blocks can carry shared references and immutable record/union carriers.
  These are capability boundaries, not new language errors.
- All supported referents are copyable and have no owned cleanup. Shared storage needs no move tracking; scalar exclusive handle moves are described
  below. Owned destruction and panic unwinding remain unimplemented.

## Records carrying references

- Immutable record primaries, named fields and nested records can contain shared
  references. Each origin path uses `Slot(0)` for the primary and `Slot(index + 1)`
  for a named field. Component paths are distinct from physical
  referent field paths and are preserved independently through aliases and copies.
- Emitting a record composes its primary and named components without collapsing
  their origins. Named/nested emissions prefix origin paths; field or primary
  projection removes exactly that prefix. Every retained reference component must
  have origins covering the target slot's complete execution guard.
- A projection can return a surviving owner's reference from a local carrier.
  Returning the entire carrier must prove every component survives, including
  components a later caller might ignore. Escaping any local referent is E303.
- The loan graph gives each reference component its own value ID. Projecting
  `pair.left` reads only that component; reading `pair.count` reads no reference
  components. A whole-record copy or equality operand consumes every directly
  contained active reference, even if a later use selects only one component.
- Numeric/scalar contexts and formatting project the primary before loan analysis.
  Formatting a scalar primary does not keep named reference fields live. A primary
  reference still requires explicit dereference for formatting. Record equality
  retains known full-record context through block operands instead of silently
  comparing their scalar primaries; scalar literals retain numeric-primary width.
- Union wrappers containing references and omitted optional reference fields are
  supported. Concrete fields of a carrier can be borrowed without reading its other
  reference components. Whole-carrier and reference-cell borrows preserve separate
  cell and pointee origins. Fixed mutable carriers use the version model below;
  indirect/capturing function contracts remain separate work.

## Transitive pointee summaries

- `Step::Deref` extends the existing flat component paths. The borrowed cell's
  physical origin is at the reference component; its stored value's origins, bounds
  and active variants are beneath that component's Deref step. Nested references
  repeat this step without recursive State objects or capacity enumeration.
- A whole-carrier or reference-cell borrow snapshots its current reference-bearing
  contents. Copying a reference transfers its summary metadata, but does not itself
  read the pointee's reference fields. Runtime pointer uses consume the direct
  component; demanded summary uses propagate backwards through explicit CFG
  transfers to the original stored reference values.
- Dereference in a copy context selects the Deref subtree. A copied contained
  reference can survive the containing Local/Slot cell when its own pointee survives.
  The outer cell origin ends at the load. Public function bounds are attached to
  every returned transitive reference leaf, so dereferencing a call result cannot
  remove an ignored input's lifetime constraint.
- Field reads and reborrows project the relevant summary before checking loaded
  origins. Reading `outer.count` or borrowing `outer.&count` does not read an unrelated
  `outer.view` pointee. A whole dereference-copy reads every directly contained
  reference component. Address equality reads only the pointer components.
- A reborrow projects the physical pointer source and keeps its outer bounds.
  Its summary is the selected referent subtree. Crossing an intermediate stored
  reference, as in `outer.view.&field`, loads that pointer once and continues from
  its original pointee source; taking `outer.&view` instead addresses the reference cell.
- Reference-free dereferences still produce fresh unknown activity. They cannot
  replay a mutable referent's borrow-time union tag. Fixed mutable carriers use
  current stored versions. A shared reference-cell view prevents
  reassignment while it is live; later reads of a reassigned mutable reference use
  its new version. Restart continues to clear iteration proofs.
- Publishing a whole borrowed carrier must prove that its physical cell and every
  transitive reference source outlive the receiving block. An unused field does not
  excuse a shorter-lived source in a published whole-carrier summary. Directly
  borrowing a separate reference-free field retains only that selected storage.
- Extra entries, Deref paths, summary copies, projection scans and graph transfers
  consume the existing work and persistent-state budgets. Long reference chains
  and duplicating carrier shapes fail with B001 before unbounded summary growth.
  Reference construction also caps inferred and aliased chains at 64 reference
  layers before deeper Type copies can reach origin analysis.

## Active union variants

- `src/borrow_value.rs` stores explicit active-member facts alongside reference
  origins, including null and members without references. `Variant(index)` path
  steps differ from record slots. Injection adds the step, extraction removes it,
  and widening/narrowing maps member types to the destination union's normalized
  indices. Reusing an old numeric tag index would change the origin's identity.
- Omitted nullable result slots receive explicit null activity on completing paths
  without writes. Completeness requires origins for every reference leaf of each
  active member; inactive leaves require none. Unknown activity is represented by
  a bounded disjoint guard partition, never by an unexplained empty origin set.
- Each unchanging owned local/field tag domain is linked to its constructor or copied
  activity at the binding. The link is conditional on binding execution and the
  enclosing active variant, so same-named optional fields in different record
  members cannot impose contradictory unconditional tag facts.
- Value snapshots retain the proofs needed by copied or emitted origins after a
  local carrier ends. Origin checking uses cached lexical assumptions; the CFG
  applies snapshots at binding and completed-result edges. There is no global
  assumption reapplied after restart. Reset edges erase those relations along
  with other iteration facts; analyses needing stronger temporal relations remain
  conservative.
- Reads of unversioned mutable storage receive unknown activity. Fixed borrowed
  carriers link predicate-site observations to the current stored version, under
  parent activity. They never reuse an initializer's tags after replacement.
- A type predicate inspects tags without reading payload references. Effectful
  operands still run, and fresh record/block construction still has its normal
  result-transfer uses. Whole copies and equality consume their active payloads;
  projection after extraction reads only demanded components of the selected arm.
  Origin inspection follows fields/coercions without validating their payloads,
  but dereferencing a pointer to reach a tag still validates that physical owner.
  This permits inspecting an expired reference's stored tag while rejecting a tag
  load through an expired reference to the containing record.
- Contextual record constructors infer an actual shape from completing named
  emissions and must select exactly one compatible union member. Candidate field
  and primary types supply literal widths; omitted nullable fields receive member
  defaults. Multiple possible widths or member identities report E207, and
  explicit annotations remain enforced. Already typed record emissions remain
  whole alternatives rather than merging their fields across branches.
- Union equality requires the same normalized union type. Compare a nullable
  union against an explicitly typed null value of that union, or inspect its null
  tag; equality does not introduce a new member conversion.

## Direct function contracts

- Direct, inferred-result, recursive and forward-group function signatures may
  pass and return shared references and immutable record/union carriers. Every
  definition is checked independently, including uncalled and mutually recursive
  definitions. No body choice weakens the public all-input lifetime contract.
- Actual origins distinguish physical `Local` storage, target-owned emitted `Slot`
  cells, statement-owned `Temporary` cells and symbolic `Input` sources. Each active parameter reference leaf gets its
  own symbolic source;
  the parameter's stack copy is not its referent. A completed return must borrow
  from such input sources. Returning any local actual origin or dependency is
  E303. A parameter's own address is a Local source and can only be used while
  its function storage survives; it never receives the Input-source exemption.
- `State.bounds` is separate from actual pointer origins. At a call, possible
  actual sources are compatible whole input referents, concrete named fields and
  bounded-list element regions, using the result leaf's exact reference type. Each returned reference also inherits every active input
  origin and transitive bound, including ignored inputs of another referent type.
  These are lifetime/loan dependencies, not claims about pointer identity.
- Input component paths can descend through Deref to identify references stored
  inside borrowed cells. A returned reference-bearing pointee receives the matching
  input summary under bounded, mutually exclusive candidate guards. Distinct
  candidate cells cannot assert conflicting active tags simultaneously. Every
  transitive returned reference leaf also receives the public all-input bounds.
- Consequently, `first(p, q)` remains bounded by both inputs even if the body
  returns only `p`. An ignored shorter-lived `q` causes E303 on escape, and a write
  to its owner before a returned reference's final use causes E302. Bounds survive
  wrapper calls and copies; scalar-only projections and inactive result variants
  carry no continuing reference loan. Temporary carrier arguments constrain the
  contained referents rather than the carrier's temporary storage.
- Every HIR call has an explicit site ID. `Facts.calls` stores a fresh substituted
  snapshot per site; it is never cached by function ID. The CFG initializes input
  components/proofs, evaluates all arguments in order, consumes their temporary
  references together at the call node and defines the returned components.
  An early argument exit prevents call consumption. Missing snapshots on a
  potentially reachable call are B001, not an invented empty result.
- Call entry validates all active argument origins and bounds, including transitive
  pointee summaries, even when the function returns only a scalar. This happens
  after every argument returns and under the entered-call guard. A later argument's
  leave or panic prevents call-entry validation; selected field reads and pointer
  equality retain their narrower direct-component checks.
- A completed reference result requires an active compatible input source under
  the currently supported capability set. Shared reborrows preserve those input
  sources or their concrete field/list-element descendants.
  There is no static safe-reference construction, allocation or capture path that
  could supply another source. Calls without such a source have no returning reference path; nullable
  results can still return null. An entered-call guard, captured after argument
  evaluation, conditions the normal-return proof so earlier leaves and skipped
  calls remain reachable. This rule must be extended before enabling static
  reference sources, additional addressable projections or reference-producing intrinsic contracts.
- The CFG applies the call snapshot's presence/proof only on its returning edge;
  restart still erases iteration relations. Signature-based result activity may
  be more conservative than a particular body. Wider reference-result exclusive authority,
  list-bearing mutable carriers, captures, indirect calls and owned cleanup remain
  unsupported.
  Existing string values are literal-backed static views and do not create local
  referent-storage dependencies merely by passing a string value.

## Parameter and dispatch storage

- Parameters and their concrete fields may be borrowed within the function or its
  nested blocks. Returning their own cell addresses,
  directly or through another call/dispatch,
  is E303. Every definition is checked even when callers infer no normal return.
- A by-value dispatch receiver is copied into an immutable `$` local before the
  body runs. Borrowing `$` or its concrete fields observes that copy. Changes
  to an original mutable owner do not change it; its address cannot leave the block.
- A shared-reference receiver instead copies the reference value. Returning `$`
  or a reborrow of its referent retains the original sources and inherited bounds.
  Reference-bearing record/union receivers use the same component/variant facts
  as ordinary immutable bindings, without adding a function-style all-input bound.
- Receiver expressions and function arguments evaluate once in order. Source/native
  tests distinguish original versus copied addresses, nullable carrier dispatch,
  earlier argument copies, effectful receivers and enclosing-scope early leaves.
- Borrowing a whole reference-bearing receiver follows the copied receiver's cell
  lifetime, while its contained reference values keep their original sources.
  Shared dispatch does not enable exclusive `$` mutation or captures.

## Shared reborrows

- `&*view`, `view.&field` and parenthesized concrete field paths address the
  original shared referent. The HIR reborrow node evaluates its parent exactly
  once; lowering applies typed field-address operations without copying records.
  Temporary reference values from calls/blocks are allowed because their referents
  retain the original lifetime. Leading carrier fields may produce that reference,
  as in `holder.view.&field`; the prefix evaluates once. Reaching a reference only
  at the final field instead requests its reference-cell storage (`holder.&view`).
  A holder's own reference-free field uses its physical storage origin instead;
  it does not inherit unrelated contained references. Copy temporary
  owners use the statement lifetime model below.
- Pure address hints resolve types without lowering expressions or changing
  application control flow. Regression coverage includes effectful calls in
  equality and argument blocks that leave an enclosing scope before the call.
- Input component paths identify references stored in parameters. Separate
  referent paths use `Field(index)` and `Element` steps for Local, Slot and Input
  sources. Element steps conservatively name any element in that particular list region;
  runtime index expressions remain exclusively in executable HIR.
  Reborrows preserve inherited lifetime bounds unchanged rather than projecting
  ignored-input dependencies as if they were actual pointers.
- Each HIR reborrow has a unique site and bounded snapshot. The CFG consumes the
  parent dependencies at reborrow creation, then defines the projected actual
  sources plus inherited bounds. Shared parents remain readable. Scalar exclusive-parent suspension uses the
  permission pass below; assignment through a shared reference is rejected.
  Missing snapshots are checked against final node reachability: known-dead
  branches need no invented origins, while any reachable proof gap remains B001.
- Function contracts enumerate compatible whole referents, concrete named fields
  and list-element types once per nested type path, independently of capacity.
  An input `&Record` or `&List` can supply a descendant reference with a different
  target type. The abstract list region preserves potential input bounds even at
  capacity zero; it does not prove initialized storage or bypass bounds checks.
  Fields inside union payloads, primary-ascription addresses,
  static sources and reference-producing intrinsic contracts remain separate work.
- Type walks, candidate frontiers, projected paths and snapshot expansion consume
  existing work/storage budgets. Many matching fields multiplied by returned
  reference components reject with B001 before unbounded contract expansion.

## Mutable borrowed carriers

Bindings with mutable owned fields participate in changing-value analysis even
when their root binding uses `:`. `Proofs.mutable` retains whole-binding replacement
permission; `Proofs.fields` records owned mutable descendants and `variable` combines
them for branch/restart snapshots and refinement invalidation. References do not
inherit this metadata from their pointees, so shared access remains read-only.
Fixed reference/allocator-bearing record aliases with mutable fields use the
existing origin/bound snapshots, publication and refresh machinery. Alias backing
still matches the declared root slot flag and complete type. This grants no
permission to replace an immutable root or store a value beyond its owner lifetime.

- Ordinary mutable locals may contain fixed Copy shared-reference or allocator
  values, records and closed unions, including nullable references. Eligibility
  uses `Type::fixed_borrowed_value`; nested referents must have supported fixed
  shapes. Lists and exclusive carriers remain gated; exact borrowed aliases follow
  the synchronized write model below.
- Whole assignment replaces the stored component versions after RHS completion.
  Copies and earlier argument operands keep their original origins, public lifetime
  bounds and loans. Mutable reference fields can be assigned on completed fixed
  records; the selected field must be mutable, independently of enclosing slots.
- Pure field stores use the root state after RHS effects and preserve sibling loan
  IDs. Completed inner writes survive Leave; an unfinished outer write is skipped.
  Reads select the relevant component before checking lifetime. Safe sibling reads
  and overwrite-before-read can therefore repair a carrier with an expired member.
- Nullable and nested variant predicates use current observation snapshots in both
  origin analysis and the loan CFG. Tag-only inspection skips payload consumption;
  a proven null path may copy null, while an active expired reference use is E303.
  Assignment invalidates overlapping refinements, so earlier tags cannot narrow
  a replacement. Live cell/pointee conflicts still report E302.
- Restart reuses canonical activity and exact predecessor transfers. Inactive
  reference paths need no origin; every active reference path needs physical origin
  coverage, and lifetime bounds cannot substitute for it. Iteration-local expired
  sources remain terminal across subsequent initializations. The existing part,
  replay and shared-work limits apply without list-capacity expansion.
- The [mutable-carriers example](../examples/mutable-carriers.mwy) alternates an empty
  reference field and a live reference through replacement and Restart. Tests cover
  field RHS/Leave effects, old copies, nullable/nested variants, argument snapshots,
  public bounds, transitive sources, header coverage and remaining storage gates.

## Mutable reference fields

- Fixed Copy shared-reference field types, including nullable references and nested
  carriers, retain field mutability in constructors and signatures. An omitted
  nullable field defaults to null. Construction enforces ordinary retained-source
  lifetimes; emitting a reference to the constructor's local still reports E303.
- Mutable borrowed emitted names retain initial component snapshots and may be read
  or borrowed. `Proofs::versioned` includes fixed reference-bearing aliases for this
  purpose. Exact-backing assignment and field paths synchronize the result as
  described below; union-view addresses/field paths and surviving published outer result slots
  enclosing an inner Restart remain B001.
- After the record finishes, `r.view = &next` replaces only that field's origins,
  bounds and activity. Nested owned paths use the selected field's mutability;
  enclosing bindings/record fields may be immutable. Incompatible mutability remains E206, immutable selected slots E305
  and incompatible assignment types E207. Lists and exclusive carriers stay gated.
- Origin state and loan bundles use the current root after RHS completion. Earlier
  sibling or whole-record writes survive, old copies retain their own loans, and
  unchanged sibling IDs are preserved. Leave skips an unfinished outer store while
  retaining completed inner stores. Live cell borrows prevent overlap as E302.
- Nullable field predicates use the current stored version, including after a
  sibling field's effectful RHS. Expired payloads may be inspected and overwritten;
  active reads remain E303. Restart preserves nullable activity, required physical
  origin coverage and terminal expired sources. Public input bounds and transitive
  call-entry validation apply to each reference field as before.
- The [reference-fields example](../examples/reference-fields.mwy) changes a field's
  referent, then updates its former owner. Library and native tests cover constructor
  reads, declared defaults, nested fields, old-copy/sibling loans, RHS/Leave effects,
  current tags, restart, transitive bounds and the emitted-alias write gate.

## Borrowed emitted-alias writes

- A mutable emitted name containing shared references may be assigned or updated
  through pure mutable field paths when its fixed Copy type matches its completed
  result field or one exact member of that field's union. Reference-bearing carriers may also contain allocator
  lifetime bounds. Allocator-only aliases retain their previous capability boundary.
- `borrow/aliases.rs` updates the alias's selected result component after RHS
  completion. It masks old state outside the completing write guard, replaces the
  selected subtree inside that guard, then merges the two. Other branches and
  sibling fields retain their own origins, bounds and activity. Each written
  reference/bound must satisfy the existing target-slot retained-source lifetime rule.
- `loans/aliases.rs` redefines the corresponding CFG result components and links
  them to the current alias versions. Overwritten loans can end; prior copies retain
  their sources. Field synchronization touches only the selected subtree, preserving
  sibling IDs. Physical alias-cell borrows still conflict with overlapping writes.
- Branches, nullable tag observations, nested RHS effects and Leave publish the last
  completed writes. A diverging RHS skips its outer store. Aliases introduced in
  mutually exclusive scopes may share a result field while keeping guarded values.
  The completed carrier can escape an earlier source only after that component has
  been replaced with a source that survives its receiving scope.
- Whole proper-subset union views use explicit tag/path conversion below. Their
  addresses and field writes remain B001. Proven discarded
  backing keeps ordinary local versions without a result transfer, as described below.
  Restart supports writes when the result owner is reset by the target or is
  independent of it. Written result owners strictly enclosing a Restart target
  remain B001 until surviving result snapshots participate in header merging.
  Ordinary mutable carriers and read-only aliases retain existing restart support.
  No list capacity expansion, representation, syntax or runtime changes are introduced.
- The [alias-writes example](../examples/alias-writes.mwy) replaces an emitted pointer,
  reads an earlier copy, then updates its former owner. Library/native coverage
  includes selected result loans, branches, Leave, nullable tags, transitive/public
  bounds and exact-backing/Restart gates. All extra paths and snapshots use existing
  work and fact budgets.

## Restart ownership for alias writes

- Restart discards its target's own result slots and those in nested scopes. Alias
  writes in those slots use existing block-entry definitions and per-iteration
  origin replay: the completed result reflects only the completing iteration.
  Writes in independent scopes also no longer trigger a body-wide capability gate.
- `borrow/mutable.rs` records lexical block parents during its bounded HIR scan and
  associates alias writes with their emission target, not the lexical write site.
  A charged ancestor walk rejects a written result owner that strictly encloses
  a Restart target. This remains conservative regardless of statement order;
  surviving outer results need their own header merging before that gate can lift.
- Parent tables are capped at 65,536 blocks; parent walks and scan depth at 256.
  Lookups and storage traversal consume shared proof work. Missing, duplicate or
  cyclic ancestry and exhausted work fail B001 rather than supplying incomplete proof.
- Canonical headers still track surviving ordinary mutable references. Copies of
  references to discarded alias cells become terminal expired sources and cannot
  revive when the same alias site is initialized next iteration. Actual expired
  use is E303; overwrite-before-read keeps its existing rules. Cell/source conflicts
  remain E302, and retained-source lifetime checks remain mandatory on completing paths.
- Whole/field writes, nullable predicates, nested RHS effects and Leave preserve
  their order. A Restart from a RHS skips its outer store and discards the target
  iteration's result. Published result backing must match the alias or one exact
  member; broader union views and bounded allocator-only aliases remain separate work.
- The [alias-restarts example](../examples/alias-restarts.mwy) initializes a fresh
  emitted pointer each iteration, changes it, and publishes only the last iteration's
  value. Library/native tests cover named outer emission targets, inner/outer resets,
  independent loops, carried input versions, old copies, expired cells and gates.

## Discarded borrowed-alias writes

- `Backing::Discarded` is assigned only after frontend completion analysis proves
  that the emission cannot initialize a completed result. The backend already
  provides a typed transient cell whose lifetime belongs to the emission target.
  Fixed shared-reference aliases may now be assigned or updated through mutable
  field paths in that storage.
- Origin and loan passes retain normal current-value versions, sibling IDs,
  physical accesses and transitive summaries. They skip result synchronization
  because no published component exists. A local source may be used while it is
  alive; later payload reads still report E303 after it expires. Overwrite-before-read
  and tag-only inspection retain their normal rules, and old copies retain their loans.
- Discarded cells do not enter the surviving-result Restart gate. Their alias
  versions use existing canonical local headers, so an inner restart can preserve
  the cell and its live sources. Restarting or leaving the target ends that cell;
  copied slot references expire and cannot revive at the next initialization.
  Published outer results use fixed identity, preexisting alias headers or proven
  late initialization. Declared scalar slots use the carried initialization proof;
  reference-bearing initialization across backedges remains separate work.
- Leave and panic preserve earlier effects and skip unfinished outer stores. A
  borrowed cell can survive the lexical scope that introduced its alias while the
  target remains active. Reads or calls cannot bypass E302 conflicts or E303 expiry
  merely because the eventual result is discarded. No owning cleanup or runtime
  representation is added.
- The [discarded-aliases example](../examples/discarded-aliases.mwy) changes a transient
  pointer to a target-local value and reads it across an inner restart before Leave.
  Tests distinguish finite, potentially published results from endless or otherwise
  non-completing paths; only proven discarded backing uses this rule. Broader union
  views, allocator-only alias bounds, lists and exclusive carriers remain gated.

## Widened borrowed-alias backing

- A reference-bearing alias may have a narrower lexical type than its completed
  result field, as when a field is emitted only on one branch and otherwise defaults
  to null. Writes now support that view when its entire type is one exact concrete
  member of the result union. The lexical type still constrains every RHS.
- `borrow/aliases.rs::result_slot` supplies the same path to origin and loan passes.
  It selects a mutable result field and, when types differ, that field's exact
  Variant member. Subsequent field paths descend beneath this member. Identical
  backing retains the previous path. Whole union views additionally return the
  backing type for conversion, as described below.
- Updating a whole alias replaces that member's payload, preserving the enclosing
  member activity established by emission. Field updates replace only their selected
  payload subtree. Other branches, absent defaults and sibling components retain
  their sources, tags and loan IDs. No outer union member is inferred from a new RHS.
- Current nested tag observations, earlier copies, RHS/Leave effects and cell/source
  loans keep their existing checks. Returned payloads still satisfy retained-source
  lifetimes. Restart supports reset slots, preinitialized aliases and proven late
  initialization. Carried scalar and reference-free record/list slots use a separate stateful proof; other carried
  initialization remains gated. Views spanning a
  proper subset of a larger union use the
  whole-assignment conversion below; their addresses and field paths stay gated.
- The [widened-aliases example](../examples/widened-aliases.mwy) replaces a pointer in
  an optional emitted field and preserves the absent path. Library/native tests
  cover declared/inferred backing, heterogeneous references, record payload fields,
  nested nullable tags, old copies, cell loans, escape lifetimes and reset iterations.
  This uses existing layouts, code generation, source syntax and proof budgets.

## Whole union-alias assignment

- A fixed reference-bearing alias with a union type may be wholly assigned when
  every lexical member belongs to its larger result union. The RHS must still fit
  the lexical type; assignment cannot introduce a member admitted only by backing.
  Identical views and exact concrete members retain their prior field-write rules.
- `result_slot` returns the result path and effective backing type. Whole subset
  writes reuse `State::convert` to remap active tags, origins and bounds by normalized
  member identity before replacing the guarded result field. Local alias state keeps
  its lexical tag domain, so subsequent predicates and copies use the correct indexes.
- Loan synchronization reuses `Graph::retag`, also used by ordinary union conversion,
  to map bundle keys into the backing domain. Matching result definitions therefore
  follow the same members as origin/activity state. RHS expressions run once; no
  synthetic read is introduced, and old copies retain their original value IDs.
- Null/reference transitions, nested record members, branches and Leave publish
  the last completed member. Reset-scope Restart and discarded backing retain their
  established rules. Physical conflicts and retained-source expiry remain E302/E303.
- Taking an address or writing a field through a proper-subset union view remains
  B001 because its lexical tag representation differs from backing. Publication
  reference-bearing initialization across backedges and bounded allocator-only aliases remain separate
  proof work. Type/path copies and member remapping use existing charged budgets.
- The [union-aliases example](../examples/union-aliases.mwy) alternates a nullable
  reference while backing admits an extra string member. Library/native tests cover
  shifted indexes, nested members, old copies, branches/Leave, retained lifetimes,
  declared/inferred backing, reset iterations and address/field/type boundaries.

## Published result snapshots

- `borrow/published.rs` records initialized mutable reference-bearing result fields
  independently of the final-completion-filtered result accumulator. Keys retain
  the owning block and field index; each snapshot stores the actual backing type,
  origins, public bounds, variant activity and entry guard.
- Emissions and returning alias writes update this state once in source order.
  Exact-member paths and whole union-view conversions reuse the existing alias
  resolver. An unfinished outer assignment leaves earlier RHS writes intact.
- Block entry and Restart capture only ancestor-owned published slots. Ending an
  alias's lexical scope does not remove its result state; ending or restarting the
  owning block excludes that partial result. Conditional initialization stays
  conditional, without inventing nullable defaults or canonical header activity.
- Snapshot copies, type walks and stored facts use the existing work and origin
  budgets. These are analysis inputs, not canonical headers or loan transfers.
  Fixed surviving publications use identity; preinitialized changing aliases use
  canonical headers. Late aliases use explicit frontier certificates. Declared
  scalar slots use carried initialization proof; reference-bearing slots remain gated.

## Fixed published restart results

- Borrowed result aliases may be written before or after a restarted inner body
  when its reference-bearing result slots stay unchanged inside that body. The bounded
  planner records both the emitted owner and the lexical block containing each
  write. Nested blocks and RHS writes count within their enclosing restart region.
- `Facts.fixed_published` records the surviving result owners certified by that
  write-scope proof. Entry and backedge publication snapshots, including explicit
  empty snapshots, supply backing types, component sources, bounds and activity.
  The loan pass checks active owner/field identity and agreement under the shared
  entry/backedge guard. The scoped-write proof supplies stability across resets,
  even when iteration guards have no overlap.
- The ancestor result keeps its existing bundle IDs across the edge. This identity
  transfer adds no payload read, copy, definition or loan. Old copies, physical cell
  loans, inherited bounds and retained-source lifetime checks continue normally.
  Nullable defaults remain completion behavior; snapshot checks do not initialize
  an absent slot or reinterpret lexical tags as backing tags.
- RHS loops finish before their outer store; Leave skips that store while preserving
  earlier writes. Lexical aliases may end before an inner loop while their result
  owner remains active. The [fixed-published example](../examples/fixed-published.mwy)
  preserves such a result and releases the replaced source's loan.
- Changing preinitialized aliases use projection; completing-path aliases use late
  initialization frontiers. Missing or malformed ancestry, snapshots and budget
  exhaustion remain capability failures. Reference-bearing initialization across backedges,
  union-view addresses/fields, allocator-only aliases and owning cleanup need separate proofs.

## Changing published restart results

- An initialized mutable borrowed alias may change inside an inner restarted body
  while its emitted result owner survives. The alias must already have a live local
  version and compatible published backing when the restarted block is entered.
  Separately proven late aliases omit header initialization and use their real
  definitions on paths that cannot reach a backedge, as described below.
- The planner retains alias identity and schedules refreshes at the restarted block
  and its ancestors through the result owner. Existing canonical local headers keep
  lexical tags, origins, bounds and terminal expiry; result projection separately
  converts them into identical, exact-member or wider-union backing domains.
  Publication updates retain the original binding-event guard across iteration
  resets, so conditional backing members remain disjoint even if tested values change.
- Normal completion refreshes before entry-environment restoration. Leave refreshes
  at the actual exit, including exits bypassing the loop or alias declaration scope.
  Final iterations without a store therefore retain backedge sources, and skipped
  outer stores cannot replace the last completed RHS assignment.
- Origin refresh does not read or eagerly validate payloads. Final owning-result
  retention checks all active sources/bounds, so expired iteration locals and
  temporaries cannot escape; repair before completion retains existing behavior.
- Loan refresh defines result summaries using demand-only links to current alias
  versions, including direct reference components. Initial/backedge transfers then
  carry demand to the actual source snapshots. Refresh creates no physical access,
  payload use or initialization event; old copies and cell loans retain their IDs.
- Fixed publications retain their identity proof. Shared backing resolution, union
  remapping and existing budgets are reused. Nullable defaults, RHS evaluation and
  ownership diagnostics remain unchanged. Lists, reference-bearing initialization across backedges,
  union-view addresses/fields, exclusive carriers and owning cleanup remain open.
- The [changing-published example](../examples/changing-published.mwy) prints current
  pointers across iterations, then the old copy and the completed result.

## Late published aliases

- A borrowed result alias may be initialized inside a restarted body when its
  emission cannot reach any backedge to that body. Completing-iteration aliases
  keep ordinary binding, backing, assignment and normal/Leave publication behavior.
- The frontend checks new enclosing emissions at each restart, except declared
  scalar obligations deferred to carried-initialization proof. Successful sites
  record their target, checked entry guard and first
  emission ID at block entry. This ID is separate from the reachable-write counter,
  since dead emissions still have HIR identities.
- The ownership scan requires a matching certificate for every actual Restart
  site. Late classification validates binding/emission evidence, consistent entry
  indices and disjoint emission/backedge guards across all sites. Missing evidence,
  overlapping initialization or exhausted proof work is B001.
- Origin and loan passes require a late alias to be absent from initial headers.
  They validate the same certificates and use its real Bind/Emit definitions later;
  no initialized header value, reference or payload read is fabricated.
- An alias can be late for an outer restart and preinitialized for a nested one.
  Existing binding-event masks, widening, expiry and normal/Leave demand transfers
  remain in force. Initializer restarts happen before binding/emission; RHS Leave
  preserves completed stores and skips unfinished ones.
- Duplicate emissions remain E205, mutability checks remain unchanged and returned
  references still obey E302/E303. Declared scalar and plain record slots can survive a backedge using
  the separate stateful proof below. Reference-bearing lists, union-view
  addresses/fields, allocator-only aliases and owning cleanup remain separate work.
- The [late-published example](../examples/late-published.mwy) runs restart prefixes
  three times, initializes once and preserves an old pointer copy after rebinding.

## Carried scalar initialization

- A declared non-nullable boolean, integer, float or static-string result slot may
  retain its initialization across an inner restart. Named fields and primaries
  use the same proof. The explicit enclosing result type supplies the storage shape;
  inferred, nullable, allocator and reference-bearing carried slots stay gated.
  Reference-free record/list slots use the bounded extensions below.
- The frontend defers only eligible enclosing-emission and completion obligations.
  Current-iteration duplicate emissions and type/mutability rules are unchanged.
  Every deferred slot must pass a separate initialization proof before native lowering.
- The proof explores bounded states on the existing loan CFG: initialized-slot
  bits, active result scopes and Boolean local values. Inner restarts preserve
  ancestor initialization; restarting or discarding an owner clears its slots.
  Normal and Leave completion require every carried slot initialized exactly once.
- Boolean local/literal trees using !, &&, ||, == and != retain value and copy semantics. Conditions
  refine Boolean states; unknown values conservatively explore both alternatives.
  Expressions containing effects remain unknown as a whole, so a post-RHS store
  cannot reinterpret an operand evaluated before those effects. Indirect stores and
  exclusive calls invalidate Boolean knowledge. Source effects are never replayed.
- The limits are 64 carried slots, 512 known Boolean locals per state and 16,384
  visited states, with existing graph/proof work charging. Missing, duplicate or
  unproved initialization and exhausted proof remain B001; ordinary E204/E205 cases
  are preserved. Success is a proof over every explored completion, not one witness.
- Shared address-taking uses the acquisition proof below. Local exclusive scalar
  borrows additionally require the [restart frontier proof](EXCLUSIVE_RESTARTS.md).
  Borrowing a copied completed result uses ordinary rules. No runtime flags, payload
  reads, storage allocations, ABI changes or dependencies are added by the analysis.
- The [carried-scalars example](../examples/carried-scalars.mwy) prints one initializer,
  three iteration values and the retained field. Stateful reference-bearing
  publications, broader Boolean/value analysis and owning cleanup remain separate work.

## Carried reference-free records

- A declared record result slot can retain its whole initialized value across an
  inner restart. Nested records may contain non-nullable Boolean, integer, float,
  static-string and unit members. Record primaries follow the same shape rules.
  Lists follow the separate extension below. Top-level unit slots, unions,
  references, allocator and owning types remain outside carried initialization.
- Eligibility visits at most 256 type parts and 32 levels, counting the slot root,
  record primaries and fields. Traversal and field-name work charge the shared proof
  budget. Exhaustion is B001, not permission to skip initialization checks.
- The existing CFG proof tracks completion of the entire slot, not separately
  published fields. An unfinished initializer cannot satisfy owner completion.
  Effects execute once in source order; panic or Leave skips unfinished stores.
  Inner restarts retain ancestor initialization, while owner resets clear it.
- Mutable field writes and whole-record replacement retain existing type and
  mutability checks. Old value copies are independent of later record writes.
  The extension adds no backend storage, runtime flags, payload reads or cleanup.
- Shared borrows of original carried record storage and field projections use the
  acquisition and lifetime proof below. Exclusive scalar-field borrows use the
  [restart frontier proof](EXCLUSIVE_RESTARTS.md#carried-record-fields); whole-record
  and non-scalar exclusive borrows remain B001. Ordinary local copies, copied
  completed results and supported scalar sibling loans retain their existing rules.
- The [carried-records example](../examples/carried-records.mwy) executes its
  initializer once and retains both fields across three iterations. Native
  regressions also cover nested unit/scalar members, copies, writes, owner resets,
  Leave, partial panics and rejected incomplete or duplicate initialization in
  debug and release.

## Carried reference-free lists

- Declared fixed-capacity lists may retain their initialized length and payload
  across inner restarts. List elements may be scalar/unit values, plain records or
  nested lists; records may contain lists too. Named slots and primary values use
  the same rule. Nullable/union/reference-bearing/foundation/owning shapes remain
  gated, including unsupported elements beneath zero capacity.
- Shape traversal visits at most 256 type parts and 32 levels. Each list counts
  one container plus its element type, independent of capacity. Existing list
  construction also enforces capacity at most 65,536 and layout at most 1 MiB.
  Exhausted shape/proof work remains B001. Empty lists still require one completed
  emission; zero elements do not satisfy whole-slot initialization by themselves.
- The existing initialized/active-slot proof tracks the entire length/payload
  value. Copies retain independent storage and length; whole-list replacement,
  `.add()` results and replacement of a record's list field keep ordinary type and
  mutability checks. Index reads retain one-based initialized-length checks.
  Owner reset clears initialization, inner reset preserves ancestor slots, and
  Leave/panic skips unfinished initializers or replacement stores.
- Shared direct/projected borrows of list-containing carried slots use the
  acquisition and lifetime proof below. Exclusive named Boolean/integer/float
  fields within containing records use the existing
  [scalar-field restart proof](EXCLUSIVE_RESTARTS.md#carried-record-fields).
  Indexed scalar acquisition uses the
  [carried-element proof](EXCLUSIVE_RESTARTS.md#carried-list-elements).
  Indexed writes use the whole-slot and reservation proof below. Independent
  copies and completed-result locals keep their existing rules.
- Ten source/shape/proof groups and seven native groups cover these boundaries.
  The [carried-lists example](../examples/carried-lists.mwy) prints one initializer,
  the old copy's length and the retained updated list. Native output, dynamic bounds,
  owner resets, Leave, partial panics and primary rejections are checked in both
  profiles. No backend, runtime or dependency changes were needed.

## Carried indexed writes

- Indexed SetPath assignments can update initialized carried reference-free lists,
  including mixed fields/indexes, scalar fields inside immutable record elements,
  and whole scalar, record, list, string or unit leaves. The existing type and
  selected-slot mutability checks apply; this does not enable new carried shapes.
- `loans/control.rs` attaches containing-slot Acquire before owner capture and at
  the completed store. The owner must be active and the entire slot initialized,
  even if the index or RHS later cancels. The blanket `carried::storage` gate is
  removed. Static field-only paths retain their previous behavior.
- The existing first-list reservation covers every returning index/bounds phase
  and the final store. It has no loan authority and ends at the last actual phase.
  Returning index or RHS replacement/nested writes in that region remain E302;
  disjoint holder siblings remain writable. The whole first-list region remains
  conservative, including different elements and their fields.
- Each index executes once after its containing length is captured. Bounds are
  checked before later indices/RHS. The final address stays fixed when RHS changes
  index variables. Stores replace only the selected leaf; surrounding lengths,
  capacities, siblings and old value copies remain intact. Shared final-use RHS
  reads can finish before the store; retained shared/exclusive loans still conflict.
- Leave, Restart or panic skips unfinished checks/stores and preserves completed
  effects and earlier reservation demand. Owner reset clears initialization and
  cancels its pending store; fresh initialization cannot revive expired references.
  Certified disjoint shared headers and local exclusive sibling loans keep their
  existing reset rules. Reference/temporary-derived write roots and broader
  reference-bearing/owning carried shapes remain gated.
- Nine source groups, four graph groups and eight native groups cover initialization,
  physical first-list regions, capture/store events, phase demand, typed scalar and
  aggregate stores, mutability, captured indices, bounds, final use, owner reset,
  cancellation and shared/exclusive conflicts. Native cases run in both profiles.
  The [carried-writes example](../examples/carried-writes.mwy) combines ordered writes,
  an independent old copy, a shared header and an exclusive scalar sibling. No
  backend, runtime, dependency or reference-fixture changes were required.

## Shared carried list borrows

- A shared whole-list or element borrow addresses the original carried slot.
  Containing records, nested lists and record fields retain canonical Slot/Field/
  Element sources. `loans/transitive.rs::referenced` emits the existing
  containing-slot Acquire event for every direct shared or exclusive root. The
  owner must be active and its entire slot initialized, including empty lists.
  No synthetic payload reads are introduced.
- Element and field reborrows retain their parent reference identity and authority.
  Index expressions execute once in source order and check the current initialized
  length at each selected list. A returning index cannot replace borrowed storage;
  a canceled index preserves completed effects and skips unfinished acquisition.
  This adds no new index arithmetic, storage layout or runtime flags.
- Shared references can survive inner restarts and alias scope exit while the
  result owner lives. Ending, leaving or restarting that owner expires old sources;
  reinitializing the same storage cannot revive an old view. Replacing an expired
  reference before reading it retains the existing overwrite rule. Reference copies
  and call-returned views retain their own sources and public input bounds.
- Whole-list replacement remains E302 while a whole/element view is live. Disjoint
  fields outside the borrowed region remain writable; mutation after final use is
  allowed. `&items[index]` copies through a shared list reference; `&(items[index])`
  borrows the selected element. Shared access never grants mutation permission.
- Known shared list headers can coexist with supported local exclusive scalar
  sibling loans under the existing complete-header certificate. Genuine call/input
  opacity and exclusive descendants retain their restart gates. Local exclusive
  scalar list elements use the [restart proof](EXCLUSIVE_RESTARTS.md#carried-list-elements).
  Indexed SetPath uses the [write proof](#carried-indexed-writes). Whole-list exclusive
  values remain separate; nullable, union, reference-bearing and owning carried
  slots retain their shape gates.
- Ten source/proof groups and seven native groups cover initialization, physical
  paths, parent identity, bounds, cancellation, old copies, final use, owner resets,
  Leave, calls and capability gates in both profiles where applicable. The
  [carried-list-borrows example](../examples/carried-list-borrows.mwy) retains an element
  view across three iterations. No backend, runtime or dependency changes were needed.

## Shared carried record borrows

- A shared borrow of a carried record alias or nested field addresses the existing
  result storage, not a temporary copy. Direct and projected acquisitions use the
  same containing-slot Acquire event: the result owner must be active and the
  whole record must be initialized on every explored path. Field projection does
  not weaken that requirement or publish partially initialized members.
- The existing physical-storage solver retains the exact field path and checks
  acquisition, source expiry and conflicting writes. A field loan allows writes
  to disjoint fields, but overlapping field writes and whole-record replacement
  remain E302 while the shared view is live. Writes after its final use retain
  ordinary semantics.
- Shared references and projected reborrows may survive inner restarts and the
  alias's lexical scope while the result owner lives. Reborrows retain parent
  authority; they do not invent new payload reads or independently revive storage.
  Owner completion, Leave and owner reset expire the old source. Using a stale
  source remains E303, even if native storage is later reused at the same address.
- Replacing an expired handle before its next use is permitted under the existing
  rules. Old reference copies retain their original sources, and reference-returning
  calls retain their public input bounds. Exclusive whole-record and non-scalar
  projected borrows remain B001; nullable, union and reference-bearing carried
  shapes are still outside this slice.
- The [carried-record-borrows example](../examples/carried-record-borrows.mwy) retains
  a field reborrow across three iterations after its alias and parent view leave
  scope. Source/proof tests cover early and inactive acquisition; debug/release
  tests cover whole records, nested projections, address identity, old copies,
  disjoint writes, final use, owner resets, Leave and rejected stale/conflicting use.

## Shared carried scalar borrows

- A shared reference to a carried scalar alias addresses the existing result cell.
  The actual borrow node records an Acquire event keyed by result owner and field.
  Every explored acquisition requires that owner active and its slot initialized;
  a missing proof reports B001 at the borrow span. Acquisition adds no runtime read.
- The existing storage solver independently checks the canonical cell at acquisition.
  Inner restarts retain ancestor storage. Ordinary mutable-reference headers can
  carry its address beyond the alias's lexical scope, without inventing definitions
  or weakening reference origin, public bound or demand-only transfer checks.
- Ending or restarting the result owner expires retained references. Reinitializing
  the same static slot cannot revive an old view; reads remain E303. Replacing an
  expired reference before reading it uses the existing overwrite rule. Shared
  last-use checks still reject conflicting slot mutation with E302.
- The [carried-borrows example](../examples/carried-borrows.mwy) retains a view across
  three iterations and replaces it after owner completion. Source and native cases
  cover owner resets, function bounds, Leave, scalar widths and storage identity.
  Exclusive loans that cross backedges, nullable/reference-bearing initialization
  and wider value analysis remain separate work. Local exclusive carried-scalar
  borrows require their own frontier proof. Runtime/backend and dependencies are unchanged.

## Mutable shared-reference bindings

- Ordinary mutable locals with a fixed `&T` type can be rebound in straight-line
  flow and guarded matcher/short-circuit paths. T may be any currently supported
  referent, including a record, union, bounded list or nested reference.
  Nullable and aggregate bindings follow the fixed carrier rules above; mutable
  emitted names with identical or exact-member backing use synchronized writes.
- The physical LocalId remains the same cell. Origin analysis preserves its
  current full State and replaces it only after a returning RHS; initial
  `Facts.locals` snapshots stay unchanged. The loan graph creates fresh immutable
  value IDs for assignments and reads. Earlier copies retain their own origins,
  bounds and transitive snapshots when the binding changes.
- `&binding` still borrows the physical reference cell, so a surviving cell view
  blocks reassignment with E302. A final-use RHS load can finish before the store.
  Copying the old reference value keeps its pointee loan instead; rebinding does
  not erase that copy's dependencies. Public function bounds remain unchanged.
- RHS expressions and call arguments evaluate in order. Straight-line nested
  assignments preserve already evaluated operands and the final outer store wins.
  A panic RHS performs no store. Expired contents can be overwritten without
  reading them; a later read of an expired current origin or bound reports E303.
- `borrow/branches.rs` snapshots the incoming mutable-reference environment after
  condition effects. Each arm starts from that same snapshot under its condition;
  only normally returning arm states contribute to the join. Origins, bounds,
  activity and proofs are masked by each arm's returning guard before merging.
  An unwritten arm retains the incoming version. The merge itself never reads a
  reference, so expired versions can still be overwritten before any use.
- Matcher arms and `&&`/`||` right operands share the same merge helper. Returning
  expressions carry completion proofs into the continuation, excluding partial
  panic paths even inside nested blocks. The join's continuation is the union of
  returning guards, and arm-local assumptions do not constrain the opposite arm.
- The loan graph merges fresh versions with demand-only predecessor transfers,
  including direct reference components. A join adds no eager payload read;
  ordinary assignments and value copies retain their runtime reads. Old operand
  versions and physical cell loans keep their existing identity.
  `loans/branches.rs` reuses unchanged versions, normalizes projected component
  paths and merges duplicate origins. Changed versions use charged completion-reach
  queries; repeated full-graph queries can reach the existing work limit.
- `borrow/mutable.rs` performs one bounded HIR scan for each entry/function body.
  It identifies reference assignments and restart targets; separate function bodies
  are checked independently.
  Facts.merging records which bodies enable branch/continuation merging;
  assignment-free bodies preserve their existing loop/proof behavior.
- Forward Leave uses `borrow/exits.rs`: each target records the mutable-reference
  IDs that existed at its entry. A Leave captures only those surviving versions
  under its actual exit guard before scopes or branches restore their environments.
  Capture and merge do not read reference contents.
- At target completion, captured exits merge with actual fallthrough using the
  shared guarded State helper. Their union continuation is installed before result
  slot proof/completeness, including when no mutable-reference IDs survive. An
  outer-target exit remains queued across inner block closes; it creates no inner
  continuation and cannot execute an unfinished RHS store.
- The loan Scope keeps its entry versions and queued Leave predecessors. Capturing
  an exit defers the edge until its target joins all predecessors through the same
  demand-only merge used by branches, then reaches the existing result-copy node.
- Target-local bindings are still removed, and statement-owned temporaries/result
  slots retain their existing lifetimes. A surviving mutable reference may contain
  an expired source after Leave and can be overwritten safely; reading that current
  value remains E303. Emitted result components still require surviving origins.
- The scan charges every push/pop and limits its frontier/depth.
  Snapshot copies, map lookups, guarded State merges and
  new value versions consume existing work, origin and graph budgets. Target entry
  snapshots and queued exit versions also count toward the persistent origin limit.

## Bounded restart headers

- Bodies with reference assignments use `borrow/replay.rs` to solve restarted
  target headers. Each pass creates a fresh origin Checker over the existing HIR
  and proofs, initializes normal body inputs, and collects initial/backedge states.
  Provisional locals, calls, reborrows, results and exit queues are discarded;
  only the final stable pass publishes Facts. Source effects are never lowered or
  executed again during this analysis.
- `borrow/restart.rs` canonicalizes each header into separate actual-origin and
  lifetime-bound `(Path, Source)` sets plus observed `(union path, member)` keys.
  Initial and feasible backedge sources and members accumulate monotonically.
  `borrow/activity.rs` retains choice guards keyed by target, local, union path and
  member across replay. Equality compares canonical source roles, member activity
  and their stable guards. The final Facts.headers map contains every mutable
  reference ID present at target entry, including bindings an iteration does not change.
- `borrow/header.rs` supplies one charged Shape validator to origin and loan
  analysis. It follows reference leaves and record Slot paths, entering Deref only
  when a reference's referent carries references. Nested reference cells and record
  pointees retain canonical per-component sources and bounds. Every active typed
  reference path needs actual-origin coverage; bounds do not substitute for stored
  pointers. Inactive null/empty alternatives may contain no reference sources.
- Traversal stops at reference-free referents, preserving references to scalar
  unions even behind another reference or inside a carrier. Unions encountered
  inside stored pointee snapshots retain disjoint alternatives over observed
  members. Child activity includes its parent's activation; actual origins and
  bounds carry the corresponding structural path guard. Reference-bearing List
  shapes remain B001; unsupported activity is never erased to admit a shape.
- Header presence and proof are widened to TRUE, and header entry resets
  continuation assumptions. Stable partitions preserve parent/child structural
  activation, while correlations between independent union fields or owners may
  widen conservatively. Entry/previous-iteration predicate correlations are lost.
  The final body pass still records current-iteration branches, calls, leaves and
  result guards normally.
- Every feasible carried origin and bound keeps its component and source role.
  Input sources and live ancestor-owned Local/Slot/Temporary storage survive.
  Ended sources and target/descendant-owned storage become Source::Expired, keyed
  by the original LocalId, slot view or temporary site. Physical projections may
  collapse only after expiry; the marker stays terminal under further projection
  and repeated restarts. Reinitializing the same static site cannot revive it.
  An enclosing statement's temporary survives an inner restart while its statement
  remains active. Malformed-source and budget failures still report B001.
- Header carriage adds no use: expired values may be overwritten before demand.
  Direct reads and active public bounds report E303 without consulting newly
  initialized storage. Transitive payloads retain lazy field/tag access, while full
  copies and call entry validate the components they consume. Expired identities
  never overlap physical storage; current predecessor values keep their live sources
  and ordinary E302 conflicts before the boundary.
- Each resolved restart has a stable RestartId. The final pass publishes initial
  predecessor snapshots by target and restart snapshots by site, retaining the
  incoming State under its entered guard before widening/reset. Shared Shape inspection
  proves typed path activity on that predecessor; provisional snapshots never
  become published facts.
- Loan headers use stable value IDs. Initial and backedge predecessors define them
  through demand-only transfers from the versions captured on that predecessor,
  then cross a reset edge. Before-entry copies retain initial-only precision;
  header joins themselves read no reference. Nested targets and outer-target
  restarts use their own header IDs and skip unfinished RHS work.
  All paths transfer on demand, so carrying a record/reference cell does not eagerly
  read unused pointees. Dereference copies retain nested public call bounds and
  original pointee sources separately from the outer cell's lifetime.
- Backward propagation resets future-iteration demand before applying the current
  predecessor's source activation. A missing transfer path is permitted only when
  proved inactive there. Missing predecessor snapshots or active source/destination
  paths report B001 when reachable; absence alone is not an inactivity proof.
- Replay shares one charged guard arena, has at most 64 passes per body, and counts
  header/choice cloning, source comparisons, deduplication and type/ownership walks.
  Each header state respects the 4,096-part limit. Scratch passes include previously
  committed facts and both retained/cloned header and choice seed maps in the
  262,144 weighted-origin cap before cloning. Choice keys and predecessor snapshots
  also count toward that cap. Final body counters carry forward, preventing
  individually small bodies from bypassing aggregate limits. This is a logical
  fact/cache budget, not a byte-accurate allocator peak; transient working values
  retain their existing per-value and shared-work limits.
  Nonconvergence or exhausted work/storage reports B001.
- Assignment-free bodies keep the previous single-pass loop behavior. Forward
  Leave joins, physical cell loans, old copies, public call bounds and emitted-slot
  lifetime checks remain independent of header convergence.

## Statement-owned Copy temporaries

- `TemporaryBorrow` materializes a Copy expression in a dedicated
  typed LocalId cell, evaluates its initializer once, then returns that cell's
  address. A Never initializer produces no cell/source use. Equal constants at
  distinct sites retain distinct identities; constant folding cannot promote a
  temporary or extend its lifetime.
- Reference-bearing temporary cells preserve the initializer's origins, bounds and
  active variants beneath Deref. The physical Temporary origin remains separate.
  Materialization copies directly stored reference components; deeper pointee
  summaries transfer on demand. Calls inside the initializer still validate all
  active input origins and bounds.
- A direct dereference copies contents while the temporary cell is alive. The copy
  can retain a surviving owner's reference afterward: `copy : *(&(&owner)); value : *copy`.
  Keeping the temporary-cell
  address instead still expires at its statement boundary. Public call results
  retain their all-input bounds, so calling through a temporary reference cell does
  not gain the direct-copy lifetime exemption.
- Materialization preserves ordinary operand typing. An unannotated integer
  literal defaults to int32, so `view <&uint8> : &1` reports E207; an incompatible
  function argument retains E212. Use a typed result such as
  `&{ byte <uint8> : 1; -> byte }`. Borrowing and narrowing ascriptions do not convert a
  cell's chosen storage type.
- Each original source statement gets a StatementId. A HIR `Statement` wrapper
  is emitted only when that statement materializes a temporary. Generated
  Bind/Emit/SlotAlias operations stay inside the same wrapper. A matcher condition
  and its controlled statement share the boundary; statements inside a nested
  block get their own boundaries. Wrappers do not close ordinary local scopes.
- `Source::Temporary` retains its materialization ID, owning StatementId and
  physical field/element projections. Origin analysis tracks the active statement
  independently of the enclosing block. Calls, reborrows and all-input bounds
  preserve that source. Same-statement use is permitted; a later statement's use
  reports E303. Storing a borrow does not extend the owner's lifetime.
- An outer statement's temporary can be used by a nested call or dispatch chain.
  A temporary created in an inner binding/emission statement cannot escape that
  statement through the block's completed result. In particular, `*(&{ -> 1 })` is
  valid, while `*({ -> &{ -> 1 } })` and `*({ p : &1; -> p })` report E303.
- Field and index borrows keep the entire computed temporary root, then project
  its actual storage. Static known list lengths retain E101; dynamic bounds keep
  P001 and evaluate before address formation. Named places and shared-reference
  reborrows continue to use their existing owners without materialization.
- Leave, restart and panic end exited statement lifetimes. The current Copy-only
  subset has no owned destructor or observable cleanup action; its backend cells
  use existing entry allocas and initialize at the expression site. This does not
  implement owned cleanup or moving non-Copy temporaries. Temporary write roots,
  reference-bearing list elements and exclusive temporary borrows remain unavailable.
- Statement and temporary IDs each cap at 65,536, and metadata lookup, type walks,
  projections and temporary origins consume existing shared work/storage budgets.

## Mutable record fields

- HIR `Field { name, ty, mutable }` keeps named-field mutability in normalized
  shape equality, ordering and union membership. LLVM payload order/layout is
  unchanged. Inferred, annotated and forwarded records retain each field's own
  flag; forwarding never stamps the outer emission flag onto nested fields.
  Branch or expected-slot mutability conflicts report E206. Whole incompatible
  record assignment keeps E207, and nullable omissions inherit declared flags.
- `owner.child.field = rhs` addresses mutable reference-free Copy local
  storage or fixed borrowed carriers. The selected named field must be mutable;
  an immutable selected slot is E305. Shared-reference, temporary and union-root payload targets remain
  B001. A field may hold any currently supported Copy value, including a whole
  list or record. Mixed paths inherit list-slot permission at indexes and take each named field's own flag; initialized-index checks remain unchanged.
- An all-field `SetPath` retains static physical offsets and the target span. RHS evaluates
  once before the selected store. These fixed typed offsets remain valid across
  same-shape Copy owner replacement inside RHS, so no list-style reservation is
  needed. Nonreturning RHS forms no later access. Final-use RHS shared reads can
  finish; surviving overlapping field/ancestor loans report E302, while proven
  siblings remain disjoint. Function all-input bounds keep their existing scope.
- Only target and overlapping ancestor/descendant refinement domains are forgotten
  after RHS checking. Unrelated sibling predicates and immutable copied values
  remain valid. Path lookup, bounded depth and refinement scans consume shared
  work budgets; path collection checks its 256-field limit before each push.
- Mutable primary emissions and mutable fields
  with unsupported list/exclusive subtrees remain B001; borrowed emitted-name writes
  use the identical/exact-member synchronization above.
- List candidate probes compare mutable flags and keep later emitted-name
  dependencies unresolved instead of reading a same-named outer binding. A known
  context permits ordinary once-only checking; unresolved scope-dependent choices
  remain B001.

## Emitted slot aliases

- A named emission evaluates its initializer once through the existing
  Bind/Emit sequence, then `SlotAlias` binds that lexical name to the actual target
  block field. The marker and private proof retain declared field mutability;
  real backing requires an exact mutability match. Reads and permitted mutable
  assignments/SetPath operations resolve that result storage; assignment is not
  a second initialization. Direct immutable alias assignment is E305; field/element
  writes require the supported reference-free owner model and mutable boundaries.
  Ordinary bindings still receive independent copies. E204/E205 initialization
  rules are unchanged.
- The lexical alias type and final slot type remain distinct. Compatible concrete
  record destinations load/coerce from the final slot and inject assignments back
  into it. A concrete Record/List alias inside a wider union field addresses that
  active member's payload for SetPath; a union-typed alias itself has no field/index
  write path. Selected-slot mutability and every bounds rule still apply.
- Missing, incompatible or non-record final destinations are permitted only when
  the original emission is proved disjoint from target completion. The initialized
  local cell then represents that discarded partial slot until leave/restart/exit.
  A retained unsupported destination is B001. Named outer emissions resolve the
  target frame rather than the alias's local declaration scope. Restarting the target
  block reinitializes its cells; inner restarts preserve initialized outer slots.
  Alias names remain lexically scoped; their shared views follow target ownership below.
- Slot aliases stay separate from ordinary addressable places. The address
  resolver supports `&name` and concrete field/list borrows only when the
  alias cell has the lexical type or contains
  it as one exact union member. A lexical union that is only a proper subset of the
  stored union has incompatible tags/layout and remains B001; no copied borrow view or silent widening is made.
  Type hints do not register a borrow; actual uses receive final backing validation.
- `Source::Slot` carries the owning target block, canonical slot root, original
  alias view ID and typed field/element projections. Lifetime lookup uses the
  target's active region rather than the alias declaration's local Storage entry;
  type lookup retains the original view even after that declaration scope closes.
  An inner block can return a slot view to code still inside its target. Publishing
  a view in that target's own result, or beyond it, is E303: completed records
  cannot contain references into their own movable result storage.
- Resolved alias metadata distinguishes compatible Result backing from Discarded
  transient backing. The latter's initialized entry cell is explicitly retained
  under the target's partial-result lifetime, including after an inner alias scope
  ends. Current Copy-only storage needs no cleanup relocation; same-slot E205 and
  the inner-restart-after-outer-emission boundary prevent reuse within a target
  iteration. Target restart ends the iteration's references; inner-loop future
  uses still keep their parent slot loans live through writes.
- Both origin analysis and the CFG use one slot-source constructor. Canonical
  roots match alias assignment/reservation identities, preserving E302 and disjoint
  fields even when aliases arise in different guarded scopes. Slot/view metadata,
  projections and lifetime/type lookups consume existing weighted work/storage
  limits. Immutable reference-bearing emitted names also use actual slot cells.
  Their copied value components preserve the initializer's pointee origins and
  all-input bounds; copying a reference adds no dependency on its containing slot.
  Borrowing a selected reference-free field instead creates only its physical Slot
  origin. A contained reference crossed by `carrier.view.&field` keeps the existing
  pointee reborrow path. Whole-carrier/reference-cell borrows add transitive summaries
  without replacing the stored value's sources. Exclusive references and writes
  through reference-bearing emitted aliases require the compatible-member write proof.
- Only mutable alias IDs enter mutable proofs and omit initializer constant/length caches.
  Unversioned mutable Bind/Local analysis seeds unknown activity of the lexical type
  before emission, preserving narrower type bounds and nullable omission guards.
  Fixed reference-bearing aliases preserve current component snapshots and synchronize
  identical/exact-member result backing at writes; ordinary copies use independent storage versions.
  A later immutable result snapshot cannot reuse a pre-mutation union tag; unrelated
  immutable reference-field origins remain intact. This can conservatively lose
  knowledge about an unmodified mutable field's initial tag.
- Immutable aliases preserve their initializer constants, immutable variant/origin
  facts and known initialized list lengths. False Boolean conditions can still
  suppress unreachable arithmetic, constant arithmetic overflow retains E107, and a
  known immutable alias length supports E101 element checks. No fabricated facts
  or mutable initializer values enter this immutable path.
- Aliases of the same target block/field share a canonical conflict identity;
  mutation invalidates related alias predicate domains. Distinct initialized result
  fields stay separate. Slot identity does not convert ordinary copies into aliases.
  Alias metadata caps at 65,536 entries and charges name copies, lookup, validation
  and predicate work to the shared budget. Final validation uses completion guards
  from the same arena and never invents an absent-field address.

## Control-flow and last use

- The `src/loans.rs` entrypoint and `src/loans/` modules build a separate graph
  for the entry body and every function.
  Its reference value IDs distinguish immutable local components, expression
  temporaries and block result components from physical referent storage IDs.
- Nodes represent reference definitions and reads, result initialization and
  transfer, direct calls, consuming operations, local assignment and branch targets.
  Operand evaluation happens before its consuming node. Assignment writes happen
  after the entire right-hand side, so `owner = *view + 1` is valid when that is
  the view's final use. A reference held by an equality operand remains live
  while the other operand runs.
- Backwards liveness reaches a fixed point over normal, leave and restart edges.
  Value definitions end earlier liveness, allowing each iteration to create and
  finish its own borrow. A reference retained outside a loop remains live when
  any subsequent iteration can use it.
- Normal edges retain the type checker's condition guards. Writes are rejected
  with E302 when their reachability, a live reference's future use and its origin
  can overlap. Proven disjoint write/use branches and conditional origin choices
  remain accepted. Whole-record assignment overlaps every borrowed field.
- Restart edges clear guard correlations propagated across iterations, both for
  reachability and future liveness. This is conservative: a loop whose safety
  requires relations between predicate values in different iterations can report
  E302 even when a stronger temporal proof would establish safety. There is no
  fixed iteration count or assumption that a restart executes at most once.
- A normally completed reference result is evaluated and transferred at the
  block end, so its result slot remains live throughout completing construction.
  Restart, panic and enclosing-leave paths that discard construction have no such
  result use. This preserves the existing result-origin lifetime boundary.
- Bounds are 65,536 graph nodes, 65,536 reference value IDs, 262,144 weighted origin
  entries, 262,144 retained liveness entries and 1,048,576 analysis work steps
  per body, including final origin-overlap scans. Exhaustion reports
  B001; guard arena exhaustion overrides tentative E302/E303 diagnostics. The
  preceding origin pass also caps its persistent local/block fact cache at 262,144
  weighted entries for the whole program, counting active-member facts and path
  lengths as well as origins. Each value has at most 4,096 origin/activity parts.
  The shared proof ledger limits snapshot/variant work to 4,194,304 charged steps,
  including path copies and growing merge scans. Fanout stops inside each push,
  before a whole oversized type is expanded. Dense-reference and many-origin alias
  source regressions exercise each stage's rejection before unbounded growth.
  A many-component carrier with repeated copies also verifies the per-leaf value
  budget; sharing one physical owner does not bypass component accounting.
  Wide unknown tag domains and repeated inactive-payload copies also exercise the
  activity/proof limits, even when no reference origin is currently active.
- Call snapshots count actual origins and all-input bounds separately. The
  returned-component by input expansion stops at the same per-value limit while
  it grows, and type comparison walks check each frontier push. Public-source
  regressions cover a many-input/many-result contract and an oversized referent
  type without requiring a large physical allocation.
- The graph enforces shared loans and scalar exclusive permissions. Exclusive
  reference headers, owned payload moves/temporaries, indirect/capturing contracts
  and generated cleanup remain unimplemented. Ordinary scalar/record reads may
  overlap shared references. See [the scalar design](EXCLUSIVE_REFERENCES.md).

## Loan access records

- `loans/access.rs` owns bounded Read, Tag, Borrow and Write records on the existing
  CFG. `Node.access` replaces the previous write-only field. The shared conflict
  solver consumes the same physical write places and retains its E302 behavior.
  Reads and acquisitions participate in scalar exclusive permission checks.
- Direct storage accesses retain the canonical Place and original lexical LocalId
  view plus typed component paths. Primary, named fields and narrowed union members
  remain distinct. Alias reads and writes agree on their canonical root without
  losing the view used to interpret the value. Indexed writes retain the first
  collection region and existing returning-index/RHS reservations.
- Indirect reads and shared acquisitions name exact immutable pointer value IDs,
  with component paths for selected pointees. They do not flatten public bounds
  into fictitious physical reads. These IDs identify snapshots, not exclusive loan
  authority. Shared provenance now has separate LoanIds and guarded parents as
  described below; mode-aware permission checks consume this provenance.
- Tag traversal maps coercions back to original storage paths and adds no payload
  demand. Static predicates on types with no stored union tag create no tag read.
  Pointer evaluation required to reach a pointee still retains its existing uses.
  Read records follow HIR evaluation order; write records occur only after returning
  RHS work. Branches preserve reach guards; reset/header transfers add no accesses.
- Access construction charges path work before cloning. Retained target/path
  weights count toward the existing 262,144 graph-origin metadata limit, and walks
  consume graph work. Missing pointee evidence may remain only on unreachable CFG
  paths; reachable gaps are B001. No access record adds a liveness use or definition
  beyond the operation's preexisting reference requirements.
- Twelve focused graph groups cover read/store order, field/primary/tag paths,
  pointer versions/public bounds, acquisitions, aliases, skipped stores, indexed
  regions, complementary guards, reset edges, missing evidence and resource limits.
  Scalar exclusive mode, parent permission and indirect writes are implemented
  on these records. The metadata is private, not a public replay artifact.

## Guarded authority provenance

- `loans/values.rs::Value` preserves actual origins and public bounds separately
  through facts ingestion, copies, guarded merges, headers, calls and dereferences.
  The existing dependency iterator still visits both roles for last-use checks.
  A public lifetime bound therefore still blocks conflicting writes with E302 but
  never supplies an actual access region or acquisition identity.
- `loans/authority.rs` allocates bounded, graph-local LoanIds for shared/exclusive acquisitions
  and shared input components. Each loan retains its reference mode. Ordinary value copies keep those identities; distinct
  acquisitions remain distinct even at the same address. These are internal analysis
  IDs, not runtime epochs, persistent source identities or public replay artifacts.
- Metadata-only copy links propagate guarded LoanId alternatives through bindings,
  reference replacement, projections, emissions, copied pointee contents, branch
  joins, short circuits and exact-target Leave. They are separate from backward
  liveness transfers and add no synthetic use. A reborrow creates its own LoanId
  with guarded parent alternatives from the captured parent value version.
- Parent alternatives stay fixed when a reference holder is replaced; copied shared
  children retain their ancestry. Parent graphs are checked for cycles and invalid
  IDs with charged work. Scalar permission checks walk guarded parents to preserve
  delegated access while suspending incompatible parent and sibling operations.
- Call result ancestry outside flat scalar signatures is opaque. Bare scalar-reference
  results now use explicit guarded argument transfer. Opacity propagates through copies and derived loans. Every reachable
  restart body is also opaque for authority purposes, so a repeated static site
  cannot be treated as a proved dynamic permission. Known site IDs may remain as
  evidence alongside that opacity; they must not bypass it. Existing shared call
  and restart behavior remains supported by the original lifetime/loan checks.
- Access regions resolve from actual pointer origins under CFG reach. Direct regions
  use canonical storage and lexical views; leading named-field steps join physical
  source projections, while primary/variant paths remain explicit. Direct field
  reads, field reads through a whole-record reference and a projected reborrow
  resolve to matching regions. Public bounds never become physical regions, and
  expired sources retain their terminal identity.
- Loan records, copy edges, guarded maps, parent snapshots, normalized regions and
  solver storage consume the existing graph-origin/work limits. Queues and parent
  walks are bounded by the current value/loan limits. Exhaustion remains B001 before
  publishing usable metadata. Missing physical origins are B001 in non-restarting graphs, even for opaque calls.
  Restart-erased source/tag correlations instead leave an explicit unresolved-region
  guard under opaque authority; those gaps cannot authorize access. Unknown ancestry
  is represented explicitly rather than invented.
- Fifteen graph groups cover identity through copies, separate acquisitions,
  parent chains, holder replacement, guard/Leave/short-circuit joins, public bounds,
  opaque calls and restarts, copied carriers, normalized fields/slots, input cells,
  expiry and resource/cycle rejection. Seventeen native groups additionally cover
  scalar exclusive permissions, availability and remaining capability gates.

## Storage lifecycle and forward availability

- `loans/storage.rs` records canonical storage cells separately from reference value
  IDs. Graph-local scope IDs identify function, block, controlled-branch and complete
  statement lifetimes. Ordinary locals belong to their lexical scope, parameters to
  their function, temporaries to their actual statement and alias cells to the exact
  target block. Distinct alias views share one canonical cell.
- Enter, End, Init and Use events are explicit on the existing CFG. Init follows
  normally returning RHS work; partial writes require an initialized owner. Temporary
  initialization precedes its acquisition. Named Leave closes exited descendants and
  closes its target at the common completion; restart closes target/descendant scopes
  before reentering the target. Panic marks active scopes ended. These are analysis
  events, not generated cleanup or a proof of destructor order within a scope.
- HIR Copy classification is explicit and exhaustive. Supported source
  scalar exclusive reference storage is non-Copy; other supported storage is Copy.
  Value contexts retain taking intent through groups/ascriptions,
  while pointer evaluation for dereference/reborrow and tag inspection do not take
  the holder. Copy taking leaves storage initialized; an exclusive reference take
  transitions its holder to moved. Other non-Copy shapes remain gated.
- `loans/init.rs` computes backward storage demand, then forward guarded availability
  for demanded cells/scopes. Reference definitions do not initialize storage, and
  reference liveness does not establish availability. Scope entry starts cells
  uninitialized; returning Init makes them ready; End ends their lifetime. A mutable
  moved cell can be reinitialized while its owning scope remains active.
- Ready, moved, uninitialized and ended alternatives join under actual edge guards.
  Reset erases predicate correlations conservatively, while explicit scope entry
  starts a new local lifetime. Uses require ready storage on every feasible path.
  Definite moved use is E301; uncertain availability or ended storage is E309.
  Origin checking still owns E303 for borrowed storage lifetime violations.
- Forward proof is checked after convergence, so a tentative path cannot cause an
  early rejection. Only states demanded by lifecycle uses are retained; unused
  declarations do not fill every snapshot. Empty scopes retain lifecycle events
  without availability state. Scope readiness is tracked independently
  to prevent initialization after End without reentry. End does not read moved or
  uninitialized cells, and failed RHS work does not commit its outer destination.
- Lifecycle metadata uses existing graph origin/work/node limits. Storage demand
  and weighted forward snapshots share the existing liveness-entry limit; cloning,
  unions and edge propagation are charged. Registries and queues are bounded.
  Fifteen groups cover ordering, consumption intent, Copy behavior, internal moves,
  guarded/short-circuit/Leave state, statement/slot ownership, restart, panic, sparse
  demand and budget rejection. Internal tests vary cell Copy metadata explicitly;
  they do not establish source-level exclusive reference or move support.
- Scalar indirect permission and parent suspension now use this availability
  alongside guarded provenance. Partial-cell initialization and runtime cleanup
  remain unimplemented. Opaque ancestry or unresolved regions cannot authorize
  exclusive access.

## Inline bounded lists

- `T[N]` stores a runtime initialized length and inline capacity, with no allocator
  or automatic growth. This milestone accepts reference-free, currently copyable
  elements, including records, unions and nested bounded lists. Reference-bearing
  or uninhabited elements remain B001 until their value/cleanup semantics are ready.
- List literals, `.size()`, one-based copy indexing, same-type/capacity equality,
  whole-value replacement and value-returning `.add()` are implemented. `.add()`
  copies these Copy lists, so an immutable source stays unchanged. Only assigning
  its result back to storage requires a mutable binding.
- Inferred literals require identical normalized types among already typed
  elements. Only pure scalar literals are deferred for contextual checking; other
  expressions are checked once in source order and the emitted operands retain
  that order. No union, numeric promotion or record-primary projection reconciles
  inferred elements. Empty literals need an expected element type.
- Expected list alternatives are filtered by capacity, scalar representability,
  concrete element types and fresh list/record literal shapes. Probes never check
  expression effects or change flow proofs. Raw declared source types keep probes
  conservative when later effects can invalidate a narrowing fact.
- A unique candidate supplies element type and capacity. Pure scalar compounds
  also constrain expected candidates: grouped/chained `-`, `!`, arithmetic,
  Boolean and comparison expressions may use literals and resolved
  immutable scalar constants. A fresh checker retains only referenced constants,
  their exact primitive types and normalized local IDs; it invokes the ordinary
  expression checker. Mutable or captured values, calls and effectful blocks are
  never replayed. Unannotated list compounds retain their already-typed semantics.
- Preflight compound probes suppress reach-dependent arithmetic errors. During
  the source-order pass, pure elements are probed at their actual reach and may
  establish a unique context before later effects. Still-ambiguous pure elements
  wait while typed/effectful expressions are checked once; their saved reach is
  restored for final checking. A nonreturning prefix can suppress later E107, but
  cannot suppress earlier arithmetic or dead-path literal/type errors. Scratch
  checks receive only definitely-dead or potentially-live reach, never live guard
  identities. Retained HIR keeps source order and ordinary assignment coercions.
- An unlabeled effectful result block may check a context-independent prefix of
  bindings, assignments and expression statements once in its ordinary live frame.
  A terminal suffix of unconditional, unannotated emissions is then
  checked against candidate element types using the prefix's actual reach. A
  unique candidate supplies that same frame's expected type before suffix checking
  and normal block finalization. The prefix is never replayed or deferred past a
  later list element, and no synthetic union substitutes for the candidates.
- Suffix probes admit pure scalar/list/fresh-record construction and same-owner
  primitive locals from the prefix or surrounding scope, preserving exact types.
  Nonconstant/mutable locals use typed unknown values with normalized local IDs;
  mutable initializer values, narrowings and live guard/borrow identities are not
  copied. Ordinary scalar probes and deferral stay constant-only, so these reads
  cannot move across later effects. Captures, reference/record/union names,
  references to names emitted by the suffix, direct matcher prefixes and effects
  after the first emission remain B001 when context is unresolved. These limits
  do not restrict ordinary checking after earlier constraints select one type.
- Prefix errors retain normal diagnostics, and Never prefixes suppress only the
  checks the ordinary checker suppresses. Common duplicate-slot/declaration errors
  are retained after every candidate fails; a candidate-specific record-composition
  error cannot discard another valid context. Multiple fits are E207 only when no
  earlier deferred or later element constraints remain; otherwise this bounded
  path returns B001 without moving later effects ahead. Pure suffix AST size/bytes
  are charged before copying, and all candidate/context work shares existing caps.
- Unknown suffix locals can lose enclosing Boolean proof relationships. If a
  failing diagnostic lies within an `&&`/`||` RHS whose scratch condition remains
  symbolic, one fresh dead-reach probe checks whether that failure is conditional.
  Group wrappers use the condition's actual lowered span. A disappearing failure
  preserves an Unknown candidate; ordinary live checking still decides a sole
  candidate, while unresolved alternatives remain B001. Unconditional structural
  failures are unchanged. Both probes and the bounded control scan share the same
  work budget; no placeholder is assigned a fabricated zero/false/string value.
- Multiple proved candidates report E207, as does no element-compatible candidate;
  when every capacity is too small, E103 applies. There is no smallest-capacity or
  default-width preference. Single-candidate literal failures retain their existing
  codes. Context-dependent effects or nested constraints that remain unresolved
  report B001 and need an explicit annotation. Selection caps alternatives at 256
  and charges source/type probe work against the shared analysis budget.
  Probes borrow declared type descriptions instead of cloning them per candidate;
  actual-type walks, failed field searches and record-shape scans also consume work.
  Scalar scratch trees are limited to 4,096 nodes. Referenced string bytes are
  charged before copying; repeated node, constant, type and lookup work consumes
  the same shared budget. Floating operations retain ordinary per-operation
  rounding/infinity behavior, unsigned negation remains invalid, and grouped
  positive signed-minimum magnitudes are not folded into compact negative literals.
- Unary operators use the operand's type or a unique literal context before
  assignment injects their result into a union. Expected unions do not turn a
  scalar operand into a union before applying `!` or `-`.
- Extents use existing checked scalar expression/constant rules. Typed width
  overflow remains E107, mixed widths remain E213, and negative or nonconstant
  extents are E104. Required extent checks run even on dead runtime paths. Effectful
  or helper-driven required evaluation remains B001. Pure constant lookup for a
  type extent does not introduce a runtime capture.
- `Type::layout()` is the checked layout source used by frontend and backend.
  Bootstrap limits are 65,536 elements and 1 MiB of inline list storage; exceeding
  those implementation budgets is B001. Target-layout arithmetic overflow is E104.
  These limits do not qualify a total native frame or stack-size budget.
- Known immutable/literal lengths and lengths shared by every completing block
  emission support E101/E103. Discarded restart/leave paths are excluded; different
  completing lengths remain unknown. Mutable bindings and function results keep
  runtime length checks. Static facts never come from the last emission alone.
- Receiver values are captured before index or append arguments run. Index checks
  use initialized length, not capacity, and preserve signedness before range checks.
  Runtime failures report P001 for bounds and P003 for capacity with the relevant
  lengths/capacity, position and source span. Equality reads only initialized
  elements, preserving element equality semantics such as NaN and signed zero.
- Whole lists and concrete record fields containing lists use existing shared
  places, reborrows and E302/E303 checks. A copied list has no continuing reference
  origin; dereference copies finish their loan before later operand effects unless
  another reference use keeps it live.
- `&(values[index])` forms a checked shared reference into original list storage.
  Nested lists and concrete record fields compose element and field reborrows.
  A temporary reference-valued parent is allowed. Computed or ascribed Copy lists
  materialize for their complete statement. Borrowed copied parameters and
  dispatch `$` belong to their local storage and cannot escape (E303).
- `ElementBorrow` evaluates its parent pointer and snapshots initialized length
  before checking its once-evaluated index. It shares integer/static E101 rules
  with copy indexing; runtime checks report P001 before address formation. The CFG
  holds parent actual origins and inherited bounds through any returning index
  evaluation, even when the resulting element reference is discarded. Index
  panic/leave paths consume no derived reference. Owner writes conflict with a
  live parent/element loan (E302), and resume after its final use.
- Physical element pointers retain actual index identity. Abstract element paths
  conservatively overlap all indices within a list, preserving enclosing record
  fields and nested list prefixes without enumerating capacity. They retain every
  direct-function all-input dependency.
- `holder.rows[first].items[next] = rhs` replaces initialized storage through
  concrete list and mutable field layers of a mutable reference-free Copy local
  or emitted slot alias. Parentheses around path prefixes are allowed. Writes through
  shared-reference targets, narrowed union-typed roots and temporary owners remain
  B001; a concrete alias may still address its payload in a wider final slot.
  Immutable list bindings report E305. This adds no source exclusive-reference
  value or `&!` semantics; the final store requires exclusive collection access.
- `SetPath` replaces separate field/element assignment nodes. It retains the local
  ID, final target span and ordered `WriteStep::Field`/`WriteStep::Index` entries.
  Each index step contains its once-evaluated operand and original prefix span.
  Lowering captures each current list's initialized length, evaluates/checks that
  index and forms its actual element address before inspecting the next layer.
  After every index succeeds, it evaluates the contextually typed RHS once and
  stores only the selected leaf. Lengths, capacities and other elements stay intact.
  Static positions use E101; P001 identifies the failing indexed prefix, excluding
  later fields, indices and RHS. Later effects cannot precede an earlier bounds check.
- Static fields before the first index identify the whole collection region used
  for reservations and final write conflicts. All indices and contained fields
  in that first list remain conservative aliases; holder fields outside it can
  stay disjoint. Finer field/index exclusion inside the list requires later proof.
  Pure field paths keep the full precise static place and need no reservation.
- The CFG defines an internal collection reservation before the first index, reads
  it at every returning bounds/address phase, then consumes it at the final store. Completing index
  or RHS owner replacements/nested writes conflict (E302), while shared reads and
  final-use RHS borrows can finish before the store. The whole collection region
  is used for exclusive-access checks; an external shared loan live after the store is
  rejected. The reservation is not a user reference and never escapes.
- A nonreturning index skips its bounds check, later indices, RHS and store;
  a nonreturning RHS skips the store. Earlier returning phases retain their uses.
  The reservation has no use after the corresponding last executed phase, so
  owner replacement before an immediate panic/leave is allowed when no other loan
  survives. Returning index effects still require valid parent storage even when
  RHS later diverges. After RHS checking, refinement invalidation uses that same
  first-list prefix, or the complete field path for static writes; holder siblings
  and immutable copied values keep their own facts. Nodes,
  reservation values and conflict work use the existing bounded CFG budgets.
  Target flattening is iterative and limited to 256 mixed steps before allocation;
  parser depth bounds also apply. Root types are charged once and consumed layer
  by layer, avoiding repeated suffix copies. Every target/typed/CFG step is charged,
  independently of list capacities.
- Exclusive references, slices, named list aliases, removal, reference-bearing/owned elements
  and list formatting remain B001.

## Next analysis stages

The [exclusive-reference implementation design](EXCLUSIVE_REFERENCES.md) defines
the first scalar slice, required access/authority/initialization facts, retained
B001 boundaries and future execution criteria. It does not enable `&!` support.

1. Extend origin and all-input bounds to exclusive reborrows, static reference
   sources and documented intrinsic contracts before enabling those capabilities.
2. Extend the existing graph with owned initialization, moves, scope ends,
   verified call effects and cleanup edges. Keep storage IDs distinct from values.
   Named leave/restart edges must preserve their exact target and owner lifetimes.
3. Preserve terminal expired identities when extending header storage shapes or
   source kinds. Improve correlations across independent header alternatives only
   with bounded temporal proofs; retain active-path lifetime checks and source-side
   predecessor transfers.
4. Check live shared/exclusive loans against overlapping places. Whole-owner access
   overlaps every field; different proven record fields can be disjoint. Reborrows
   suspend conflicting parent access. Dynamic indexing remains conservative.
5. Track copy/move capabilities and partial initialization; reject moved reads and
   moving owners out of borrowed storage. End references before moving/destroying
   their owner. Verify all-input returned-view contracts at functions and callers.
6. Extend statement-owned temporaries to owned values only with move and cleanup
   proofs for normal, leave, restart and unwind
   edges. Construction cleans only initialized
   slots. Coordinate task joins before owner cleanup with the runtime prototype.
7. Extend storage and transitive summaries only alongside proved mutation, source
   and lifetime rules. Preserve separate cell/pointee origins across future temporary
   owners and exclusive reborrows. Add finer indexed disjointness,
   exclusive references, slice/alias metadata and non-Copy state separately.

## Verification

Execute `reference_identity` unchanged and native scalar/record dereference cases
in debug and release. Check complementary owner selections, aliases, discarded
emissions and all possible escaping roots. Check unsupported ownership boundaries,
shadowing and storage provenance. Preserve all scalar/union checks. No reference
fixture depending on `bytes` becomes supported just from pointer lowering.

## Scalar exclusive access

- `Type::Exclusive` holds an ordinary mutable Bool/Int/Float local address. Moving
  its handle preserves LoanId identity. `&*p`, `&!*p` and dereference inspect the
  handle; expected shared-reference conversion creates a shared child loan.
- `loans/permissions.rs` walks guarded parent edges within the work/metadata caps.
  Live descendants retain ancestors. An actor can use its own loan or delegated
  ancestors; shared descendants suspend parent writes and exclusive descendants
  suspend parent reads too. External overlapping access reports E302. Public bounds
  remain conservative dependencies and never authorize a store.
- Store captures the pointer before RHS evaluation and retains it only through a
  returning final store. Completed RHS moves/replacements survive skipped stores.
  The backend evaluates each operand once and preserves scalar storage layout.
- Calls, emissions, reference-bearing cells/temporaries and dispatch receivers
  carry semantic boundary markers. Guarded ancestry checks reject exclusive-derived
  shared values crossing unsupported boundaries. Flat scalar direct calls use
  explicit entry accesses and guarded result transfer instead. Wider result contracts, carriers,
  indexed/reference roots, comparisons, carrier results and resolved restart bodies remain B001.
- Availability produces E301 for definite moves and E309 for uncertain storage;
  origin lifetime failures remain E303. Mutable owner requirements and shared scalar
  store rejection produce E305. Runtime cleanup and owned destruction are unproved.

## Scalar exclusive function inputs

The [function argument contract](EXCLUSIVE_FUNCTIONS.md) restricts exclusive
signatures to primitive or bare scalar-reference results and primitive/scalar-reference arguments.

- Caller evaluation captures argument values once, left to right. After every
  argument returns, Read/Write entry accesses retain every captured reference
  through the call. This validates a moved suspended parent and arguments whose
  last use is the call itself; parameter use inside the body cannot weaken entry.
- Exclusive input grants retain their mode on symbolic Source::Input roots. Same
  input roots overlap; local parameter cells remain distinct. Different scalar
  roots are independent for exclusive accesses because each direct caller checks
  conflicting arguments. Shared parameters may alias; no LLVM noalias is emitted.
- Passing an exclusive handle consumes it. Passing `&!*p` delegates a child;
  expected shared parameters create shared reborrows. The same entry checks apply
  to direct receiver syntax and recursive/forwarded calls. Mutating calls clear
  caller mutable refinements. Primitive results carry no returned loan authority;
  bare scalar-reference results use the guarded transfer described below.
- A later argument's Leave/panic skips entry and the call without undoing earlier
  moves. A non-returning callee still validates entry. Exclusive parameters trigger
  restart exclusions even when unused; shared-only restart callees remain supported.
- Seventeen native groups cover these paths in both profiles, including exact
  E301/E302/E309 and B001 boundaries. Wider results/signatures, carriers
  and dispatch blocks remain separate proof-bearing work.

## Guarded scalar reference results

The [result contract](REFERENCE_RETURNS.md) explicitly records candidate argument
indexes and guards in `Facts.returns`. Actual origins and captured-parent copy links
use the same choice. Exclusive results accept only compatible exclusive inputs;
shared results may use shared or exclusive inputs. Callee-root lifetime and mode
checking proves the body; callers conservatively consider every compatible argument.

Result loans are created only after normal return. Missing/incomplete evidence or
invalid argument indexes are B001. Equal physical addresses never merge distinct
argument authority. Caller entry still validates every argument's maximum access.
Scalar-reference emissions retain demand until target completion; later conflicting parent access
is E302. Exclusive emissions and returned handles consume their holders.

Public bounds include every borrow-carrying input, including ignored or incompatible
pointee types. They retain scope and write/acquisition protection, but cannot become
actual result addresses, authorize access or prohibit a read solely because the
result is exclusive. Wider calls retain opaque ancestry. Anonymous scalar-reference blocks now retain guarded identity and consuming
emissions. Carriers and generated destruction remain separate contracts.

## Anonymous scalar-reference blocks

The [block-result contract](REFERENCE_BLOCKS.md) permits ordinary anonymous scalar
reference results and known cancelled anonymous emissions. `loans/control.rs` checks
result type or explicit cancellation evidence; dispatch BlockIds and named fields
retain their boundaries. Missing emission/completion evidence is B001.

Existing copy links transfer the same guarded loan IDs; they do not copy exclusive
ownership. Availability consumes emitted holders immediately, and retained result
demand protects loans through target completion. Own-target Leave returns the value;
ancestor Leave/panic may cancel it without undoing moves or earlier effects.
Local scalar owners still cannot escape their block. Block-valued indirect targets
are captured once and retained only through a returning final store.

Fifteen native groups and two graph evidence groups cover these paths, including
guarded same-address acquisitions, missing cancellation evidence, dispatch boundaries
and short-circuit availability. The standalone Never-operator fix preserves skipped
block typing and actual non-returning effects without changing the backend.

## Exclusive scalar record fields

The [field contract](EXCLUSIVE_FIELDS.md) admits mutable named scalar fields on
owned reference-free Copy records. `check/references.rs` produces the original
root/field Place; shared field lookup retains the selected slot's mutability.
The existing Borrow HIR, source lifetimes, loan modes and backend addresses are reused.

Normalized fields prove sibling disjointness; whole-owner and ancestor accesses
still conflict. `access_overlap` keeps Slot(0) primary components distinct from named
descendants without excluding ancestor loans. Copies of the owner have separate
storage. Calls, returns and block results preserve field roots and public bounds;
captured stores keep the pointer chosen before RHS effects and only through a
returning store. Field owners cannot escape their scope.

Fourteen native groups plus projection/path-limit evidence cover these rules in both
profiles where applicable. Indexed/union/reference paths, non-scalar exclusive
pointees, owned carriers and generated cleanup remain separate capabilities.

## Exclusive mutable emitted scalars

The [slot contract](EXCLUSIVE_SLOTS.md) permits direct mutable scalar aliases.
`Alias.exclusive` records intent until the completed target type is known; exclusive
backing must equal the declared local type. Shared union-member borrowing retains
its existing compatibility rule. Proven discarded results retain their typed local
fallback cells. No backend addressing or HIR extension was needed.

Existing Slot sources retain canonical roots and lexical views. Guarded aliases
share the target's storage scope but keep distinct acquisitions. Ordinary reads,
writes and pointer stores resolve to that canonical slot. Reference moves, children,
call/block transfer and lifetime bounds use the existing passes. Aliases initialize
after RHS completion; pointers may outlive the alias scope but not their target.

Fourteen native groups and canonical-storage/backing evidence cover exact types,
mutation, widths, conflicts, guards, initialization, target lifetimes, cancellation
and panic. Supported self-containing shared views and direct escapes are E303;
exclusive carriers remain B001. Named scalar-field paths through mutable emitted
reference-free Copy records now reuse the same strict owner-backing check. Indexed
paths and generated cleanup remain separate contracts.

## Exclusive emitted record fields

Mutable emitted record aliases support named scalar-field paths through the same
bounded resolver used for ordinary records. Only the selected field must be mutable;
Alias.exclusive validates the complete backing record, not only the selected scalar.
A different unselected field still makes widened backing unsupported. Shared union
views retain their existing compatibility rule.

Existing Slot sources append concrete field indexes while retaining target, canonical
root and lexical view. Sibling/primary disjointness, ancestor conflicts, target-owned
lifetime, captured stores, cancellation and call/block transfer reuse existing
analysis and lowering. Cancelled records keep their declared fallback layout.

Fourteen new native groups plus canonical nested-projection and whole-record backing
evidence cover these paths. No new HIR, backend operation, runtime ABI or dependency
was needed. Wider indexed/union/reference paths, whole-record exclusive pointees, owning
carriers and generated cleanup remain separate capabilities.

## Owned exclusive scalar elements

The [element contract](EXCLUSIVE_ELEMENTS.md) uses ExclusivePath HIR for owned
scalar-list paths, including named fields, nested indexes and exact-backed emitted
aliases. The origin and loan passes independently require the existing owner/type
proof. Private list reservations grant no authority and protect returning index
evaluation from conflicting writes to captured storage.

The final root exclusive loan retains mixed Field/Element projections and is not
derived from a shared reference. Non-returning indices create no loan or future
reservation demand, while completed outer indices retain their earlier demand.
Lowering captures each actual list length before evaluating its index once and
reuses initialized-length bounds and element addressing. No runtime ABI changed.

Local exclusive carried scalar paths use the
[restart extension](EXCLUSIVE_RESTARTS.md#carried-list-elements): whole-slot Acquire
before capture and at acquisition, exact mutable scalar source qualification and
no live exclusive ancestry across reset. Indexed stores use the
[write proof](#carried-indexed-writes). Element
overlap and owner metadata access remain conservative; reference/temporary roots,
wider pointees, owning elements and exclusive header carriage remain separate.
