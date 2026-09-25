---
title: Getting started
description: Add Crankshaft to a project and run your first task on Docker.
---

# Getting started

<p class="lede">By the end of this page you'll have a small Rust program that registers a Docker backend, runs <code>echo</code> in an Alpine container, and prints the exit status.</p>

## Before you begin

You need:

- **Rust 1.91.1 or newer.** Check with `rustc --version`. We recommend installing Rust with [rustup](https://rustup.rs), which also makes it easy to update later with `rustup update`.
- **A running Docker daemon.** Check with `docker info`. The Docker backend talks to it over the default socket. To install it, see [Get Docker](https://docs.docker.com/get-started/get-docker/).

::: info Docker licensing
Docker has [licensing requirements](https://docs.docker.com/subscription/desktop-license/) that can apply to your organization. Check with your IT or legal team before you install or use it.
:::

## Add the dependencies

Crankshaft is async and expects you to supply the Tokio runtime. The example below also uses `tokio-util` for cancellation, `nonempty` for the list of executions, and `anyhow` for errors.

```sh
cargo new hello-crankshaft && cd hello-crankshaft
cargo add crankshaft tokio-util nonempty anyhow
cargo add tokio --features full
```

## Write the program

Replace `src/main.rs` with the following.

```rust
use crankshaft::Engine;
use crankshaft::config::backend::{Config, Kind, docker};
use crankshaft::engine::Task;
use crankshaft::engine::task::Execution;
use nonempty::NonEmpty;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Register a Docker backend under the name "docker".
    let backend = Config::builder()
        .name("docker")
        .kind(Kind::Docker(docker::Config::default()))
        .max_tasks(50)
        .build();
    let engine = Engine::default().with(backend).await?;

    // Describe one execution: run `echo` in an Alpine container.
    let task = Task::builder()
        .name("hello")
        .executions(NonEmpty::new(
            Execution::builder()
                .images(["alpine"])?
                .program("echo")
                .args([String::from("hello, world!")])
                .build(),
        ))
        .build();

    // Spawn it on the backend by name and wait for the result.
    let handle = engine
        .spawn("docker", task, CancellationToken::new())
        .await?;
    let results = handle.wait().await?;
    println!("{}", results.first().status);

    engine.shutdown().await;
    Ok(())
}
```

## Run it

```sh
cargo run
```

The first run pulls `alpine` if you don't have it. Then you should see:

```text
exit status: 0
```

The status is a standard `ExitStatus`, which prints as `exit status: 0`. Call `.code()` on it to get the number as an `Option<i32>`.

## What just happened

1. `Engine::default()` created an engine with no backends. `.with(backend)` connected to Docker, checked what resources it has, and registered it as `"docker"`.
2. `Task::builder()` described the work. A task holds a non-empty list of executions, and each execution names one or more images to try.
3. `engine.spawn("docker", …)` queued the task. The engine allows at most `max_tasks` tasks to run on that backend at once; the rest wait their turn.
4. `handle.wait()` resolved when the container exited, with one `ExecutionResult` per execution.
5. `engine.shutdown()` closed the event channel and stopped the engine.

::: warning Backend names must match
`spawn` panics if no backend is registered under the name you pass. Keep backend names in one place, or check `engine.runners()` first when the name comes from user input.
:::

## Try the bundled examples

The repository has four runnable examples. Clone it and run them from the root.

| Command | What it shows |
| --- | --- |
| `cargo run --release --bin docker` | Many tasks on the local Docker daemon |
| `cargo run --release --bin docker-monitored` | The same, with the gRPC monitor on `127.0.0.1:8080` |
| `cargo run --release --bin tes` | Tasks on a TES server |
| `cargo run --release --bin lsf` | Tasks on LSF through the Generic backend |

## Next

- Learn what else a task can carry in [Tasks](/concepts/tasks).
- Load backends from a file instead of code in [Configuration](/concepts/configuration).
- Watch tasks as they run in [Events](/observe/events).
