# Tasks and channels

[Pawterns](README.md) · Previous: [Time and calendars](time-and-calendars.md) · Next: [Deadlocks and shutdown](deadlocks-and-shutdown.md)

Use a task when a computation has one eventual result. Use a channel when work
transfers a sequence of owned messages. A group bounds submissions; a channel
bounds queued messages; the executor bounds admitted tasks and running workers.
Choose each budget explicitly. Four queue slots do not secretly hire four
workers; the queue has enough on its plate.

These recipes use lexical task owners and handle every join outcome. For shutdown
ordering and cancellation, continue with [Deadlocks and shutdown](deadlocks-and-shutdown.md).

## Let the squares race; keep the answers in order

**Use this when:** a small, bounded batch can run independently, and the caller
needs results in the same order as its inputs. Completion order is irrelevant.

Create a directory containing the following two files. This `mod.mwy` supplies
the executor; importing `tasks` or writing `>>` does not supply one:

```meowy
-> build : {
    -> entry : "./main.mwy"
    -> profile : "debug"
    -> executor : {
        -> workers : 2
        -> max_tasks : 4
        -> stack_bytes : 65_536
        -> allocator : "system"
    }
}
```

`main.mwy`:

```meowy
debug : @"debug"

square <int32> : (value <int32>) {
    -> value * value
}

status <int32> := 0
%squares <int32[4]>

%squares >> square(2)
%squares >> square(3)
%squares >> square(4)

results : << %squares
index <usize> := 1

'show {
    | index > results.size() | 'show.leave()

    result : &(results[index])
    | *result <error> | {
        status = 1
        debug.print("Result {index}: {*result}")
        index = index + 1
        'show.restart()
    }

    debug.print("Result {index}: {*result}")
    index = index + 1
    'show.restart()
}

-> status
```

From that directory:

```sh
meowy check
meowy run
```

With successful task admission and completion:

```text
Result 1: 4
Result 2: 9
Result 3: 16
```

`<< %squares` seals and consumes the group. It returns three outcomes in a list
with capacity four. Position one belongs to `square(2)` even if that task finishes
last. The parent prints after the join, so this output does not depend on worker
print interleaving.

Each element is `tasks.Outcome<int32>`, which can also hold cancellation,
timeout, panic, or admission failure. Read it through a reference: an error
diagnostic may own storage, and indexing must not accidentally copy that owner.
The matcher recognizes concrete errors without boxing them.

**Failure and ownership:** a failed task still occupies its original position.
The program prints that outcome and returns status 1. For dynamic input, split
the input into batches that fit the group; an extra submission past the group's
capacity is an error or panic, not a request to grow it. Join one batch before
admitting another if the executor budget requires it.

