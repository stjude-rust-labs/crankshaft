//! Task resource specifications.

use std::borrow::Cow;
use std::collections::HashMap;

use bollard::secret::HostConfig;
use bon::Builder;
use crankshaft_config::backend::Defaults;

/// A set of requested resources.
#[derive(Builder, Clone, Debug)]
#[builder(builder_type = Builder)]
pub struct Resources {
    /// The number of CPU cores requested.
    #[builder(into)]
    cpu: Option<usize>,

    /// Whether or not the task may use preemptible resources.
    #[builder(into)]
    preemptible: Option<bool>,

    /// The requested random access memory size in gigabytes.
    #[builder(into)]
    ram: Option<f64>,

    /// The requested disk size in gigabytes.
    #[builder(into)]
    disk: Option<f64>,

    /// The associated compute zones.
    #[builder(into)]
    zones: Vec<String>,
}

impl Resources {
    /// A number of CPU cores.
    pub fn cpu(&self) -> Option<usize> {
        self.cpu
    }

    /// Whether the instance should be preemptible.
    pub fn preemptible(&self) -> Option<bool> {
        self.preemptible
    }

    /// The amount of RAM in gigabytes.
    pub fn ram(&self) -> Option<f64> {
        self.ram
    }

    /// The amount of disk space in gigabytes.
    pub fn disk(&self) -> Option<f64> {
        self.disk
    }

    /// The set of requested zones.
    pub fn zones(&self) -> &[String] {
        &self.zones
    }

    /// Applies any provided options in `other` to the [`Resources`].
    pub fn apply(mut self, other: &Self) -> Self {
        if let Some(cores) = other.cpu {
            self.cpu = Some(cores);
        }

        if let Some(preemptible) = other.preemptible {
            self.preemptible = Some(preemptible);
        }

        if let Some(ram) = other.ram {
            self.ram = Some(ram);
        }

        if let Some(disk) = other.disk {
            self.disk = Some(disk);
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

        if let Some(preemptible) = self.preemptible {
            map.insert("preemptible".into(), preemptible.to_string().into());
        }

        if let Some(ram) = self.ram {
            map.insert("ram".into(), ram.to_string().into());
            // TODO(clay): improve this.
            map.insert("ram_mb".into(), (ram * 1024.0).to_string().into());
        }

        if let Some(disk) = self.disk {
            map.insert("disk".into(), disk.to_string().into());
            // TODO(clay): improve this.
            map.insert("disk_mb".into(), (disk * 1024.0).to_string().into());
        }

        // Zones are explicitly not included.
        map
    }
}

impl Default for Resources {
    fn default() -> Self {
        Self {
            cpu: Some(1),
            preemptible: Some(false),
            ram: Some(2.0),
            disk: Some(8.0),
            zones: Default::default(),
        }
    }
}

impl From<&Defaults> for Resources {
    fn from(defaults: &Defaults) -> Self {
        Self {
            cpu: defaults.cpu(),
            preemptible: Default::default(),
            ram: defaults.ram(),
            disk: defaults.disk(),
            zones: Default::default(),
        }
    }
}

impl From<&Resources> for HostConfig {
    fn from(resources: &Resources) -> Self {
        let mut host_config = HostConfig::default();
        if let Some(ram) = resources.ram() {
            host_config.memory = Some((ram * 1024. * 1024. * 1024.) as i64);
        }

        if let Some(cpu) = resources.cpu() {
            host_config.cpu_count = Some(cpu as i64);
        }

        if let Some(disk) = resources.disk() {
            let mut storage_opt: HashMap<String, String> = HashMap::new();
            storage_opt.insert("size".to_string(), disk.to_string());
            host_config.storage_opt = Some(storage_opt);
        }

        host_config
    }
}
