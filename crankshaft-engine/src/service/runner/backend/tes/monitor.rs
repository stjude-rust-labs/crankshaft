//! Implements the TES task monitor.

use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::SystemTime;

use anyhow::Context;
use anyhow::Result;
use anyhow::anyhow;
use crankshaft_events::Event;
use crankshaft_events::TaskId;
use crankshaft_events::TaskResourceUsage;
use crankshaft_events::send_event;
use tes::v1::types::requests::ListTasksParams;
use tes::v1::types::requests::MAX_PAGE_SIZE;
use tes::v1::types::requests::View;
use tes::v1::types::responses::ListTasks;
use tes::v1::types::task::State as TesState;
use tokio::select;
use tokio::sync::broadcast;
use tokio::sync::oneshot;
use tokio::time::MissedTickBehavior;
use tracing::debug;
use tracing::info;

/// The name of the tag used to group tasks together for monitoring.
pub const CRANKSHAFT_GROUP_TAG_NAME: &str = "crankshaft-task-group";

/// The identifier and state extracted from a polled TES task.
struct MonitoredTaskState {
    /// The TES task identifier.
    id: String,
    /// The TES task state.
    state: Option<TesState>,
}

/// Parses the documented resource usage keys from a TES task log's metadata.
///
/// The TES specification designates `TaskLog.metadata` for
/// implementation-specific data; servers that report resource usage are
/// expected to use the following keys, with values as JSON numbers or numeric
/// strings:
///
/// * `peak_memory_bytes` — peak sampled memory, in bytes
/// * `avg_memory_bytes` — average sampled memory, in bytes; the averaging
///   method is server-defined
/// * `cpu_time_ms` — total CPU time, in milliseconds
/// * `user_cpu_time_ms` — user-mode CPU time, in milliseconds
/// * `system_cpu_time_ms` — system-mode CPU time, in milliseconds
/// * `disk_used_bytes` — disk space used, in bytes
///
/// Unknown keys and unparseable values are ignored.
#[allow(clippy::field_reassign_with_default)]
fn parse_resource_usage_metadata(metadata: &serde_json::Value) -> TaskResourceUsage {
    /// Gets a numeric metadata value as a `u64`, accepting JSON numbers and
    /// numeric strings.
    fn get_u64(metadata: &serde_json::Value, key: &str) -> Option<u64> {
        let value = metadata.get(key)?;
        value
            .as_u64()
            .or_else(|| value.as_str().and_then(|s| s.trim().parse().ok()))
    }

    let mut usage = TaskResourceUsage::default();
    usage.max_memory = get_u64(metadata, "peak_memory_bytes");
    usage.avg_memory = get_u64(metadata, "avg_memory_bytes");
    usage.cpu_time_ms = get_u64(metadata, "cpu_time_ms");
    usage.user_cpu_time_ms = get_u64(metadata, "user_cpu_time_ms");
    usage.system_cpu_time_ms = get_u64(metadata, "system_cpu_time_ms");
    usage.disk_used = get_u64(metadata, "disk_used_bytes");
    usage
}

