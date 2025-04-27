# Core Concepts

Understanding these core concepts is key to effectively using Crankshaft.

## Engine (`crankshaft::Engine`)

The `Engine` is the central orchestrator in Crankshaft. It's the main entry point for interacting with the framework.

*   **Responsibilities:**
    *   Manages configured execution [Backends](#backend).
    *   Initializes backend connections and resources.
    *   Receives [Tasks](#task) and routes them to the correct backend.
    *   Enforces backend-specific concurrency limits (`max_tasks`).
*   **Usage:** Typically, a single `Engine` instance is created and configured within your application.

## Backend

A Backend represents a specific execution environment where tasks can run.

*   **Types (`crankshaft_config::backend::Kind`):**
    *   `Docker`: Executes tasks in local Docker containers or Docker Swarm.
    *   `TES`: Interacts with a GA4GH Task Execution Service v1 endpoint.
    *   `Generic`: Executes commands via configured scripts (often over SSH), adaptable to HPC schedulers or custom setups.
*   **Configuration:** Defined in `Crankshaft.toml` (or other sources) under the `[[backends]]` array. Each backend needs a unique `name`, a `kind`, and a `max_tasks` concurrency limit. See [Configuration](../configuration.md).
*   **Engine Integration:** Backends are added to the `Engine` using `engine.with(backend_config).await`.

## Task (`crankshaft_engine::Task`)

A `Task` represents a logical unit of work.

*   **Composition:**
    *   Metadata (optional `name`, `description`).
    *   One or more [Execution](#execution) steps (`executions`).
    *   [Input](#input) data dependencies (`inputs`).
    *   Expected [Output](#output) products (`outputs`).
    *   [Resource](#resources) requirements (`resources`).
    *   Shared `volumes` (primarily for Docker).
*   **Definition:** Created programmatically using `Task::builder()`. See [Task API](../api/task.md).

## Execution (`crankshaft_engine::task::Execution`)

An `Execution` is a single command or script run within a specific environment (usually a container). It's the fundamental step within a `Task`.

*   **Key Properties:**
    *   `image`: Container image (e.g., `ubuntu:latest`).
    *   `program`: Command to run (e.g., `bwa`).
    *   `args`: Arguments for the program.
    *   `work_dir`: Working directory inside the environment.
    *   `env`: Environment variables.
    *   `stdin`, `stdout`, `stderr`: Optional file paths for stream redirection *within* the environment.
*   **Definition:** Created using `Execution::builder()`. See [Execution API](../api/execution.md).

## Input (`crankshaft_engine::task::Input`)

Defines an input dependency for a `Task`.

*   **Key Properties:**
    *   `contents`: Source of the data (`Contents::Path`, `Contents::Literal`, `Contents::Url`).
    *   `path`: Destination path *inside* the execution environment.
    *   `ty`: `Type::File` or `Type::Directory`.
    *   `read_only`: Mount read-only (default: `true`).
*   **Definition:** Created using `Input::builder()`. See [Input/Output API](../api/io.md).

## Output (`crankshaft_engine::task::Output`)

Defines an expected output from a `Task`.

*   **Key Properties:**
    *   `path`: Source path *inside* the execution environment where the output is generated.
    *   `url`: Destination URL for uploading the output (primarily used by TES).
    *   `ty`: `Type::File` or `Type::Directory`.
*   **Definition:** Created using `Output::builder()`. See [Input/Output API](../api/io.md).

## Resources (`crankshaft_engine::task::Resources`)

Specifies computational resources for a `Task`.

*   **Key Properties:**
    *   `cpu`: Requested cores.
    *   `ram`: Requested RAM (GiB).
    *   `disk`: Requested disk space (GiB).
    *   `cpu_limit`, `ram_limit`: Hard limits (backend-dependent).
    *   `preemptible`: Hint for using spot instances (backend-dependent).
    *   `zones`: Preferred compute zones (backend-dependent).
*   **Definition:** Created using `Resources::builder()`. Defaults can be set per-backend. See [Resources API](../api/resources.md).

## Task Handle (`crankshaft_engine::service::runner::TaskHandle`)

Returned by `engine.spawn()`. Represents the asynchronous execution.

*   **Usage:** Use `handle.wait().await` to wait for completion and get the `Result<NonEmpty<std::process::Output>>`.

## Cancellation Token (`tokio_util::sync::CancellationToken`)

Passed to `engine.spawn()`.

*   **Usage:** Call `.cancel()` on the token to signal a graceful cancellation attempt to the running task. Backend behavior varies (e.g., `docker rm -f`, `scancel`, TES `cancel_task`).
