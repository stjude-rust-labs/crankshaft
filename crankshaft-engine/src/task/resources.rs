//! Task resource specifications.

use std::borrow::Cow;
use std::collections::HashMap;

use bollard::secret::DeviceRequest;
use bollard::secret::HostConfig;
use bollard::secret::TaskSpecResources;
use bon::Builder;
use crankshaft_config::backend::Defaults;
use tracing::debug;

/// A set of requested resources.
#[derive(Builder, Clone, Debug)]
#[builder(builder_type = Builder)]
pub struct Resources {
    /// The requested number of CPU cores.
    ///
    /// Partial CPU requests are supported but not always respected depending on
    /// the backend.
    pub(crate) cpu: Option<f64>,

    /// The requested CPU limit.
    ///
    /// Not all backends support limits on CPU usage.
    pub(crate) cpu_limit: Option<f64>,

    /// The requested random access memory size (in GiB).
    pub(crate) ram: Option<f64>,

    /// The requested RAM limit (in GiB).
    ///
    /// Not all backends support limits on memory usage.
    pub(crate) ram_limit: Option<f64>,

    /// The requested disk size (in GiB).
    pub(crate) disk: Option<f64>,

    /// Whether or not the task may use preemptible resources.
    #[builder(into)]
    pub(crate) preemptible: Option<bool>,

    /// The associated compute zones.
    #[builder(into, default)]
    pub(crate) zones: Vec<String>,

    /// The number of GPUs requested.
    #[builder(into)]
    pub(crate) gpu: Option<u64>,
}

impl Resources {
    /// The number of CPU cores.
    pub fn cpu(&self) -> Option<f64> {
        self.cpu
    }

    /// The CPU limit.
    pub fn cpu_limit(&self) -> Option<f64> {
        self.cpu_limit
    }

    /// The amount of RAM in gibibytes (GiB).
    pub fn ram(&self) -> Option<f64> {
        self.ram
    }

    /// The RAM limit in gibibytes (GiB).
    pub fn ram_limit(&self) -> Option<f64> {
        self.ram_limit
    }

    /// The amount of disk space in gibibytes (GiB).
    pub fn disk(&self) -> Option<f64> {
        self.disk
    }

    /// Whether the instance should be preemptible.
    pub fn preemptible(&self) -> Option<bool> {
        self.preemptible
    }

    /// The set of requested zones.
    pub fn zones(&self) -> &[String] {
        &self.zones
    }

    /// The number of GPUs requested.
    pub fn gpu(&self) -> Option<u64> {
        self.gpu
    }

    /// Applies any provided options in `other` to the [`Resources`].
    pub fn apply(mut self, other: &Self) -> Self {
        if let Some(cores) = other.cpu {
            self.cpu = Some(cores);
        }

        if let Some(limit) = other.cpu_limit {
            self.cpu_limit = Some(limit);
        }

        if let Some(ram) = other.ram {
            self.ram = Some(ram);
        }

        if let Some(limit) = other.ram_limit {
            self.ram_limit = Some(limit);
        }

        if let Some(disk) = other.disk {
            self.disk = Some(disk);
        }

        if let Some(preemptible) = other.preemptible {
            self.preemptible = Some(preemptible);
        }

        if let Some(gpu) = other.gpu {
            self.gpu = Some(gpu);
        }

        self.zones = other.zones.clone();
        self
    }

    /// Creates a [`HashMap`] representation of the resources.
    ///
    /// This is used when doing command substitution for generic backends.
    // NOTE: keys in this HashMap are intended to _exactly_ match the names of
    // the fields in the struct. This is to ensure that mapping between the
    // underlying code and the configuration objects for generic configuration
    // is as seamless as possible (no extra translations unnecessarily).
    //
    // Please do not deviate from this unless you have a really strong,
    // articulated reason that is agreed upon by the core developers.
    pub fn to_hashmap(&self) -> HashMap<Cow<'static, str>, Cow<'static, str>> {
        let mut map = HashMap::new();

        if let Some(cores) = self.cpu {
            map.insert("cpu".into(), cores.to_string().into());
        }

        if let Some(limit) = self.cpu_limit {
            map.insert("cpu_limit".into(), limit.to_string().into());
        }

        if let Some(ram) = self.ram {
            map.insert("ram".into(), ram.to_string().into());
            // TODO(clay): improve this.
            map.insert("ram_mb".into(), (ram * 1024.0).to_string().into());
        }

        if let Some(limit) = self.ram_limit {
            map.insert("ram_limit".into(), limit.to_string().into());
        }

        if let Some(disk) = self.disk {
            map.insert("disk".into(), disk.to_string().into());
            // TODO(clay): improve this.
            map.insert("disk_mb".into(), (disk * 1024.0).to_string().into());
        }

        if let Some(preemptible) = self.preemptible {
            map.insert("preemptible".into(), preemptible.to_string().into());
        }

        if let Some(gpu) = self.gpu {
            map.insert("gpu".into(), gpu.to_string().into());
        }

        // Zones are explicitly not included.
        map
    }
}

impl Default for Resources {
    fn default() -> Self {
        Self {
            cpu: Some(1.0),
            cpu_limit: None,
            ram: Some(2.0),
            ram_limit: None,
            disk: Some(8.0),
            preemptible: Some(false),
            zones: Default::default(),
            gpu: None,
        }
    }
}

impl From<&Defaults> for Resources {
    fn from(defaults: &Defaults) -> Self {
        Self {
            cpu: defaults.cpu(),
            cpu_limit: defaults.cpu(),
            ram: defaults.ram(),
            ram_limit: defaults.ram_limit(),
            disk: defaults.disk(),
            preemptible: Default::default(),
            zones: Default::default(),
            gpu: defaults.gpu(),
        }
    }
}

