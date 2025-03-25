# Configuration Guide

This guide covers all aspects of configuring Crankshaft for different environments and use cases.

## Environment Setup

### 1. Basic Configuration

Create a configuration file `crankshaft.toml` in your project root:

```toml
[engine]
max_concurrent_tasks = 100
task_timeout_seconds = 3600
retry_attempts = 3

[resources]
default_cpus = 2
default_memory_mb = 4096
default_disk_gb = 100

[logging]
level = "info"
file = "crankshaft.log"
```

### 2. Environment Variables

Crankshaft supports configuration through environment variables:

```bash
export CRANKSHAFT_MAX_CONCURRENT_TASKS=100
export CRANKSHAFT_TASK_TIMEOUT=3600
export CRANKSHAFT_LOG_LEVEL=info
```

## Engine Configuration

### 1. Task Execution Settings

```rust
use crankshaft::engine::Engine;
use crankshaft::config::EngineConfig;

let config = EngineConfig::new()
    .with_max_concurrent_tasks(100)
    .with_task_timeout_seconds(3600)
    .with_retry_attempts(3);

let engine = Engine::with_config(config)?;
```

### 2. Resource Management

```rust
use crankshaft::resources::Resources;
use crankshaft::config::ResourceConfig;

let resource_config = ResourceConfig::new()
    .with_default_cpus(2)
    .with_default_memory_mb(4096)
    .with_default_disk_gb(100)
    .with_resource_limits(true);

let engine = Engine::with_resource_config(resource_config)?;
```

## Docker Configuration

### 1. Basic Docker Setup

```toml
[docker]
registry = "docker.io"
insecure_registry = false
pull_policy = "if-not-present"
```

### 2. Advanced Docker Settings

```toml
[docker]
registry = "private-registry.example.com"
insecure_registry = true
pull_policy = "always"
network_mode = "host"
privileged = false
capabilities = ["SYS_ADMIN"]
```

## LSF Configuration

### 1. Cluster Settings

```toml
[lsf]
server = "lsf-server.example.com"
cluster = "main"
default_queue = "normal"
project = "default"
```

### 2. Resource Limits

```toml
[lsf.resources]
max_cpus = 32
max_memory_gb = 256
max_runtime_hours = 72
```

## TES Configuration

### 1. Server Settings

```toml
[tes]
server_url = "http://tes-server:8080"
auth_token = "your-auth-token"
insecure_ssl = false
```

### 2. Task Settings

```toml
[tes.task]
default_work_dir = "/scratch"
default_storage = "s3://bucket"
```

## Logging Configuration

### 1. Basic Logging

```toml
[logging]
level = "info"
format = "json"
output = "file"
file = "crankshaft.log"
```

### 2. Advanced Logging

```toml
[logging]
level = "debug"
format = "json"
output = "both"
file = "crankshaft.log"
syslog = true
syslog_facility = "daemon"
```

## Security Configuration

### 1. Authentication

```toml
[security]
auth_type = "token"
token_file = "/etc/crankshaft/token"
token_expiry = 3600
```

### 2. Access Control

```toml
[security.access]
allowed_users = ["user1", "user2"]
allowed_groups = ["admin", "scientists"]
restricted_commands = ["rm", "mkfs"]
```

## Performance Tuning

### 1. Task Execution

```toml
[performance]
task_batch_size = 100
task_queue_size = 1000
task_poll_interval_ms = 100
```

### 2. Resource Management

```toml
[performance.resources]
resource_check_interval_ms = 5000
resource_cleanup_interval_ms = 300000
max_resource_usage_percent = 90
```

## Configuration Best Practices

1. **Environment-Specific Configs**
   - Use different config files for development, staging, and production
   - Use environment variables for sensitive information
   - Keep configuration files in version control

2. **Resource Management**
   - Set appropriate resource limits
   - Monitor resource usage
   - Implement resource cleanup policies

3. **Security**
   - Use secure authentication methods
   - Implement proper access controls
   - Follow security best practices

4. **Performance**
   - Tune batch sizes and intervals
   - Monitor system performance
   - Implement proper logging

## Configuration Validation

Crankshaft provides configuration validation:

```rust
use crankshaft::config::Config;

let config = Config::from_file("crankshaft.toml")?;
config.validate()?;
```

## Next Steps

1. [Explore Examples](../examples/overview.md)
2. [Read the API Reference](../api/overview.md)
3. [Check Troubleshooting Guide](../troubleshooting/overview.md) 