use super::{ProviderError, SourceKind, WallpaperInfo, WallpaperProvider};

/// Unsplash provider using the keyless `source.unsplash.com` endpoint.
/// Each fetch returns N redirect URLs that resolve to fresh random images.
pub struct UnsplashProvider {
    pub topic: String,
}

impl UnsplashProvider {
    pub fn new(topic: &str) -> Self {
        Self {
            topic: topic.trim().to_string(),
        }
    }
}

impl WallpaperProvider for UnsplashProvider {
    fn kind(&self) -> SourceKind {
        SourceKind::Unsplash
    }

    fn name(&self) -> &str {
        "Unsplash"
    }

    fn fetch_wallpapers(
        &self,
        count: usize,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Vec<WallpaperInfo>, ProviderError>> + Send>,
    > {
        let topic = self.topic.clone();
        Box::pin(async move {
            // source.unsplash.com redirects each request to a random photo.
            // We append a random sig to bypass any caching and get N unique URLs.
            let mut wallpapers = Vec::new();
            for _ in 0..count.min(15) {
                let sig: u32 = rand::random();
                let url = if topic.is_empty() {
                    format!("https://source.unsplash.com/random/3840x2160?sig={sig}")
                } else {
                    format!("https://source.unsplash.com/random/3840x2160/?{topic}&sig={sig}")
                };
                wallpapers.push(WallpaperInfo {
                    source: SourceKind::Unsplash,
                    url,
                    title: if topic.is_empty() {
                        "Unsplash".to_string()
                    } else {
                        format!("Unsplash · {topic}")
                    },
                    copyright: "Unsplash".to_string(),
                    local_path: None,
                    brightness: None,
                });
            }
            Ok(wallpapers)
        })
    }
}