/// Folds resource usage from a task's logs into a cumulative snapshot.
///
/// TES servers create a new `TaskLog` for each internal retry, so usage must
/// be folded across every attempt rather than read from the latest log:
///
/// * peak memory and disk used take the maximum across attempts;
/// * CPU times sum across attempts (saturating);
/// * average memory is the duration-weighted mean of the attempts' averages,
///   weighted by each log's `start_time`/`end_time` span (an attempt still in
///   flight is weighted by the time elapsed since its start); if any
///   contributing log lacks timestamps, an unweighted mean is used instead.
///
/// Returns `None` if no log carries any usage.
fn fold_task_log_usage(logs: &[tes::v1::types::responses::TaskLog]) -> Option<TaskResourceUsage> {
    /// An attempt's average memory and its duration weight, in seconds.
    struct Average {
        /// The attempt's reported average memory, in bytes.
        avg: u64,
        /// The attempt's duration, in seconds, if its log carries timestamps.
        weight: Option<f64>,
    }

    let mut usage = TaskResourceUsage::default();
    let mut averages = Vec::new();
    let mut any = false;

    for log in logs {
        let Some(metadata) = log.metadata.as_ref() else {
            continue;
        };

        let parsed = parse_resource_usage_metadata(metadata);
        if parsed.is_empty() {
            continue;
        }

        any = true;

        if let Some(peak) = parsed.max_memory {
            usage.max_memory = Some(usage.max_memory.unwrap_or(0).max(peak));
        }

        if let Some(disk) = parsed.disk_used {
            usage.disk_used = Some(usage.disk_used.unwrap_or(0).max(disk));
        }

        for (total, value) in [
            (&mut usage.cpu_time_ms, parsed.cpu_time_ms),
            (&mut usage.user_cpu_time_ms, parsed.user_cpu_time_ms),
            (&mut usage.system_cpu_time_ms, parsed.system_cpu_time_ms),
        ] {
            if let Some(value) = value {
                *total = Some(total.unwrap_or(0).saturating_add(value));
            }
        }

        if let Some(avg) = parsed.avg_memory {
            let weight = log.start_time.map(|start| {
                let end = log.end_time.unwrap_or_else(chrono::Utc::now);
                (end - start).num_milliseconds().max(0) as f64 / 1_000.0
            });
            averages.push(Average { avg, weight });
        }
    }

    if !averages.is_empty() {
        // Weight by duration only when every contributing attempt has one
        let weighted = averages.iter().all(|a| a.weight.is_some());
        let mut sum = 0.0;
        let mut total_weight = 0.0;
        for a in &averages {
            let weight = if weighted {
                a.weight.expect("checked above").max(f64::EPSILON)
            } else {
                1.0
            };
            sum += a.avg as f64 * weight;
            total_weight += weight;
        }
        usage.avg_memory = Some((sum / total_weight) as u64);
    }

    if any { Some(usage) } else { None }
}

/// Represents a monitored task.
#[derive(Debug)]
struct Task {
    /// The name of the task.
    name: String,
    /// The TES id of the task.
    ///
    /// This is `None` until the task is created on the TES server.
    tes_id: Option<String>,
    /// The events sender for the task.
    events: Option<broadcast::Sender<Event>>,
    /// The sender for the "completed" notification.
    completed: oneshot::Sender<Result<()>>,
}

/// Represents state for the task monitor.
#[derive(Debug, Default)]
struct TaskMonitorState {
    /// The current tag to group TES tasks with.
    tag: String,
    /// The map of Crankshaft id to monitored task.
    tasks: HashMap<TaskId, Task>,
    /// The map of TES task id to Crankshaft task id
    ids: HashMap<String, TaskId>,
    /// Set of known running tasks
    running: HashSet<TaskId>,
    /// The last resource usage snapshot sent for each task, used to avoid
    /// resending identical snapshots on every poll.
    usage: HashMap<TaskId, TaskResourceUsage>,
}

/// Represents a TES task monitor.
///
/// The TES task monitor is responsible for polling the TES server for task
/// state at a set interval.
///
/// The monitor uses a current "tag" that is used to associate newly created TES
/// tasks with the monitor.
///
/// When the monitor queries for task state, it selects only the tasks with the
/// current tag.
///
/// The tag changes when the monitor is not monitoring any tasks and a task is
/// added for monitoring.
#[derive(Debug, Clone)]
pub struct TaskMonitor {
    /// The base name used by the task monitor for formatting tags.
    name: Arc<String>,
    /// The shared task monitor state.
    state: Arc<Mutex<TaskMonitorState>>,
    /// A channel to notify that the task manager was dropped.
    _drop: Arc<oneshot::Sender<()>>,
}

impl TaskMonitor {
    /// Constructs a new task monitor with the given name.
    ///
    /// The name is used for formatting the tag used to create new TES tasks.
    pub async fn new(name: String, backend_state: Arc<super::BackendState>) -> Self {
        let state: Arc<Mutex<TaskMonitorState>> = Default::default();
        let (tx, rx) = oneshot::channel();
        tokio::spawn(Self::monitor(state.clone(), backend_state, rx));
        Self {
            name: name.into(),
            state,
            _drop: tx.into(),
        }
    }

