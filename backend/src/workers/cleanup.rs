use bollard::{image::RemoveImageOptions, Docker};
use std::collections::HashMap;
use std::time::Duration;

pub async fn cleanup_worker(docker: Docker) {
    loop {
        tokio::time::sleep(Duration::from_secs(3600)).await;
        let images = docker
            .list_images(Some(bollard::image::ListImagesOptions {
                filters: HashMap::from([("dangling".to_string(), vec!["true".to_string()])]),
                ..Default::default()
            }))
            .await
            .unwrap_or_default();
        for img in &images {
            let id = &img.id;
            if !id.is_empty() {
                let _ = docker
                    .remove_image(
                        id,
                        Some(RemoveImageOptions {
                            force: false,
                            noprune: false,
                        }),
                        None,
                    )
                    .await;
                tracing::info!("🧹 Cleaned dangling image: {}", &id[..12.min(id.len())]);
            }
        }
    }
}
