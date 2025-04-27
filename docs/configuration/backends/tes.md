# TES Backend Configuration

The `TES` backend submits tasks to endpoints compliant with the GA4GH Task Execution Service (TES) v1 specification.

## Configuration

Set `kind = "TES"` within a `[[backends]]` table.

```toml
[[backends]]
  name = "cloud_tes_service"
  kind = "TES"

  # Max concurrent tasks Crankshaft manages (submitting/polling) for this backend
  max_tasks = 100

  # --- Required TES-specific fields ---

  # Base URL of the TES v1 endpoint (e.g., ending in /v1)
  url = "https://tes-server.example.com/ga4gh/tes/v1"

  # --- Optional TES-specific fields ---

  # Optional table for HTTP client settings
  [backends.http]
    # For HTTP Basic Authentication: Base64 encoded "username:password"
    # Example: echo -n "user:pass" | base64 -> dXNlcjpwYXNz
    basic_auth_token = "dXNlcjpwYXNz"

  # --- Optional Common fields ---
  # Default resources map directly to TES Task Resources.
  defaults = { cpu = 2.0, ram = 4.0, disk = 50.0, preemptible = false }


# TES-Specific Fields

| Field | Type | Required | Description |
|------|------|-----------|-------------|
| url | String | Yes | Full base URL of the TES v1 API endpoint. |
| [backends.http] | Table | No | Optional settings for the HTTP client. |
| basic_auth_token | String | No | (Inside [backends.http]) Base64 encoded username:password string for HTTP Basic Authentication. |

> **Tip**: Other Authentication  
> For authentication methods other than Basic Auth (e.g., Bearer tokens), you may need to configure the Engine programmatically by building a `tes::Client` manually and potentially extending the configuration schema if loading from files is desired.

## Common Fields

See **Backends Overview** for `name`, `kind`, `max_tasks`, `defaults`.

## How it Works

- **Task Conversion**: A `crankshaft::engine::Task` is translated into a `tes::v1::types::Task` JSON object.
  - `Execution` -> TES `Executor`
  - `Input` -> TES `Input`
    - `Contents::Url` -> `Input.url`
    - `Contents::Path/Literal` -> `Input.content` (must be UTF-8)
  - `Output` -> TES `Output`
  - `Resources` -> TES `Resources`
- **Submission**: Sends `POST /tasks` to the `url` with the TES Task object. Includes auth headers if configured. Receives a TES Task ID.
- **Monitoring**: Periodically sends `GET /tasks/{id}?view=FULL` to poll the task state.
- **Completion/Cancellation**:
  - Stops polling when state is terminal (`COMPLETE`, `EXECUTOR_ERROR`, `SYSTEM_ERROR`, `CANCELED`).
  - On `CancellationToken.cancel()`, sends `POST /tasks/{id}:cancel`.
- **Result Retrieval**: Extracts exit code(s), stdout, stderr from the `TaskLogs` in the final `GET /tasks/{id}` response.

## Resource Mapping

Crankshaft Resources fields map directly to TES Task.Resources:

| Crankshaft Field | TES Field | Notes |
|------------------|-----------|-------|
| cpu | cpu_cores | Integer conversion. |
| ram | ram_gb | GiB. |
| disk | disk_gb | GiB. |
| preemptible | preemptible | Boolean hint. |
| zones | zones | List of strings (currently not in defaults). |
| cpu_limit | Ignored | Not part of TES v1 standard. |
| ram_limit | Ignored | Not part of TES v1 standard. |

## Considerations

- **TES Compliance**: The target `url` must point to a compliant GA4GH TES v1 service.
- **Data Staging**: The TES service is responsible for handling data transfers based on `Input.url`, `Input.content`, and `Output.url`. Ensure the service has the necessary permissions (e.g., cloud storage access).
- **Resource Enforcement**: Resource requests are hints to the TES scheduler. Actual enforcement depends on the TES implementation and its underlying compute environment.
- **UTF-8 Content**: When using `Contents::Path` or `Contents::Literal` for inputs, the content is sent inline and must be valid UTF-8 as required by the TES schema for the content field. This is unsuitable for large binary files; use `Contents::Url` instead.
