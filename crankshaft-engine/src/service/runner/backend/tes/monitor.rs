//! Implements the TES task monitor.

use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
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
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::time::MissedTickBehavior;
use tracing::debug;
use tracing::info;

/// The maximum number of requests the monitor channel will buffer before
/// blocking.
const MONITOR_CAPACITY: usize = 100;

/// The name of the tag used to group tasks together for monitoring.
pub const CRANKSHAFT_GROUP_TAG_NAME: &str = "crankshaft-task-group";

/// Represents an "add task" request.
#[derive(Debug)]
struct AddTaskRequest {
    /// The Crankshaft task id.
    id: u64,
    /// The Crankshaft name of the task.
    name: String,
    /// The sender for notifying the completion of the task.
    completed: oneshot::Sender<Result<()>>,
    /// The sender for the response from the monitor.
    response: oneshot::Sender<AddTaskResponse>,
}

/// Represents the response for an "add task" request.
#[derive(Debug)]
struct AddTaskResponse {
    /// The tag the TES task should be created with.
    tag: String,
}

/// Represents an "associate task id" request.
///
/// This is used to associate a TES task id with a Crankshaft task id.
#[derive(Debug)]
struct AssociateTaskIdRequest {
    /// The Crankshaft task id.
    id: u64,
    /// The TES task id.
    tes_id: String,
}

/// Represents a "remove task" request.
#[derive(Debug)]
struct RemoveTaskRequest {
    /// The TES task id to remove.
    tes_id: String,
}

/// A request to the task monitor.
#[derive(Debug)]
enum MonitorRequest {
    /// Add a new task to the monitor.
    AddTask(AddTaskRequest),
    /// Associate a TES task id with a Crankshaft task id.
    AssociateTaskId(AssociateTaskIdRequest),
    /// Removes a task from the monitor.
    RemoveTask(RemoveTaskRequest),
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
pub struct TaskMonitor(mpsc::Sender<MonitorRequest>);

impl TaskMonitor {
    /// Constructs a new task monitor with the given name.
    ///
    /// The name is used for formatting the tag used to create new TES tasks.
    pub async fn new(name: String, state: Arc<super::State>) -> Self {
        let (tx, rx) = mpsc::channel(MONITOR_CAPACITY);
        tokio::spawn(Self::monitor(name, state, rx));
        Self(tx)
    }

    /// Adds a task to the monitor.
    ///
    /// The given completed channel is sent `Ok(_)` when the task has been
    /// completed or `Err(_)` if there was an error monitoring the task.
    ///
    /// Returns the tag to use when creating the TES task or an error if
    pub async fn add_task(
        &self,
        id: u64,
        name: String,
        completed: oneshot::Sender<Result<()>>,
    ) -> String {
        let (tx, rx) = oneshot::channel();
        self.0
            .send(MonitorRequest::AddTask(AddTaskRequest {
                id,
                name,
                completed,
                response: tx,
            }))
            .await
            .expect("failed to send request");
        rx.await.map(|r| r.tag).expect("failed to receive response")
    }

    /// Associates a TES task id with a Crankshaft task id.
    ///
    /// This is called after the TES task has been created.
    pub async fn associate_task_id(&self, id: u64, tes_id: String) {
        self.0
            .send(MonitorRequest::AssociateTaskId(AssociateTaskIdRequest {
                id,
                tes_id,
            }))
            .await
            .expect("failed to send request");
    }

    /// Removes a task from the monitor.
    pub async fn remove_task(&self, tes_id: String) {
        self.0
            .send(MonitorRequest::RemoveTask(RemoveTaskRequest { tes_id }))
            .await
            .expect("failed to send request");
    }

