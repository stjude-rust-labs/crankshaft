//! A Docker client that uses [`bollard`].

use bollard::secret::ImageDeleteResponseItem;
use bollard::secret::ImageSummary;

pub mod container;
pub mod images;

pub use crate::container::Container;
use crate::images::*;

/// A global error within this crate.
#[derive(Debug)]
pub enum Error {
    /// An error from [`bollard`].
    Docker(bollard::errors::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Docker(err) => write!(f, "docker error: {err}"),
        }
    }
}

impl std::error::Error for Error {}

/// A [`Result`](std::result::Result) with an [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// A Docker client.
#[derive(Clone, Debug)]
pub struct Docker(bollard::Docker);

impl Docker {
    /// Creates a new [`Docker`] with the specified [client](bollard::Docker).
    pub fn new(client: bollard::Docker) -> Self {
        Self(client)
    }

    /// Attempts to create a new [`Docker`] with the default socket connection.
    pub fn with_socket_defaults() -> Result<Self> {
        let client = bollard::Docker::connect_with_socket_defaults().map_err(Error::Docker)?;
        Ok(Self::new(client))
    }

    /// Attempts to create a new [`Docker`] with the default HTTP connection.
    pub fn with_http_defaults() -> Result<Self> {
        let client = bollard::Docker::connect_with_http_defaults().map_err(Error::Docker)?;
        Ok(Self::new(client))
    }

    /// Attempts to create a new [`Docker`] with the default connection details.
    pub fn with_defaults() -> Result<Self> {
        let client = bollard::Docker::connect_with_defaults().map_err(Error::Docker)?;
        Ok(Self::new(client))
    }

    /// Gets a reference to the inner [`bollard::Docker`].
    pub fn inner(&self) -> &bollard::Docker {
        &self.0
    }

    //----------------------------------------------------------------------------------
    // Images
    //----------------------------------------------------------------------------------

    /// Gets all of the images stored in the Docker daemon.
    pub async fn list_images(&self) -> Result<Vec<ImageSummary>> {
        list_images(self).await
    }

    /// Ensures that an image exists in the Docker daemon.
    ///
    /// It does this by:
    ///
    /// * Confirming that the image already exists there, or
    /// * Pulling the image from the remote repository.
    pub async fn ensure_image(&self, name: impl AsRef<str>, tag: impl AsRef<str>) -> Result<()> {
        ensure_image(self, name, tag).await
    }

    /// Removes an image from the Docker daemon.
    pub async fn remove_image(
        &self,
        name: impl AsRef<str>,
        tag: impl AsRef<str>,
    ) -> Result<impl IntoIterator<Item = ImageDeleteResponseItem>> {
        remove_image(self, name, tag).await
    }

    /// Removes all images from the Docker daemon.
    pub async fn remove_all_images(&self) -> Result<Vec<ImageDeleteResponseItem>> {
        remove_all_images(self).await
    }

    //----------------------------------------------------------------------------------
    // Containers
    //----------------------------------------------------------------------------------

    /// Creates a container builder.
    ///
    /// This is the typical way you will create containers.
    pub fn container_builder(&self) -> container::Builder {
        Container::builder(self.0.clone())
    }

    /// Creates a container from a known id.
    ///
    /// You should typically use [`Self::container_builder()`] unless you
    /// receive the container name externally from a user (say, on the command
    /// line as an argument).
    pub fn container_from_name(&self, id: impl Into<String>, attached: bool) -> Container {
        Container::new(self.0.clone(), id.into(), attached)
    }
}

#[cfg(test)]
mod tests {}
