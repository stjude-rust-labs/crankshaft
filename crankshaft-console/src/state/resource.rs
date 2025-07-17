use std::collections::HashMap;

use crankshaft_monitor::proto::Event;

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

impl Resource{
    pub fn new()
}

impl ResourceState {
    pub fn update(&mut self, message: Event) {
        let resource = self
            .resources
            .entry(message.task_id.clone())
            .or_insert_with(|| Resource::new(resource_update.id.clone()));
    }

    pub(crate) fn resources(&self) -> &HashMap<String, Resource> {
        &self.resources
    }
}
