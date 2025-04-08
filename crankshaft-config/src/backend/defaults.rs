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
    pub cpu: Option<f64>,

    /// The default limit of CPU cores that a container can use.
    ///
    /// Not all backends support limits on CPU usage.
    pub cpu_limit: Option<f64>,

    /// The amount of RAM (in GiB) to use during execution.
    ///
    /// This is a float because RAM can be allocated more granularly than in
    /// gigabytes. These may be rounded to any level of precision that is
    /// required for a particular environment.
    pub ram: Option<f64>,

    /// The default limit of random access memory that a container can use (in
    /// GiB).
    ///
    /// Not all backends support limits on memory usage.
    pub ram_limit: Option<f64>,

    /// The amount of disk (in GiB) to use during execution.
    ///
    /// This is a float because disks can be allocated more granularly than in
    /// gigabytes. These may be rounded to any level of precision that is
    /// required for a particular environment.
    pub disk: Option<f64>,
}
