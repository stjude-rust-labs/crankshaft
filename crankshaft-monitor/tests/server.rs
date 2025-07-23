//! Tests for the gRPC server are written here
use std::net::SocketAddr;
use std::sync::Arc;

use crankshaft_monitor::proto::Event;
use crankshaft_monitor::proto::EventType;
use crankshaft_monitor::proto::GetServerStateRequest;
use crankshaft_monitor::proto::Resources;
use crankshaft_monitor::proto::SubscribeEventsRequest;
use crankshaft_monitor::proto::event::Payload::Message;
use crankshaft_monitor::proto::event::Payload::Resources as ResourcesPayload;
use crankshaft_monitor::proto::monitor_client::MonitorClient;
use crankshaft_monitor::proto::monitor_server::Monitor;
use crankshaft_monitor::server::MonitorService;
use crankshaft_monitor::server::ServerState;
use crankshaft_monitor::start_monitoring;
use futures_util::StreamExt;
use tokio::sync::RwLock;
use tokio::sync::broadcast;
use tokio::time::Duration;
use tokio::time::timeout;
use tonic::Request;

#[tokio::test]
async fn test_subscribe_events_streams_all_task_events() {
    // Set up a broadcast channel
    let (tx, rx) = broadcast::channel::<Event>(16);
    let state = Arc::new(RwLock::new(ServerState::default()));
    // create a new server instance
    let service = MonitorService::new(rx, state.clone());

    // Create test events for different tasks
    let event1 = Event {
        event_id: "t1".to_string(),
        event_type: EventType::TaskStarted as i32,
        timestamp: 1625234567,
        payload: Some(Message("Task t1 started".to_string())),
    };
    let event2 = Event {
        event_id: "t2".to_string(),
        event_type: EventType::TaskCompleted as i32,
        timestamp: 1625234568,
        payload: Some(Message("Task t2 completed".to_string())),
    };
    let event3 = Event {
        event_id: "resource_update".to_string(),
        event_type: EventType::ContainerStarted as i32,
        timestamp: 1625234569,
        payload: Some(ResourcesPayload(Resources {
            nodes: 2.0,
            cpu: 75.0,
            memory: 2048.0,
            max_cpu: 150.0,
            max_memory: 4096.0,
        })),
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
    tx.send(event3.clone()).expect("Failed to send event3");

    // Check the first event
    let received1 = timeout(Duration::from_secs(1), stream.next())
        .await
        .expect("Timeout waiting for event")
        .expect("No event received");

    let received_event1 = received1.expect("Error in stream");
    assert_eq!(received_event1.event_id, "t1");
    assert_eq!(received_event1.event_type, EventType::TaskStarted as i32);
    assert_eq!(
        received_event1.payload,
        Some(Message("Task t1 started".to_string()))
    );

    // Check the second event
    let received2 = timeout(Duration::from_secs(1), stream.next())
        .await
        .expect("Timeout waiting for event")
        .expect("No event received");

    let received_event2 = received2.expect("Error in stream");
    assert_eq!(received_event2.event_id, "t2");
    assert_eq!(received_event2.event_type, EventType::TaskCompleted as i32);
    assert_eq!(
        received_event2.payload,
        Some(Message("Task t2 completed".to_string()))
    );

    // Check the third event
    let received3 = timeout(Duration::from_secs(1), stream.next())
        .await
        .expect("Timeout waiting for event")
        .expect("No event received");

    let received_event3 = received3.expect("Error in stream");
    assert_eq!(received_event3.event_id, "resource_update");
    assert_eq!(
        received_event3.event_type,
        EventType::ContainerStarted as i32
    );

    // Assert state changes
    let s = state.read().await;
    assert_eq!(s.tasks.get("t1").unwrap(), &(EventType::TaskStarted as i32));
    assert_eq!(
        s.tasks.get("t2").unwrap(),
        &(EventType::TaskCompleted as i32)
    );
    assert_eq!(s.resources.nodes, 2.0);
    assert_eq!(s.resources.cpu, 75.0);
    assert_eq!(s.resources.memory, 2048.0);
}

#[tokio::test]
async fn test_get_server_state() {
    // Arrange: Set up the server state
    let mut state = ServerState::default();
    state.resources = crankshaft_monitor::server::Resource {
        nodes: 1.0,
        cpu: 50.0,
        memory: 1024.0,
        max_cpu: 100.0,
        max_memory: 2048.0,
    };
    state
        .tasks
        .insert("task1".to_string(), EventType::TaskStarted as i32);
    let state = Arc::new(RwLock::new(state));

    let (_tx, rx) = broadcast::channel::<Event>(16);
    let service = MonitorService::new(rx, state.clone());

    // Act: Call get_server_state
    let request = Request::new(GetServerStateRequest {});
    let response = service
        .get_server_state(request)
        .await
        .expect("get_server_state failed");

    // Assert: Check the response
    let server_state = response.into_inner();
    assert_eq!(server_state.resources.as_ref().unwrap().nodes, 1.0);
    assert_eq!(server_state.resources.as_ref().unwrap().cpu, 50.0);
    assert_eq!(
        server_state.tasks.get("task1").unwrap(),
        &(EventType::TaskStarted as i32)
    );
}

#[tokio::test]
async fn test_start_server_and_subscribe_events() {
    // Arrange: Set up the broadcast channel and start the server
    let addr = "127.0.0.1:8081"
        .parse::<SocketAddr>()
        .expect("Invalid address");

    let (sender, server_handle) = start_monitoring(addr).expect("Failed to start server");

    // we have to introduce a little delay to allow the server to start otherwise
    // the connection is refused
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create a gRPC transport channel to connect to the server
    let channel = tonic::transport::Channel::from_shared(format!("http://{addr}"))
        .expect("Invalid URI")
        .connect()
        .await
        .expect("Failed to connect");

    let mut client = MonitorClient::new(channel);

    //  request to subscribe to all events
    let request = Request::new(SubscribeEventsRequest {});
    let mut stream = client
        .subscribe_events(request)
        .await
        .expect("Failed to start stream")
        .into_inner();

    let event1 = Event {
        event_id: "t1".to_string(),
        event_type: EventType::TaskStarted as i32,
        timestamp: 1625234567,
        payload: Some(Message("Task t1 started".to_string())),
    };
    let event2 = Event {
        event_id: "t2".to_string(),
        event_type: EventType::TaskFailed as i32,
        timestamp: 1625234568,
        payload: Some(Message("Task t2 failed".to_string())),
    };
    sender.send(event1.clone()).expect("Failed to send event1");
    sender.send(event2.clone()).expect("Failed to send event2");

    let received1 = timeout(Duration::from_secs(1), stream.next())
        .await
        .expect("Timeout waiting for event")
        .expect("No event received");

    let received_event1 = received1.expect("Error in stream");
    assert_eq!(received_event1.event_id, "t1");
    assert_eq!(received_event1.event_type, EventType::TaskStarted as i32);

    let received2 = timeout(Duration::from_secs(1), stream.next())
        .await
        .expect("Timeout waiting for event")
        .expect("No event received");

    let received_event2 = received2.expect("Error in stream");
    assert_eq!(received_event2.event_id, "t2");
    assert_eq!(received_event2.event_type, EventType::TaskFailed as i32);

    // Now, let's get server state and verify
    let state_request = Request::new(GetServerStateRequest {});
    let state_response = client
        .get_server_state(state_request)
        .await
        .expect("get_server_state failed")
        .into_inner();

    assert_eq!(
        state_response.tasks.get("t1").unwrap(),
        &(EventType::TaskStarted as i32)
    );
    assert_eq!(
        state_response.tasks.get("t2").unwrap(),
        &(EventType::TaskFailed as i32)
    );

    // I can also send a resource update event and check that.
    let resource_event = Event {
        event_id: "resource_update".to_string(),
        event_type: EventType::ServiceStarted as i32,
        timestamp: 1625234569,
        payload: Some(ResourcesPayload(Resources {
            nodes: 4.0,
            cpu: 10.0,
            memory: 100.0,
            max_cpu: 20.0,
            max_memory: 200.0,
        })),
    };
    sender
        .send(resource_event.clone())
        .expect("Failed to send resource_event");

    // We need to consume this event from the stream
    let _received3 = timeout(Duration::from_secs(1), stream.next())
        .await
        .expect("Timeout waiting for event")
        .expect("No event received");

    // Now, get state again
    let state_request = Request::new(GetServerStateRequest {});
    let state_response = client
        .get_server_state(state_request)
        .await
        .expect("get_server_state failed")
        .into_inner();

    assert_eq!(state_response.resources.as_ref().unwrap().nodes, 4.0);
    assert_eq!(state_response.resources.as_ref().unwrap().cpu, 10.0);

    server_handle.abort();
}
