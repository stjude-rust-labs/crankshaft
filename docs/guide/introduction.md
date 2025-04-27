# Introduction

Welcome to Crankshaft!

Crankshaft is a **headless task execution framework** built in Rust. It's designed to be a high-performance library that developers can integrate into their own applications (like workflow managers, scientific analysis platforms, or custom automation tools) to handle the execution of computational tasks across various environments.

## What does "Headless" mean?

"Headless" means Crankshaft doesn't have its own command-line interface (CLI) or graphical user interface (GUI). It's purely a library (`lib.rs`) that provides the core engine for:

1.  ✅ **Defining Tasks:** Programmatically describe the work to be done (commands, container images, inputs, outputs, resources).
2.  ⚙️ **Configuring Backends:** Specify *where* and *how* tasks should run (e.g., local Docker, a TES endpoint, an HPC cluster via SSH).
3.  ➡️ **Submitting Tasks:** Send defined tasks to the appropriate configured backend.
4.  🚦 **Managing Concurrency:** Control how many tasks run simultaneously on each backend.
5.  📊 **Monitoring & Results:** Track task progress (where possible) and retrieve results (exit status, stdout, stderr).

The application *using* Crankshaft is responsible for the user-facing parts, workflow logic, task definition generation, and result interpretation.

## Target Audience

Crankshaft is primarily for **Rust developers** building systems that need to orchestrate and execute potentially many computational tasks, especially in bioinformatics or other scientific domains, but applicable anywhere batch-style task execution is needed.

## Key Features

*   🚀 **Performance:** Leverages Rust and Tokio for efficient asynchronous operations, suitable for managing thousands of concurrent tasks.
*   🧩 **Multiple Backends:** Out-of-the-box support for:
    *   **Docker:** Running tasks locally in Docker containers (including Docker Swarm).
    *   **TES (Task Execution Service):** Interacting with GA4GH TES v1 compliant endpoints.
    *   **Generic:** A highly configurable backend for running commands via SSH on remote systems or HPC clusters (e.g., LSF, Slurm, Grid Engine - requires user configuration).
*   🔧 **Configuration:** Flexible configuration via TOML files (`Crankshaft.toml`) and environment variables.
*   📝 **Task Definition API:** A fluent Rust API (`TaskBuilder`, `ExecutionBuilder`, etc.) for defining tasks, their steps, inputs, outputs, and resource requirements.
*   🛑 **Cancellation Support:** Graceful task cancellation via `tokio_util::sync::CancellationToken`.

## Next Steps

*   Understand the [Core Concepts](./concepts.md).
*   Follow the [Getting Started](./getting-started.md) guide for a practical example.
*   Explore the [Configuration](../configuration.md) options.
