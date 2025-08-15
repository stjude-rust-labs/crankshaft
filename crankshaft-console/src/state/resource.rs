//! The resource module contains the state of the resources.
use crankshaft_monitor::proto::Event;
use crankshaft_monitor::proto::Resources as ProtoResource;
use crankshaft_monitor::proto::event::Payload::Resources;

/// The `ResourceState` struct holds the state of the resources.
#[derive(Debug, Default)]
pub struct ResourceState {
    /// The resources.
    resources: ProtoResource,
}

impl ResourceState {
    /// Updates the resource state.
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

    /// Returns the resources.
    pub(crate) fn resources(&self) -> &ProtoResource {
        &self.resources
    }

    /// Sets the initial resource state.
    pub fn set_initial(&mut self, resources: Option<ProtoResource>) {
        if let Some(r) = resources {
            self.resources = r;
        }
    }
}
