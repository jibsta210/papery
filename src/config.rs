use cosmic_config::cosmic_config_derive::CosmicConfigEntry;
use cosmic_config::CosmicConfigEntry;
use serde::{Deserialize, Serialize};

pub const APP_ID: &str = "dev.papery.CosmicApplet";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, CosmicConfigEntry)]
#[version = 1]
pub struct PaperyConfig {
    /// Enabled wallpaper sources
    pub source_bing: bool,
    pub source_nasa: bool,
    pub source_wallhaven: bool,
    pub source_earthview: bool,
    pub source_local: bool,
    pub source_unsplash: bool,
    pub source_pexels: bool,

    /// Pexels API key (free at https://www.pexels.com/api/)
    pub pexels_api_key: String,
    /// Unsplash search topic (e.g. "nature", "wallpapers", or empty for any)
    pub unsplash_topic: String,

    /// Rotation interval in seconds
    pub rotation_interval_secs: u64,

    /// Whether rotation is paused
    pub paused: bool,

    /// Scaling mode: "zoom", "fit", "stretch"
    pub scaling_mode: String,

    /// Theme filter: "any", "light", "dark"
    pub theme_filter: String,

    /// Brightness threshold for light/dark classification (0.0-1.0)
    pub brightness_threshold: f64,

    /// Maximum number of cached wallpapers
    pub max_cache_size: u32,

    /// Local folders to scan for wallpapers
    pub local_folders: Vec<String>,

    /// Favorite wallpaper paths (kept permanently in cache)
    pub favorites: Vec<String>,

    /// Wallhaven categories (100 = general, 010 = anime, 001 = people)
    /// Kept for backwards compat; computed from the bools below at runtime.
    pub wallhaven_categories: String,

    /// Wallhaven purity (100 = SFW)
    pub wallhaven_purity: String,

    /// Wallhaven category toggles (multi-select)
    pub wallhaven_general: bool,
    pub wallhaven_anime: bool,
    pub wallhaven_people: bool,
}

impl PaperyConfig {
    /// Compute the Wallhaven categories binary string from the toggle bools.
    /// Falls back to "100" (general only) if all are off.
    pub fn wallhaven_categories_str(&self) -> String {
        let g = if self.wallhaven_general { '1' } else { '0' };
        let a = if self.wallhaven_anime { '1' } else { '0' };
        let p = if self.wallhaven_people { '1' } else { '0' };
        let s: String = [g, a, p].iter().collect();
        if s == "000" {
            "100".to_string()
        } else {
            s
        }
    }
}

impl Default for PaperyConfig {
    fn default() -> Self {
        Self {
            source_bing: true,
            source_nasa: true,
            source_wallhaven: false,
            source_earthview: false,
            source_local: false,
            source_unsplash: true,
            source_pexels: false,

            pexels_api_key: String::new(),
            unsplash_topic: "wallpapers".to_string(),

            rotation_interval_secs: 1800, // 30 minutes
            paused: false,

            scaling_mode: "zoom".to_string(),
            theme_filter: "any".to_string(),
            brightness_threshold: 0.5,
            max_cache_size: 200,

            local_folders: Vec::new(),
            favorites: Vec::new(),

            wallhaven_categories: "100".to_string(),
            wallhaven_purity: "100".to_string(),
            wallhaven_general: true,
            wallhaven_anime: false,
            wallhaven_people: false,
        }
    }
}
