//! Definition of the events broadcast by Crankshaft.

use std::process::ExitStatus;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use bytes::Bytes;
use nonempty::NonEmpty;
use tokio_util::sync::CancellationToken;

/// Gets the next task id.
pub fn next_task_id() -> u64 {
    static NEXT_TASK_ID: AtomicU64 = AtomicU64::new(0);
    NEXT_TASK_ID.fetch_add(1, Ordering::SeqCst)
}

/// An event sent by task execution backends.
#[derive(Debug, Clone)]
pub enum Event {
    /// A task has been created.
    ///
    /// Note: a task is not "running" until the started event.
    ///
    /// This event is always paired with a `TaskCompleted`, `TaskFailed`,
    /// `TaskCanceled`, or `TaskPreempted` event.
    TaskCreated {
        /// The id of the task.
        id: u64,
        /// The name of the task.
        ///
        /// This may be a display name provided by the user or a name provided
        /// by the backend if the user did not provide a name for the task.
        name: String,
        /// The TES identifier of the task.
        ///
        /// This is `Some` only for the TES backend.
        tes_id: Option<String>,

        /// The cancellation token provided by the backend
        token: CancellationToken,
    },
    /// A task has started execution.
    ///
    /// A task is considered "running" upon the receipt of this event.
    TaskStarted {
        /// The id of the task.
        id: u64,
    },
    /// A container has been created for a task.
    ///
    /// This event is only sent by the Docker backend.
    TaskContainerCreated {
        /// The id of the task.
        id: u64,
        /// The name of the container that was created.
        container: String,
    },
    /// A container has exited for a task.
    ///
    /// This event is only sent by the Docker backend.
    TaskContainerExited {
        /// The id of the task.
        id: u64,
        /// The name of the container that has exited.
        container: String,
        /// The exit status of the container.
        exit_status: ExitStatus,
    },
    /// A task has completed.
    ///
    /// This event occurs after all task executions have completed successfully.
    TaskCompleted {
        /// The id of the task.
        id: u64,
        /// The exit statuses for the task's executions.
        exit_statuses: NonEmpty<ExitStatus>,
    },
    /// A task has failed.
    ///
    /// This event occurs after any error encountered running a task.
    TaskFailed {
        /// The id of the task.
        id: u64,
        /// The error message.
        message: String,
    },
    /// A task has been canceled.
    TaskCanceled {
        /// The id of the task.
        id: u64,
    },
    /// The task was preempted.
    TaskPreempted {
        /// The id of the task.
        id: u64,
    },
    /// A task has logged stdout.
    ///
    /// Note: only locally executing tasks will send this event.
    TaskStdout {
        /// The id of the task.
        id: u64,
        /// The bytes logged to stdout.
        message: Bytes,
    },
    /// A task has logged stderr.
    ///
    /// Note: only locally executing tasks will send this event.
    TaskStderr {
        /// The id of the task.
        id: u64,
        /// The bytes logged to stdout.
        message: Bytes,
    },
}

/// Sends an event through a broadcast channel.
///
/// If the sender is `None`, the event expression is not evaluated and no event
/// is sent.
#[macro_export]
macro_rules! send_event {
    ($sender:expr, $event:expr $(,)?) => {
        if let Some(sender) = &$sender {
            sender.send($event).ok();
        }
    };
}
