# TES Backend Example

This example demonstrates submitting multiple simple tasks to a GA4GH Task Execution Service (TES) v1 endpoint using the Crankshaft engine.

## Prerequisites

*   **Rust & Cargo:** Ensure Rust and Cargo are installed.
*   **TES Service:** Access to a running TES v1 service endpoint URL. This could be a local instance (like [Funnel](https://github.com/ohsu-comp-bio/funnel) or [TESK](https://github.com/EMBL-EBI-TSI/TESK) in Minikube) or a cloud-based service.
*   **(Optional) Credentials:** If the TES service requires authentication, you need the appropriate credentials. This example supports HTTP Basic Authentication via environment variables.

## Authentication (Basic Auth)

If your TES endpoint uses HTTP Basic Authentication:

1.  **Set Environment Variables:** Before running, export your credentials:
    ```bash
    export USER="your_tes_username"
    export PASSWORD="your_tes_password"
    ```
2.  **Run:** The example automatically detects these variables, encodes them (`username:password` -> Base64), and adds the `Authorization: Basic <token>` header to TES requests.

If `USER` and `PASSWORD` are not set, it connects without authentication.

## Running the Example

1.  Navigate to the root of the `crankshaft` repository.
2.  Run the example using `cargo run`, providing the TES URL:

    ```bash
    # Example without authentication
    cargo run --release --bin tes -- http://localhost:8000/ga4gh/tes/v1 --n-jobs 50 --max-tasks 10

    # Example with authentication (set USER/PASSWORD env vars first)
    cargo run --release --bin tes -- https://my-secure-tes.com/v1 --n-jobs 50 --max-tasks 10
    ```

### Command Line Arguments

*   `<URL>`: (Required) The base URL of the TES v1 API endpoint (e.g., `http://localhost:8000/ga4gh/tes/v1`).
*   `--n-jobs <NUMBER>`: (Optional) Total number of identical tasks to submit.
    *   Default: `1000`
*   `--max-tasks <NUMBER>`: (Optional) Max concurrent tasks Crankshaft manages (submitting/polling) for this TES backend. Limits Crankshaft's API interactions, not the TES service's execution capacity.
    *   Default: `50`

## What it Does

1.  **Parses Args:** Reads the `<URL>`, `--n-jobs`, and `--max-tasks`.
2.  **Configures Engine:** Programmatically creates a `TES` backend configuration using the URL and detected Basic Auth credentials (if any). Sets the `max_tasks` limit.
3.  **Initializes Engine:** Creates the `Engine` and adds the configured TES backend.
4.  **Defines Task:** Creates a simple `Task` definition suitable for TES:
    *   Image: `alpine:latest`
    *   Command: `echo "hello, world!"`
    *   This is converted internally to a TES Task JSON object.
5.  **Spawns Tasks:** For each task:
    *   Submits the task to the TES service (`POST /tasks`), getting a TES Task ID.
    *   Periodically polls the TES service (`GET /tasks/{id}?view=FULL`) for status.
    *   Stops polling when the state is terminal (`COMPLETE`, `EXECUTOR_ERROR`, `SYSTEM_ERROR`, `CANCELED`).
    *   If cancelled via Crankshaft's `CancellationToken`, sends `POST /tasks/{id}:cancel`.
6.  **Displays Output:** Shows a progress bar tracking terminal tasks. Prints the exit code, stdout, and stderr retrieved from the final `TaskLogs` in the TES response.
