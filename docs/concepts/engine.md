---
title: Engine
description: Create an engine, register named backends, spawn tasks, cancel them, and shut down.
---

# Engine

<p class="lede">The <code>Engine</code> is the one object your code holds. It keeps a set of named backends, sends tasks to them, limits how many run at once, and broadcasts events about every task.</p>

## Create an engine

There are two constructors.

```rust
use crankshaft::Engine;

// No monitor. This is what most programs want.
let engine = Engine::default(); // same as Engine::new()

// With the gRPC monitor listening on an address (needs the `monitoring` feature).
let engine = Engine::new_with_monitoring("127.0.0.1:8080".parse()?).await;
```

The monitor is covered in [Monitoring](/observe/monitoring). Either way, the engine starts with no backends.

## Register backends by name

A backend is added with `.with(config)`, which takes a backend [`Config`](/concepts/configuration) and returns the engine. You can chain as many as you like, and each needs a unique name.

```rust
let engine = Engine::default()
    .with(local_docker)   // named "docker"
    .await?
    .with(cluster)        // named "lsf"
    .await?;

for name in engine.runners() {
    println!("registered: {name}");
}
```

`.with` does real work: it connects to Docker, builds an HTTP client for TES, or prepares the shell driver for a Generic backend. It fails if that setup fails, so a bad configuration surfaces at startup rather than on the first task.

## Spawn tasks

```rust
use tokio_util::sync::CancellationToken;

let token = CancellationToken::new();
let handle = engine.spawn("docker", task, token.clone()).await?;
```

`spawn` returns as soon as the task is queued. The engine runs it on a Tokio task of its own, so you can spawn thousands of tasks in a loop and collect the handles.

::: warning Backend names must match
`spawn` panics if no backend is registered under the name you pass. If the name comes from user input or a configuration file, check it against `engine.runners()` first.
:::

### Concurrency limits

Each backend has a `max_tasks` limit, set in its configuration. The engine holds a semaphore per backend, and a task waits for a permit before the backend sees it. Tasks beyond the limit stay queued in memory, not on the remote system, so a TES server or scheduler never receives more than `max_tasks` of your tasks at once.

### Waiting for results

```rust
let results = handle.wait().await?;
for result in results.iter() {
    println!("{:?} → {}", result.image, result.status);
}
```

`wait` resolves to a `NonEmpty<ExecutionResult>`, one per execution in the task, in order. Each result has the image that actually ran (if the backend uses images) and a standard `ExitStatus`. Dropping a handle doesn't cancel the task; the task keeps running and its result is discarded.

When a task can't finish, `wait` returns a `TaskRunError`:

| Variant | Meaning |
| --- | --- |
| `Canceled` | The cancellation token fired before the task finished. |
| `Preempted` | The backend reclaimed the resources. Only TES reports this today. |
| `Other(anyhow::Error)` | Anything else, like an image that couldn't be pulled or a submit command that failed. |

```rust
use crankshaft::engine::service::runner::backend::TaskRunError;

match handle.wait().await {
    Ok(results) => println!("done: {}", results.last().status),
    Err(TaskRunError::Canceled) => println!("canceled"),
    Err(TaskRunError::Preempted) => println!("preempted; submit it again"),
    Err(TaskRunError::Other(e)) => eprintln!("failed: {e:#}"),
}
```

A non-zero exit code is not an error. The task ran; it just didn't succeed. Check `status.success()` on each result.

## Cancel tasks

Each spawned task watches the `CancellationToken` you pass in. Cancelling it stops the task at the next opportunity. Docker force-removes the container (unless you turned `cleanup` off), TES asks the server to cancel the remote task, and Generic runs your `kill` command.

Tokens compose. Give every task a `child_token()` of one parent token and you can cancel a whole run with a single call.

```rust
let run = CancellationToken::new();

let handle = engine.spawn("docker", task, run.child_token()).await?;

// Later, for example on Ctrl-C:
run.cancel();
```

## Shut down

```rust
engine.shutdown().await;
```

`shutdown` consumes the engine. It closes the event channel, so subscribers see `RecvError::Closed`, and stops the monitor if there is one. Wait for the handles you care about before you call it.

## Reference

- [`Engine`](https://docs.rs/crankshaft/latest/crankshaft/struct.Engine.html) on docs.rs
- [`TaskHandle`](https://docs.rs/crankshaft-engine/latest/crankshaft_engine/service/runner/struct.TaskHandle.html) on docs.rs
