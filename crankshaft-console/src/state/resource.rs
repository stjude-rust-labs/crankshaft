use std::collections::HashMap;

use crankshaft_monitor::proto::Event;
use crankshaft_monitor::proto::event::Payload::Resources;

#[derive(Debug, Default)]
pub struct ResourceState {
    resources: HashMap<String, Resource>,
}

#[derive(Debug)]
pub struct Resource {
    pub id: String,
    pub cpu: Option<f64>,
    pub memory: Option<f64>,
    pub nodes: Option<f64>,
    pub max_cpu: Option<f64>,
    pub max_memory: Option<f64>,
}

impl Resource {
    pub fn new(engine_name: &str) -> Self {
        Self {
            id: engine_name.to_owned(),
            cpu: Some(1.0),
            memory: Some(1.0),
            nodes: Some(1.0),
            max_cpu: Some(2.0),
            max_memory: Some(3.0),
        }
    }
}

impl ResourceState {
    pub fn update(&mut self, message: Event) {
        let payload = match message.payload {
            Some(p) => p,
            None => return,
        };

        let resource = self
            .resources
            .entry(message.event_id.clone())
            .or_insert_with(|| Resource::new(&message.event_id));

        match payload {
            Resources(r) => {
                *resource = Resource {
                    id: resource.id.clone(),
                    cpu: Some(r.cpu),
                    memory: Some(r.memory),
                    max_cpu: Some(r.max_cpu),
                    max_memory: Some(r.max_memory),
                    nodes: Some(r.nodes),
                };
            }
            _ => {}
        }
    }

    pub(crate) fn resources(&self) -> &HashMap<String, Resource> {
        &self.resources
    }
}