    /// Adds a task to the monitor.
    ///
    /// The given completed channel is sent `Ok(_)` when the task has been
    /// completed or `Err(_)` if there was an error monitoring the task.
    ///
    /// Returns the tag to use when creating the TES task.
    pub async fn add_task(
        &self,
        id: TaskId,
        name: String,
        events: Option<broadcast::Sender<Event>>,
        completed: oneshot::Sender<Result<()>>,
    ) -> String {
        let mut state = self.state.lock().expect("failed to lock TES monitor state");

        // If there are no monitored tasks, create a new tag
        if state.tasks.is_empty() {
            state.running.clear();
            state.tag = format!(
                "{name}-{timestamp}-{id}",
                name = self.name,
                timestamp = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            );
        }

        state.tasks.insert(
            id,
            Task {
                name,
                events,
                tes_id: None,
                completed,
            },
        );
        state.tag.clone()
    }

    /// Associates a TES task id with a Crankshaft task id.
    ///
    /// This is called after the TES task has been created.
    pub async fn associate_task_id(&self, id: TaskId, tes_id: String) {
        let mut state = self.state.lock().expect("failed to lock TES monitor state");
        if let Some(task) = state.tasks.get_mut(&id) {
            task.tes_id = Some(tes_id.clone());
            state.ids.insert(tes_id, id);
        }
    }

    /// Removes a task from the monitor.
    pub async fn remove_task(&self, id: u64) {
        let mut state = self.state.lock().expect("failed to lock TES monitor state");
        if let Some(task) = state.tasks.remove(&id)
            && let Some(tes_id) = task.tes_id
        {
            state.ids.remove(&tes_id);
        }

        state.running.remove(&id);
        state.usage.remove(&id);
    }

