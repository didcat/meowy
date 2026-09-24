# Task and channel APIs

[Library index](README.md)

## Tasks

| API                                                  | Result                   | Contract                                                            |
| ---------------------------------------------------- | ------------------------ | ------------------------------------------------------------------- |
| `job.deadline(instant)`                              | `<null>`                 | Sets or shortens a task's deadline before join                      |
| `%group.deadline(instant)`                           | `<null>`                 | Sets a group deadline before submission                             |
| `job.cancel()`, `ticket.cancel()`, `%group.cancel()` | `<null>`                 | Nonblocking, idempotent cancellation request                        |
| `job.status()`, `ticket.status()`                    | `<tasks.State>`          | Snapshot, never an ownership or completion proof                    |
| `tasks.checkpoint()`                                 | `<null>` on continuation | Acknowledges pending cancellation by unwinding to the task boundary |

`tasks.Outcome<T>` is `T` plus `Cancelled`, `Timeout`, `Panicked`, and
`SpawnFailed`. These error types use allocation-free descriptors when needed.
A panic diagnostic may carry an explicitly runtime-owned trace buffer; failure
to allocate that trace must still preserve the basic panic code.

`tasks.State` is the literal-string union `<"queued"><"running"><"settled">`.
It is copyable, `Send`, and `Sync` and needs no allocation:

- `"queued"`: admitted work whose body has not started.
- `"running"`: the body has started and has not published a terminal outcome;
  this includes a suspended wait and cancellation cleanup still in progress.
- `"settled"`: one terminal outcome is published and ready for join. This includes
  admission failure and cancellation before the body starts.

A status call observes one synchronized state during that call. The task may
advance immediately afterward. Cancellation requests do not introduce a fourth
state, and a deadline does not establish that a task has settled. `"joined"` is
not a `State` alternative: a consuming join makes later handle or ticket access
a static error. Read the outcome to distinguish success, domain failure, panic,
timeout, cancellation, or failed admission.

```meowy
snapshot : job.status()
| snapshot == "running" | debug.print("Work has started")
outcome : << job  # The snapshot never replaces this owning join. #
```

All tasks require an executor supplied by the program runtime. Its worker count,
allocator, stack budget, and admission limit are explicit runtime configuration,
not inferred from a group or channel capacity. Programs that do not use tasks do
not require an executor. Executor exhaustion produces `SpawnFailed`, and the
language does not promise one operating-system thread per task.

`tasks.Send` means ownership can transfer to another task; `tasks.Sync` means a
shared borrow can be used by another task. The
[capability table](../tasks-and-channels.md#capability-rules) defines derivation
for primitive, structural, borrowed, and opaque types. A group parameter uses `<tasks.Group<T, N>>` behind an
exclusive reference; it grants submission access only and cannot outlive its
owner or be retained by a child. `group.submit(function)` takes a transferable
zero-argument callable and returns a borrowed ticket, following the same admission
and capture rules as a labeled submission. It cannot seal or join the group.

## Channels

| API                                               | Result                                          | Contract                                                       |
| ------------------------------------------------- | ----------------------------------------------- | -------------------------------------------------------------- |
| `channel.bounded<T>(allocator, capacity <usize>)` | Endpoint record or `<memory.AllocationFailure>` | Allocates a queue for a transferable, owned message type       |
| `sender.clone()`                                  | `<channel.Sender<T>>`                           | Creates another sender owner without growing the queue         |
| `sender.send(value <T>)`                          | `<null><channel.Rejected<T>>`                   | Waits, then transfers the value or returns it in the rejection |
| `sender.try_send(value <T>)`                      | Also `<channel.Full<T>>`                        | Never waits; a full result retains the value                   |
| `receiver.receive()`                              | `<channel.Item<T>><channel.Closed>`             | Waits for an item or drained closure                           |
| `receiver.try_receive()`                          | Also `<channel.Empty>`                          | Never waits                                                    |
| `sender.close()`, `receiver.close()`              | `<null>`                                        | Consumes the endpoint and applies its closure rules            |

The endpoint record has `sender <channel.Sender<T>>` and
`receiver <channel.Receiver<T>>`. `Item<T>`, `Rejected<T>`, and `Full<T>` expose
`value <T>`. Rejection/full errors retain payloads inline, as collection errors do.
`Closed` and `Empty` are allocation-free concrete errors. Endpoint clone count
overflow panics instead of wrapping. Sender and receiver operations need exclusive
access to that endpoint; each producer uses its own sender owner.

See [task and channel semantics](../tasks-and-channels.md) for submission, joins,
capture, endpoint closure, and cancellation. The [time module](time-and-date.md)
defines duration units, monotonic clock domains, timers, and ticker ownership.
