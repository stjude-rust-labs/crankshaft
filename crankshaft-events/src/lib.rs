//! Definition of the events broadcast by Crankshaft.

use std::process::ExitStatus;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use bytes::Bytes;
use nonempty::NonEmpty;
use tokio_util::sync::CancellationToken;

/// Represents a Crankshaft task identifier.
pub type TaskId = u64;

/// Gets the next task id.
pub fn next_task_id() -> TaskId {
    static NEXT_TASK_ID: AtomicU64 = AtomicU64::new(0);
    NEXT_TASK_ID.fetch_add(1, Ordering::SeqCst)
}

/// The resource utilization observed for a task.
///
/// Every field is optional: backends report the subset of measurements their
/// execution environment provides.
///
/// A backend reports utilization as a cumulative snapshot: each emission
/// describes the task's utilization from its start up to the moment of
/// observation, so the last snapshot received for a task is authoritative.
/// Backends that can only observe utilization once (e.g. from a scheduler's
/// accounting of a finished job) emit a single snapshot at the task's
/// termination.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub struct TaskResourceUsage {
    /// The maximum resident memory observed, in bytes.
    pub max_memory: Option<u64>,
    /// The average resident memory observed, in bytes.
    ///
    /// The averaging method is producer-defined and averages are therefore
    /// not comparable across backends: sampling backends typically report an
    /// arithmetic mean over polling samples (which is not time-weighted and
    /// may be skewed by missed or delayed ticks), while backends that forward
    /// externally reported values (e.g. a TES server's task log metadata)
    /// inherit that source's weighting.
    pub avg_memory: Option<u64>,
    /// The total CPU time consumed, in milliseconds.
    pub cpu_time_ms: Option<u64>,
    /// The user-mode CPU time consumed, in milliseconds.
    pub user_cpu_time_ms: Option<u64>,
    /// The system-mode CPU time consumed, in milliseconds.
    pub system_cpu_time_ms: Option<u64>,
    /// The disk space used, in bytes.
    pub disk_used: Option<u64>,
}

impl TaskResourceUsage {
    /// Returns whether the snapshot contains no measurements at all.
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

/// An event sent by task execution backends.
#[derive(Debug, Clone)]
pub enum Event {
    /// A task has been created.
    ///
    /// Note: a task is not "running" until the [`Event::TaskStarted`] event.
    ///
    /// This event is always paired with a `TaskCompleted`, `TaskFailed`,
    /// `TaskCanceled`, or `TaskPreempted` event.
    TaskCreated {
        /// The id of the task.
        id: TaskId,
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
        id: TaskId,
    },
    /// A container has been created for a task.
    ///
    /// This event is only sent by the Docker backend.
    TaskContainerCreated {
        /// The id of the task.
        id: TaskId,
        /// The name of the container that was created.
        container: String,
    },
    /// A container has exited for a task.
    ///
    /// This event is only sent by the Docker backend.
    TaskContainerExited {
        /// The id of the task.
        id: TaskId,
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
        id: TaskId,
        /// The exit statuses for the task's executions.
        exit_statuses: NonEmpty<ExitStatus>,
    },
    /// A task has failed.
    ///
    /// This event occurs after any error encountered running a task.
    TaskFailed {
        /// The id of the task.
        id: TaskId,
        /// The error message.
        message: String,
    },
    /// A task has been canceled.
    TaskCanceled {
        /// The id of the task.
        id: TaskId,
    },
    /// The task was preempted.
    TaskPreempted {
        /// The id of the task.
        id: TaskId,
    },
    /// The resource utilization observed for a task.
    ///
    /// Backends that can observe resource utilization emit this event zero or
    /// more times over the task's lifetime; each emission is a cumulative
    /// snapshot and the last one received is authoritative (see
    /// [`TaskResourceUsage`]). It may be emitted for any terminal outcome —
    /// completed, failed, canceled, or preempted alike. Backends that cannot
    /// observe utilization never emit this event.
    TaskResourceUsage {
        /// The id of the task.
        id: TaskId,
        /// The observed resource utilization.
        usage: TaskResourceUsage,
    },
    /// A task has logged stdout.
    ///
    /// Note: only locally executing tasks will send this event.
    TaskStdout {
        /// The id of the task.
        id: TaskId,
        /// The bytes logged to stdout.
        message: Bytes,
    },
    /// A task has logged stderr.
    ///
    /// Note: only locally executing tasks will send this event.
    TaskStderr {
        /// The id of the task.
        id: TaskId,
        /// The bytes logged to stdout.
        message: Bytes,
    },
    /// A container image pull was started.
    ///
    /// ## Implementation Notes
    ///
    /// * This event indicates that an actual fetch process is initiated.
    ///   Backends **should not** emit this if the image is already present.
    /// * Backends *may* emit this event multiple times for the same image if
    ///   multiple executions request it.
    ///
    /// This event is always paired with either an [`Event::ImagePullFinished`]
    /// or [`Event::ImagePullFailed`] event.
    ImagePullStarted {
        /// The id of the task that triggered the pull.
        id: TaskId,
        /// The name of the image being pulled.
        name: String,
    },
    /// Failed to pull a container image.
    ///
    /// Note: This indicates the termination of an image pull. It **will not**
    /// be paired with an [`Event::ImagePullFinished`] event.
    ImagePullFailed {
        /// The id of the task that triggered the pull.
        id: TaskId,
        /// The name of the image that failed.
        name: String,
        /// The error message.
        message: String,
    },
    /// A container image was successfully pulled.
    ImagePullFinished {
        /// The id of the task that triggered the pull.
        id: TaskId,
        /// The name of the image that was pulled.
        name: String,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_usage_snapshots_are_detected() {
        assert!(TaskResourceUsage::default().is_empty());

        let usage = TaskResourceUsage {
            max_memory: Some(1024),
            ..Default::default()
        };
        assert!(!usage.is_empty());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn usage_snapshots_round_trip_through_serde() {
        let usage = TaskResourceUsage {
            max_memory: Some(241_172_480),
            avg_memory: Some(100_000_000),
            cpu_time_ms: Some(324_000),
            ..Default::default()
        };

        let json = serde_json::to_string(&usage).expect("usage should serialize");
        let parsed: TaskResourceUsage =
            serde_json::from_str(&json).expect("usage should deserialize");
        assert_eq!(parsed, usage);
    }
}
