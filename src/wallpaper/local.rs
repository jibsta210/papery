use super::{ProviderError, SourceKind, WallpaperInfo, WallpaperProvider};
use std::path::PathBuf;

pub struct LocalProvider {
    pub folders: Vec<PathBuf>,
}

impl LocalProvider {
    pub fn new(folders: Vec<PathBuf>) -> Self {
        Self { folders }
    }
}

const SUPPORTED_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "bmp"];

impl WallpaperProvider for LocalProvider {
    fn kind(&self) -> SourceKind {
        SourceKind::Local
    }

    fn name(&self) -> &str {
        "Local Folders"
    }

    fn fetch_wallpapers(
        &self,
        count: usize,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Vec<WallpaperInfo>, ProviderError>> + Send>,
    > {
        let folders = self.folders.clone();
        Box::pin(async move {
            let mut images = Vec::new();

            for folder in &folders {
                if !folder.is_dir() {
                    continue;
                }
                let entries = std::fs::read_dir(folder)?;
                for entry in entries.flatten() {
                    let path = entry.path();
                    if !path.is_file() {
                        continue;
                    }
                    let ext = path
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    if SUPPORTED_EXTENSIONS.contains(&ext.as_str()) {
                        let filename = path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("Unknown")
                            .to_string();
                        images.push(WallpaperInfo {
                            source: SourceKind::Local,
                            url: String::new(),
                            title: filename,
                            copyright: String::new(),
                            local_path: Some(path),
                            brightness: None,
                        });
                    }
                }
            }

            use rand::seq::SliceRandom;
            let mut rng = rand::rng();
            images.shuffle(&mut rng);
            images.truncate(count);

            Ok(images)
        })
    }
}