    /// Updates the tasks by querying the TES server for the current task state.
    ///
    /// Responsible for sending task started events and for sending completion
    /// messages.
    async fn update_tasks(
        state: &Arc<Mutex<TaskMonitorState>>,
        backend_state: &super::BackendState,
    ) {
        let mut page_token = None;
        'poll: loop {
            // Get the current tag from the state
            let tag = {
                let state = state.lock().expect("failed to TES lock monitor state");
                if state.tasks.is_empty() {
                    return;
                }

                assert!(!state.tag.is_empty(), "should have a current tag");

                debug!(
                    "querying for the state of TES tasks with tag `{tag}` and page token \
                     `{page_token:?}`",
                    tag = state.tag
                );

                state.tag.clone()
            };

            let list = async {
                let permit = backend_state
                    .permits
                    .acquire()
                    .await
                    .context("failed to acquire network request permit")?;

                let result = backend_state
                    .client
                    .list_tasks(
                        Some(&ListTasksParams {
                            tag_keys: Some(vec![CRANKSHAFT_GROUP_TAG_NAME.to_string()]),
                            tag_values: Some(vec![tag]),
                            page_size: Some(MAX_PAGE_SIZE - 1),
                            page_token,
                            view: Some(if backend_state.resource_usage_metadata {
                                // The `BASIC` view includes task logs, whose
                                // metadata may carry resource usage.
                                View::Basic
                            } else {
                                View::Minimal
                            }),
                            ..Default::default()
                        }),
                        backend_state.policy(),
                    )
                    .await
                    .context("failed to get task information from TES server");

                // Drop the permit now that the request has completed
                drop(permit);
                result
            };

            // Get the list of tasks
            match list.await {
                Ok(ListTasks {
                    tasks: tes_tasks,
                    next_page_token,
                }) => {
                    let mut state = state.lock().expect("failed to TES lock monitor state");

                    // For any task that is completed and in the map, notify of completion
                    for task in tes_tasks {
                        // Extract the identifier, state, and any reported
                        // resource usage from whichever view was requested.
                        let (task_id, task_state, usage) = match task {
                            tes::v1::types::responses::TaskResponse::Minimal(t) => {
                                (t.id, t.state, None)
                            }
                            t => {
                                let t = t.into_task().expect("task should be basic");

                                // A task response without an identifier
                                // cannot be attributed to any monitored task;
                                // if it were ignored, that task's terminal
                                // state would never be observed and its
                                // runner would wait forever. Fail the
                                // monitored tasks explicitly instead.
                                let Some(id) = t.id else {
                                    state.running.clear();
                                    state.ids.clear();
                                    state.usage.clear();
                                    for (_, task) in state.tasks.drain() {
                                        let _ = task.completed.send(Err(anyhow!(
                                            "TES server returned a task response without an id"
                                        )));
                                    }
                                    break 'poll;
                                };

                                let usage = t
                                    .logs
                                    .as_deref()
                                    .and_then(fold_task_log_usage)
                                    .filter(|usage| !usage.is_empty());
                                (id, t.state, usage)
                            }
                        };

                        // Report any resource usage the server included; each
                        // report is a cumulative snapshot and the last one
                        // received is authoritative. Only emit for tasks that
                        // are still monitored, and only when the snapshot
                        // changed since the last emission. The event is sent
                        // on the monitored task's own events channel.
                        if let Some(usage) = usage
                            && let Some(id) = state.ids.get(&task_id).copied()
                            && state.usage.get(&id) != Some(&usage)
                            && let Some(task) = state.tasks.get(&id)
                        {
                            let events = task.events.clone();
                            state.usage.insert(id, usage.clone());
                            send_event!(events, Event::TaskResourceUsage { id, usage });
                        }

                        let task = MonitoredTaskState {
                            id: task_id,
                            state: task_state,
                        };

                        match task.state.unwrap_or_default() {
                            TesState::Running | TesState::Paused => {
                                // The task is now running, send the started event
                                if let Some(id) = state.ids.get(&task.id).copied()
                                    && state.running.insert(id)
                                    && let Some(Task { name, events, .. }) = state.tasks.get(&id)
                                {
                                    info!(
                                        "TES task `{tes_id}` (task `{name}`) is now running",
                                        tes_id = task.id
                                    );

                                    send_event!(events, Event::TaskStarted { id });
                                }
                            }
                            TesState::Complete
                            | TesState::ExecutorError
                            | TesState::SystemError
                            | TesState::Canceled
                            | TesState::Preempted => {
                                // The task has completed, send the completion message
                                if let Some(id) = state.ids.remove(&task.id) {
                                    state.running.remove(&id);
                                    state.usage.remove(&id);
                                    if let Some(task) = state.tasks.remove(&id) {
                                        let _ = task.completed.send(Ok(()));
                                    }
                                }
                            }
                            _ => {}
                        }
                    }

                    if next_page_token
                        .as_ref()
                        .map(|t| t.is_empty())
                        .unwrap_or(true)
                    {
                        break;
                    }

                    page_token = next_page_token;
                }
                Err(e) => {
                    let mut state = state.lock().expect("failed to TES lock monitor state");

                    // Complete the current set of monitored tasks with an error
                    state.running.clear();
                    for (_, task) in state.tasks.drain() {
                        let _ = task
                            .completed
                            .send(Err(anyhow!("failed to monitor TES tasks: {e:#}")));
                    }
                    break;
                }
            }
        }
    }

    /// Performs the TES task monitoring.
    async fn monitor(
        state: Arc<Mutex<TaskMonitorState>>,
        backend_state: Arc<super::BackendState>,
        mut drop: oneshot::Receiver<()>,
    ) {
        info!(
            "TES task monitor is starting with polling interval of {interval} seconds",
            interval = backend_state.interval.as_secs()
        );

        // The timer for the querying TES task state
        let mut timer = tokio::time::interval(backend_state.interval);
        timer.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            select! {
                _ = &mut drop => break,
                _ = timer.tick() => Self::update_tasks(&state, backend_state.as_ref()).await,
            }
        }

        info!("TES task monitor has shut down");
    }
}

#[cfg(test)]
mod tests {
    use tes::v1::types::responses::TaskLog;

    use super::*;

