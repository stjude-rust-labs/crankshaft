use crankshaft_monitor::proto::Event;
use crankshaft_monitor::proto::Resources as ProtoResource;
use crankshaft_monitor::proto::event::Payload::Resources;

/// The `ResourceState` struct represents the state of resources in the system.
#[derive(Debug, Default)]
pub struct ResourceState {
    /// The `resources` field has the same type as `ProtoResource`.
    resources: ProtoResource,
}

impl ResourceState {
    /// Update resource state based on event payload.
    pub fn update(&mut self, message: Event) {
        let payload = match message.payload {
            Some(p) => p,
            None => return,
        };

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

    pub fn set_initial(&mut self, resources: Option<ProtoResource>) {
        if let Some(r) = resources {
            self.resources = r;
        }
    }
}