impl From<&Resources> for HostConfig {
    fn from(resources: &Resources) -> Self {
        let mut host_config = Self::default();

        // Note: Docker doesn't have a CPU reservation for containers
        if resources.cpu().is_some() {
            debug!(
                "ignoring minimum CPU reservation for a Docker daemon not participating in a swarm"
            );
        }

        if let Some(cpu) = resources.cpu_limit() {
            host_config.nano_cpus = Some((cpu * 1_000_000_000.0) as i64);
        }

        // Note: Docker doesn't have a memory reservation for containers
        if resources.ram().is_some() {
            debug!(
                "ignoring minimum memory reservation for a Docker daemon not participating in a \
                 swarm"
            );
        }

        // The Docker `memory_reservation` setting acts as a soft limit and not as
        // something informing a scheduler of minimum requirements for the container

        if let Some(ram) = resources.ram_limit() {
            host_config.memory = Some((ram * 1024. * 1024. * 1024.) as i64);
        }

        if let Some(disk) = resources.disk() {
            let mut storage_opt: HashMap<String, String> = HashMap::new();
            storage_opt.insert("size".to_string(), disk.to_string());
            host_config.storage_opt = Some(storage_opt);
        }

        if let Some(gpu) = resources.gpu() {
            // TODO(clay): Only NVIDIA GPUs are supported at the moment. Add
            // support for other GPU vendors (AMD, Intel) in the future.
            //
            // These are specified as documented in
            // https://docs.docker.com/compose/how-tos/gpu-support/.
            const NVIDIA_DRIVER: &str = "nvidia";
            const GPU_CAPABILITY: &str = "gpu";

            host_config.device_requests = Some(vec![DeviceRequest {
                driver: Some(NVIDIA_DRIVER.into()),
                count: Some(gpu as i64),
                device_ids: None,
                capabilities: Some(vec![vec![GPU_CAPABILITY.into()]]),
                options: None,
            }]);
        }

        host_config
    }
}

impl From<&Resources> for TaskSpecResources {
    fn from(resources: &Resources) -> Self {
        let mut spec = Self::default();

        if let Some(cpu) = resources.cpu() {
            spec.reservations.get_or_insert_default().nano_cpus =
                Some((cpu * 1_000_000_000.0) as i64);
        }

        if let Some(cpu) = resources.cpu_limit() {
            spec.limits.get_or_insert_default().nano_cpus = Some((cpu * 1_000_000_000.0) as i64);
        }

        if let Some(ram) = resources.ram() {
            spec.reservations.get_or_insert_default().memory_bytes =
                Some((ram * 1024. * 1024. * 1024.) as i64);
        }

        if let Some(ram) = resources.ram_limit() {
            spec.limits.get_or_insert_default().memory_bytes =
                Some((ram * 1024. * 1024. * 1024.) as i64);
        }

        spec
    }
}

impl From<Resources> for tes::v1::types::task::Resources {
    fn from(resources: Resources) -> Self {
        fn gib_to_gb(v: f64) -> f64 {
            (v * (1024.0 * 1024.0 * 1024.0)) / (1000.0 * 1000.0 * 1000.0)
        }

        Self {
            cpu_cores: resources.cpu().map(|inner| inner.ceil() as i32),
            ram_gb: resources.ram().map(gib_to_gb),
            disk_gb: resources.disk().map(gib_to_gb),
            preemptible: resources.preemptible(),
            zones: if resources.zones.is_empty() {
                None
            } else {
                Some(resources.zones)
            },
            backend_parameters: None,
            backend_parameters_strict: None,
        }
    }
}

#[cfg(test)]
mod test {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn tes_resource_conversion() {
        let resources = Resources {
            cpu: Some(1.5),
            cpu_limit: None,
            ram: Some(16.),
            ram_limit: None,
            disk: Some(80.),
            preemptible: Some(true),
            zones: vec!["foo".into(), "bar".into(), "baz".into()],
            gpu: None,
        };

        let tes: tes::v1::types::task::Resources = resources.into();
        assert_eq!(tes.cpu_cores, Some(2));
        assert_relative_eq!(tes.ram_gb.unwrap(), 17.179869184);
        assert_relative_eq!(tes.disk_gb.unwrap(), 85.89934592);
        assert_eq!(tes.preemptible, Some(true));
        assert_eq!(
            tes.zones,
            Some(vec!["foo".into(), "bar".into(), "baz".into()])
        );
        assert_eq!(tes.backend_parameters, None);
        assert_eq!(tes.backend_parameters_strict, None);
    }

    #[test]
    fn gpu_creates_device_request() {
        let resources = Resources {
            cpu: None,
            cpu_limit: None,
            ram: None,
            ram_limit: None,
            disk: None,
            preemptible: None,
            zones: vec![],
            gpu: Some(1),
        };

        let host_config: HostConfig = (&resources).into();

        assert!(host_config.device_requests.is_some());
        let device_requests = host_config.device_requests.unwrap();
        assert_eq!(device_requests.len(), 1);

        let device_request = &device_requests[0];
        assert_eq!(device_request.driver.as_deref(), Some("nvidia"));
        assert_eq!(device_request.count, Some(1));
        assert_eq!(device_request.device_ids, None);
        assert_eq!(
            device_request.capabilities.as_ref(),
            Some(&vec![vec!["gpu".into()]])
        );
        assert_eq!(device_request.options, None);
    }

    #[test]
    fn no_gpu_creates_no_device_request() {
        let resources = Resources {
            cpu: None,
            cpu_limit: None,
            ram: None,
            ram_limit: None,
            disk: None,
            preemptible: None,
            zones: vec![],
            gpu: None,
        };

        let host_config: HostConfig = (&resources).into();

        assert!(host_config.device_requests.is_none());
    }
}