See the complete [ordered-task project](../programs/tasks/README.md), which adds
a monotonic deadline, and the rules for
[bounded groups](../reference/tasks-and-channels.md#bounded-groups).

## Pass the batch without copying its luggage

**Use this when:** a producer builds a batch, then hands exclusive ownership to
a consumer. The consumer should process the existing allocation rather than
borrow the producer's local storage.

Keep the preceding `mod.mwy` and replace `main.mwy` with this complete example:

```meowy
channel : @"channel"
collections : @"collections"
debug : @"debug"
memory : @"memory"

consume <uint32> : (
    receiver <channel.Receiver<collections.Vector<uint32>>>
) {
    input := receiver
    total <uint32> := 0

    'drain {
        item : input.receive()
        | item <channel.Closed> | 'drain.leave()

        batch : item.value
        index <usize> := 1
        'sum {
            | index > batch.size() | 'sum.leave()
            value : batch.get_copy(index)
            | value <collections.Bounds> | {
                debug.panic("Batch index escaped its checked bound")
            }

            total = total + value
            index = index + 1
            'sum.restart()
        }
        'drain.restart()
    }

    -> total
}

status <int32> := 0

'main {
    endpoints : channel.bounded<collections.Vector<uint32>>(memory.heap, 1)
    | endpoints <memory.AllocationFailure> | {
        debug.print("Cannot allocate the queue")
        status = 1
        'main.leave()
    }

    batch := collections.vector<uint32>(memory.heap, 4)
    | batch <memory.AllocationFailure> | {
        debug.print("Cannot allocate the batch")
        status = 1
        'main.leave()
    }

    value <uint32> := 1
    'fill {
        | value > 4 | 'fill.leave()
        pushed : batch.push(value)
        | pushed <collections.PushFailure<uint32>> | {
            debug.print("Cannot append to the batch")
            status = 1
            'main.leave()
        }
        value = value + 1
        'fill.restart()
    }

    input : endpoints.receiver
    output := endpoints.sender
    consumer : >> consume(input)
    sent : output.send(batch)

    | sent <channel.Rejected<collections.Vector<uint32>>> | {
        debug.print("Batch rejected: {sent.value.size()} values remain owned")
        status = 1
    }

    output.close()
    consumed : << consumer
    | consumed <error> | {
        debug.print("Consumer: {consumed}")
        status = 1
        'main.leave()
    }

    | status == 0 | debug.print("Sum: {consumed}")
}

-> status
```

Successful output is `Sum: 10`. The batch contains four initialized values in
four reserved slots. `send(batch)` transfers the Vector owner, including its
allocator handle and buffer ownership. It does not copy four integers into a
second vector or extend a borrowed slice's lifetime.

The queue has capacity **one message**, not one integer. While a message is queued,
the queue owns that Vector; after receipt, the consumer owns it. A pending sender
can additionally own its blocked message, so queue capacity alone is not a bound
on all live payload bytes. Each received batch is released at the end of its
drain iteration.

**Failure and ownership:** allocation is checked before any child starts. If
admission fails, the task's transferred receiver is released. A later send is
rejected and the join exposes `tasks.SpawnFailed`. A rejection keeps the original
Vector in `sent.value`; this example inspects and then releases it. A successful
send establishes transfer, not proof that the consumer processed the batch: the
join is still required.

The bounds branch guards an invariant; with this immutable batch and the loop's
checked index it cannot occur. It panics rather than publishing a partial sum as
success if that invariant is broken. The parent closes its only sender before joining the
consumer, allowing the consumer's next receive to observe drained closure.

For a nonblocking producer, `try_send` adds `channel.Full<T>` as a possible
outcome. Its `value` also owns the unsent payload. Decide whether to retain it for
a later attempt, process it locally, or release it. Repeated immediate retries
spend CPU without establishing that a consumer can make progress.

See [channel ownership](../reference/tasks-and-channels.md#channels),
[Vector operations](../reference/stdlib/collections.md#growable-vectors), and
[overlapping memory budgets](../reference/optimization.md#budget-the-owners-that-overlap).

## One task seat, three guests

**Use this when:** task admission must stay within a small memory budget, or you
want to exercise the path where a task never starts.

Return to the first recipe's `main.mwy`. Change only the executor in its manifest:

```meowy
-> build : {
    -> entry : "./main.mwy"
    -> profile : "debug"
    -> executor : {
        -> workers : 1
        -> max_tasks : 1
        -> stack_bytes : 65_536
        -> allocator : "system"
    }
}
```

Run `meowy run`. The group still has four result slots, but the executor allows
one admitted, unjoined task. Even if the first square finishes immediately, its
slot stays occupied until joined. The second and third submissions therefore
settle with `tasks.SpawnFailed`; their function bodies never execute. Under
otherwise successful allocation and execution, the first result is 4, and the
program prints failures in positions two and three and returns status 1.

The second and third tasks cannot sneak into the first task's seat just because
it finished quickly: its result is still waiting there. Increasing `workers`
alone does not make the other tasks admissible. For the original three submissions to succeed,
restore `max_tasks : 4`, or change the application to submit and join smaller
batches. A group cannot be joined halfway through and then reopened.

| Limit            | What it bounds                      | What it does not bound                  |
| ---------------- | ----------------------------------- | --------------------------------------- |
| Group capacity   | Submission slots and their outcomes | Executor admission or queue storage     |
| `max_tasks`      | Admitted tasks until joined         | Payload bytes owned by each task        |
| `workers`        | Simultaneously executing work       | All queued or settled task storage      |
| Channel capacity | Queued message count                | Producer-held or consumer-held messages |

**Failure and ownership:** a failed submission releases captures that were moved
into it. Do not write recovery code that assumes those owners remain in the
parent. If retry must preserve an input owner, design where that owner waits
before submission; it cannot be reclaimed from a failed task's discarded capture.
Scope cleanup joins forgotten children, but unobserved `SpawnFailed` or `Panicked`
outcomes become an owner panic. Explicit joins make recovery a deliberate policy.

See the [task outcome API](../reference/stdlib/tasks-and-channels.md#tasks),
[capture and admission rules](../reference/tasks-and-channels.md#starting-and-joining-one-task),
and [runtime executor settings](../reference/modules-and-ffi.md#build-settings).
