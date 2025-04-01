# Getting Started with Crankshaft

This guide will help you get started with Crankshaft, from installation to running your first task.

## Prerequisites

Before you begin, ensure you have the following installed:

- Rust (latest stable version)
- Cargo (comes with Rust)
- Git

## Installation

1. **Install Rust**
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Add Crankshaft to your project**
   ```bash
   cargo add crankshaft
   ```

3. **Add required features**
   ```toml
   [dependencies]
   crankshaft = { version = "0.1.0", features = ["docker", "lsf"] }  # Add features as needed
   ```

## Quick Start

Here's a minimal example to get you started:

```rust
use crankshaft::engine::Engine;
use crankshaft::task::Task;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a new engine
    let engine = Engine::new()?;

    // Define a simple task
    let task = Task::new("echo", vec!["Hello, Crankshaft!"])
        .with_stdout("/tmp/output.txt");

    // Submit the task
    let task_id = engine.submit(task).await?;

    // Wait for completion
    let status = engine.wait_for_completion(task_id).await?;
    
    println!("Task completed with status: {:?}", status);
    Ok(())
}
```

## Basic Concepts

### Engine
The `Engine` is the core component of Crankshaft. It manages task execution, resource allocation, and task lifecycle.

### Task
A `Task` represents a unit of work to be executed. It includes:
- Command to execute
- Arguments
- Resource requirements
- Input/output specifications
- Environment variables

### Task Status
Tasks can be in various states:
- `Submitted`: Task has been submitted to the engine
- `Running`: Task is currently executing
- `Completed`: Task has finished successfully
- `Failed`: Task encountered an error
- `Cancelled`: Task was cancelled by the user

## Next Steps

1. [Explore Core Concepts](./core-concepts/overview.md)
2. [Learn about Configuration](./configuration/overview.md)
3. [Check out Examples](./examples/overview.md)
4. [Read the API Reference](./api/overview.md)

## Common Issues

If you encounter any issues during installation or getting started:

1. Ensure you have the latest version of Rust:
   ```bash
   rustup update
   ```

2. Check your Cargo.toml for correct dependencies:
   ```toml
   [dependencies]
   crankshaft = "0.1.0"
   tokio = { version = "1.0", features = ["full"] }
   ```

3. For more help, visit our [Troubleshooting Guide](./troubleshooting/overview.md) 