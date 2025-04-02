//! Images.

use std::collections::HashMap;

use bollard::query_parameters::CreateImageOptions;
use bollard::query_parameters::ListImagesOptions;
use bollard::query_parameters::RemoveImageOptions;
use bollard::secret::ImageDeleteResponseItem;
use bollard::secret::ImageSummary;
use futures::stream::FuturesUnordered;
use tokio_stream::StreamExt as _;
use tracing::Level;
use tracing::debug;
use tracing::enabled;
use tracing::trace;

use crate::Docker;
use crate::Error;
use crate::Result;

/// Gets all of the images stored in the Docker daemon.
pub(crate) async fn list_images(docker: &Docker) -> Result<Vec<ImageSummary>> {
    debug!("listing images");

    let images = docker
        .inner()
        .list_images(Some(ListImagesOptions {
            all: true,
            ..Default::default()
        }))
        .await
        .map_err(Error::Docker)?;

    debug!("found {} images", images.len());

    if enabled!(Level::TRACE) {
        for image in &images {
            trace!(
                "  image: {} (tags: {})",
                image.id,
                image
                    .repo_tags
                    .iter()
                    .map(|v| format!("`{v}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            );

            for tag in &image.repo_tags {
                trace!("    {}", tag);
            }
        }
    }

    Ok(images)
}

/// Ensures that an image exists in the Docker daemon.
///
/// If the image does not specify a tag, a default tag of `latest` will be used.
///
/// It does this by:
///
/// * Confirming that the image already exists there, or
/// * Pulling the image from the remote repository.
pub(crate) async fn ensure_image(docker: &Docker, image: impl Into<String>) -> Result<()> {
    let image = image.into();

    debug!("ensuring image `{image}` exists locally");

    let mut filters = HashMap::new();
    filters.insert(String::from("reference"), vec![image.clone()]);
    let results = docker
        .inner()
        .list_images(Some(ListImagesOptions {
            filters: Some(filters),
            ..Default::default()
        }))
        .await
        .map_err(Error::Docker)?;

    if !results.is_empty() {
        debug!("image `{image}` exists locally");

        if enabled!(Level::TRACE) {
            trace!(
                "image SHA = {}",
                results.first().unwrap().id.trim_start_matches("sha256:")
            );
        }

        return Ok(());
    }

    debug!("image `{image}` does not exist locally; attempting to pull from remote");
    let mut stream = docker.inner().create_image(
        Some(CreateImageOptions {
            tag: Some(if image.contains(':') {
                String::from("")
            } else {
                String::from("latest")
            }),
            from_image: Some(image),
            ..Default::default()
        }),
        None,
        None,
    );

    while let Some(result) = stream.next().await {
        let update = result.map_err(Error::Docker)?;

        if enabled!(Level::TRACE) {
            trace!(
                "pull update: {}",
                [
                    update.id.map(|id| format!("id: {id}")),
                    update.error.map(|err| format!("error: {err}")),
                    update.status.map(|status| format!("status: {status}")),
                    update.progress.map(|progress| format!(
                        "progress: {progress}{}",
                        update
                            .progress_detail
                            .map(|detailed| format!(
                                " ({}/{})",
                                detailed
                                    .current
                                    .map(|v| v.to_string())
                                    .unwrap_or(String::from("?")),
                                detailed
                                    .total
                                    .map(|v| v.to_string())
                                    .unwrap_or(String::from("?"))
                            ))
                            .unwrap_or_default()
                    ))
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join("; ")
            )
        }
    }

    Ok(())
}

/// Removes an image from the Docker daemon.
pub(crate) async fn remove_image<T: AsRef<str>, U: AsRef<str>>(
    docker: &Docker,
    name: T,
    tag: U,
) -> Result<impl IntoIterator<Item = ImageDeleteResponseItem> + use<T, U>> {
    let name = name.as_ref();
    let tag = tag.as_ref();

    debug!("removing image: {name} ({tag})");
    let images = docker
        .inner()
        .remove_image(name, None::<RemoveImageOptions>, None)
        .await
        .map_err(Error::Docker)?;

    if enabled!(Level::TRACE) {
        for image in &images {
            if let Some(untagged) = &image.untagged {
                trace!("  untagged image: {untagged}");
            }

            if let Some(deleted) = &image.deleted {
                trace!("  deleted image: {deleted}");
            }
        }
    }

    Ok(images)
}

/// Removes all images from the Docker daemon.
pub(crate) async fn remove_all_images(docker: &Docker) -> Result<Vec<ImageDeleteResponseItem>> {
    debug!("removing all images");
    let mut results = Vec::new();

    for image in docker.list_images().await? {
        let mut futures = FuturesUnordered::new();

        for tag in image.repo_tags {
            futures.push(docker.remove_image(&image.id, tag));
        }

        while let Some(result) = futures.next().await {
            results.extend(result?);
        }
    }

    if !results.is_empty() {
        debug!("removed {} images in total", results.len());
    }

    Ok(results)
}
