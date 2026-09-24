---
title: Events
description: Subscribe to the engine's event stream and follow each task from creation to completion.
---

# Events

<p class="lede">Every task reports what it's doing on a broadcast channel. Subscribe once and you'll hear about every task on every backend, which is enough to drive a progress bar, a log, or a user interface.</p>

## Subscribe

```rust
use crankshaft::events::Event;
use tokio::sync::broadcast::error::RecvError;

let mut events = engine.subscribe()?;

tokio::spawn(async move {
    loop {
        match events.recv().await {
            Ok(Event::TaskCreated { id, name, .. }) => println!("#{id} created: {name}"),
            Ok(Event::TaskStarted { id }) => println!("#{id} started"),
            Ok(Event::TaskCompleted { id, exit_statuses }) => {
                println!("#{id} completed: {}", exit_statuses.last())
            }
            Ok(Event::TaskFailed { id, message }) => eprintln!("#{id} failed: {message}"),
            Ok(_) => {}
            Err(RecvError::Lagged(skipped)) => eprintln!("missed {skipped} events"),
            Err(RecvError::Closed) => break,
        }
    }
});
```

Subscribe before you spawn. A receiver only sees events sent after it was created. `subscribe` fails only after the engine has shut down.

## The events

Each event carries the task's `id`, a `u64` that is unique within the process.

| Event | Extra fields | Sent by | When |
| --- | --- | --- | --- |
| `TaskCreated` | `name`, `tes_id`, `token` | all | The backend has accepted the task. |
| `TaskStarted` | | all | The task is running. For Generic, this is when the job is submitted. |
| `ImagePullStarted` | `name` | Docker | A pull has begun for an image that isn't present. |
| `ImagePullFinished` | `name` | Docker | That pull succeeded. |
| `ImagePullFailed` | `name`, `message` | Docker | That pull failed. The next fallback image is tried. |
| `TaskContainerCreated` | `container` | Docker | A container exists for the current execution. |
| `TaskContainerExited` | `container`, `exit_status` | Docker | That container stopped. |
| `TaskStdout`, `TaskStderr` | `message` (bytes) | Docker | The container wrote output. You can turn these off. |
| `TaskCompleted` | `exit_statuses` | all | Every execution ran. Exit codes may still be non-zero. |
| `TaskFailed` | `message` | all | Something went wrong running the task. |
| `TaskCanceled` | | all | The cancellation token fired. |
| `TaskPreempted` | | TES | The server reclaimed the resources. |

### Lifecycle

Every `TaskCreated` is followed by exactly one of `TaskCompleted`, `TaskFailed`, `TaskCanceled`, or `TaskPreempted`. Every `ImagePullStarted` is followed by exactly one of `ImagePullFinished` or `ImagePullFailed`. If you're counting tasks in flight, count on those pairs.

`TaskCreated` also carries the task's `CancellationToken`, so a UI that only sees the event stream can still cancel a task. That's how the [console](/observe/monitoring) does it.

## Keep up with the stream

The channel holds the latest 100 events. A receiver that falls further behind gets `RecvError::Lagged(n)` and loses the oldest `n`. The engine never waits for slow subscribers.

::: tip Don't do slow work in the receive loop
Hand events to another task over an unbounded or larger channel, or keep only a counter per task. On Docker, turn off `send-stdout` and `send-stderr` if nothing reads them; they're usually the bulk of the traffic.
:::

## Reference

- [`crankshaft::events::Event`](https://docs.rs/crankshaft-events/latest/crankshaft_events/enum.Event.html) on docs.rs
