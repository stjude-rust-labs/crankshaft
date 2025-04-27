# Docker Backend Configuration

The `Docker` backend executes tasks within Docker containers locally or on a Docker Swarm manager node.

## Configuration

Set `kind = "Docker"` within a `[[backends]]` table.

```toml
[[backends]]
  # Unique name for this backend
  name = "local_docker_dev"
  kind = "Docker"

  # Max concurrent tasks Crankshaft manages for this backend
  max_tasks = 8

  # --- Optional Docker-specific fields ---

  # Remove container after task completion (default: true)
  # Set to false to keep containers for debugging.
  cleanup = true

  # --- Optional Common fields ---
  # Default resources for tasks on this backend.
  defaults = { cpu = 1.0, ram = 1.0, disk = 5.0, cpu_limit = 1.5, ram_limit = 2.0 }

# Docker-Specific Fields

## Field Descriptions

| Field   | Type    | Default | Description |
|---------|---------|---------|-------------|
| cleanup | Boolean | true    | If true, automatically removes the container (docker rm) or service (docker service rm) after execution. Forced removal on cancel. |

## Common Fields

See Backends Overview for name, kind, max_tasks, defaults.

## How it Works

- **Image Check**: Ensures the task's Execution.image exists locally (docker pull if needed).
- **Environment Check**: Detects if Docker is running standalone or in Swarm mode (and if on a manager node).
- **Execution**:
  - **Standalone/Single-Node Swarm**:
    - Creates a container (docker create) with task specs (command, args, env, limits, mounts).
    - Starts the container (docker start).
    - Attaches to logs (docker attach).
    - Waits for exit and collects status, stdout, stderr.
    - Inputs are bind-mounted from the host (direct paths or temp files for literals/URLs).
  - **Multi-Node Swarm (Manager Node)**:
    - Creates a service (docker service create) with 1 replica, no restart policy, mapping resource requests/limits.
    - Monitors the service's task.
    - Waits for the underlying container to complete.
    - Collects logs from the completed container.
    - Inputs are currently bind-mounted from the manager node where Crankshaft runs.
- **Cleanup**: If cleanup = true, removes the container or service.

## Resource Mapping

| Resource Field (defaults or Task::resources) | Standalone Container (docker run) | Swarm Service (docker service create) | Notes |
|----------------------------------------------|-----------------------------------|----------------------------------------|-------|
| cpu         | Ignored | Reservation (--reserve-cpu) | Hints scheduler. |
| ram         | Ignored | Reservation (--reserve-memory) | Hints scheduler (GiB converted to bytes). |
| disk        | Ignored | Ignored | Docker disk handling depends heavily on storage drivers. |
| cpu_limit   | Limit (--cpus) | Limit (--limit-cpu) | Hard limit on CPU usage. |
| ram_limit   | Limit (--memory) | Limit (--limit-memory) | Hard limit on RAM usage (GiB converted to bytes). |
| preemptible | Ignored | Ignored | Not applicable to Docker/Swarm. |
| zones       | Ignored | Ignored | Not applicable to Docker/Swarm. |

## Considerations

- **Permissions**: The user running Crankshaft needs Docker daemon access (e.g., docker group).
- **Input Paths**: Contents::Path must be accessible from the Crankshaft host machine for bind-mounting.
- **Swarm Mode**: Requires running Crankshaft on a manager node. Input data handling might need shared storage accessible by worker nodes if tasks aren't constrained to the manager, as Crankshaft currently relies on manager-local bind mounts for inputs.
- **Task Naming**: The Docker backend requires tasks to have a name (Task::builder().name(...)). If a task is submitted without a name, Crankshaft will generate a unique alphanumeric one automatically. This name is used for the container or service name.
