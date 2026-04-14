use crate::wallpaper::WallpaperInfo;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;

pub struct DownloadManager {
    pub cache_dir: PathBuf,
    thumbnail_dir: PathBuf,
}

impl DownloadManager {
    pub fn new(cache_dir: PathBuf) -> Self {
        let thumbnail_dir = cache_dir.join("thumbnails");
        Self {
            cache_dir,
            thumbnail_dir,
        }
    }

    pub fn default_cache_dir() -> PathBuf {
        directories::BaseDirs::new()
            .map(|dirs| dirs.cache_dir().join("papery"))
            .unwrap_or_else(|| PathBuf::from("/tmp/papery"))
    }

    pub async fn ensure_dirs(&self) -> Result<(), std::io::Error> {
        fs::create_dir_all(&self.cache_dir).await?;
        fs::create_dir_all(&self.thumbnail_dir).await?;
        Ok(())
    }

    fn url_hash(url: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(url.as_bytes());
        let result = hasher.finalize();
        hex_encode(&result[..8])
    }

    fn cache_path_for(&self, url: &str) -> PathBuf {
        let hash = Self::url_hash(url);
        let ext = url_extension(url);
        self.cache_dir.join(format!("{hash}.{ext}"))
    }

    pub fn thumbnail_path_for(&self, url: &str) -> PathBuf {
        let hash = Self::url_hash(url);
        self.thumbnail_dir.join(format!("{hash}_thumb.jpg"))
    }

    pub async fn download(&self, wallpaper: &mut WallpaperInfo) -> Result<PathBuf, DownloadError> {
        self.ensure_dirs().await?;

        // Local files don't need downloading
        if let Some(ref path) = wallpaper.local_path {
            return Ok(path.clone());
        }

        let cached = self.cache_path_for(&wallpaper.url);

        // Return cached file if it exists
        if cached.exists() {
            wallpaper.local_path = Some(cached.clone());
            return Ok(cached);
        }

        // Download the image
        tracing::info!("Downloading: {}", wallpaper.url);
        let response = reqwest::get(&wallpaper.url).await?;
        let bytes = response.bytes().await?;

        let mut file = fs::File::create(&cached).await?;
        file.write_all(&bytes).await?;
        file.flush().await?;

        wallpaper.local_path = Some(cached.clone());

        // Generate thumbnail in the background
        let thumb_path = self.thumbnail_path_for(&wallpaper.url);
        if !thumb_path.exists() {
            let cached_clone = cached.clone();
            tokio::task::spawn_blocking(move || {
                generate_thumbnail(&cached_clone, &thumb_path);
            });
        }

        Ok(cached)
    }

    pub async fn cleanup(&self, max_files: u32, favorites: &[String]) -> Result<(), std::io::Error> {
        let mut entries: Vec<(PathBuf, std::time::SystemTime)> = Vec::new();

        let mut dir = fs::read_dir(&self.cache_dir).await?;
        while let Some(entry) = dir.next_entry().await? {
            let path = entry.path();
            if path.is_file() && path != self.thumbnail_dir.as_path() {
                let modified = entry.metadata().await?.modified()?;
                entries.push((path, modified));
            }
        }

        // Sort oldest first
        entries.sort_by_key(|(_, t)| *t);

        // Remove oldest files until under limit, skipping favorites
        while entries.len() > max_files as usize {
            if let Some((path, _)) = entries.first() {
                let path_str = path.to_string_lossy().to_string();
                if !favorites.contains(&path_str) {
                    let _ = fs::remove_file(path).await;
                    entries.remove(0);
                } else {
                    entries.remove(0); // Skip but still remove from list
                }
            } else {
                break;
            }
        }

        Ok(())
    }
}

fn generate_thumbnail(source: &Path, dest: &Path) {
    match image::open(source) {
        Ok(img) => {
            let thumb = img.thumbnail(400, 225);
            if let Err(e) = thumb.save(dest) {
                tracing::warn!("Failed to save thumbnail: {e}");
            }
        }
        Err(e) => {
            tracing::warn!("Failed to open image for thumbnail: {e}");
        }
    }
}

fn url_extension(url: &str) -> &str {
    let path = url.split('?').next().unwrap_or(url);
    path.rsplit('.')
        .next()
        .filter(|ext| matches!(*ext, "jpg" | "jpeg" | "png" | "webp" | "bmp"))
        .unwrap_or("jpg")
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[derive(Debug)]
pub enum DownloadError {
    Network(reqwest::Error),
    Io(std::io::Error),
}

impl std::fmt::Display for DownloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Network(e) => write!(f, "Network error: {e}"),
            Self::Io(e) => write!(f, "IO error: {e}"),
        }
    }
}

impl From<reqwest::Error> for DownloadError {
    fn from(e: reqwest::Error) -> Self {
        Self::Network(e)
    }
}

impl From<std::io::Error> for DownloadError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
