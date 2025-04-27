# Backends

Backends are the core of Crankshaft's execution strategy. They define *where* and *how* your tasks will be run. You configure backends in your `Crankshaft.toml` file (or equivalent sources) and then tell the Crankshaft `Engine` which named backend to use when spawning a task.

## Defining Backends

Backends are defined as an array under the `[[backends]]` key in your TOML configuration. Each element in the array represents one configured backend instance.

```toml
# --- Example: Multiple Backends in Crankshaft.toml ---

[[backends]]
  name = "local_dev"
  kind = "Docker"
  max_tasks = 4
  cleanup = false # Keep containers for debugging
  defaults = { cpu = 1.0, ram = 1.0 }

[[backends]]
  name = "hpc_cluster_slurm"
  kind = "Generic"
  max_tasks = 100
  defaults = { cpu = 2.0, ram = 8.0 }
  submit = "sbatch --cpus-per-task=~{cpu} --mem=~{ram_mb}G --output=~{cwd}/slurm-%j.out --wrap='~{command}'"
  monitor = "squeue -h -j ~{job_id} -o %T" # Exit 0 if running/pending
  kill = "scancel ~{job_id}"
  job_id_regex = "Submitted batch job (\\d+)"
  [backends.locale]
    kind = "SSH"
    host = "login.slurm.cluster"

[[backends]]
  name = "cloud_tes_prod"
  kind = "TES"
  max_tasks = 200
  url = "https://prod-tes.example.com/v1"
  defaults = { preemptible = true }
  [backends.http]
    # basic_auth_token = "...."

# Common Backend Configuration Fields

All backend definitions share these common fields within their `[[backends]]` table:

| Field     | Type    | Required | Description |
|-----------|---------|----------|-------------|
| name      | String  | Yes      | Unique identifier for this backend instance. Used in `engine.spawn("name", ...)`. |
| kind      | String  | Yes      | Type of backend: "Docker", "TES", or "Generic". Determines other relevant fields. |
| max_tasks | Integer | Yes      | Max concurrent tasks Crankshaft manages for this backend. Acts as a rate limiter within Crankshaft to avoid overwhelming the backend API or scheduler submission queue. |
| defaults  | Table   | No       | Default Resource requirements (`cpu`, `ram`, `disk`, `cpu_limit`, `ram_limit`, `preemptible`). Task-specific resources override these. |

## Backend Types

Choose the kind that matches your target execution environment:

### Docker

**Use Case:** Running tasks in Docker containers locally or on a Docker Swarm. Ideal for local development, testing, and containerized workflows where Docker is available.

**Key Features:** Simple setup, direct container execution, Swarm support.

### TES

**Use Case:** Submitting tasks to a GA4GH Task Execution Service v1 compliant endpoint. Common in cloud environments (e.g., Google Cloud Life Sciences API) or platforms exposing a TES API (Funnel, TESK).

**Key Features:** Standardized API, cloud-friendly, relies on TES service for execution and data staging.

### Generic

**Use Case:** Interacting with systems via command-line tools, typically over SSH. Highly flexible for HPC schedulers (Slurm, LSF, SGE, PBS/Torque) or custom remote execution setups.

**Key Features:** Adaptable via submit/monitor/kill commands, requires detailed configuration specific to the target system, supports SSH execution locale.

---

Refer to the specific documentation page for each kind for its unique configuration options and behavior details.
