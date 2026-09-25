---
title: Tasks
description: The parts of a Task, including executions, fallback images, inputs, outputs, resources, and shared volumes.
---

# Tasks

<p class="lede">A <code>Task</code> is a description of work that doesn't depend on where it runs. It says what to run and what it needs. The backend decides how.</p>

Crankshaft's representation of a task is largely derived from the GA4GH [Task Execution Schema](https://ga4gh.github.io/task-execution-schemas/) (TES). If you know TES, most of the fields below will look familiar: a task has executors (here called executions), inputs, outputs, resources, and volumes.

## Anatomy

| Field | Type | Required | What it's for |
| --- | --- | --- | --- |
| `executions` | `NonEmpty<Execution>` | yes | The commands to run, in order. |
| `name` | `String` | no | A human name, shown in events and the console. |
| `description` | `String` | no | Free text. |
| `inputs` | `Vec<Input>` | no | Files and directories to place in the container. |
| `outputs` | `Vec<Output>` | no | Files and directories to collect afterwards. |
| `resources` | `Resources` | no | CPU, memory, disk, GPU, and scheduling hints. |
| `volumes` | `Vec<String>` | no | Paths shared by every execution in the task. |

Every type has a builder. Here's a task that uses most of them.

```rust
use std::path::PathBuf;
use crankshaft::engine::Task;
use crankshaft::engine::task::{Execution, Input, Output, Resources};
use crankshaft::engine::task::input::{Contents, Type as InputType};
use crankshaft::engine::task::output::Type as OutputType;
use nonempty::NonEmpty;
use url::Url;

let polish = Execution::builder()
    .images([
        "ghcr.io/whirligig-works/burnish:3.1.4",
        "whirligig-works/burnish:latest",
    ])?
    .program("burnish")
    .args([
        "--gizmos".into(),
        "/data/gizmos.json".into(),
        "--recipes".into(),
        "/recipes".into(),
    ])
    .work_dir("/work")
    .stdout("/work/shine-report.txt")
    .stderr("/work/burnish.log")
    .env([("BURNISH_ELBOW_GREASE".to_string(), "extra".to_string())])
    .build();

let task = Task::builder()
    .name("polish-gizmos-0142")
    .description("Polish the gizmos in order 0142 to a mirror finish")
    .inputs(vec![
        Input::builder()
            .path("/data/gizmos.json")
            .contents(Contents::Path(PathBuf::from("orders/0142/gizmos.json")))
            .ty(InputType::File)
            .build(),
        Input::builder()
            .path("/recipes")
            .contents(Contents::url_from_str("https://example.com/polish-recipes/")?)
            .ty(InputType::Directory)
            .build(),
    ])
    .outputs(vec![
        Output::builder()
            .path("/work/shine-report.txt")
            .url(Url::parse("file:///results/0142/shine-report.txt")?)
            .ty(OutputType::File)
            .build(),
    ])
    .resources(Resources::builder().cpu(4.0).ram(16.0).disk(50.0).build())
    .executions(NonEmpty::new(polish))
    .volumes(vec!["/work".to_string()])
    .build();
```

This needs the `url` crate alongside the others from [Getting started](/guide/getting-started).

## Executions

An execution is one command in one container.

| Builder method | What it sets |
| --- | --- |
| `images([…])?` | Images to try, in order. Returns `NoImageError` if the list is empty. |
| `program(…)` | The executable. |
| `args([…])` | Arguments, as a list of `String`. No shell is involved. |
| `work_dir(…)` | The working directory inside the container. |
| `stdin(…)`, `stdout(…)`, `stderr(…)` | Paths inside the container for the standard streams. |
| `env([…])` | Environment variables. Order is kept. |

A task's executions run one after another, and every one runs, even if an earlier one exits non-zero. You get one result per execution.

### Fallback images

The image list is a fallback chain, not a set. The Docker backend tries to pull each image in order and uses the first one that succeeds. The chosen image comes back in `ExecutionResult::image`, and the attempts show up as `ImagePullStarted`, `ImagePullFailed`, and `ImagePullFinished` [events](/observe/events).

::: tip
Put a pinned, digest-qualified image first and a looser tag after it. You get reproducibility when the registry cooperates and a working run when it doesn't.
:::

## Inputs

An input puts something at `path` inside the container. The `contents` say where it comes from.

| `Contents` | Source |
| --- | --- |
| `Contents::Path(PathBuf)` | A file or directory on the host. |
| `Contents::Url(Url)` | A remote location. Build one with `Contents::url_from_str("…")?`. |
| `Contents::Literal(Vec<u8>)` | Bytes you already have in memory. |

Set `ty` to `input::Type::File` or `input::Type::Directory`. Inputs are read-only unless you call `.read_only(false)`.

How an input arrives depends on the backend. Docker bind-mounts host paths directly, and fetches URLs or writes literals into a temporary directory before mounting them. TES passes URLs to the server as they are, but reads host paths and literals and sends their contents inline, which means they must be UTF-8 files, not directories.

## Outputs

An output names a `path` inside the container and a `url` to deliver it to, with a `ty` of `output::Type::File` or `output::Type::Directory`.

::: warning Docker only uses outputs for stdout and stderr
On the Docker backend, an output is used only when its `path` matches an execution's `stdout` or `stderr`. The URL must use the `file://` scheme, and the stream is written to that host path. To keep other files, bind-mount a host directory as a `Contents::Path` input with `.read_only(false)`.
:::

## Resources

| Field | Unit | Notes |
| --- | --- | --- |
| `cpu` | cores | Requested CPUs. May be fractional; TES rounds up. |
| `cpu_limit` | cores | An upper bound, where the backend supports one. |
| `ram` | GiB | Requested memory. |
| `ram_limit` | GiB | An upper bound, where the backend supports one. |
| `disk` | GiB | Requested scratch space. |
| `gpu` | count | Number of GPUs. |
| `preemptible` | bool | Whether the task may run on reclaimable capacity. |
| `zones` | list | Preferred placement zones, for backends that have them. |

`Resources::default()` is 1 CPU, 2 GiB of memory, 8 GiB of disk, and not preemptible. For Generic backends, a backend's configured `defaults` fill in whatever the task leaves unset; see [Configuration](/concepts/configuration#defaults).

## Shared volumes

Paths in `volumes` are shared by every execution in a task. That's how one execution leaves files for the next. The Docker backend creates each one in a temporary directory on the host and mounts it into every container.

## Reference

- [`Task`](https://docs.rs/crankshaft-engine/latest/crankshaft_engine/task/struct.Task.html), [`Execution`](https://docs.rs/crankshaft-engine/latest/crankshaft_engine/task/struct.Execution.html), [`Input`](https://docs.rs/crankshaft-engine/latest/crankshaft_engine/task/struct.Input.html), [`Output`](https://docs.rs/crankshaft-engine/latest/crankshaft_engine/task/struct.Output.html), and [`Resources`](https://docs.rs/crankshaft-engine/latest/crankshaft_engine/task/struct.Resources.html) on docs.rs
