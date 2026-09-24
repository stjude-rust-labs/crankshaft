---
title: Docker
description: Run tasks on a local Docker daemon or across a Docker Swarm.
---

# Docker

<p class="lede">The Docker backend runs each execution as a container on the local Docker daemon. If that daemon is a Swarm manager, it creates Swarm services instead and lets the Swarm place them.</p>

## Configure

```toml
[[backends]]
name = "docker"
kind = "Docker"
max-tasks = 20
cleanup = true
events = { send-stdout = true, send-stderr = true }
```

| Option | Default | Meaning |
| --- | --- | --- |
| `cleanup` | `true` | Remove each container when it finishes, whether it succeeded or not. |
| `events.send-stdout` | `true` | Emit a `TaskStdout` event for each chunk of standard output. |
| `events.send-stderr` | `true` | Emit a `TaskStderr` event for each chunk of standard error. |

In code, `docker::Config::default()` gives you these defaults, and `docker::Config::builder().cleanup(false).build()` changes one.

```rust
use crankshaft::config::backend::{Config, Kind, docker};

let backend = Config::builder()
    .name("docker")
    .kind(Kind::Docker(docker::Config::default()))
    .max_tasks(20)
    .build();
```

## What happens when a task runs

For each execution, in order:

1. **Pick an image.** The backend tries to pull each image in the list and keeps the first that succeeds. When an image isn't already present, the pull emits `ImagePullStarted`, then `ImagePullFinished` or `ImagePullFailed`.
2. **Mount inputs and volumes.** Host paths are bind-mounted. URL and literal inputs are written to a temporary directory and mounted from there. Each shared volume gets its own directory in that temporary space.
3. **Run.** A container (or a Swarm service) is created, emits `TaskContainerCreated`, runs, and emits `TaskContainerExited`.
4. **Clean up.** With `cleanup` on, the container is removed.

After the last execution, the task emits `TaskCompleted` with every exit status.

## Local daemon or Swarm

When the backend starts, it asks the daemon what it's part of.

- **Not in a Swarm:** it uses containers and reads the host's CPU and memory totals.
- **In an active Swarm, on a manager node:** it adds up the CPU and memory of the ready nodes and runs every execution as a Swarm service.
- **In a Swarm, but not a manager:** it refuses to start, because it couldn't schedule anything.

## How resources map

Docker has no notion of a minimum reservation for a plain container, so requests and limits behave differently depending on the mode.

| Task resource | Plain container | Swarm service |
| --- | --- | --- |
| `cpu` | Ignored | CPU reservation |
| `cpu_limit` | CPU limit | CPU limit |
| `ram` | Ignored | Memory reservation |
| `ram_limit` | Memory limit | Memory limit |
| `disk` | Storage size option | Not applied |
| `gpu` | NVIDIA device request | Not applied |

GPU requests use Docker's `nvidia` driver, so the host needs the [NVIDIA Container Toolkit](https://docs.nvidia.com/datacenter/cloud-native/container-toolkit/latest/install-guide.html). Other GPU vendors aren't supported yet. If you need one, please [file an issue](https://github.com/stjude-rust-labs/crankshaft/issues/new). The `size` storage option only works with storage drivers that support it.

## Things to know

::: warning Outputs are only used for stdout and stderr
The Docker backend uses a task's outputs only to find where to write each execution's `stdout` and `stderr`. The output's URL must be `file://`. To collect other files, bind-mount a writable host directory as an input.
:::

- Use `cleanup = false` when you're debugging a failing image, so you can `docker inspect` the stopped container. Remember to remove them yourself.
- `TaskStdout` and `TaskStderr` events carry raw bytes. On a busy engine they can outnumber everything else, so turn them off if nothing reads them.

## Example

`cargo run --release --bin docker` in the repository runs a batch of tasks on your local daemon. `docker-monitored` does the same with the [monitor](/observe/monitoring) turned on.
