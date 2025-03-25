# Crankshaft Examples

This guide provides a collection of examples demonstrating various features and use cases of Crankshaft.

## Basic Examples

### 1. Simple Command Execution

```rust
use crankshaft::engine::Engine;
use crankshaft::task::Task;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = Engine::new()?;
    
    let task = Task::new("echo", vec!["Hello, World!"])
        .with_stdout("/tmp/output.txt");
    
    let task_id = engine.submit(task).await?;
    let status = engine.wait_for_completion(task_id).await?;
    
    println!("Task completed: {:?}", status);
    Ok(())
}
```

### 2. Resource-Constrained Task

```rust
use crankshaft::engine::Engine;
use crankshaft::task::Task;
use crankshaft::resources::Resources;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = Engine::new()?;
    
    let resources = Resources::new()
        .with_cpus(2)
        .with_memory_mb(4096)
        .with_disk_gb(100);
    
    let task = Task::new("heavy_computation", vec!["--input", "data.txt"])
        .with_resources(resources)
        .with_stdout("/tmp/result.txt");
    
    let task_id = engine.submit(task).await?;
    let status = engine.wait_for_completion(task_id).await?;
    
    println!("Task completed: {:?}", status);
    Ok(())
}
```

## Docker Integration

### 1. Running Containerized Task

```rust
use crankshaft::engine::Engine;
use crankshaft::task::Task;
use crankshaft::docker::DockerConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = Engine::new()?;
    
    let docker_config = DockerConfig::new("ubuntu:latest")
        .with_volume("/host/path:/container/path")
        .with_environment("KEY", "value");
    
    let task = Task::new("python", vec!["script.py"])
        .with_docker(docker_config)
        .with_stdout("/tmp/output.txt");
    
    let task_id = engine.submit(task).await?;
    let status = engine.wait_for_completion(task_id).await?;
    
    println!("Container task completed: {:?}", status);
    Ok(())
}
```

## LSF Integration

### 1. Submitting to LSF Cluster

```rust
use crankshaft::engine::Engine;
use crankshaft::task::Task;
use crankshaft::lsf::LsfConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = Engine::new()?;
    
    let lsf_config = LsfConfig::new()
        .with_queue("normal")
        .with_project("project_name")
        .with_runtime_hours(24);
    
    let task = Task::new("analysis_script", vec!["--input", "data.bam"])
        .with_lsf(lsf_config)
        .with_stdout("/scratch/output.txt");
    
    let task_id = engine.submit(task).await?;
    let status = engine.wait_for_completion(task_id).await?;
    
    println!("LSF task completed: {:?}", status);
    Ok(())
}
```

## TES Integration

### 1. Using TES Backend

```rust
use crankshaft::engine::Engine;
use crankshaft::task::Task;
use crankshaft::tes::TesConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = Engine::new()?;
    
    let tes_config = TesConfig::new("http://tes-server:8080")
        .with_auth_token("your-auth-token");
    
    let task = Task::new("bioinformatics_tool", vec!["--input", "sample.fastq"])
        .with_tes(tes_config)
        .with_stdout("/output/result.txt");
    
    let task_id = engine.submit(task).await?;
    let status = engine.wait_for_completion(task_id).await?;
    
    println!("TES task completed: {:?}", status);
    Ok(())
}
```

## Advanced Examples

### 1. Parallel Task Execution

```rust
use crankshaft::engine::Engine;
use crankshaft::task::Task;
use futures::future::join_all;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = Engine::new()?;
    
    // Create multiple tasks
    let tasks: Vec<Task> = (0..10)
        .map(|i| Task::new("process_data", vec!["--sample", &i.to_string()])
            .with_stdout(format!("/tmp/output_{}.txt", i)))
        .collect();
    
    // Submit all tasks
    let task_ids: Vec<_> = join_all(
        tasks.into_iter().map(|task| engine.submit(task))
    ).await.into_iter().collect::<Result<_, _>>()?;
    
    // Wait for all tasks to complete
    let statuses: Vec<_> = join_all(
        task_ids.into_iter().map(|id| engine.wait_for_completion(id))
    ).await.into_iter().collect::<Result<_, _>>()?;
    
    println!("All tasks completed: {:?}", statuses);
    Ok(())
}
```

### 2. Task Dependencies

```rust
use crankshaft::engine::Engine;
use crankshaft::task::Task;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = Engine::new()?;
    
    // First task: Prepare data
    let prep_task = Task::new("prepare_data", vec!["--input", "raw_data.txt"])
        .with_stdout("/tmp/prepared_data.txt");
    
    let prep_id = engine.submit(prep_task).await?;
    engine.wait_for_completion(prep_id).await?;
    
    // Second task: Process prepared data
    let process_task = Task::new("process_data", vec!["--input", "/tmp/prepared_data.txt"])
        .with_stdout("/tmp/processed_data.txt");
    
    let process_id = engine.submit(process_task).await?;
    let status = engine.wait_for_completion(process_id).await?;
    
    println!("Pipeline completed: {:?}", status);
    Ok(())
}
```

## Best Practices

1. **Resource Management**
   - Always specify appropriate resource requirements
   - Monitor resource usage and adjust as needed
   - Use resource limits to prevent task starvation

2. **Error Handling**
   - Implement proper error handling for task failures
   - Use retry mechanisms for transient failures
   - Log task outputs for debugging

3. **Performance Optimization**
   - Use parallel execution for independent tasks
   - Implement task dependencies efficiently
   - Monitor and optimize resource allocation

4. **Security**
   - Use secure authentication methods
   - Implement proper access controls
   - Follow security best practices for sensitive data

## Next Steps

1. [Explore Core Concepts](../core-concepts/overview.md)
2. [Learn about Configuration](../configuration/overview.md)
3. [Read the API Reference](../api/overview.md)
4. [Check Troubleshooting Guide](../troubleshooting/overview.md) 