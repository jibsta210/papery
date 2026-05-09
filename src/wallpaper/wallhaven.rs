use super::{ProviderError, SourceKind, WallpaperInfo, WallpaperProvider};
use serde::Deserialize;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::OnceLock;

fn last_page_file() -> Option<std::path::PathBuf> {
    directories::BaseDirs::new()
        .map(|d| d.cache_dir().join("papery").join("wallhaven_last_page"))
}

fn load_persisted_last_page() -> Option<u32> {
    let path = last_page_file()?;
    let s = std::fs::read_to_string(&path).ok()?;
    s.trim().parse().ok()
}

fn persist_last_page(p: u32) {
    if let Some(path) = last_page_file() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&path, p.to_string());
    }
}

static LOAD_ONCE: OnceLock<()> = OnceLock::new();
fn ensure_loaded() {
    LOAD_ONCE.get_or_init(|| {
        if let Some(p) = load_persisted_last_page() {
            LAST_PAGE.store(p, Ordering::Relaxed);
        }
    });
}

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
/// Initialized to a high estimate matching Wallhaven's actual catalog
/// (~20k pages for general+SFW); refined from each response.
static LAST_PAGE: AtomicU32 = AtomicU32::new(15000);

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
            ensure_loaded();
            // We were using `sorting=random&seed=X` but Wallhaven's random sort
            // appears to draw from a narrower curated pool than the full
            // catalog, so the same popular wallpapers kept coming back.
            //
            // Switch to `sorting=date_added` with a random page across the full
            // ~20k pages. Page N and page M return wallpapers uploaded at
            // entirely different times — there is no overlap possible.
            let last_page = LAST_PAGE.load(Ordering::Relaxed).max(1);
            let page: u32 = (rand::random::<u32>() % last_page) + 1;
            let url = format!(
                "https://wallhaven.cc/api/v1/search?categories={categories}&purity={purity}&sorting=date_added&order=desc&page={page}&atleast=1920x1080"
            );
            let resp: WallhavenResponse = super::http_client().get(&url).send().await?.json().await?;

            // Update the global last_page cache so future fetches sample the
            // full range. Persist to disk so the next process starts with
            // the right value.
            if let Some(meta) = resp.meta {
                if let Some(lp) = meta.last_page {
                    if lp > 0 {
                        let prev = LAST_PAGE.swap(lp, Ordering::Relaxed);
                        if prev != lp {
                            persist_last_page(lp);
                        }
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
