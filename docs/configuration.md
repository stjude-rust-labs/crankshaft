# Configuration Overview

Crankshaft's behavior, particularly its execution backends, is primarily controlled through configuration. This allows you to adapt Crankshaft to different environments (local development, cloud, HPC) without changing your core application code.

Configuration can be provided through multiple sources, merged in order of precedence (later sources override earlier ones):

1.  **TOML Files:** The most common method.
2.  **Environment Variables:** For overrides or secrets.
3.  **Programmatic Configuration:** Building config objects directly in Rust.

## Configuration Loading

When using `crankshaft::config::Config::load()` (or `load_with_paths`), Crankshaft automatically merges settings from:

1.  **System/User Config:** `<CONFIG_DIR>/crankshaft/Crankshaft.toml` (Path determined by `dirs` crate).
2.  **Current Directory:** `Crankshaft.toml` in the application's working directory.
3.  **Explicit Paths:** Files passed to `load_with_paths`.
4.  **Environment Variables:** Prefixed with `CRANKSHAFT_`.

### Default File Name & Formats

The default base name is `Crankshaft`. Supported extensions include:

*   `.toml` (Recommended)
*   `.yaml` / `.yml`
*   `.json`
*   `.json5`
*   `.ini`

## Core Structure: `[[backends]]`

The main part of the configuration defines an array of execution backends. Each backend represents a distinct execution environment.

```toml
# --- Example Crankshaft.toml ---

# Define the first backend (e.g., local Docker)
[[backends]]
# (Required) Unique name used in `engine.spawn()`
name = "local_docker"

# (Required) Type of backend: "Docker", "TES", or "Generic"
kind = "Docker"

# (Required) Max concurrent tasks Crankshaft manages for *this* backend
max_tasks = 10

# (Optional) Default resources for tasks on this backend
defaults = { cpu = 1.0, ram = 2.0 }

# (Optional) Kind-specific settings for Docker
cleanup = true # Default: true

# --- Define a second backend (e.g., HPC via Generic/SSH) ---
[[backends]]
name = "hpc_slurm"
kind = "Generic"
max_tasks = 100
defaults = { cpu = 4.0, ram = 16.0 }

# Required Generic settings
submit = "sbatch --cpus-per-task=~{cpu} --mem=~{ram_mb}G --output=~{cwd}/slurm-%j.out --wrap='~{command}'"
monitor = "squeue -h -j ~{job_id} -o %T" # Must exit 0 if active, non-zero if finished
kill = "scancel ~{job_id}"

# Optional Generic settings
job_id_regex = "Submitted batch job (\\d+)" # Extract job ID from submit stdout
monitor_frequency = 15 # Check status every 15s

# Driver configuration for Generic backend
[backends.locale]
kind = "SSH"
host = "login.hpc.example.com"
# username = "myuser" # Optional: defaults to local user

[backends.attributes] # Custom placeholders
partition = "compute"
account = "acc123"

# --- Define a third backend (e.g., Cloud TES) ---
[[backends]]
name = "cloud_tes"
kind = "TES"
max_tasks = 50
defaults = { preemptible = true }

# Required TES setting
url = "https://tes.cloud.example.com/v1"

# Optional TES HTTP settings
[backends.http]
# basic_auth_token = "dXNlcjpwYXNzd29yZA==" # Base64("user:pass")

# Key Configuration Sections

## [[backends]]: Defines an array element for each backend configuration

- **name** (String, Required): Unique identifier used in `engine.spawn("name", ...)`.
- **kind** (String, Required): Type of backend (Docker, TES, Generic). Determines other relevant fields.
- **max_tasks** (Integer, Required): Crankshaft's concurrency limit for this specific backend. Prevents overwhelming the backend API/scheduler.
- **defaults** (Table, Optional): Default Resources (cpu, ram, disk, cpu_limit, ram_limit, preemptible). Task-specific resources override these.

## Kind-Specific Settings
Fields required or optional based on the kind. See detailed pages:
- Docker Backend
- TES Backend
- Generic Backend

## Environment Variables
Override TOML settings using environment variables prefixed with `CRANKSHAFT_`. Use double underscores (`__`) for nesting and array indices (starting from 0).

Examples:
- `CRANKSHAFT_BACKENDS__0__MAX_TASKS=20` (Overrides max_tasks for the first backend in the array).
- `CRANKSHAFT_BACKENDS__1__URL="http://new.tes.com"` (Overrides url for the second backend).
- `CRANKSHAFT_BACKENDS__0__DEFAULTS__RAM=4.0` (Overrides default RAM for the first backend).
- `CRANKSHAFT_BACKENDS__1__LOCALE__HOST="new-hpc.com"` (Overrides SSH host for the second backend, assuming it's Generic).

See the [config-rs documentation](https://docs.rs/config) for advanced mapping details.