    /// Builds a task log with the given metadata and optional start/end
    /// times (RFC 3339).
    fn task_log(metadata: serde_json::Value, start: Option<&str>, end: Option<&str>) -> TaskLog {
        TaskLog {
            logs: Vec::new(),
            metadata: Some(metadata),
            start_time: start.map(|s| s.parse().expect("valid timestamp")),
            end_time: end.map(|s| s.parse().expect("valid timestamp")),
            outputs: Vec::new(),
            system_logs: None,
        }
    }

    #[test]
    fn usage_folds_across_task_logs() {
        // Two attempts: the second (a server-side retry) reports lower
        // values, which must not regress the cumulative snapshot
        let logs = [
            task_log(
                serde_json::json!({
                    "peak_memory_bytes": "1000",
                    "avg_memory_bytes": "800",
                    "cpu_time_ms": "5000",
                    "disk_used_bytes": "300",
                }),
                // 30 second attempt
                Some("2026-08-26T00:00:00Z"),
                Some("2026-08-26T00:00:30Z"),
            ),
            task_log(
                serde_json::json!({
                    "peak_memory_bytes": "400",
                    "avg_memory_bytes": "200",
                    "cpu_time_ms": "1000",
                    "disk_used_bytes": "100",
                }),
                // 10 second attempt
                Some("2026-08-26T00:01:00Z"),
                Some("2026-08-26T00:01:10Z"),
            ),
        ];

        let usage = fold_task_log_usage(&logs).expect("should have usage");
        // Peaks take the maximum
        assert_eq!(usage.max_memory, Some(1000));
        assert_eq!(usage.disk_used, Some(300));
        // CPU sums across attempts
        assert_eq!(usage.cpu_time_ms, Some(6000));
        // Average memory is duration-weighted: (800*30 + 200*10) / 40 = 650
        assert_eq!(usage.avg_memory, Some(650));
    }

    #[test]
    fn averages_fall_back_to_unweighted_without_timestamps() {
        let logs = [
            task_log(
                serde_json::json!({ "avg_memory_bytes": "800" }),
                Some("2026-08-26T00:00:00Z"),
                Some("2026-08-26T00:00:30Z"),
            ),
            // No timestamps: the fold must not weight by duration
            task_log(serde_json::json!({ "avg_memory_bytes": "200" }), None, None),
        ];

        let usage = fold_task_log_usage(&logs).expect("should have usage");
        assert_eq!(usage.avg_memory, Some(500));
    }

    #[test]
    fn logs_without_usage_fold_to_none() {
        let logs = [task_log(serde_json::json!({}), None, None)];
        assert!(fold_task_log_usage(&logs).is_none());

        assert!(fold_task_log_usage(&[]).is_none());
    }

    #[test]
    fn metadata_parses_numbers_and_numeric_strings() {
        let metadata = serde_json::json!({
            "peak_memory_bytes": 1073741824u64,
            "avg_memory_bytes": "536870912",
            "cpu_time_ms": "12500 ",
            "user_cpu_time_ms": 12000,
            "system_cpu_time_ms": 500,
            "disk_used_bytes": "2147483648",
            "some_other_key": "ignored",
        });

        let usage = parse_resource_usage_metadata(&metadata);
        assert_eq!(usage.max_memory, Some(1073741824));
        assert_eq!(usage.avg_memory, Some(536870912));
        assert_eq!(usage.cpu_time_ms, Some(12500));
        assert_eq!(usage.user_cpu_time_ms, Some(12000));
        assert_eq!(usage.system_cpu_time_ms, Some(500));
        assert_eq!(usage.disk_used, Some(2147483648));
        assert!(!usage.is_empty());
    }

    #[test]
    fn unparseable_and_missing_metadata_is_ignored() {
        let metadata = serde_json::json!({
            "peak_memory_bytes": "not a number",
            "cpu_time_ms": true,
        });

        let usage = parse_resource_usage_metadata(&metadata);
        assert!(usage.is_empty());

        let usage = parse_resource_usage_metadata(&serde_json::json!("free-form string"));
        assert!(usage.is_empty());

        let usage = parse_resource_usage_metadata(&serde_json::json!({}));
        assert!(usage.is_empty());
    }
}
