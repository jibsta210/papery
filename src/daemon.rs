use crate::background;
use crate::brightness;
use crate::config::{PaperyConfig, APP_ID};
use crate::download::DownloadManager;
use crate::tray::{self, TrayAction};
use crate::wallpaper::bing::BingProvider;
use crate::wallpaper::earth_view::EarthViewProvider;
use crate::wallpaper::local::LocalProvider;
use crate::wallpaper::nasa_apod::NasaApodProvider;
use crate::wallpaper::wallhaven::WallhavenProvider;
use crate::wallpaper::{WallpaperInfo, WallpaperProvider};
use cosmic_config::CosmicConfigEntry;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Run Papery in headless background mode: no window, just wallpaper
/// rotation and system tray icon.
pub fn run_background() {
    tracing::info!("Running in background mode");

    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    rt.block_on(async { background_loop().await });
}

async fn background_loop() {
    let config_handler = cosmic_config::Config::new(APP_ID, PaperyConfig::VERSION).ok();
    let config = config_handler
        .as_ref()
        .and_then(|h| PaperyConfig::get_entry(h).ok())
        .unwrap_or_default();

    let paused = Arc::new(AtomicBool::new(config.paused));

    // Spawn tray
    tray::spawn_tray(paused.clone());
    let mut tray_rx = tray::take_receiver();

    let cache_dir = DownloadManager::default_cache_dir();
    let dm = DownloadManager::new(cache_dir);
    let _ = dm.ensure_dirs().await;

    let mut queue: VecDeque<WallpaperInfo> = VecDeque::new();
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
    let mut seconds_left = config.rotation_interval_secs;
    let mut config = config;

    loop {
        tokio::select! {
            _ = interval.tick() => {
                if paused.load(Ordering::Relaxed) {
                    continue;
                }
                if seconds_left > 0 {
                    seconds_left -= 1;
                    continue;
                }
                // Time to change wallpaper
                seconds_left = config.rotation_interval_secs;

                if queue.is_empty() {
                    fetch_into_queue(&config, &mut queue).await;
                }

                if let Some(mut wp) = queue.pop_front() {
                    if let Ok(path) = dm.download(&mut wp).await {
                        if config.theme_filter != "any" {
                            if let Ok(b) = tokio::task::spawn_blocking({
                                let p = path.clone();
                                move || brightness::analyze_brightness(&p)
                            }).await {
                                if let Ok(b) = b {
                                    if !brightness::matches_theme(b, config.brightness_threshold, &config.theme_filter) {
                                        continue;
                                    }
                                }
                            }
                        }
                        let _ = background::set_wallpaper(&path, &config.scaling_mode);
                    }
                }
            }

            Some(action) = tray_rx.recv() => {
                match action {
                    TrayAction::ShowWindow => {
                        tracing::info!("Tray: ShowWindow received, launching GUI");
                        use std::os::unix::process::CommandExt;
                        // systemd-run gives the new process proper session context
                        // so Wayland allows it to show a window
                        match std::process::Command::new("systemd-run")
                            .args(["--user", "--scope", "papery"])
                            .process_group(0)
                            .spawn()
                        {
                            Ok(_) => tracing::info!("Tray: GUI process spawned"),
                            Err(e) => tracing::error!("Tray: Failed to spawn GUI: {e}"),
                        }
                    }
                    TrayAction::NextWallpaper => {
                        if queue.is_empty() {
                            fetch_into_queue(&config, &mut queue).await;
                        }
                        if let Some(mut wp) = queue.pop_front() {
                            if let Ok(path) = dm.download(&mut wp).await {
                                let _ = background::set_wallpaper(&path, &config.scaling_mode);
                            }
                        }
                        seconds_left = config.rotation_interval_secs;
                    }
                    TrayAction::TogglePause => {
                        let was_paused = paused.load(Ordering::Relaxed);
                        paused.store(!was_paused, Ordering::Relaxed);
                        if was_paused {
                            seconds_left = config.rotation_interval_secs;
                        }
                    }
                    TrayAction::Quit => {
                        std::process::exit(0);
                    }
                }
            }
        }

        // Reload config periodically
        if let Some(ref h) = config_handler {
            if let Ok(new_config) = PaperyConfig::get_entry(h) {
                if new_config.rotation_interval_secs != config.rotation_interval_secs {
                    seconds_left = new_config.rotation_interval_secs;
                }
                config = new_config;
                paused.store(config.paused, Ordering::Relaxed);
            }
        }
    }
}

async fn fetch_into_queue(config: &PaperyConfig, queue: &mut VecDeque<WallpaperInfo>) {
    let providers = build_providers(config);
    for provider in &providers {
        match provider.fetch_wallpapers(5).await {
            Ok(wps) => queue.extend(wps),
            Err(e) => tracing::warn!("Failed to fetch from {}: {e}", provider.name()),
        }
    }
    // Shuffle
    let mut v: Vec<_> = queue.drain(..).collect();
    use rand::seq::SliceRandom;
    v.shuffle(&mut rand::rng());
    queue.extend(v);
}

fn build_providers(config: &PaperyConfig) -> Vec<Box<dyn WallpaperProvider>> {
    let mut providers: Vec<Box<dyn WallpaperProvider>> = Vec::new();
    if config.source_bing {
        providers.push(Box::new(BingProvider));
    }
    if config.source_nasa {
        providers.push(Box::new(NasaApodProvider));
    }
    if config.source_wallhaven {
        providers.push(Box::new(WallhavenProvider::new(
            &config.wallhaven_categories,
            &config.wallhaven_purity,
        )));
    }
    if config.source_earthview {
        providers.push(Box::new(EarthViewProvider));
    }
    if config.source_local {
        let folders: Vec<PathBuf> = config.local_folders.iter().map(PathBuf::from).collect();
        providers.push(Box::new(LocalProvider::new(folders)));
    }
    providers
}
