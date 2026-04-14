use super::{ProviderError, SourceKind, WallpaperInfo, WallpaperProvider};
use serde::Deserialize;

pub struct BingProvider;

#[derive(Deserialize)]
struct BingResponse {
    images: Vec<BingImage>,
}

#[derive(Deserialize)]
struct BingImage {
    urlbase: String,
    copyright: String,
    #[serde(default)]
    title: String,
}

impl WallpaperProvider for BingProvider {
    fn kind(&self) -> SourceKind {
        SourceKind::Bing
    }

    fn name(&self) -> &str {
        "Bing Photo of the Day"
    }

    fn fetch_wallpapers(
        &self,
        count: usize,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Vec<WallpaperInfo>, ProviderError>> + Send>,
    > {
        let n = count.min(8); // Bing returns max 8
        Box::pin(async move {
            let url = format!(
                "https://www.bing.com/HPImageArchive.aspx?format=js&idx=0&n={n}&mkt=en-US"
            );
            let resp: BingResponse = reqwest::get(&url).await?.json().await?;

            let wallpapers = resp
                .images
                .into_iter()
                .map(|img| {
                    let image_url = format!("https://www.bing.com{}_UHD.jpg", img.urlbase);
                    WallpaperInfo {
                        source: SourceKind::Bing,
                        url: image_url,
                        title: if img.title.is_empty() {
                            img.copyright.split('(').next().unwrap_or("Bing").trim().to_string()
                        } else {
                            img.title
                        },
                        copyright: img.copyright,
                        local_path: None,
                        brightness: None,
                    }
                })
                .collect();

            Ok(wallpapers)
        })
    }
}
