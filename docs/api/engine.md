# Engine API (`crankshaft::Engine`)

The `Engine` is the central struct for managing backends and executing tasks in Crankshaft.

## Creating an Engine

You typically start by creating a default engine instance:

```rust
use crankshaft::Engine;

let engine = Engine::default();

# Adding Backends to Crankshaft Engine

To enable an engine to execute tasks, you need to configure and add backends. This can be done using the `.with()` method, which takes `crankshaft::config::backend::Config` objects. Since backend initialization might be asynchronous and can fail, the method returns a `Result<Self>`.

## Example Code

```rust
use crankshaft::Engine;
use crankshaft::config;
use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let docker_config = config::backend::Config::builder()
        .name("local_docker")
        .kind(config::backend::Kind::Docker(Default::default()))
        .max_tasks(10)
        .build();

    let engine_with_docker = Engine::default().with(docker_config).await?;

    println!("Engine initialized with backends: {:?}", engine_with_docker.runners());

    Ok(())
}
# Spawning Tasks

Submit tasks for execution using `engine.spawn()`.

```rust
use crankshaft::Engine;
use crankshaft::engine::Task;
use crankshaft::engine::task::Execution;
use nonempty::NonEmpty;
use tokio_util::sync::CancellationToken;
use eyre::Result;
# use crankshaft::config;
# async fn get_engine() -> Result<Engine> {
#     let docker_config = config::backend::Config::builder()
#         .name("local_docker")
#         .kind(config::backend::Kind::Docker(Default::default()))
#         .max_tasks(10)
#         .build();
#     Ok(Engine::default().with(docker_config).await?)
# }
```

```rust
async fn run_task(engine: &Engine) -> Result<()> {

    let task = Task::builder()
        .name("my_simple_task")
        .executions(NonEmpty::new(Execution::builder()
            .image("alpine")
            .program("echo")
            .args(["Hello!"])
            .build()))
        .build();

    let cancellation_token = CancellationToken::new();

    let handle: TaskHandle = engine.spawn(
        "local_docker",
        task,
        cancellation_token.clone()
    )?;
    println!("Task spawned. Waiting for completion...");

    let results: NonEmpty<std::process::Output> = handle.wait().await?;

    println!("Task finished with status: {}", results.first().status);
    println!("Stdout: {}", String::from_utf8_lossy(&results.first().stdout));

    Ok(())
}
```

```rust
# #[tokio::main]
# async fn main() -> Result<()> {
#     let engine = get_engine().await?;
#     run_task(&engine).await?;
#     Ok(())
# }
```

---

## API Reference

### `engine.spawn(backend_name: impl AsRef<str>, task: Task, token: CancellationToken) -> Result<TaskHandle>`

- **backend_name**: The name of the configured backend to use. Panics if the name is not found.
- **task**: The Task object to execute.
- **token**: A CancellationToken to signal cancellation for this task.

**Returns**:  
`Ok(TaskHandle)` on successful submission queuing, or `Err` if spawning fails immediately.

# Task Handle (`TaskHandle`)

Represents the asynchronous task execution returned by `spawn`.

## `handle.wait() -> impl Future<Output = Result<NonEmpty<std::process::Output>>>`

- **Description**: Asynchronously waits for the task to complete or fail.
- **Returns**:
  - `Ok(results)` on successful completion (where results contains the `std::process::Output` for each execution step).
  - `Err(error)` if task execution fails or is cancelled.

---

# Cancellation

Signal cancellation by calling `.cancel()` on the `CancellationToken` passed to `spawn`.

---

## Example

```rust
# use tokio::time::{sleep, Duration};
# use tokio_util::sync::CancellationToken;
# use crankshaft::Engine;
# use crankshaft::engine::Task;
# use crankshaft::engine::task::Execution;
# use nonempty::NonEmpty;
# use eyre::Result;
# use crankshaft::config;
# async fn get_engine() -> Result<Engine> {
#     let docker_config = config::backend::Config::builder()
#         .name("local_docker")
#         .kind(config::backend::Kind::Docker(Default::default()))
#         .max_tasks(10)
#         .build();
#     Ok(Engine::default().with(docker_config).await?)
# }
```

```rust
# async fn run_and_cancel(engine: &Engine) -> Result<()> {
#     let task = Task::builder()
#         .name("cancellable_task")
#         .executions(NonEmpty::new(Execution::builder()
#             .image("alpine")
#             .program("sleep")
#             .args(["5"])
#             .build()))
#         .build();
#
    let cancellation_token = CancellationToken::new();
    let handle = engine.spawn("local_docker", task, cancellation_token.clone())?;

    tokio::spawn(async move {
        sleep(Duration::from_secs(1)).await;
        println!("Signalling cancellation...");
        cancellation_token.cancel();
    });

    match handle.wait().await {
        Ok(_) => println!("Task completed without cancelling?!"),
        Err(e) => println!("Task cancelled or failed: {}", e),
    }
#   Ok(())
# }
```

```rust
# #[tokio::main]
# async fn main() -> Result<()> {
#     let engine = get_engine().await?;
#     run_and_cancel(&engine).await?;
#     Ok(())
# }
```
# Getting Backend Names

Retrieve the names of all configured backends.

---

## `engine.runners() -> impl Iterator<Item = &str>`

- **Description**: Returns an iterator over the names of configured backends.

---

## Example

```rust
# use crankshaft::Engine;
# use crankshaft::config;
# use eyre::Result;
# #[tokio::main]
# async fn main() -> Result<()> {
#     let config = config::backend::Config::builder()
#         .name("b1")
#         .kind(config::backend::Kind::Docker(Default::default()))
#         .max_tasks(1)
#         .build();
#     let engine = Engine::default().with(config).await?;
```

```rust
let backend_names: Vec<&str> = engine.runners().collect();
println!("Available backends: {:?}", backend_names);
```

```rust
#     Ok(())
# }
```
