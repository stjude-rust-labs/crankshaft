# Docker Backend Example

This example demonstrates submitting multiple simple tasks to a locally running Docker daemon using the Crankshaft engine.

## Prerequisites

*   **Rust & Cargo:** Ensure Rust and Cargo are installed (e.g., via [rustup](https://rustup.rs/)).
*   **Docker:** Docker Desktop or Docker Engine must be installed and running. Your user needs permission to interact with the Docker socket. Test with `docker run hello-world`.

## Running the Example

1.  Navigate to the root of the `crankshaft` repository.
2.  Run the example using `cargo run`:

    ```bash
    cargo run --release --bin docker -- --n-jobs 100 --max-tasks 10
    ```

### Command Line Arguments

*   `--n-jobs <NUMBER>`: (Optional) Specifies the total number of identical tasks to submit.
    *   Default: `1000`
*   `--max-tasks <NUMBER>`: (Optional) Sets the maximum number of concurrent tasks Crankshaft will manage for the Docker backend. This limits simultaneous container creation/monitoring by Crankshaft.
    *   Default: `50`

## What it Does

1.  **Parses Args:** Reads the `--n-jobs` and `--max-tasks` arguments.
2.  **Configures Engine:** Programmatically creates a `Docker` backend configuration named `"docker"` using the specified `max_tasks` limit. It uses default Docker backend settings (like `cleanup = true`).
3.  **Initializes Engine:** Creates a `crankshaft::Engine` instance and adds the configured Docker backend using `engine.with(...).await`.
4.  **Defines Task:** Creates a simple `Task` definition:
    *   Image: `alpine:latest`
    *   Command: `echo "hello, world!"`
5.  **Spawns Tasks:** Submits `--n-jobs` copies of the task to the `"docker"` backend via `engine.spawn()`. Each call returns a `TaskHandle`.
6.  **Waits for Results:** Uses `futures::stream::FuturesUnordered` to concurrently wait for all `TaskHandle`s to complete (`handle.wait().await`).
7.  **Displays Output:**
    *   Shows an `indicatif` progress bar tracking completed tasks.
    *   After all tasks finish, prints the exit status, standard output, and standard error for each task result.
