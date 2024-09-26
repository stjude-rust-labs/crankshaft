//! Images.

use std::collections::HashMap;

use bollard::image::CreateImageOptions;
use bollard::image::ListImagesOptions;
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
        .list_images(Some(ListImagesOptions::<String> {
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
/// It does this by:
///
/// * Confirming that the image already exists there, or
/// * Pulling the image from the remote repository.
pub(crate) async fn ensure_image(
    docker: &Docker,
    name: impl AsRef<str>,
    tag: impl AsRef<str>,
) -> Result<()> {
    let name = name.as_ref();
    let tag = tag.as_ref();
    debug!("ensuring image: `{name}:{tag}`");

    let mut filters = HashMap::new();
    filters.insert(String::from("reference"), vec![format!("{name}:{tag}")]);

    debug!("checking if image exists locally: `{name}:{tag}`");
    let results = docker
        .inner()
        .list_images(Some(ListImagesOptions {
            filters,
            ..Default::default()
        }))
        .await
        .map_err(Error::Docker)?;

    if !results.is_empty() {
        debug!("image exists locally");

        if enabled!(Level::TRACE) {
            trace!(
                "image SHA = {}",
                results.first().unwrap().id.trim_start_matches("sha256:")
            );
        }

        return Ok(());
    }

    debug!("image does NOT exist locally; attempting to pull from remote");
    let mut stream = docker.inner().create_image(
        Some(CreateImageOptions {
            from_image: name,
            tag,
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
pub(crate) async fn remove_image(
    docker: &Docker,
    name: impl AsRef<str>,
    tag: impl AsRef<str>,
) -> Result<impl IntoIterator<Item = ImageDeleteResponseItem>> {
    let name = name.as_ref();
    let tag = tag.as_ref();

    debug!("removing image: {name} ({tag})");
    let images = docker
        .inner()
        .remove_image(name, None, None)
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
