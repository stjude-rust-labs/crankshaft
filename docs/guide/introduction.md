---
title: Introduction
description: What Crankshaft is, who it's for, and the four parts you work with.
---

# Introduction

<p class="lede">Crankshaft is a Rust library for running tasks. You register one or more backends with an engine, describe a task, and the engine runs it on the backend you choose: a local Docker daemon, a GA4GH TES server, or an HPC scheduler such as LSF or Slurm.</p>

It was developed at St. Jude for the bioinformatics execution engine, [Sprocket](https://sprocket.bio), and is designed to manage tens to hundreds of thousands of concurrent tasks. Crankshaft doesn't parse workflows or provide a user interface. Those are left to the engine you build with it.

## Who it's for

Crankshaft is for people writing a workflow engine, a pipeline runner, or any service that needs to fan work out to compute it doesn't own. If you want to *run* WDL workflows rather than build the thing that runs them, you want Sprocket instead.

::: tip Need an engine that already works?
[Sprocket](https://sprocket.bio) is a WDL workflow engine built on Crankshaft. It handles parsing, scheduling, and call caching, and uses Crankshaft for the execution.
:::

## The four parts

Everything in Crankshaft is one of four things.

| Part | What it is | Where to read |
| --- | --- | --- |
| **Engine** | Holds named backends, spawns tasks, and broadcasts events. | [Engine](/concepts/engine) |
| **Task** | A description of work: one or more container executions, plus inputs, outputs, and resources. | [Tasks](/concepts/tasks) |
| **Backend** | Where a task runs. Docker, TES, and Generic (shell or SSH) ship in the box. | [Backends](/backends/docker) |
| **Event** | A message about a task's progress, from creation to completion. | [Events](/observe/events) |

Every run follows the same steps:

1. Register one or more backends on an engine, under names you choose.
2. Build a `Task`.
3. Call `engine.spawn(name, task, token)` to get a `TaskHandle`.
4. Call `wait()` on the handle to get one result per execution.

While the task runs, anything subscribed to the engine receives an event for each step.

Because the task description doesn't change between backends, moving a workload from a laptop to a cluster is a configuration change, not a code change.

## What's in the crate

The `crankshaft` crate re-exports a handful of smaller crates behind feature flags. The defaults are enough for most engines.

| Feature | Default | Brings in |
| --- | --- | --- |
| `engine` | yes | `crankshaft::engine` and `crankshaft::Engine` |
| `config` | yes | `crankshaft::config` and `crankshaft::Config` |
| `events` | yes | `crankshaft::events` |
| `docker` | no | `crankshaft::docker`, the lower-level Docker client wrapper |
| `monitoring` | no | `Engine::new_with_monitoring` and the gRPC monitor service |

The full API is on [docs.rs](https://docs.rs/crankshaft). These pages explain how the pieces fit; docs.rs is where to look up an exact signature.

## Project status

Crankshaft is developed by St. Jude Rust Labs and is dual licensed under MIT and Apache 2.0. It requires Rust 1.91.1 or newer (edition 2024). Questions are welcome in the `#sprocket` channel on the [OpenWDL Slack](https://join.slack.com/t/openwdl/shared_invite/zt-ctmj4mhf-cFBNxIiZYs6SY9HgM9UAVw), and the source is on [GitHub](https://github.com/stjude-rust-labs/crankshaft).
