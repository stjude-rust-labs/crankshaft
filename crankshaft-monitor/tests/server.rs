//! Tests for the gRPC server are written here
use std::net::SocketAddr;

use crankshaft_monitor::proto::Event;
use crankshaft_monitor::proto::EventType;
use crankshaft_monitor::proto::SubscribeEventsRequest;
use crankshaft_monitor::proto::monitor_server::Monitor;
use crankshaft_monitor::server::MonitorService;
use crankshaft_monitor::start_monitoring;
use futures_util::StreamExt;
use tokio::sync::broadcast;
use tokio::time::Duration;
use tokio::time::timeout;
use tonic::Request;

#[tokio::test]
async fn test_subscribe_events_streams_all_task_events() {
    // Set up a broadcast channel
    let (tx, rx) = broadcast::channel::<Event>(16);
    // create a new server instance
    let service = MonitorService::new(rx);

    // Create test events for different tasks
    let event1 = Event {
        task_id: "t1".to_string(),
        event_type: EventType::TaskStarted as i32,
        timestamp: 1625234567,
        message: "Task t1 started".to_string(),
    };
    let event2 = Event {
        task_id: "t2".to_string(),
        event_type: EventType::TaskCompleted as i32,
        timestamp: 1625234568,
        message: "Task t2 completed".to_string(),
    };
    // wrap it in Request
    let request = Request::new(SubscribeEventsRequest {});

    // we need to have a subscriber to even send a message in broadcast channel
    let response = service
        .subscribe_events(request)
        .await
        .expect("Stream failed");
    let mut stream = response.into_inner();

    // Send events after subscription
    tx.send(event1.clone()).expect("Failed to send event1");
    tx.send(event2.clone()).expect("Failed to send event2");

    // Check the first event
    let received1 = timeout(Duration::from_secs(1), stream.next())
        .await
        .expect("Timeout waiting for event")
        .expect("No event received");

    let received_event1 = received1.expect("Error in stream");
    assert_eq!(received_event1.task_id, "t1");
    assert_eq!(received_event1.event_type, EventType::TaskStarted as i32);
    assert_eq!(received_event1.message, "Task t1 started");

    // Check the second event
    let received2 = timeout(Duration::from_secs(1), stream.next())
        .await
        .expect("Timeout waiting for event")
        .expect("No event received");

    let received_event2 = received2.expect("Error in stream");
    assert_eq!(received_event2.task_id, "t2");
    assert_eq!(received_event2.event_type, EventType::TaskCompleted as i32);
    assert_eq!(received_event2.message, "Task t2 completed");
}

#[tokio::test]
async fn test_start_server_and_subscribe_events() {
    // Arrange: Set up the broadcast channel and start the server
    let addr = "127.0.0.1:8080"
        .parse::<SocketAddr>()
        .expect("Invalid address");

    let (sender, server_handle) = start_monitoring(addr).expect("Failed to start server");

    // we have to introduce a little delay to allow the server to start otherwise
    // the connection is refused
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create a gRPC transport channel to connect to the server
    let channel = tonic::transport::Channel::from_shared(format!("http://{}", addr))
        .expect("Invalid URI")
        .connect()
        .await
        .expect("Failed to connect");

    let mut client = crankshaft_monitor::proto::monitor_client::MonitorClient::new(channel);

    //  request to subscribe to all events
    let request = Request::new(SubscribeEventsRequest {});
    let mut stream = client
        .subscribe_events(request)
        .await
        .expect("Failed to start stream")
        .into_inner();

    let event1 = Event {
        task_id: "t1".to_string(),
        event_type: EventType::TaskStarted as i32,
        timestamp: 1625234567,
        message: "Task t1 started".to_string(),
    };
    let event2 = Event {
        task_id: "t2".to_string(),
        event_type: EventType::TaskFailed as i32,
        timestamp: 1625234568,
        message: "Task t2 failed".to_string(),
    };
    sender.send(event1.clone()).expect("Failed to send event1");
    sender.send(event2.clone()).expect("Failed to send event2");

    let received1 = timeout(Duration::from_secs(1), stream.next())
        .await
        .expect("Timeout waiting for event")
        .expect("No event received");

    let received_event1 = received1.expect("Error in stream");
    assert_eq!(received_event1.task_id, "t1");
    assert_eq!(received_event1.event_type, EventType::TaskStarted as i32);
    assert_eq!(received_event1.message, "Task t1 started");

    let received2 = timeout(Duration::from_secs(1), stream.next())
        .await
        .expect("Timeout waiting for event")
        .expect("No event received");

    let received_event2 = received2.expect("Error in stream");
    assert_eq!(received_event2.task_id, "t2");
    assert_eq!(received_event2.event_type, EventType::TaskFailed as i32);
    assert_eq!(received_event2.message, "Task t2 failed");

    server_handle.abort();
}
