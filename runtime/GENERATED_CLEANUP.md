# Private generated cleanup bridge

[generated.hpp](include/meowy/generated.hpp) exposes scalar C-linkage calls over the
existing cleanup Stack, Owned payloads and owning Panic snapshot. [generated.cpp](src/generated.cpp)
is linked into the compiler's native archive. The interface is private to the pinned
Linux x86-64 bootstrap; it is not a release ABI or a new meowy language feature.

Normal meowy code generation still has no owning resource type or cleanup calls.
Compiler backend tests generate LLVM callbacks and explicit cleanup edges, emit native
objects and link the actual archive. They prove this boundary can be called correctly;
they do not prove automatic scope lowering, task cancellation or DWARF unwinding.

## Storage and scalar ABI

All functions have the `meowy_cleanup_` prefix and `_v0` suffix. Status is an i32:
0 means ok, 1 means full, 2 means invalid. Unwind reasons are i32: complete=0,
leave=1, restart=2, panic=3, cancel=4. A reason names an already-selected exit;
passing cancel does not request task cancellation or join children.

| Storage | Layout or sizing                                               | Lifetime                                                                        |
| ------- | -------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| Frame   | `bytes(capacity)` and `alignment()` queries; opaque bytes      | From successful open through successful finish                                  |
| Token   | `{ptr owner, i64 index, i64 id}`, size 24, alignment 8         | Only within its original live frame; stale or foreign tokens reject             |
| Mark    | `{ptr owner, i64 depth, i64 anchor}`, size 24, alignment 8     | Existing Stack prefix rules; reuse follows that Stack's mark contract           |
| Panic   | `panic_bytes()`; opaque owning bytes, aligned to `alignment()` | Explicitly initialized before use; source text may then end                     |
| Payload | Caller-defined generated layout passed as `ptr`                | Stable until its armed callback finishes or ownership is explicitly transferred |

Static assertions check the token/mark layout and the opaque alignment assumptions.
Alignment 16 is sufficient for the current target, as checked in the bridge. Size
queries return zero on overflow. Open rejects null/misaligned/undersized storage.
The frame contains its Stack, entries and callback records in one caller-owned region;
no heap allocation, hidden growth, process registry or host-destructor cleanup occurs.

Callers supply valid, writable, nonoverlapping storage for live frames, output handles,
payloads and diagnostics. Passing arbitrary dangling pointers or pretending an unopened
buffer is a live frame violates the ABI. Frame APIs cannot validate pointer provenance.
Do not open an already-live frame or mix tokens across frame lifetimes, even if the
same address is reused. All access is confined to the current worker and is synchronous.

## Initialization and unwind

Open starts an empty frame with the requested entry capacity. Mark records its current
Stack prefix. Reserve consumes one entry but does not initialize a payload; capacity
failure leaves the output token untouched. Construct the payload, then arm its token
with a non-suspending drop callback and a nonempty static operation name. Failed arm
preserves the previous entry and callback record. Partially constructed payloads must
clean their own initialized pieces before leaving the reserved entry unarmed.

A callback has the LLVM signature `void(ptr payload, ptr panic)`. The second pointer
is a constructed, initially empty owning Panic. To report a cleanup failure, call
`panic_init` while the message bytes are still valid and check its status. It captures
at most 256 bytes using the existing UTF-8 truncation rule and retains original size.
The callback can then destroy its message source. Omit capture for successful cleanup.
Callbacks cannot suspend, unwind through C++, mutate their active frame or end its
lifetime. Bridge reentry returns invalid, including finish during the last callback.

Disarm cancels one live cleanup obligation; it does not move or release its payload.
A compiler must complete actual relocation and arm the new owner before disarming the
old obligation. The owned transfer operation below performs these steps together.

Unwind validates the mark and cause before releasing entries in reverse order.
Reserved-but-unarmed and disarmed entries are skipped. Panic exits require a nonzero
owning snapshot; other causes require no active panic. The initial snapshot is copied
before callbacks run. A callback panic uses the existing fatal P008 path with the
original cause and static operation name. No C++ exception crosses the ABI.

Finish succeeds only with an empty, idle frame. It ends metadata lifetime and never
silently unwinds live entries. The caller still owns and releases the backing bytes.

## Generated payload ownership

The `meowy_owned_..._v0` family wraps Owned. Its i32 statuses are separate from cleanup
statuses: ok=0, invalid=1, full=2, misaligned=3, occupied=4, overlap=5. Check each
function's family; a full cleanup reservation returns 1, not the owner's full=2.

`ops_bytes()` sizes an opaque ValueOps descriptor; `ops_init` initializes caller-owned
static descriptor storage from payload size/alignment, move callback, drop callback and
static diagnostic name. The descriptor must remain immutable and alive through every
owner and transfer that refers to it. Descriptor and owner metadata alignment is bounded by `cleanup_alignment()`; payload
alignment is supplied by the descriptor.
The generated move ABI is `void(ptr destination, ptr source)` and the generated drop
ABI is `void(ptr payload, ptr panic)`. ValueOps accepts exactly one native-return or
generated-output drop callback. Existing native descriptors keep their original form.

