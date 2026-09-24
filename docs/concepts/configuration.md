---
title: Configuration
description: Describe backends in code or in a TOML or YAML file, with shared fields, per-kind options, and resource defaults.
---

# Configuration

<p class="lede">A backend is a name, a kind, a concurrency limit, and some options for that kind. You can build that in code or read it from a file with any serde format.</p>

## Fields every backend has

| Field | Required | Meaning |
| --- | --- | --- |
| `name` | yes | What you pass to `engine.spawn`. Must be unique within an engine. |
| `kind` | yes | `Docker`, `TES`, or `Generic`. Chooses which other fields apply. |
| `max-tasks` | yes | How many of this backend's tasks may run at once. |
| `defaults` | no | Resource values to use when a task doesn't set them. See [Defaults](#defaults). |

The options for each kind sit alongside these fields, at the same level. They're described on the backend pages: [Docker](/backends/docker), [TES](/backends/tes), and [Generic](/backends/generic).

## Load from a file

`crankshaft::Config` is a list of backends that implements `serde::Deserialize`, so any serde format works. Field names are kebab-case.

```toml
# backends.toml
[[backends]]
name = "local"
kind = "Docker"
max-tasks = 20
cleanup = true

[[backends]]
name = "cloud"
kind = "TES"
url = "https://tes.example.org/"
max-tasks = 500
interval = 5
http = { retries = 3, max-concurrency = 20, auth = { type = "bearer", token = "…" } }
```

```rust
use crankshaft::{Config, Engine};

let config: Config = toml::from_str(&std::fs::read_to_string("backends.toml")?)?;

let mut engine = Engine::default();
for backend in config.into_backends() {
    engine = engine.with(backend).await?;
}
```

This uses the `toml` crate. For YAML, use a serde YAML crate the same way.

::: warning Keep secrets out of the file
TES credentials sit in plain text in this format. Read the token from an environment variable or a secret store and build that backend in code instead.
:::

## Build in code

Every configuration type has a builder. This is the same TES backend as above.

```rust
use crankshaft::config::backend::{Config, Defaults, Kind, tes};

let cloud = Config::builder()
    .name("cloud")
    .kind(Kind::TES(
        tes::Config::builder()
            .url("https://tes.example.org/".parse::<url::Url>()?)
            .interval(5)
            .build(),
    ))
    .max_tasks(500)
    .defaults(Defaults::builder().cpu(2.0).ram(8.0).build())
    .build();
```

## Defaults

`defaults` accepts `cpu`, `cpu-limit`, `ram`, `ram-limit`, `disk` (all numbers, memory and disk in GiB) and `gpu` (a count).

::: info Only Generic backends use defaults today
The engine passes `defaults` to Generic backends, where they fill the `~{cpu}`, `~{ram}`, and other placeholders when a task leaves them unset. Docker and TES accept the field but don't read it. For those, set resources on each task.
:::

For a Generic backend, resources are resolved in three layers: the built-in `Resources::default()` first, then the backend's `defaults`, then whatever the task sets.

## Reference

- [`crankshaft::config`](https://docs.rs/crankshaft-config/latest/crankshaft_config/) on docs.rs
