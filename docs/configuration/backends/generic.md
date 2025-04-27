# Generic Backend Configuration

The `Generic` backend provides maximum flexibility by allowing you to define the exact commands Crankshaft uses to interact with an execution system, typically over SSH for HPC clusters or remote machines.

## Configuration

Set `kind = "Generic"` within a `[[backends]]` table. **Careful configuration is essential.**

```toml
[[backends]]
  name = "hpc_slurm_ssh"
  kind = "Generic"

  # Max concurrent tasks Crankshaft manages (submit/monitor) for this backend
  max_tasks = 100

  # --- Required Generic-specific fields ---

  # Command to submit a job. Must contain ~{command}. Can use other placeholders.
  submit = "sbatch --parsable --cpus-per-task=~{cpu} --mem=~{ram_mb}G --output=~{cwd}/slurm-%j.out --job-name=~{task_name:-job} --wrap='~{command}'"

  # Command to check job status. Must contain ~{job_id}.
  # MUST exit 0 if job is active (PENDING, RUNNING, etc.).
  # MUST exit non-zero if job is finished (COMPLETED, FAILED, CANCELLED, TIMEOUT, etc.).
  monitor = "squeue -h -j ~{job_id} -o %t"

  # Command to cancel a job. Must contain ~{job_id}.
  kill = "scancel ~{job_id}"

  # --- Optional Generic-specific fields ---

  # Regex to extract job ID from *stdout* of the 'submit' command.
  # ID MUST be in the first capture group `(...)`.
  # If omitted, 'submit' is assumed synchronous (no monitoring).
  job_id_regex = "^(\\d+)$" # Assumes sbatch --parsable outputs only the ID

  # Seconds between 'monitor' command executions (default: 5).
  monitor_frequency = 15

  # --- Driver Configuration (Nested Tables) ---

  # How/where to run submit/monitor/kill commands.
  [backends.locale]
    # kind = "Local" # Default: run locally
    kind = "SSH" # Run via SSH
    host = "login.hpc.example.com" # Required for SSH
    # port = 22 # Optional (default: 22)
    # username = "myhpcuser" # Optional (default: local user, uses ssh-agent)
    # [backends.locale.options] # Future SSH options

  # Shell used to execute commands.
  [backends.shell]
    # kind = "Bash" # Default
    kind = "Sh"

  # Custom key-value pairs for substitution in commands.
  [backends.attributes]
    account = "project123"
    qos = "high_priority"
    # Example usage in submit: "sbatch -A ~{account} --qos=~{qos} ..."

  # --- Optional Common fields ---
  defaults = { cpu = 4.0, ram = 16.0 }

# Generic-Specific Fields

---

## Fields

| Field | Type | Required | Description |
| :--- | :--- | :--- | :--- |
| `submit` | String | Yes | Command template to submit a job. Must contain `~{command}`. See [Placeholders](#placeholders). |
| `monitor` | String | Yes | Command template to check job status. Must contain `~{job_id}`. Exit 0 if active, non-zero if finished. |
| `kill` | String | Yes | Command template to cancel a job. Must contain `~{job_id}`. |
| `job_id_regex` | String | No | Regex to extract Job ID from `submit` stdout. If omitted, submit is synchronous. |
| `monitor_frequency` | Integer | No | Seconds between monitor calls. Default: 5. Only used if `job_id_regex` is set. |
| `[backends.locale]` | Table | No | Driver: Defines where commands run (`kind="Local"` or `kind="SSH"`). See [Driver Configuration](#driver-configuration). |
| `[backends.shell]` | Table | No | Driver: Defines how commands are run (`kind="Bash"` or `"Sh"`). |
| `[backends.attributes]` | Table | No | Defines custom key-value pairs for substitution (e.g., `~{account}`). |

---

## Driver Configuration

### `[backends.locale]`
- **kind**: `"Local"` (default) or `"SSH"`.
- If `kind = "SSH"`:
  - **host** (String, Required): Remote hostname/IP.
  - **port** (Integer, Optional, Default: 22): SSH port.
  - **username** (String, Optional): SSH username (defaults to local user).
  - **options** (Table, Optional): Placeholder for future SSH options.

### `[backends.shell]`
- **kind**: `"Bash"` (default) or `"Sh"`.

### `[backends.attributes]`
- Defines custom key-value pairs available as placeholders (e.g., `account = "abc"` for `~{account}`).

---

## Placeholders

Available for substitution in templates:

| Placeholder | Available In | Description | Example Value |
| :--- | :--- | :--- | :--- |
| `~{command}` | submit | Shell-quoted command + args from Execution. | `'bwa mem ref.fa'` |
| `~{job_id}` | monitor, kill | Job ID extracted by `job_id_regex`. | `12345` |
| `~{cwd}` | submit, monitor, kill | Working directory from Execution.work_dir. | `/path/to/work` |
| `~{task_name}` | submit, monitor, kill | Optional Task name. | `align_job` |
| `~{cpu}` | submit, monitor, kill | CPU cores from Resources. | `4.0` |
| `~{cpu_limit}` | submit, monitor, kill | CPU limit from Resources. | `4.0` |
| `~{ram}` | submit, monitor, kill | RAM in GiB from Resources. | `16.0` |
| `~{ram_mb}` | submit, monitor, kill | RAM in MiB (calculated). | `16384.0` |
| `~{ram_limit}` | submit, monitor, kill | RAM limit in GiB. | `16.0` |
| `~{disk}` | submit, monitor, kill | Disk in GiB from Resources. | `100.0` |
| `~{disk_mb}` | submit, monitor, kill | Disk in MiB (calculated). | `102400.0` |
| `~{preemptible}` | submit, monitor, kill | Boolean from Resources. | `true` |
| `~{custom_key}` | submit, monitor, kill | Value from `[backends.attributes]`. | `my_value` |

> **Tip:** You can provide default values: `~{task_name:-default_job_name}`.

---

## Substitution Process

- Placeholders are replaced.
- Whitespace (including newlines) collapses into single spaces.
- Final string executed via configured shell and locale.

---

## How it Works

1. **Submit**: Substitute placeholders -> Execute -> Extract `job_id` using `job_id_regex` (if defined).
2. **Monitor**: Substitute (including `job_id`) -> Execute -> Check exit status:
   - 0 → wait `monitor_frequency`, repeat.
   - Non-0 → stop and treat output as task result.
3. **Kill**: On cancellation, substitute (including `job_id`) and execute.

---

## Considerations

- **Monitor Command**: Must reliably distinguish active vs. finished states.
- **job_id_regex**: Must precisely capture only the job ID.
- **SSH**: Requires passwordless `ssh-agent` authentication.
- **Placeholders**: Missing ones remain literal (e.g., `~{undefined_placeholder}`).
- **Quoting**: Crankshaft quotes `~{command}` automatically.
- **Job Output**: Final `Output` comes from last monitor or submit (if no monitor).
- **Image Field**: `Execution.image` is ignored unless you use `~{image}` in templates.
