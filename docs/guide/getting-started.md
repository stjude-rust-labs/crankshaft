# Getting Started

This guide walks you through setting up a minimal Rust project using Crankshaft to run a simple "Hello World" task via the Docker backend.

## Prerequisites

1.  **Rust Toolchain:** Install Rust and Cargo, preferably using [rustup](https://rustup.rs/).
    ```bash
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    # Follow instructions, then ensure cargo is in your PATH
    source "$HOME/.cargo/env"
    cargo --version
    ```
2.  **Docker:** Install [Docker Desktop](https://www.docker.com/products/docker-desktop/) or [Docker Engine](https://docs.docker.com/engine/install/). Ensure the Docker daemon is running and your user has permissions to interact with it.
    ```bash
    docker --version
    docker run hello-world # Test basic Docker functionality
    ```

## Steps

### 1. Create a New Rust Project

Open your terminal and run:

```bash
cargo new crankshaft_hello_world
cd crankshaft_hello_world


## 2. Add Dependencies

Edit your `Cargo.toml` file and add the necessary crates under `[dependencies]`:

```toml
[package]
name = "crankshaft_hello_world"
version = "0.1.0"
edition = "2021"

[dependencies]
eyre = "0.6"
tokio = { version = "1", features = ["full"] }
nonempty = "0.11"
```

Then, fetch the dependencies:

```bash
cargo fetch
```

## 3. Write the Application Code

Replace the contents of `src/main.rs` with the following:

```rust
// src/main.rs
use crankshaft::Engine;
use crankshaft::config;
use crankshaft::engine::Task;
use crankshaft::engine::task::Execution;
use nonempty::NonEmpty;
use tokio_util::sync::CancellationToken;
use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Starting Crankshaft Hello World Example...");

    println!("⚙️  Configuring Docker backend...");
    let docker_backend_config = config::backend::Config::builder()
        .name("local_docker")
        .kind(config::backend::Kind::Docker(
            config::backend::docker::Config::default()
        ))
        .max_tasks(5)
        .build();

    println!("🏗️  Initializing Crankshaft Engine...");
    let engine = Engine::default()
        .with(docker_backend_config)
        .await
        .expect("Failed to initialize engine with Docker backend");

    println!("Engine initialized with backend: 'local_docker'");

    println!("📝 Defining 'hello_task'...");
    let hello_task = Task::builder()
        .name("hello_task")
        .executions(
            NonEmpty::new(
                Execution::builder()
                    .image("alpine:latest")
                    .program("echo")
                    .args(["Hello from Crankshaft via Docker!"])
                    .build()
            )
        )
        .build();

    let cancellation_token = CancellationToken::new();

    println!("➡️  Spawning 'hello_task' on 'local_docker' backend...");
    let task_handle = engine.spawn(
        "local_docker",
        hello_task,
        cancellation_token.clone()
    )?;

    println!("⏳ Waiting for task to complete...");
    let task_results = task_handle.wait().await?;

    let execution_output = task_results.first();

    println!("Task completed!");
    println!("   Status: {}", execution_output.status);
    println!("   Stdout: {}", String::from_utf8_lossy(&execution_output.stdout).trim());
    println!("   Stderr: {}", String::from_utf8_lossy(&execution_output.stderr).trim());

    println!("🏁 Example Finished.");
    Ok(())
}
```

## 4. Run the Application

Make sure your Docker daemon is running in the background. Then, run your Rust application:

```bash
cargo run
```

## 5. Expected Output

You should see output in your terminal similar to this:

```
🚀 Starting Crankshaft Hello World Example...
⚙️  Configuring Docker backend...
🏗️  Initializing Crankshaft Engine...
✅ Engine initialized with backend: 'local_docker'
📝 Defining 'hello_task'...
➡️  Spawning 'hello_task' on 'local_docker' backend...
⏳ Waiting for task to complete...
✅ Task completed!
   Status: exit code: 0
   Stdout: Hello from Crankshaft via Docker!
   Stderr:
🏁 Example Finished.
```

This output confirms that Crankshaft successfully configured the Docker backend, defined the task, submitted it to Docker, executed the echo command within an Alpine container, and retrieved the successful exit code and standard output.

## Next Steps

- Learn about loading configurations from Crankshaft.toml in the Configuration Overview.
- Explore the different Backend Types (TES, Generic).
- Dive deeper into the Engine API and Task Definition API.
- Check out the other runnable Examples.
