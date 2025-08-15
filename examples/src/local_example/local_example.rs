//!c # Usage
//!
//! ```bash
//! cargo run --bin local_example
//! ```
//!
//! # Expected Output
//!
//! The example will generate multiple random numbers between 10,000 and 100,000,
//! and find the next Fibonacci number for each using separate Crankshaft tasks.
use std::path::Path;
use std::process::Command;
use std::time::Instant;

use anyhow::{Context, Result};
use rand::Rng;

/// Represents a task to be executed locally.
#[derive(Debug, Clone)]
struct LocalTask {
    /// Unique identifier for the task.
    id: usize,
    /// Program/command to execute.
    program: String,
    /// Command-line arguments to pass to the program.
    args: Vec<String>,
    /// Human-readable description of the task.
    description: String,
}

impl LocalTask {
    /// Creates a new local task.
    fn new(id: usize, program: &str, args: Vec<&str>, description: &str) -> Self {
        Self {
            id,
            program: program.to_string(),
            args: args.into_iter().map(String::from).collect(),
            description: description.to_string(),
        }
    }

    /// Executes the task and returns the result.
    fn execute(&self) -> Result<TaskResult> {
        let start_time = Instant::now();

        println!("Task {}: {}", self.id, self.description);

        let output = Command::new(&self.program)
            .args(&self.args)
            .output()
            .with_context(|| {
                format!(
                    "Failed to execute command: {} {}",
                    self.program,
                    self.args.join(" ")
                )
            })?;

        let duration = start_time.elapsed();

        let result = if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            println!("Result: {}", stdout);
            println!("Completed in {:?}", duration);

            TaskResult {
                task_id: self.id,
                success: true,
                stdout,
                stderr: String::new(),
                duration,
            }
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            println!("Error: {}", stderr);
            println!("Failed after {:?}", duration);

            TaskResult {
                task_id: self.id,
                success: false,
                stdout: String::new(),
                stderr,
                duration,
            }
        };

        println!();
        Ok(result)
    }
}

/// Result of task execution.
#[allow(dead_code)] // Leaving unused fields for clarity and demonstration purposes; You might want these!
#[derive(Debug)]
struct TaskResult {
    /// The task ID that produced this result.
    task_id: usize,
    /// Whether the task executed successfully.
    success: bool,
    /// Standard output captured from the task.
    stdout: String,
    /// Standard error captured from the task.
    stderr: String,
    /// Duration of task execution.
    duration: std::time::Duration,
}

/// Sample Crankshaft task execution workflow in a local environment.
#[tokio::main]
async fn main() -> Result<()> {
    println!("Crankshaft Fibonacci Calculator Example");
    println!("=========================================\n");
    println!("Demonstrating Crankshaft concepts with multiple Fibonacci calculations.\n");

    // 1. Generate multiple random numbers between 10,000 and 100,000
    let mut rng = rand::rng();
    const NUM_TASKS: usize = 5;
    let random_numbers: Vec<u32> = (0..NUM_TASKS)
        .map(|_| rng.random_range(10_000..=100_000))
        .collect();

    println!("Generated random numbers:");
    for (i, num) in random_numbers.iter().enumerate() {
        println!("   Task {}: {}", i + 1, num);
    }
    println!();

    // Verify the Fibonacci calculator script exists
    let script_path = Path::new("examples/src/local_example/fibonacci_calculator.py");
    if !script_path.exists() {
        return Err(anyhow::anyhow!(
            "Fibonacci calculator script not found at {}",
            script_path.display()
        ));
    }

    // 2. Define tasks to calculate the next Fibonacci number for each random number
    let tasks: Vec<LocalTask> = random_numbers
        .iter()
        .enumerate()
        .map(|(i, &random_number)| {
            LocalTask::new(
                i + 1,
                "python3",
                vec![script_path.to_str().unwrap(), &random_number.to_string()],
                &format!("Find next Fibonacci number after {}", random_number),
            )
        })
        .collect();

    println!(
        "Executing {} Fibonacci calculation tasks sequentially:\n",
        tasks.len()
    );

    // 3. Execute tasks with proper error handling
    let mut results = Vec::new();
    let total_start = Instant::now();

    for task in tasks {
        match task.execute() {
            Ok(result) => results.push(result),
            Err(e) => {
                eprintln!("Task {} failed: {:#}", task.id, e);
                // In real Crankshaft, failed tasks would be retried or reported
                continue;
            }
        }
    }

    let total_duration = total_start.elapsed();

    // 4. Report execution summary
    println!("Execution Summary:");
    println!("==================\n");

    let successful_tasks = results.iter().filter(|r| r.success).count();
    let failed_tasks = results.len() - successful_tasks;

    println!("Results:");
    println!("   • Total tasks: {}", results.len());
    println!("   • Successful: {}", successful_tasks);
    println!("   • Failed: {}", failed_tasks);
    println!("   • Total time: {:?}", total_duration);

    if !results.is_empty() {
        let avg_time = total_duration / results.len() as u32;
        println!("   • Average time per task: {:?}\n", avg_time);

        println!("Fibonacci Results Summary:");
        for (i, result) in results.iter().enumerate() {
            if result.success {
                let lines: Vec<&str> = result.stdout.lines().collect();
                if lines.len() >= 3 {
                    let input = lines[0].split(": ").nth(1).unwrap_or("N/A");
                    let next_fib = lines[2].split(": ").nth(1).unwrap_or("N/A");
                    println!("   Task {}: {} → {}", i + 1, input, next_fib);
                }
            }
        }
    }
    println!();

    Ok(())
}
