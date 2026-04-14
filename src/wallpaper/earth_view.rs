use super::{ProviderError, SourceKind, WallpaperInfo, WallpaperProvider};
use serde::Deserialize;

pub struct EarthViewProvider;

#[derive(Deserialize)]
struct EarthViewEntry {
    slug: String,
    #[serde(default)]
    country: String,
    #[serde(default)]
    region: String,
    #[serde(rename = "photoUrl")]
    #[serde(default)]
    photo_url: Option<String>,
}

impl WallpaperProvider for EarthViewProvider {
    fn kind(&self) -> SourceKind {
        SourceKind::EarthView
    }

    fn name(&self) -> &str {
        "Google Earth View"
    }

    fn fetch_wallpapers(
        &self,
        count: usize,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Vec<WallpaperInfo>, ProviderError>> + Send>,
    > {
        Box::pin(async move {
            let url =
                "https://new-images-preview-dot-earth-viewer.appspot.com/_api/photos.json";
            let entries: Vec<EarthViewEntry> = reqwest::get(url).await?.json().await?;

            use rand::seq::SliceRandom;
            let mut rng = rand::rng();
            let mut selected: Vec<_> = entries.into_iter().collect();
            selected.shuffle(&mut rng);
            selected.truncate(count);

            let wallpapers = selected
                .into_iter()
                .filter_map(|entry| {
                    let image_url = entry.photo_url?;
                    let title = if entry.region.is_empty() {
                        entry.country.clone()
                    } else {
                        format!("{}, {}", entry.region, entry.country)
                    };
                    Some(WallpaperInfo {
                        source: SourceKind::EarthView,
                        url: image_url,
                        title: if title.is_empty() {
                            entry.slug.clone()
                        } else {
                            title
                        },
                        copyright: "Google Earth".to_string(),
                        local_path: None,
                        brightness: None,
                    })
                })
                .collect();

            Ok(wallpapers)
        })
    }
}
