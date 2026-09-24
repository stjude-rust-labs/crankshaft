---
title: TES
description: Run tasks on any server that implements the GA4GH Task Execution Service API.
---

# TES

<p class="lede">The TES backend sends each task to a server that speaks the <a href="https://ga4gh.github.io/task-execution-schemas/">GA4GH Task Execution Service API</a>. The server runs the containers; Crankshaft submits, polls, and cancels.</p>

Any server that implements TES will work, including these (listed alphabetically).

<TesServers />

The GA4GH keeps a [full list of TES implementations](https://github.com/ga4gh/task-execution-schemas#tes-compliant-implementations).

## Configure

```toml
[[backends]]
name = "cloud"
kind = "TES"
url = "https://tes.example.org/"
max-tasks = 500
interval = 5

[backends.http]
retries = 3
max-concurrency = 20
auth = { type = "bearer", token = "…" }
```

| Option | Default | Meaning |
| --- | --- | --- |
| `url` | required | Base URL of the TES server. |
| `interval` | `1` | Seconds between status polls. |
| `http.retries` | `0` | Retries for each HTTP request. |
| `http.max-concurrency` | `10` | How many HTTP requests the backend makes at once. |
| `http.auth` | none | `{ type = "basic", username, password }` or `{ type = "bearer", token }`. |

`max-tasks` and `http.max-concurrency` limit different things. `max-tasks` caps how many of your tasks exist on the server at once. `max-concurrency` caps how many requests are in flight, which protects the server when thousands of tasks poll at once.

::: warning Credentials
Don't commit a token in a configuration file. Build the `HttpAuthConfig` in code from an environment variable or your secret store.
:::

## How a task maps to TES

A Crankshaft task becomes one TES task. Each execution becomes one TES executor, in the same order, and the server runs them in sequence.

| Crankshaft | TES |
| --- | --- |
| `name`, `description` | `name`, `description` |
| Each `Execution` | An executor (image, command, workdir, env, streams) |
| `Input` with `Contents::Url` | Input with `url` |
| `Input` with `Contents::Path` or `Literal` | Input with inline `content`. Must be a UTF-8 file. |
| `Output` | Output with `url` and `path` |
| `cpu` (cores, may be fractional) | `cpu_cores` (whole cores), rounded up |
| `ram` (GiB) | `ram_gb` (GB), multiplied by 1.073741824 |
| `disk` (GiB) | `disk_gb` (GB), multiplied by 1.073741824 |
| `cpu_limit`, `ram_limit`, `gpu` | Not sent. TES has no fields for them. |
| `preemptible`, `zones` | `preemptible`, `zones` |
| `volumes` | `volumes` |

TES executors take a single image, so a Crankshaft execution sends only the first image in its fallback list.

## Results and preemption

When the server reports a final state, the backend reads the last task log (servers may retry internally) and returns one exit status per executor.

| TES state | What `wait()` returns |
| --- | --- |
| `COMPLETE` or `EXECUTOR_ERROR` | `Ok`, with each executor's exit code |
| `SYSTEM_ERROR` | `TaskRunError::Other`, with the server's system logs |
| `CANCELED` | `TaskRunError::Canceled` |
| `PREEMPTED` | `TaskRunError::Preempted`, and a `TaskPreempted` event |

Preemption is the one case where the right response is usually to submit the same task again. `TaskCreated` carries the server's `tes_id`, so you can match your records to the server's.

## Example

`cargo run --release --bin tes` in the repository submits tasks to a TES server. Read `examples/src/tes/main.rs` for the arguments it expects.
