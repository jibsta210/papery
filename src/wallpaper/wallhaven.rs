use super::{ProviderError, SourceKind, WallpaperInfo, WallpaperProvider};
use serde::Deserialize;

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

#[derive(Deserialize)]
struct WallhavenResponse {
    data: Vec<WallhavenEntry>,
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
            let url = format!(
                "https://wallhaven.cc/api/v1/search?categories={categories}&purity={purity}&sorting=toplist&topRange=1M&atleast=1920x1080"
            );
            let resp: WallhavenResponse = super::http_client().get(&url).send().await?.json().await?;

            let wallpapers = resp
                .data
                .into_iter()
                .map(|entry| WallpaperInfo {
                    source: SourceKind::Wallhaven,
                    url: entry.path,
                    title: format!("Wallhaven {} {}", entry.category, entry.resolution),
                    copyright: "Wallhaven".to_string(),
                    local_path: None,
                    brightness: None,
                })
                .collect();

            Ok(wallpapers)
        })
    }
}
