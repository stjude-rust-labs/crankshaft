use crankshaft_monitor::proto::Event;
use crankshaft_monitor::proto::Resources as ProtoResource;
use crankshaft_monitor::proto::event::Payload::Resources;

/// ResourceState
#[derive(Debug, Default)]
pub struct ResourceState {
    /// Resources
    resources: ProtoResource,
}

// /// Resource
// #[derive(Debug)]
// pub struct Resource {
//     /// Resource ID
//     pub id: String,
//     /// CPU usage
//     pub cpu: Option<f64>,
//     /// Memory usage
//     pub memory: Option<f64>,
//     /// Number of nodes
//     pub nodes: Option<f64>,
//     /// Maximum CPU usage
//     pub max_cpu: Option<f64>,
//     /// Maximum memory usage
//     pub max_memory: Option<f64>,
// }

// impl Resource {
//     /// Create a new Resource instance with default values.
//     pub fn new(engine_name: &str) -> Self {
//         Self {
//             id: engine_name.to_owned(),
//             cpu: Some(1.0),
//             memory: Some(1.0),
//             nodes: Some(1.0),
//             max_cpu: Some(2.0),
//             max_memory: Some(3.0),
//         }
//     }
// }

impl ResourceState {
    /// Update resource state based on event payload.
    pub fn update(&mut self, message: Event) {
        let payload = match message.payload {
            Some(p) => p,
            None => return,
        };

        // let resource = self
        //     .resources
        //     .entry(message.event_id.clone())
        //     .or_insert_with(|| Resources::new(&message.event_id));

        if let Resources(r) = payload {
            self.resources = ProtoResource {
                cpu: r.cpu,
                memory: r.memory,
                max_cpu: r.max_cpu,
                max_memory: r.max_memory,
                nodes: r.nodes,
            };
        }
    }

    /// Get a reference to the resources map.
    pub(crate) fn resources(&self) -> &ProtoResource {
        &self.resources
    }
}
