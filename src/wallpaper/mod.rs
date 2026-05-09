pub mod bing;
pub mod earth_view;
pub mod local;
pub mod nasa_apod;
pub mod pexels;
pub mod unsplash;
pub mod wallhaven;

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SourceKind {
    Bing,
    NasaApod,
    Wallhaven,
    EarthView,
    Local,
    Unsplash,
    Pexels,
}

impl SourceKind {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Bing => "Bing Photo of the Day",
            Self::NasaApod => "NASA APOD",
            Self::Wallhaven => "Wallhaven",
            Self::EarthView => "Google Earth View",
            Self::Local => "Local Folders",
            Self::Unsplash => "Unsplash",
            Self::Pexels => "Pexels",
        }
    }

    /// Estimated total catalog size for this source. Used to weight random
    /// selection so a small source (Bing's 64 wallpapers) doesn't dominate
    /// the queue when mixed with a huge one (Wallhaven's 1M+).
    pub fn pool_size(&self) -> u64 {
        match self {
            Self::Bing => 64,            // 8 days × 8 markets
            Self::NasaApod => 10_000,    // ~30 years of daily APODs
            Self::Wallhaven => 1_000_000, // 1M+ wallpapers
            Self::EarthView => 1_500,    // fixed catalog
            Self::Local => 100,          // user's folders, usually small
            Self::Unsplash => 1_000,     // Picsum has ~1000 curated photos
            Self::Pexels => 30_000,      // curated set is large
        }
    }
}

/// Pick the next wallpaper index in the queue using two-stage weighted random:
/// first pick a source weighted by `pool_size / total_pool`, then pick a
/// random wallpaper from that source's items in the queue.
///
/// Example: if Bing (pool=64) and Wallhaven (pool=1_000_000) are both
/// enabled, Bing only gets picked ~64/(64+1_000_000) ≈ 0.006% of the time.
pub fn weighted_pop_index(queue: &std::collections::VecDeque<WallpaperInfo>) -> Option<usize> {
    use std::collections::HashMap;
    if queue.is_empty() {
        return None;
    }

    // Group indices by source
    let mut by_source: HashMap<SourceKind, Vec<usize>> = HashMap::new();
    for (i, wp) in queue.iter().enumerate() {
        by_source.entry(wp.source.clone()).or_default().push(i);
    }

    // Weighted random source selection (probability ∝ pool_size)
    let sources: Vec<SourceKind> = by_source.keys().cloned().collect();
    let weights: Vec<u64> = sources.iter().map(|s| s.pool_size()).collect();
    let total: u64 = weights.iter().sum();
    if total == 0 {
        return Some(0);
    }
    let r = rand::random::<u64>() % total;
    let mut acc = 0u64;
    let mut chosen_source = &sources[0];
    for (i, w) in weights.iter().enumerate() {
        acc += w;
        if r < acc {
            chosen_source = &sources[i];
            break;
        }
    }

    // Random wallpaper within the chosen source
    let indices = by_source.get(chosen_source)?;
    let pick = (rand::random::<u32>() as usize) % indices.len();
    Some(indices[pick])
}

#[derive(Debug, Clone)]
pub struct WallpaperInfo {
    pub source: SourceKind,
    pub url: String,
    pub title: String,
    pub copyright: String,
    pub local_path: Option<PathBuf>,
    pub brightness: Option<f64>,
}

#[derive(Debug)]
pub enum ProviderError {
    Network(reqwest::Error),
    Parse(String),
    Io(std::io::Error),
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Network(e) => write!(f, "Network error: {e}"),
            Self::Parse(e) => write!(f, "Parse error: {e}"),
            Self::Io(e) => write!(f, "IO error: {e}"),
        }
    }
}

impl From<reqwest::Error> for ProviderError {
    fn from(e: reqwest::Error) -> Self {
        Self::Network(e)
    }
}

impl From<std::io::Error> for ProviderError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// Shared HTTP client with a 15-second timeout to prevent hangs.
pub fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .unwrap_or_default()
}

pub trait WallpaperProvider: Send + Sync {
    fn kind(&self) -> SourceKind;
    fn name(&self) -> &str;
    fn fetch_wallpapers(
        &self,
        count: usize,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Vec<WallpaperInfo>, ProviderError>> + Send>,
    >;
}
