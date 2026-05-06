use super::{http_client, ProviderError, SourceKind, WallpaperInfo, WallpaperProvider};
use serde::Deserialize;

pub struct PexelsProvider {
    pub api_key: String,
}

impl PexelsProvider {
    pub fn new(api_key: &str) -> Self {
        Self {
            api_key: api_key.trim().to_string(),
        }
    }
}

#[derive(Deserialize)]
struct PexelsResponse {
    photos: Vec<PexelsPhoto>,
}

#[derive(Deserialize)]
struct PexelsPhoto {
    #[serde(default)]
    photographer: String,
    #[serde(default)]
    alt: String,
    src: PexelsSrc,
}

#[derive(Deserialize)]
struct PexelsSrc {
    #[serde(default)]
    original: String,
    #[serde(default)]
    landscape: String,
}

impl WallpaperProvider for PexelsProvider {
    fn kind(&self) -> SourceKind {
        SourceKind::Pexels
    }

    fn name(&self) -> &str {
        "Pexels"
    }

    fn fetch_wallpapers(
        &self,
        count: usize,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Vec<WallpaperInfo>, ProviderError>> + Send>,
    > {
        let api_key = self.api_key.clone();
        Box::pin(async move {
            if api_key.is_empty() {
                return Err(ProviderError::Parse(
                    "Pexels API key not configured. Get one free at https://www.pexels.com/api/".into(),
                ));
            }

            let per_page = count.clamp(1, 80);
            // Pick a random page within the curated photos to vary results
            let page = (rand::random::<u32>() % 50) + 1;
            let url = format!(
                "https://api.pexels.com/v1/curated?per_page={per_page}&page={page}"
            );
            let resp = http_client()
                .get(&url)
                .header("Authorization", &api_key)
                .send()
                .await?
                .error_for_status()?;
            let body: PexelsResponse = resp.json().await?;

            let wallpapers = body
                .photos
                .into_iter()
                .filter_map(|p| {
                    let url = if !p.src.original.is_empty() {
                        p.src.original
                    } else if !p.src.landscape.is_empty() {
                        p.src.landscape
                    } else {
                        return None;
                    };
                    let title = if p.alt.is_empty() {
                        format!("Pexels · {}", p.photographer)
                    } else {
                        p.alt
                    };
                    Some(WallpaperInfo {
                        source: SourceKind::Pexels,
                        url,
                        title,
                        copyright: format!("© {} / Pexels", p.photographer),
                        local_path: None,
                        brightness: None,
                    })
                })
                .collect();
            Ok(wallpapers)
        })
    }
}
