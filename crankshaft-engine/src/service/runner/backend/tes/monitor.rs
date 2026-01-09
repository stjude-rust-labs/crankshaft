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
use crankshaft_events::send_event;
use tes::v1::types::requests::ListTasksParams;
use tes::v1::types::requests::MAX_PAGE_SIZE;
use tes::v1::types::requests::View;
use tes::v1::types::responses::ListTasks;
use tes::v1::types::task::State as TesState;
use tokio::select;
use tokio::sync::oneshot;
use tokio::time::MissedTickBehavior;
use tracing::debug;
use tracing::info;

/// The name of the tag used to group tasks together for monitoring.
pub const CRANKSHAFT_GROUP_TAG_NAME: &str = "crankshaft-task-group";

/// Represents a monitored task.
#[derive(Debug)]
struct Task {
    /// The name of the task.
    name: String,
    /// The sender for the "completed" notification.
    completed: oneshot::Sender<Result<()>>,
}

/// Represents state for the task monitor.
#[derive(Debug, Default)]
struct TaskMonitorState {
    /// The current tag to group TES tasks with.
    tag: String,
    /// The map of Crankshaft id to monitored task.
    tasks: HashMap<u64, Task>,
    /// The map of TES task id to Crankshaft task id
    ids: HashMap<String, u64>,
    /// Set of known running tasks
    running: HashSet<u64>,
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
        id: u64,
        name: String,
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

        state.tasks.insert(id, Task { name, completed });

        state.tag.clone()
    }

    /// Associates a TES task id with a Crankshaft task id.
    ///
    /// This is called after the TES task has been created.
    pub async fn associate_task_id(&self, id: u64, tes_id: String) {
        let mut state = self.state.lock().expect("failed to lock TES monitor state");
        state.ids.insert(tes_id, id);
    }

    /// Removes a task from the monitor.
    pub async fn remove_task(&self, tes_id: &str) {
        let mut state = self.state.lock().expect("failed to lock TES monitor state");
        if let Some(id) = state.ids.get(tes_id).copied() {
            state.tasks.remove(&id);
            state.running.remove(&id);
        }
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
        loop {
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
                            view: Some(View::Minimal),
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
                    for task in tes_tasks
                        .into_iter()
                        .map(|t| t.into_minimal().expect("task should be minimal"))
                    {
                        match task.state.unwrap_or_default() {
                            TesState::Running | TesState::Paused => {
                                // The task is now running, send the started event
                                if let Some(id) = state.ids.get(&task.id).copied()
                                    && state.running.insert(id)
                                {
                                    if let Some(Task { name, .. }) = state.tasks.get(&id) {
                                        info!(
                                            "TES task `{tes_id}` (task `{name}`) is now running",
                                            tes_id = task.id
                                        );
                                    }

                                    send_event!(backend_state.events, Event::TaskStarted { id });
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