`owned_bytes()` sizes an opaque owner token. Open binds it to separate caller-owned
payload bytes, initially empty. Reserve validates the descriptor and buffer without
constructing the payload. `data()` returns the reserved storage for construction;
commit marks it live. A cancelled constructor releases the reserved token without
calling drop, after the constructor cleans any initialized pieces itself. Finish
requires an empty token and never implicitly releases a live payload. While move/drop
callbacks run, data access, reserve, release, commit and finish reject reentry.

Descriptors, owner metadata, payload buffers, frames and output handles must obey the
original nonoverlap/lifetime contract. A payload address remains stable while owned;
it changes only through explicit relocation. Open must not overwrite a live owner,
and finish must not precede the final armed callback or successful transfer. An owner
must have one active cleanup obligation; do not duplicate-arm it in multiple frames.

`owned_arm(frame, token, owner)` binds a committed owner's destruction to a reserved
cleanup slot. It uses the descriptor name and returns the owned drop snapshot into the
enclosing frame, preserving the original unwind reason and panic. Explicit
`owned_release(owner)` is a standalone complete-cause release; it is not the callback
used while an enclosing frame unwinds. A bound owner must stay valid until its cleanup
obligation is removed; release alone does not disarm that obligation.

`owned_transfer(source_frame, source_token, source_owner, destination_frame,
destination_token, destination_owner)` requires an active matching source obligation,
a reserved top destination slot and an empty destination owner. Stack::can_rebind and
Owned::fits validate all handles, initialization, capacity, alignment and overlapping
payload storage before mutation. Failed preflight leaves both owners and obligations
unchanged; the destination slot remains reserved for retry or unwind.

On success, both cleanup frames are held against callback reentry. Owned invokes the
infallible, non-suspending move callback, which constructs the destination and ends the
source payload lifetime without releasing the resource. The bridge then arms the new
obligation and disarms the old one. No callback or suspension occurs between these
last two changes. Unexpected failure after successful preflight is a fatal protocol
violation. The old owner is empty, its disarmed entry no longer reads its metadata,
and the destination retains the static descriptor. Same-frame transfers are supported.

After failed reservation/admission, an initialized untransferred source is still owned
by its original token. Clean it through that token exactly once. No new allocation,
implicit byte-copy move, scheduler or C++ exception mechanism implements this transfer.

## Required compiler exit mapping

| Exit                    | Future generated action                                                                                                                 |
| ----------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| Normal block completion | Relocate/retain emitted owners, then unwind that block's locals while result and parent storage remain valid                            |
| Leave                   | Unwind exited scopes from innermost through the target, retaining the completed target result                                           |
| Restart                 | Unwind the target iteration and inner scopes, invalidate old initialized slots, then establish fresh iteration marks                    |
| Panic                   | Capture the owning diagnostic before releasing source storage; follow explicit cleanup edges or qualified landing pads to the task root |
| Cancellation            | Request cooperative child cancellation, drain/join children while parent borrows remain valid, then release parent owners               |

Backend `block` and Leave/Restart branches currently emit direct control flow.
[Scalar panic outcomes](../compiler/docs/PANIC_OUTCOMES.md) now carry owning snapshots
through explicit generated failure exits while preserving streamed diagnostics.
Native probes pass returned snapshots into this bridge with the panic cause; no
automatic owning-value cleanup is emitted yet. Initialization/transfer metadata
and drop schedules remain necessary; loan lifecycle events are not drop schedules.

Task::mark/close belongs to the scheduler contract, separate from Stack::mark/unwind.
Close can suspend, return report_full or retain a release failure. Generated code must
preserve the mark, report storage, consumed-progress state and parent locals until all
children settle and every failure report is drained. Close alone does not implement
cancellation. No scheduler, stack switching or DWARF dependency is added to this bridge.

## Evidence and next work

The runtime runner checks six bridge groups and two exact fatal subprocesses in each
selected profile. They cover LIFO/partial construction, all causes, foreign/stale/full
handles, nested marks, rejected initialization, callback reentry and snapshot lifetime.
Compiler LLVM probes exercise the real scalar ABI in debug/release, including an
owning diagnostic captured from callback-local bytes before those bytes are overwritten.

Seven additional runtime groups and two fatal subprocesses per profile exercise owned
payload relocation and failure preservation. Two additional LLVM-native groups use
static descriptors and generated move/drop callbacks, including a self-pointer payload,
rejected short destination and an owned cleanup panic with its original cause intact.

Next define initialized-state/drop schedules for a contract-supported owning HIR value
and lower normal/Leave/Restart exits, retaining emitted ownership. The bridge and LLVM
fixtures are executable infrastructure; ordinary meowy programs still do not use it.
Panic/task support additionally needs retained outcomes, child-close progress,
cancellation and the pinned unwind mechanism.
