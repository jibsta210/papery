use crate::background;
use crate::brightness;
use crate::config::{PaperyConfig, APP_ID};
use crate::download::DownloadManager;
use crate::tray::{self, TrayAction};
use crate::wallpaper::bing::BingProvider;
use crate::wallpaper::earth_view::EarthViewProvider;
use crate::wallpaper::local::LocalProvider;
use crate::wallpaper::nasa_apod::NasaApodProvider;
use crate::wallpaper::pexels::PexelsProvider;
use crate::wallpaper::unsplash::UnsplashProvider;
use crate::wallpaper::wallhaven::WallhavenProvider;
use crate::wallpaper::{WallpaperInfo, WallpaperProvider};
use cosmic_config::CosmicConfigEntry;
use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Cap on remembered URLs to avoid unbounded growth.
const SEEN_URLS_CAP: usize = 1000;

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
    let mut seen_urls: HashSet<String> = HashSet::new();
    let mut total_shown: u64 = 0;
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

                if let Some(mut wp) = next_unseen(&config, &mut queue, &seen_urls).await {
                    match dm.download(&mut wp).await {
                        Ok(path) => {
                            // Skip broken/all-gray placeholder images
                            let broken_path = path.clone();
                            let is_broken = tokio::task::spawn_blocking(move || brightness::is_broken(&broken_path))
                                .await
                                .unwrap_or(false);
                            if is_broken {
                                tracing::info!("Skipping broken/blank wallpaper: {}", wp.title);
                                mark_seen(&mut seen_urls, &wp);
                                continue;
                            }

                            if config.theme_filter != "any" {
                                let skip = match tokio::task::spawn_blocking({
                                    let p = path.clone();
                                    move || brightness::analyze_brightness(&p)
                                }).await {
                                    Ok(Ok(b)) => !brightness::matches_theme(b, config.brightness_threshold, &config.theme_filter),
                                    Ok(Err(e)) => { tracing::warn!("Brightness analysis failed: {e}"); false }
                                    Err(e) => { tracing::warn!("Brightness task panicked: {e}"); false }
                                };
                                if skip { continue; }
                            }
                            mark_seen(&mut seen_urls, &wp);
                            total_shown += 1;
                            tray::set_counter(total_shown);
                            tracing::info!("Wallpaper #{total_shown}: {}", wp.title);
                            let _ = background::set_wallpaper(&path, &config.scaling_mode);
                        }
                        Err(e) => tracing::warn!("Download failed: {e}"),
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
                        if let Some(mut wp) = next_unseen(&config, &mut queue, &seen_urls).await {
                            if let Ok(path) = dm.download(&mut wp).await {
                                mark_seen(&mut seen_urls, &wp);
                                total_shown += 1;
                                tray::set_counter(total_shown);
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
        match provider.fetch_wallpapers(15).await {
            Ok(wps) => queue.extend(wps),
            Err(e) => tracing::warn!("Failed to fetch from {}: {e}", provider.name()),
        }
    }
    let mut v: Vec<_> = queue.drain(..).collect();
    use rand::seq::SliceRandom;
    v.shuffle(&mut rand::rng());
    queue.extend(v);
}

/// Pop the next wallpaper from the queue that hasn't been shown yet.
/// Refetches up to 3 times if the queue is empty or full of duplicates.
async fn next_unseen(
    config: &PaperyConfig,
    queue: &mut VecDeque<WallpaperInfo>,
    seen: &HashSet<String>,
) -> Option<WallpaperInfo> {
    for _ in 0..3 {
        while let Some(wp) = queue.pop_front() {
            let key = wallpaper_key(&wp);
            if !seen.contains(&key) {
                return Some(wp);
            }
        }
        fetch_into_queue(config, queue).await;
        if queue.is_empty() {
            return None;
        }
    }
    // All sources exhausted with seen wallpapers — return any.
    queue.pop_front()
}

fn wallpaper_key(wp: &WallpaperInfo) -> String {
    if !wp.url.is_empty() {
        wp.url.clone()
    } else if let Some(ref p) = wp.local_path {
        p.to_string_lossy().to_string()
    } else {
        wp.title.clone()
    }
}

fn mark_seen(seen: &mut HashSet<String>, wp: &WallpaperInfo) {
    if seen.len() >= SEEN_URLS_CAP {
        // Reset when full so we don't permanently exhaust sources.
        seen.clear();
    }
    seen.insert(wallpaper_key(wp));
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
            &config.wallhaven_categories_str(),
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
    if config.source_unsplash {
        providers.push(Box::new(UnsplashProvider::new(&config.unsplash_topic)));
    }
    if config.source_pexels && !config.pexels_api_key.is_empty() {
        providers.push(Box::new(PexelsProvider::new(&config.pexels_api_key)));
    }
    providers
}
