# Troubleshooting Guide

This guide helps you identify and resolve common issues when using Crankshaft.

## Common Issues

### 1. Task Execution Failures

#### Task Not Starting
**Symptoms:**
- Task remains in "Submitted" state
- No logs generated
- No resource usage

**Solutions:**
1. Check resource availability:
   ```bash
   # Check system resources
   free -h
   nproc
   df -h
   ```

2. Verify task configuration:
   ```rust
   let task = Task::new("command", args)
       .with_resources(Resources::new()
           .with_cpus(2)
           .with_memory_mb(4096))
       .with_stdout("/path/to/output.log");
   ```

3. Enable debug logging:
   ```toml
   [logging]
   level = "debug"
   ```

#### Task Crashes
**Symptoms:**
- Task fails with non-zero exit code
- Missing output files
- Incomplete execution

**Solutions:**
1. Check task logs:
   ```bash
   cat /path/to/task.log
   ```

2. Verify input files:
   ```rust
   let task = Task::new("command", args)
       .with_input("/path/to/input")
       .with_stdout("/path/to/output.log");
   ```

3. Implement retry logic:
   ```rust
   let config = EngineConfig::new()
       .with_retry_attempts(3)
       .with_retry_delay_seconds(5);
   ```

### 2. Resource Management Issues

#### Resource Exhaustion
**Symptoms:**
- Tasks failing with "Out of Memory"
- System becoming unresponsive
- Tasks timing out

**Solutions:**
1. Monitor resource usage:
   ```rust
   let metrics = engine.get_resource_metrics().await?;
   println!("CPU Usage: {}%", metrics.cpu_usage);
   println!("Memory Usage: {}MB", metrics.memory_usage);
   ```

2. Implement resource limits:
   ```rust
   let task = Task::new("command", args)
       .with_resources(Resources::new()
           .with_cpus(2)
           .with_memory_mb(4096)
           .with_disk_gb(100));
   ```

3. Clean up resources:
   ```rust
   engine.cleanup_resources().await?;
   ```

### 3. Docker Integration Issues

#### Container Not Starting
**Symptoms:**
- Container fails to pull
- Container exits immediately
- Permission issues

**Solutions:**
1. Check Docker daemon:
   ```bash
   docker info
   docker ps
   ```

2. Verify image exists:
   ```bash
   docker images | grep your-image
   ```

3. Check container logs:
   ```bash
   docker logs container-id
   ```

#### Volume Mount Issues
**Symptoms:**
- Missing files in container
- Permission denied errors
- Path not found errors

**Solutions:**
1. Verify volume mounts:
   ```rust
   let docker_config = DockerConfig::new("image")
       .with_volume("/host/path:/container/path")
       .with_volume_permissions("rw");
   ```

2. Check file permissions:
   ```bash
   ls -la /host/path
   ```

3. Use absolute paths:
   ```rust
   let task = Task::new("command", args)
       .with_work_dir("/absolute/path");
   ```

### 4. LSF Integration Issues

#### Job Submission Failures
**Symptoms:**
- Jobs not appearing in LSF queue
- Jobs failing to start
- Resource allocation errors

**Solutions:**
1. Check LSF status:
   ```bash
   bjobs
   bqueues
   ```

2. Verify LSF configuration:
   ```toml
   [lsf]
   server = "lsf-server"
   cluster = "main"
   default_queue = "normal"
   ```

3. Check job requirements:
   ```rust
   let lsf_config = LsfConfig::new()
       .with_queue("normal")
       .with_project("project")
       .with_runtime_hours(24);
   ```

### 5. TES Integration Issues

#### Connection Problems
**Symptoms:**
- Connection timeouts
- Authentication failures
- API errors

**Solutions:**
1. Verify server URL:
   ```toml
   [tes]
   server_url = "http://tes-server:8080"
   insecure_ssl = false
   ```

2. Check authentication:
   ```toml
   [tes]
   auth_token = "your-token"
   token_expiry = 3600
   ```

3. Enable debug logging:
   ```toml
   [logging]
   level = "debug"
   ```

## Performance Optimization

### 1. Task Execution Performance

#### Slow Task Execution
**Solutions:**
1. Optimize batch size:
   ```toml
   [performance]
   task_batch_size = 100
   ```

2. Adjust polling interval:
   ```toml
   [performance]
   task_poll_interval_ms = 100
   ```

3. Implement parallel execution:
   ```rust
   let tasks: Vec<Task> = (0..10)
       .map(|i| Task::new("command", vec![i.to_string()]))
       .collect();
   
   let task_ids = join_all(
       tasks.into_iter().map(|task| engine.submit(task))
   ).await;
   ```

### 2. Resource Usage Optimization

#### High Resource Usage
**Solutions:**
1. Implement resource limits:
   ```rust
   let resource_config = ResourceConfig::new()
       .with_max_cpus(32)
       .with_max_memory_gb(256);
   ```

2. Monitor resource usage:
   ```rust
   let metrics = engine.get_resource_metrics().await?;
   if metrics.cpu_usage > 80.0 {
       engine.throttle_tasks().await?;
   }
   ```

3. Clean up unused resources:
   ```rust
   engine.cleanup_unused_resources().await?;
   ```

## Debugging Tools

### 1. Logging

Enable detailed logging:
```toml
[logging]
level = "debug"
format = "json"
output = "file"
file = "crankshaft.log"
```

### 2. Metrics Collection

Collect performance metrics:
```rust
let metrics = engine.collect_metrics().await?;
println!("Task Count: {}", metrics.task_count);
println!("Resource Usage: {:?}", metrics.resource_usage);
```

### 3. Health Checks

Implement health checks:
```rust
let health = engine.check_health().await?;
if !health.is_healthy() {
    println!("Health check failed: {:?}", health.errors);
}
```

## Getting Help

1. Check the [GitHub Issues](https://github.com/stjude-rust-labs/crankshaft/issues)
2. Join the [Community Chat](https://rustseq.zulipchat.com)
3. Review the [API Documentation](../api/overview.md)
4. Consult the [Examples](../examples/overview.md)

## Next Steps

1. [Review Configuration Guide](../configuration/overview.md)
2. [Check Examples](../examples/overview.md)
3. [Read API Reference](../api/overview.md) 