    /// Performs the TES task monitoring.
    async fn monitor(
        monitor_name: String,
        state: Arc<super::State>,
        mut rx: mpsc::Receiver<MonitorRequest>,
    ) {
        info!(
            "TES task monitor is starting with polling interval of {interval} seconds",
            interval = state.interval.as_secs()
        );

        /// Represents a monitored task.
        struct Task {
            /// The name of the task.
            name: String,
            /// The sender for the "completed" notification.
            completed: oneshot::Sender<Result<()>>,
        }

        // The map of Crankshaft id to monitored task
        let mut tasks = HashMap::new();
        // The map of TES task id to Crankshaft task id
        let mut ids = HashMap::new();
        // Set of known running tasks
        let mut running = HashSet::new();
        // The current tag
        let mut tag = String::new();

        // The timer for the querying TES task state
        let mut timer = tokio::time::interval(state.interval);
        timer.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            select! {
                msg = rx.recv() => match msg {
                    Some(request) => match request {
                        MonitorRequest::AddTask(req) => {
                            // If there are no monitored tasks, create a new tag
                            if tasks.is_empty() {
                                running.clear();
                                tag = format!(
                                    "{monitor_name}-{timestamp}-{id}",
                                    timestamp = SystemTime::now()
                                        .duration_since(SystemTime::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_secs(),
                                    id = req.id,
                                );
                            }

                            tasks.insert(
                                req.id,
                                Task {
                                    name: req.name,
                                    completed: req.completed,
                                },
                            );
                            req.response.send(AddTaskResponse { tag: tag.clone() }).expect("failed to send add task response");
                        }
                        MonitorRequest::AssociateTaskId(req) => {
                            ids.insert(req.tes_id, req.id);
                        }
                        MonitorRequest::RemoveTask(req) => {
                            if let Some(id) = ids.get(&req.tes_id) {
                                tasks.remove(id);
                                running.remove(id);
                            }
                        }
                    },
                    None => break,
                },
                _ = timer.tick() => {
                    // Don't do anything if there are no tasks being monitored
                    if tasks.is_empty() {
                        continue;
                    }

                    assert!(!tag.is_empty(), "should have a current tag");
                    let mut page_token = None;
                    loop {
                        debug!(
                            "querying for the state of TES tasks with tag `{tag}` and page token `{page_token:?}`"
                        );
                        let list = async {
                            let _permit = state
                                .permits
                                .acquire()
                                .await
                                .context("failed to acquire network request permit")?;

                            state
                                .client
                                .list_tasks(
                                    Some(&ListTasksParams {
                                        tag_keys: Some(vec![CRANKSHAFT_GROUP_TAG_NAME.to_string()]),
                                        tag_values: Some(vec![tag.clone()]),
                                        page_size: Some(MAX_PAGE_SIZE - 1),
                                        page_token,
                                        view: Some(View::Minimal),
                                        ..Default::default()
                                    }),
                                    state.policy(),
                                )
                                .await
                                .context("failed to get task information from TES server")
                        };

                        // Get the list of tasks
                        let result = list.await;

                        match result {
                            Ok(ListTasks {
                                tasks: tes_tasks,
                                next_page_token,
                            }) => {
                                // For any task that is completed and in the map, notify of completion
                                for task in tes_tasks
                                    .into_iter()
                                    .map(|t| t.into_minimal().expect("task should be minimal"))
                                {
                                    match task.state.unwrap_or_default() {
                                        TesState::Running | TesState::Paused => {
                                            // The task is now running, send the started event
                                            if let Some(id) = ids.get(&task.id) {
                                                if let Some(Task { name, .. }) = tasks.get(id) {
                                                    if running.insert(*id) {
                                                        info!(
                                                            "TES task `{tes_id}` (task `{name}`) is now running",
                                                            tes_id = task.id
                                                        );
                                                        send_event!(state.events, Event::TaskStarted { id: *id });
                                                    }
                                                }
                                            }
                                        }
                                        TesState::Complete
                                        | TesState::ExecutorError
                                        | TesState::SystemError
                                        | TesState::Canceled
                                        | TesState::Preempted => {
                                            // The task has completed, send the completion message
                                            if let Some(id) = ids.remove(&task.id) {
                                                running.remove(&id);
                                                if let Some(task) = tasks.remove(&id) {
                                                    let _ = task.completed.send(Ok(()));
                                                }
                                            }
                                        }
                                        _ => {}
                                    }
                                }

                                if next_page_token.is_none() {
                                    break;
                                }

                                page_token = next_page_token;
                            }
                            Err(e) => {
                                // Complete the current set of monitored tasks with an error
                                running.clear();
                                for (_, task) in tasks.drain() {
                                    let _ = task
                                        .completed
                                        .send(Err(anyhow!("failed to monitor TES tasks: {e:#}")));
                                }
                                break;
                            }
                        }
                    }
                }
            }
        }

        info!("TES task monitor has shut down");
    }
}
