//! Configuration options related to the default execution resources
//! requested/required.

use bon::Builder;
use serde::Deserialize;
use serde::Serialize;

/// Default resource requests.
#[derive(Builder, Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[builder(builder_type = Builder)]
pub struct Defaults {
    /// The number of CPUs to use during execution.
    ///
    /// Partial CPU requests are supported but not always respected depending on
    /// the backend.
    cpu: Option<f64>,

    /// The default limit of CPU cores that a container can use.
    ///
    /// Not all backends support limits on CPU usage.
    cpu_limit: Option<f64>,

    /// The amount of RAM (in GiB) to use during execution.
    ///
    /// This is a float because RAM can be allocated more granularly than in
    /// gigabytes. These may be rounded to any level of precision that is
    /// required for a particular environment.
    ram: Option<f64>,

    /// The default limit of random access memory that a container can use (in
    /// GiB).
    ///
    /// Not all backends support limits on memory usage.
    ram_limit: Option<f64>,

    /// The amount of disk (in GiB) to use during execution.
    ///
    /// This is a float because disks can be allocated more granularly than in
    /// gigabytes. These may be rounded to any level of precision that is
    /// required for a particular environment.
    disk: Option<f64>,
}

impl Defaults {
    /// Gets the number of CPUs.
    pub fn cpu(&self) -> Option<f64> {
        self.cpu
    }

    /// Gets the CPU limit.
    pub fn cpu_limit(&self) -> Option<f64> {
        self.cpu_limit
    }

    /// Gets the amount of RAM (in GiB).
    pub fn ram(&self) -> Option<f64> {
        self.ram
    }

    /// Gets the RAM limit (in GiB).
    pub fn ram_limit(&self) -> Option<f64> {
        self.ram_limit
    }

    /// Gets the amount of disk space (in GiB).
    pub fn disk(&self) -> Option<f64> {
        self.disk
    }
}
