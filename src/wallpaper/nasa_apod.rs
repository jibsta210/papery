use super::{ProviderError, SourceKind, WallpaperInfo, WallpaperProvider};
use serde::Deserialize;

pub struct NasaApodProvider;

#[derive(Deserialize)]
struct ApodEntry {
    #[serde(default)]
    title: String,
    #[serde(default)]
    #[allow(dead_code)]
    explanation: String,
    #[serde(default)]
    hdurl: Option<String>,
    #[serde(default)]
    url: String,
    #[serde(default)]
    media_type: String,
    #[serde(default)]
    copyright: Option<String>,
}

impl WallpaperProvider for NasaApodProvider {
    fn kind(&self) -> SourceKind {
        SourceKind::NasaApod
    }

    fn name(&self) -> &str {
        "NASA Astronomy Picture of the Day"
    }

    fn fetch_wallpapers(
        &self,
        count: usize,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Vec<WallpaperInfo>, ProviderError>> + Send>,
    > {
        let n = count.min(10);
        Box::pin(async move {
            let url = format!(
                "https://api.nasa.gov/planetary/apod?api_key=DEMO_KEY&count={n}&thumbs=false"
            );
            let entries: Vec<ApodEntry> = reqwest::get(&url).await?.json().await?;

            let wallpapers = entries
                .into_iter()
                .filter(|e| e.media_type == "image")
                .filter_map(|e| {
                    let image_url = e.hdurl.or(Some(e.url)).filter(|u| !u.is_empty())?;
                    Some(WallpaperInfo {
                        source: SourceKind::NasaApod,
                        url: image_url,
                        title: e.title,
                        copyright: e.copyright.unwrap_or_else(|| "NASA".to_string()),
                        local_path: None,
                        brightness: None,
                    })
                })
                .collect();

            Ok(wallpapers)
        })
    }
}
