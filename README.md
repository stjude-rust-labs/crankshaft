<img style="margin: 0px" alt="Repository Header Image" src="https://stjude-rust-labs.github.io/crankshaft/header.png" />
<hr/>

<div align="center">
  <p style="display: flex; align-items: center; justify-content: center;">
    <a href="https://github.com/stjude-rust-labs/crankshaft/actions/workflows/CI.yml"
      target="_blank"
      style="margin-inline-start: 2px; margin-inline-end: 2px; text-decoration: none;">
      <img alt="CI: Status" src="https://github.com/stjude-rust-labs/crankshaft/actions/workflows/CI.yml/badge.svg" />
    </a>
    <a href="https://crates.io/crates/crankshaft"
      target="_blank"
      style="margin-inline-start: 2px; margin-inline-end: 2px; text-decoration: none;">
      <img alt="crates.io version" src="https://img.shields.io/crates/v/crankshaft">
    </a>
    <a href="https://rustseq.zulipchat.com"
       target="_blank"
       style="margin-inline-start: 2px; margin-inline-end: 2px; text-decoration: none;">
      <img alt="CI: Status" src="https://img.shields.io/badge/chat-%23workflows--lib--crankshaft-blue?logo=zulip&logoColor=f6f6f6" />
    </a>
    <img alt="crates.io downloads"
         src="https://img.shields.io/crates/d/crankshaft"
         style="margin-inline-start: 2px; margin-inline-end: 2px; text-decoration: none;">
  </p>

  <p align="center">
    A headless task execution framework that supports local, cloud, and HPC.
    <br />
    <br />
    <a href="https://github.com/stjude-rust-labs/crankshaft/issues/new?assignees=&title=Descriptive%20Title&labels=enhancement">Request Feature</a>
    ·
    <a href="https://github.com/stjude-rust-labs/crankshaft/issues/new?assignees=&title=Descriptive%20Title&labels=bug">Report Bug</a>
    ·
    ⭐ Consider starring the repo! ⭐
    <br />
  </p>
</div>

## Overview

`crankshaft` is a headless task execution framework written in Rust: it's being
developed in coordination with the [`sprocket`] project with the goal of enabling
large-scale bioinformatics analyses. There is no associated `crankshaft` command line
tool—the end user is really engine _developers_ who want to include it as a core task
execution library in their own command line tools.

## Guiding Principles

- `crankshaft` aims to be a **high-performance** task execution framework
  capable of concurrently managing and executing upwards of 20,000 concurrent
  tasks. The core focus is enabling middle- to large-scale bioinformatics
  analyses, though it can also be used to design smaller scale execution
  engines.
- `crankshaft` is **headless**, which means that it doesn't do anything on its
  own; in fact, it _must_ be driven by some external orchestration code. This
  allows the `crankshaft` library itself to focus on performance improvements
  that can be enjoyed across the entire community.
- `crankshaft` is developed **independently of any particular workflow
  language**. Though it's part of the Sprocket project, it's not based on WDL,
  and, in theory, multiple frontends based on different workflow
  languages can exist (and we hope this is the case)!

## 📚 Getting Started

### Installation

To use `crankshaft`, you'll need to install [Rust](https://www.rust-lang.org/).
We recommend using [rustup](https://rustup.rs/) to accomplish this. Once Rust is
installed, you can create a new project and add the latest version of
`crankshaft` using the following command.

```bash
cargo add crankshaft
```

Once you've added `crankshaft` to your dependencies, you should head over to the
[`examples`] to see how you can use the library in your projects.

### Task Resource Usage

Backends that can observe task resource usage emit cumulative
`TaskResourceUsage` events. Every measurement is optional, and the latest event
for a task is authoritative.

Docker sampling is disabled by default. Set `resource-usage-interval` to a
positive number of seconds to sample local containers:

```rust
let config = crankshaft::config::backend::docker::Config::builder()
    .resource_usage_interval(5)
    .build();
```

Sampling requires an event channel and is unavailable for Docker Swarm
services. Memory measurements follow Docker CLI semantics by subtracting
inactive file cache from the container's reported usage. The average is the
arithmetic mean of successful polling samples, not a time-weighted value.
Sampling is best effort. Containers that finish before the first interval may
emit no resource-usage event, cumulative CPU time may be undercounted by up to
one interval, and `max_memory` is the largest observed sample rather than the
container's true lifetime peak.

TES metadata reporting is also disabled by default. Enable
`resource-usage-metadata` to poll with the TES `BASIC` view and read supported
keys from `TaskLog.metadata`:

```rust
let config = crankshaft::config::backend::tes::Config::builder()
    .url(url)
    .resource_usage_metadata(true)
    .build();
```

Crankshaft accepts `peak_memory_bytes`, `avg_memory_bytes`, `cpu_time_ms`,
`user_cpu_time_ms`, `system_cpu_time_ms`, and `disk_used_bytes` as JSON numbers
or numeric strings. The meaning of memory measurements and averages is defined
by the TES server.

Planetary currently emits top-level `peak_memory_bytes`, `avg_memory_bytes`,
and `cpu_time_ms` numeric strings aggregated across executor containers only.
Its memory values are Kubernetes working set, not process RSS; its average is
the arithmetic mean of samples; and CPU time is cumulative and rounded to
milliseconds. Planetary's nested `resource_usage` per-container breakdown is
not represented by Crankshaft's task-level event and is ignored.

## 🖥️ Development

### Prerequisites

A protobuf compiler is required to build Crankshaft with the `monitoring` 
feature enabled. Please follow the [installation guide](https://protobuf.dev/installation/)
to install the compiler.

### Building

To bootstrap a development environment, please use the following commands.

```bash
# Clone the repository
git clone git@github.com:stjude-rust-labs/crankshaft.git
cd crankshaft

# Build the crate in release mode
cargo build --release
```

## 🚧️ Tests

Before submitting any pull requests, please make sure the code passes the
following checks (from the root directory).

```bash
# Run the project's tests.
cargo test --all-features

# Ensure the project doesn't have any linting warnings.
cargo clippy --all-features

# Ensure the project passes `cargo fmt`.
cargo fmt --check

# Ensure the docs build.
cargo doc
```

## 🤝 Contributing

Contributions, issues and feature requests are welcome! Feel free to check
[issues page](https://github.com/stjude-rust-labs/crankshaft/issues).

## 📝 License

This project is licensed as either [Apache 2.0][license-apache] or
[MIT][license-mit] at your discretion. Additionally, please see [the
disclaimer](https://github.com/stjude-rust-labs#disclaimer) that applies to all
crates and command line tools made available by St. Jude Rust Labs.

Copyright © 2024-Present [St. Jude Children's Research Hospital](https://github.com/stjude).

[`examples`]: https://github.com/stjude-rust-labs/crankshaft/tree/main/examples
[license-apache]: https://github.com/stjude-rust-labs/crankshaft/blob/main/LICENSE-APACHE
[license-mit]: https://github.com/stjude-rust-labs/crankshaft/blob/main/LICENSE-MIT
[`sprocket`]: https://github.com/stjude-rust-labs/sprocket
