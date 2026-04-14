pub mod bing;
pub mod earth_view;
pub mod local;
pub mod nasa_apod;
pub mod wallhaven;

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SourceKind {
    Bing,
    NasaApod,
    Wallhaven,
    EarthView,
    Local,
}

impl SourceKind {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Bing => "Bing Photo of the Day",
            Self::NasaApod => "NASA APOD",
            Self::Wallhaven => "Wallhaven",
            Self::EarthView => "Google Earth View",
            Self::Local => "Local Folders",
        }
    }
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
