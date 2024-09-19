//! Task resource specifications.

mod builder;

use std::collections::HashMap;

use bollard::secret::HostConfig;
pub use builder::Builder;
use crankshaft_config::backend::Defaults;
use nonempty::NonEmpty;

/// A set of requested resources.
#[derive(Clone, Debug)]
pub struct Resources {
    /// The number of CPU cores requested.
    cpu: Option<usize>,

    /// Whether or not the task may use preemptible resources.
    preemptible: Option<bool>,

    /// The requested random access memory size in gigabytes.
    ram: Option<f64>,

    /// The requested disk size in gigabytes.
    disk: Option<f64>,

    /// The associated compute zones.
    zones: Option<NonEmpty<String>>,
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
    pub fn zones(&self) -> Option<&NonEmpty<String>> {
        self.zones.as_ref()
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

        if let Some(zones) = &other.zones {
            self.zones = Some(zones.clone());
        }

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
    pub fn to_hashmap(&self) -> Option<HashMap<String, String>> {
        let mut hm = HashMap::new();

        if let Some(cores) = self.cpu {
            hm.insert(String::from("cpu"), cores.to_string());
        }

        if let Some(preemptible) = self.preemptible {
            hm.insert(String::from("preemptible"), preemptible.to_string());
        }

        if let Some(ram) = self.ram {
            hm.insert(String::from("ram"), ram.to_string());
            // TODO(clay): improve this.
            hm.insert(String::from("ram_mb"), (ram * 1024.0).to_string());
        }

        if let Some(disk) = self.disk {
            hm.insert(String::from("disk"), disk.to_string());
            // TODO(clay): improve this.
            hm.insert(String::from("disk_mb"), (disk * 1024.0).to_string());
        }

        // Zones are explicitly not included.
        if !hm.is_empty() { Some(hm) } else { None }
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
