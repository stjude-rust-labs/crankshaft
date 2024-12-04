//! Builders for a [`Resources`].

use nonempty::NonEmpty;

use crate::task::Resources;

/// A builder for a [`Resources`].
#[derive(Debug, Default)]
pub struct Builder {
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

impl Builder {
    /// Adds a requested number of CPU core(s) to the [`Builder`].
    ///
    /// # Notes
    ///
    /// This will silently overwrite any previously requested number of CPU
    /// core(s) provided to the builder.
    pub fn cpu(mut self, value: impl Into<usize>) -> Self {
        self.cpu = Some(value.into());
        self
    }

    /// Sets whether the request resources are preemptible or not within the
    /// [`Builder`].
    ///
    /// # Notes
    ///
    /// This will silently overwrite any previous preemptible designation
    /// provided to the builder.
    pub fn preemptible(mut self, value: impl Into<bool>) -> Self {
        self.preemptible = Some(value.into());
        self
    }

    /// Adds a requested amount of RAM in gigabytes to the [`Builder`].
    ///
    /// # Notes
    ///
    /// This will silently overwrite any previously requested amount of RAM
    /// provided to the builder.
    pub fn ram(mut self, value: impl Into<f64>) -> Self {
        self.ram = Some(value.into());
        self
    }

    /// Adds a requested amount of disk space in gigabytes to the [`Builder`].
    ///
    /// # Notes
    ///
    /// This will silently overwrite any previously requested amount of disk
    /// space provided to the builder.
    pub fn disk(mut self, disk: impl Into<f64>) -> Self {
        self.disk = Some(disk.into());
        self
    }

    /// Resets the zones to [`None`].
    pub fn reset_zones(mut self) -> Self {
        self.zones = None;
        self
    }

    /// Adds zones to the [`Builder`].
    ///
    /// # Notes
    ///
    /// This will append to any previously assigned zones (use
    /// [`reset_zones()`](Self::reset_zones) if you need to erase the previously
    /// provided zones).
    pub fn zones(mut self, values: impl Iterator<Item: Into<String>>) -> Self {
        let mut values = values.map(|s| s.into());

        self.zones = match self.zones {
            Some(mut zones) => {
                zones.extend(values);
                Some(zones)
            }
            None => match values.next() {
                Some(zone) => {
                    let mut zones = NonEmpty::new(zone);
                    zones.extend(values);
                    Some(zones)
                }
                _ => None,
            },
        };

        self
    }

    /// Consumes `self` and returns a built [`Resources`].
    pub fn build(self) -> Resources {
        Resources {
            cpu: self.cpu,
            preemptible: self.preemptible,
            ram: self.ram,
            disk: self.disk,
            zones: self.zones,
        }
    }
}
