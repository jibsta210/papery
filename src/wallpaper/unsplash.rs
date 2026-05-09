//! "Unsplash" source — historically backed by `source.unsplash.com`, but that
//! endpoint was deprecated in 2023 and now redirects to a small curated pool,
//! so different ?sig= URLs ended up resolving to the same handful of photos
//! and our deduplication couldn't catch the repetition.
//!
//! Now backed by Lorem Picsum (https://picsum.photos), which is keyless and
//! exposes a paginated list of ~1000+ curated photos with stable IDs. Each
//! photo has a unique URL so dedup works correctly.

use super::{ProviderError, SourceKind, WallpaperInfo, WallpaperProvider};
use serde::Deserialize;
use std::sync::atomic::{AtomicU32, Ordering};

pub struct UnsplashProvider {
    /// Kept as a label hint; Picsum doesn't actually filter by topic.
    pub topic: String,
}

impl UnsplashProvider {
    pub fn new(topic: &str) -> Self {
        Self {
            topic: topic.trim().to_string(),
        }
    }
}

/// Picsum exposes a known total of ~1084 photos (as of late 2024).
/// We refine this dynamically as we walk pages.
static MAX_PAGE: AtomicU32 = AtomicU32::new(20);

#[derive(Deserialize)]
struct PicsumPhoto {
    id: String,
    #[serde(default)]
    author: String,
    width: u32,
    height: u32,
}

impl WallpaperProvider for UnsplashProvider {
    fn kind(&self) -> SourceKind {
        SourceKind::Unsplash
    }

    fn name(&self) -> &str {
        "Picsum"
    }

    fn fetch_wallpapers(
        &self,
        count: usize,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Vec<WallpaperInfo>, ProviderError>> + Send>,
    > {
        let _topic = self.topic.clone();
        Box::pin(async move {
            let limit = count.clamp(1, 100) as u32;
            let max_page = MAX_PAGE.load(Ordering::Relaxed).max(1);
            let page = (rand::random::<u32>() % max_page) + 1;
            let url = format!("https://picsum.photos/v2/list?page={page}&limit={limit}");
            let resp: Vec<PicsumPhoto> = super::http_client()
                .get(&url)
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;

            // Picsum returns empty when we walk past the end. Probe forward
            // a few pages on first run to get a better MAX_PAGE estimate.
            if !resp.is_empty() && page == max_page {
                MAX_PAGE.store(max_page.saturating_add(10), Ordering::Relaxed);
            }

            let wallpapers = resp
                .into_iter()
                .map(|p| {
                    // Use the photo's native resolution (capped at 4K) so
                    // dedup keys stay stable across runs.
                    let w = p.width.min(3840).max(1920);
                    let h = p.height.min(2160).max(1080);
                    let download_url = format!("https://picsum.photos/id/{}/{w}/{h}", p.id);
                    WallpaperInfo {
                        source: SourceKind::Unsplash,
                        url: download_url,
                        title: if p.author.is_empty() {
                            format!("Picsum #{}", p.id)
                        } else {
                            format!("Picsum · {}", p.author)
                        },
                        copyright: format!("© {} / Lorem Picsum", p.author),
                        local_path: None,
                        brightness: None,
                    }
                })
                .collect();
            Ok(wallpapers)
        })
    }
}
