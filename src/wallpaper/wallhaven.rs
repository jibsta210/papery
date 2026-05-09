use super::{ProviderError, SourceKind, WallpaperInfo, WallpaperProvider};
use serde::Deserialize;
use std::sync::atomic::{AtomicU32, Ordering};

pub struct WallhavenProvider {
    pub categories: String,
    pub purity: String,
}

impl WallhavenProvider {
    pub fn new(categories: &str, purity: &str) -> Self {
        Self {
            categories: categories.to_string(),
            purity: purity.to_string(),
        }
    }
}

/// Cached last_page across calls so we know how far we can sample.
/// Initialized to a conservative guess; refined from each response.
static LAST_PAGE: AtomicU32 = AtomicU32::new(500);

#[derive(Deserialize)]
struct WallhavenResponse {
    data: Vec<WallhavenEntry>,
    #[serde(default)]
    meta: Option<WallhavenMeta>,
}

#[derive(Deserialize)]
struct WallhavenMeta {
    #[serde(default)]
    last_page: Option<u32>,
}

#[derive(Deserialize)]
struct WallhavenEntry {
    path: String,
    #[serde(default)]
    category: String,
    #[serde(default)]
    resolution: String,
}

impl WallpaperProvider for WallhavenProvider {
    fn kind(&self) -> SourceKind {
        SourceKind::Wallhaven
    }

    fn name(&self) -> &str {
        "Wallhaven"
    }

    fn fetch_wallpapers(
        &self,
        _count: usize,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Vec<WallpaperInfo>, ProviderError>> + Send>,
    > {
        let categories = self.categories.clone();
        let purity = self.purity.clone();
        Box::pin(async move {
            // Pick a random page within the last known total. Wallhaven's
            // unauthenticated `random` sort with a fresh seed each call gives
            // genuinely different results, but capping the page meant we were
            // missing 99% of wallhaven's catalog (20k+ pages exist).
            let last_page = LAST_PAGE.load(Ordering::Relaxed).max(1);
            let seed: u32 = rand::random();
            let page: u32 = (rand::random::<u32>() % last_page) + 1;
            let url = format!(
                "https://wallhaven.cc/api/v1/search?categories={categories}&purity={purity}&sorting=random&seed={seed}&page={page}&atleast=1920x1080"
            );
            let resp: WallhavenResponse = super::http_client().get(&url).send().await?.json().await?;

            // Update the global last_page cache so future fetches sample the
            // full range. Only grow it (don't shrink on empty responses).
            if let Some(meta) = resp.meta {
                if let Some(lp) = meta.last_page {
                    if lp > 0 {
                        LAST_PAGE.store(lp, Ordering::Relaxed);
                    }
                }
            }

            let wallpapers = resp
                .data
                .into_iter()
                .map(|entry| {
                    let category_label = match entry.category.as_str() {
                        "general" => "General",
                        "anime" => "Anime",
                        "people" => "People",
                        _ => "",
                    };
                    WallpaperInfo {
                        source: SourceKind::Wallhaven,
                        url: entry.path,
                        title: if entry.resolution.is_empty() {
                            format!("Wallhaven · {category_label}")
                        } else {
                            format!("Wallhaven · {category_label} · {}", entry.resolution)
                        },
                        copyright: "Wallhaven".to_string(),
                        local_path: None,
                        brightness: None,
                    }
                })
                .collect();

            Ok(wallpapers)
        })
    }
}
