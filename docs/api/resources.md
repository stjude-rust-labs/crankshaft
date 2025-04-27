# Resources API (`crankshaft_engine::task::Resources`)

Specify computational requirements for a [Task](./task.md) using the `Resources` struct and its builder. These act as requests or limits depending on the backend.

---

## Creating Resources (`Resources::builder()`)

```rust
use crankshaft::engine::task::Resources;

let resources = Resources::builder()
    // --- Core Requests ---
    .cpu(4.0)       // Request 4 CPU cores
    .ram(16.0)      // Request 16 GiB RAM
    .disk(200.0)    // Request 200 GiB disk (backend support varies)

    // --- Limits (Optional, Backend Dependent) ---
    .cpu_limit(4.0) // Hard limit of 4.0 CPU cores (e.g., Docker)
    .ram_limit(16.0) // Hard limit of 16 GiB RAM (e.g., Docker)

    // --- Other Hints (Backend Dependent) ---
    .preemptible(true) // OK to use preemptible/spot instances (e.g., TES)
    .zones(["us-west1-a".to_string()]) // Preferred compute zone (e.g., TES)

    .build();
```

---

### Builder Methods

- `.cpu(impl Into<Option<f64>>)` — Requested CPU cores.
- `.ram(impl Into<Option<f64>>)` — Requested RAM in GiB.
- `.disk(impl Into<Option<f64>>)` — Requested disk space in GiB.
- `.cpu_limit(impl Into<Option<f64>>)` — Hard CPU limit (cores).
- `.ram_limit(impl Into<Option<f64>>)` — Hard RAM limit (GiB).
- `.preemptible(impl Into<Option<bool>>)` — Hint for preemptible instances.
- `.zones(impl IntoIterator<Item = String>)` — Preferred compute zones.
- `.build() -> Resources` — Creates the `Resources` object.

> **Tip:**  
> If a field is not set via the builder, it remains `None` in the `Resources` struct. However, Crankshaft applies defaults during execution based on precedence.

---

## Defaults and Precedence

Resource values for a task are determined in this order (later steps override earlier ones):

1. **Crankshaft Internal Defaults:** Minimal values (e.g., 1 CPU, 2 GiB RAM, 8 GiB Disk).
2. **Backend Defaults:** Values set in the `defaults` table of the backend's configuration in `Crankshaft.toml`.
3. **Task Resources:** Values explicitly set using `Task::builder().resources(...)`.

### Example: Backend Defaults in `Crankshaft.toml`

```toml
[[backends]]
name = "compute_optimized"
kind = "TES"
max_tasks = 20
defaults = { cpu = 8.0, ram = 32.0, preemptible = false }
url = "..."
```

If a task submitted to `compute_optimized` specifies:

```rust
.resources(Resources::builder().ram(64.0).build())
```

it will request **8 CPUs** (from backend default) and **64 GiB RAM** (from task override).

---

## Accessing Resource Properties

Use getter methods on a `Resources` instance:

- `.cpu() -> Option<f64>`
- `.ram() -> Option<f64>`
- `.disk() -> Option<f64>`
- `.cpu_limit() -> Option<f64>`
- `.ram_limit() -> Option<f64>`
- `.preemptible() -> Option<bool>`
- `.zones() -> &[String]`

---

## Backend Interpretation & Mapping

| Crankshaft Field | Docker (Standalone) | Docker (Swarm Service) | TES v1 | Generic Backend Placeholder | Notes |
|:-----------------|:---------------------|:------------------------|:-------|:-----------------------------|:------|
| `cpu`            | Ignored               | Reservation              | `cpu_cores` | `~{cpu}` | Request/Hint |
| `ram`            | Ignored               | Reservation (bytes)      | `ram_gb` | `~{ram}`, `~{ram_mb}` | Request/Hint (GiB) |
| `disk`           | Ignored               | Ignored                  | `disk_gb` | `~{disk}`, `~{disk_mb}` | Request/Hint (GiB) |
| `cpu_limit`      | Limit (`--cpus`)       | Limit                    | Ignored | `~{cpu_limit}` | Hard Limit |
| `ram_limit`      | Limit (`--memory`)     | Limit (bytes)            | Ignored | `~{ram_limit}` | Hard Limit (GiB) |
| `preemptible`    | Ignored               | Ignored                  | `preemptible` | `~{preemptible}` | Hint |
| `zones`          | Ignored               | Ignored                  | `zones` | N/A | Hint |

> **Warning:**  
> For the Generic backend, resource values only have an effect if you use the corresponding placeholders (like `~{cpu}`, `~{ram_mb}`) in your submit command template within the configuration.

---
