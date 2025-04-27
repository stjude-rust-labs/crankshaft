# Execution API (`crankshaft_engine::task::Execution`)

An `Execution` defines a single command run within a [Task](./task.md), specifying the environment, executable, arguments, and stream handling.

## Creating an Execution (`Execution::builder()`)

Use the fluent `ExecutionBuilder` API.

```rust
use crankshaft::engine::task::Execution;
use indexmap::indexmap;

let execution_step = Execution::builder()
    // --- Environment ---
    .image("python:3.10-slim") 
    .work_dir("/app")   

    // --- Command ---
    .program("python")        
    .args([                   
        "-u",
        "my_script.py",
        "--input", "/data/input.csv",
        "--output", "/app/results.txt"
    ])

    // --- Environment Variables ---
    .env(indexmap!{
        "API_KEY".into() => "secret123".into(), 
        "MODE".into() => "production".into()
    })

    .stdout("stdout.log")            
    .stderr("stderr.log")      

    .build();

# Execution Builder Methods

## Builder Methods

- `.image(impl Into<String>)`: **Required.** Container image ID (e.g., `ubuntu:latest`).
- `.program(impl Into<String>)`: **Required.** The executable/script to run.
- `.args(impl IntoIterator<Item = impl Into<String>>)`:
  Sets command arguments.
- `.work_dir(impl Into<Option<String>>)`:
  Sets the working directory inside the environment.
- `.env(impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>)`:
  Sets environment variables.
- `.stdin(impl Into<Option<String>>)`:
  Sets path inside environment for stdin redirection.
- `.stdout(impl Into<Option<String>>)`:
  Sets path inside environment for stdout redirection.
- `.stderr(impl Into<Option<String>>)`:
  Sets path inside environment for stderr redirection.
- `.build() -> Execution`:
  Creates the `Execution` object.

---

## Accessing Execution Properties

Use getter methods on an `Execution` instance:

- `.image() -> &str`
- `.program() -> &str`
- `.args() -> &[String]`
- `.work_dir() -> Option<&str>`
- `.env() -> &IndexMap<String, String>`
- `.stdin() -> Option<&str>`
- `.stdout() -> Option<&str>`
- `.stderr() -> Option<&str>`

---

## Role within a Task

- A `Task` contains a `NonEmpty<Execution>` list.
- Simple tasks usually have **one** `Execution`.
- Complex tasks can chain **multiple Execution steps**, executed sequentially in the order defined.
- State (files) between steps is managed via a **shared `work_dir`** or **volumes** defined at the Task level, depending on the backend.
