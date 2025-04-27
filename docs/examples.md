# Examples

The `examples/` directory provides practical, runnable demonstrations of Crankshaft's capabilities with different backends.

## Running the Examples

1.  **Clone:** `git clone https://github.com/stjude-rust-labs/crankshaft.git && cd crankshaft`
2.  **Build:** `cargo build --examples` (optional, fetches dependencies)
3.  **Run:** `cargo run --release --bin <EXAMPLE_NAME> -- [ARGS]`
    *   Check each example's specific README or run with `--help` for available `[ARGS]`.

---

## Docker Example (`--bin docker`)

*   **Goal:** Demonstrate basic task submission to a local Docker daemon.
*   **Task:** Runs `echo "hello, world!"` in an `alpine` container.
*   **Backend Config:** Defined programmatically in the example code.
*   **Key Concepts Shown:** Engine initialization, programmatic backend config, simple task definition, spawning, waiting for results.
*   **Prerequisites:** Docker daemon running.
*   **[View Docker Example README](https://github.com/stjude-rust-labs/crankshaft/blob/main/examples/src/docker/README.md)**

---

## TES Example (`--bin tes`)

*   **Goal:** Show interaction with a GA4GH Task Execution Service (TES) v1 endpoint.
*   **Task:** Runs `echo "hello, world!"` in an `alpine` container via TES.
*   **Backend Config:** Defined programmatically, taking URL from command line and Basic Auth from environment variables (`USER`, `PASSWORD`).
*   **Key Concepts Shown:** TES backend configuration, handling authentication (Basic Auth), task submission to a remote service, polling for status.
*   **Prerequisites:** Access to a running TES v1 endpoint URL. Optional `USER`/`PASSWORD` env vars.
*   **[View TES Example README](https://github.com/stjude-rust-labs/crankshaft/blob/main/examples/src/tes/README.md)**

---

## LSF Example (`--bin lsf`)

*   **Goal:** Illustrate using the `Generic` backend to submit jobs to an LSF cluster via SSH. **Requires significant adaptation.**
*   **Task:** Runs `echo "hello, world!"` via `bsub`.
*   **Backend Config:** Hardcoded YAML string within the example, demonstrating `submit`, `monitor`, `kill` commands, `job_id_regex`, SSH locale, and placeholder substitution.
*   **Key Concepts Shown:** `Generic` backend configuration for HPC, SSH execution locale, command template substitution, job ID extraction, custom monitoring logic.
*   **Prerequisites:** LSF cluster access, passwordless SSH via agent, custom `check-job-alive` script on the cluster.
*   **[View LSF Example README](https://github.com/stjude-rust-labs/crankshaft/blob/main/examples/src/lsf/README.md)**

---
