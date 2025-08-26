//! Tests for the gRPC server are written here
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt as _;
#[cfg(windows)]
use std::os::windows::process::ExitStatusExt as _;
use std::process::ExitStatus;

use crankshaft_events::Event as CrankshaftEvent;
use crankshaft_monitor::Monitor;
use crankshaft_monitor::proto::ServiceStateRequest;
use crankshaft_monitor::proto::SubscribeEventsRequest;
use crankshaft_monitor::proto::event::EventKind;
use crankshaft_monitor::proto::exit_status::ExitStatusKind;
use crankshaft_monitor::proto::monitor_client::MonitorClient;
use futures_util::StreamExt;
use nonempty::NonEmpty;
use tokio::sync::broadcast;
use tokio_retry2::Retry;
use tokio_retry2::RetryError;
use tokio_retry2::strategy::ExponentialFactorBackoff;
use tokio_retry2::strategy::MaxInterval;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn test_subscribe_events() {
    let (tx, _) = broadcast::channel(16);

    let monitor = Monitor::start("127.0.0.1:32000".parse().unwrap(), tx.clone());

    // Perform a retry with backoff for connecting as the monitor starts
    // asynchronously
    let strategy = ExponentialFactorBackoff::from_millis(50, 2.0)
        .max_interval(1000)
        .take(10);

    let mut client = Retry::spawn(strategy, || async {
        MonitorClient::connect("http://127.0.0.1:32000")
            .await
            .map_err(RetryError::transient)
    })
    .await
    .expect("failed to connect to monitor");

    let mut events = client
        .subscribe_events(SubscribeEventsRequest {})
        .await
        .expect("failed to subscribe to events")
        .into_inner();

    // Send some dummy events at the start
    tx.send(CrankshaftEvent::TaskCreated {
        id: 0,
        name: "first".into(),
        tes_id: None,
        token: CancellationToken::new(),
    })
    .unwrap();
    tx.send(CrankshaftEvent::TaskStarted { id: 0 }).unwrap();
    tx.send(CrankshaftEvent::TaskCompleted {
        id: 0,
        exit_statuses: NonEmpty::new(ExitStatus::from_raw(0)),
    })
    .unwrap();
    tx.send(CrankshaftEvent::TaskCreated {
        id: 1,
        name: "second".into(),
        tes_id: Some("tes".into()),
        token: CancellationToken::new(),
    })
    .unwrap();
    tx.send(CrankshaftEvent::TaskStarted { id: 1 }).unwrap();

    // Read the events back from the client
    let event = events
        .next()
        .await
        .expect("failed to read event")
        .expect("failed to read event");
    match event.event_kind {
        Some(EventKind::Created(event)) => {
            assert_eq!(event.id, 0);
            assert_eq!(event.name, "first");
            assert!(event.tes_id.is_none());
        }
        _ => panic!("unexpected event"),
    }

    let event = events
        .next()
        .await
        .expect("failed to read event")
        .expect("failed to read event");
    match event.event_kind {
        Some(EventKind::Started(event)) => {
            assert_eq!(event.id, 0);
        }
        _ => panic!("unexpected event"),
    }

    let event = events
        .next()
        .await
        .expect("failed to read event")
        .expect("failed to read event");
    match event.event_kind {
        Some(EventKind::Completed(event)) => {
            assert_eq!(event.id, 0);
            assert_eq!(event.exit_statuses.len(), 1);
            assert_eq!(
                event.exit_statuses[0].exit_status_kind,
                Some(ExitStatusKind::Code(0))
            );
        }
        _ => panic!("unexpected event"),
    }

    let event = events
        .next()
        .await
        .expect("failed to read event")
        .expect("failed to read event");
    match event.event_kind {
        Some(EventKind::Created(event)) => {
            assert_eq!(event.id, 1);
            assert_eq!(event.name, "second");
            assert_eq!(event.tes_id.as_deref(), Some("tes"));
        }
        _ => panic!("unexpected event"),
    }

    let event = events
        .next()
        .await
        .expect("failed to read event")
        .expect("failed to read event");
    match event.event_kind {
        Some(EventKind::Started(event)) => {
            assert_eq!(event.id, 1);
        }
        _ => panic!("unexpected event"),
    }

    drop(tx);
    monitor.stop().await;
    assert!(events.next().await.is_none());
}

#[tokio::test]
async fn test_service_state() {
    let (tx, _) = broadcast::channel(16);

    let monitor = Monitor::start("127.0.0.1:32001".parse().unwrap(), tx.clone());

    // Send some dummy events before the server starts
    // These events will be persisted in the service state
    tx.send(CrankshaftEvent::TaskCreated {
        id: 0,
        name: "first".into(),
        tes_id: None,
        token: CancellationToken::new(),
    })
    .unwrap();
    tx.send(CrankshaftEvent::TaskStarted { id: 0 }).unwrap();
    tx.send(CrankshaftEvent::TaskCompleted {
        id: 0,
        exit_statuses: NonEmpty::new(ExitStatus::from_raw(0)),
    })
    .unwrap();
    tx.send(CrankshaftEvent::TaskCreated {
        id: 1,
        name: "second".into(),
        tes_id: Some("tes".into()),
        token: CancellationToken::new(),
    })
    .unwrap();
    tx.send(CrankshaftEvent::TaskStarted { id: 1 }).unwrap();

    // Perform a retry with backoff for connecting as the monitor starts
    // asynchronously
    let strategy = ExponentialFactorBackoff::from_millis(50, 2.0)
        .max_interval(1000)
        .take(10);

    let mut client = Retry::spawn(strategy, || async {
        MonitorClient::connect("http://127.0.0.1:32001")
            .await
            .map_err(RetryError::transient)
    })
    .await
    .expect("failed to connect to monitor");

    let state = client
        .get_service_state(ServiceStateRequest {})
        .await
        .expect("failed to get service state")
        .into_inner();

    // There should be one task in the state because the first task finished
    assert_eq!(state.tasks.len(), 1);

    let task = state.tasks.get(&1).expect("should have a task with id 1");
    assert_eq!(task.events.len(), 2);

    match &task.events[0].event_kind {
        Some(EventKind::Created(event)) => {
            assert_eq!(event.id, 1);
            assert_eq!(event.name, "second");
            assert_eq!(event.tes_id.as_deref(), Some("tes"));
        }
        _ => panic!("unexpected event"),
    }

    match &task.events[1].event_kind {
        Some(EventKind::Started(event)) => {
            assert_eq!(event.id, 1);
        }
        _ => panic!("unexpected event"),
    }

    drop(tx);
    monitor.stop().await;
}
