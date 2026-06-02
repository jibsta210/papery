//! Cross-desktop wallpaper backend.
//!
//! At runtime we detect the active desktop environment from XDG env vars and
//! pick the appropriate mechanism to set the desktop background:
//!
//! - **COSMIC**: write to `cosmic-bg-config` (cosmic-bg picks it up via inotify)
//! - **KDE Plasma**: shell out to `plasma-apply-wallpaperimage`
//! - **GNOME/other**: shell out to `gsettings` (best-effort)

use cosmic_bg_config::{Entry, FilterMethod, SamplingMethod, ScalingMode, Source};
use std::path::Path;

#[derive(Debug)]
pub enum BackgroundError {
    Config(cosmic_config::Error),
    Command(String),
}

impl std::fmt::Display for BackgroundError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Config(e) => write!(f, "Config error: {e}"),
            Self::Command(e) => write!(f, "Command error: {e}"),
        }
    }
}

impl From<cosmic_config::Error> for BackgroundError {
    fn from(e: cosmic_config::Error) -> Self {
        Self::Config(e)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Desktop {
    Cosmic,
    Kde,
    Gnome,
    Other,
}

fn detect_desktop() -> Desktop {
    // Allow explicit override for testing.
    if let Ok(v) = std::env::var("PAPERY_DESKTOP") {
        match v.to_lowercase().as_str() {
            "cosmic" => return Desktop::Cosmic,
            "kde" | "plasma" => return Desktop::Kde,
            "gnome" => return Desktop::Gnome,
            _ => {}
        }
    }

    let xdg = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    let session = std::env::var("XDG_SESSION_DESKTOP").unwrap_or_default();
    let combined = format!("{xdg}:{session}").to_lowercase();

    if combined.contains("cosmic") {
        Desktop::Cosmic
    } else if combined.contains("kde") || combined.contains("plasma") {
        Desktop::Kde
    } else if combined.contains("gnome") {
        Desktop::Gnome
    } else {
        Desktop::Other
    }
}

pub fn scaling_mode_from_str(s: &str) -> ScalingMode {
    match s {
        "fit" => ScalingMode::Fit([0.0, 0.0, 0.0]),
        "stretch" => ScalingMode::Stretch,
        _ => ScalingMode::Zoom,
    }
}

pub fn set_wallpaper(image_path: &Path, scaling: &str) -> Result<(), BackgroundError> {
    // Always publish the current path so the KDE wallpaper plugin (and any
    // other consumer) can render it directly, regardless of which backend
    // ended up handling the system-level set.
    write_current_path(image_path);

    match detect_desktop() {
        Desktop::Cosmic => set_wallpaper_cosmic(image_path, scaling),
        Desktop::Kde => set_wallpaper_kde(image_path, scaling),
        Desktop::Gnome => set_wallpaper_gnome(image_path),
        Desktop::Other => set_wallpaper_cosmic(image_path, scaling),
    }
}

/// Publish the active wallpaper to ~/.cache/papery/ for the KDE Papery
/// wallpaper plugin (and anything else watching). Writes both:
///   - current_path  (text file with the wallpaper path)
///   - current.jpg   (symlink to the wallpaper, polled by the QML plugin)
fn write_current_path(image_path: &Path) {
    let Some(dirs) = directories::BaseDirs::new() else {
        return;
    };
    let papery_cache = dirs.cache_dir().join("papery");
    let _ = std::fs::create_dir_all(&papery_cache);

    // Text path for diagnostic / scripted consumers.
    let state = papery_cache.join("current_path");
    let tmp = papery_cache.join("current_path.tmp");
    let content = image_path.to_string_lossy().into_owned();
    if std::fs::write(&tmp, &content).is_ok() {
        let _ = std::fs::rename(&tmp, &state);
    }

    // Symlink the QML plugin loads directly. Use a stable filename so the
    // plugin can bind to a fixed file:// URL and only cache-bust on changes.
    let symlink_path = papery_cache.join("current.jpg");
    let _ = std::fs::remove_file(&symlink_path);
    let _ = std::os::unix::fs::symlink(image_path, &symlink_path);
}

fn set_wallpaper_cosmic(image_path: &Path, scaling: &str) -> Result<(), BackgroundError> {
    let context = cosmic_bg_config::context()?;

    let entry = Entry::new("all".to_string(), Source::Path(image_path.to_path_buf()))
        .filter_by_theme(false)
        .rotation_frequency(0)
        .filter_method(FilterMethod::Lanczos)
        .scaling_mode(scaling_mode_from_str(scaling))
        .sampling_method(SamplingMethod::Alphanumeric);

    let mut config = cosmic_bg_config::Config::load(&context)?;
    config.set_entry(&context, entry)?;

    tracing::info!("Wallpaper set (cosmic): {}", image_path.display());
    Ok(())
}

fn set_wallpaper_kde(image_path: &Path, scaling: &str) -> Result<(), BackgroundError> {
    // If the user has selected the Papery wallpaper plugin in Plasma, do NOT
    // call plasma-apply-wallpaperimage or our scaling qdbus script — both
    // would silently switch the plugin back to org.kde.image and the user's
    // selection would revert on every rotation. The plugin reads
    // ~/.cache/papery/current_path itself and refreshes live.
    let primary_result = if plasma_wallpaper_plugin_is_papery() {
        tracing::info!(
            "Plasma is using the Papery plugin; wrote current_path={}",
            image_path.display()
        );
        Ok(())
    } else {
        let out = std::process::Command::new("plasma-apply-wallpaperimage")
            .arg(image_path)
            .output();
        match out {
            Ok(o) if o.status.success() => {
                tracing::info!("Wallpaper set (kde): {}", image_path.display());
                let _ = apply_kde_scaling(scaling);
                Ok(())
            }
            Ok(o) => {
                tracing::warn!(
                    "plasma-apply-wallpaperimage failed (status {:?}): {}; falling back to qdbus6",
                    o.status.code(),
                    String::from_utf8_lossy(&o.stderr)
                );
                set_wallpaper_kde_qdbus(image_path, scaling)
            }
            Err(e) => {
                tracing::warn!("plasma-apply-wallpaperimage not available ({e}); using qdbus6");
                set_wallpaper_kde_qdbus(image_path, scaling)
            }
        }
    };

    // Best-effort: also update the lock screen image. Only if it's currently
    // configured to use the org.kde.image plugin (the user hasn't picked a
    // dynamic plugin like dev.papery.wallpaper or org.kde.potd themselves).
    let _ = set_lockscreen_kde(image_path, scaling);

    // Best-effort: refresh the SDDM login background if the helper is set up.
    let _ = set_login_screen_kde(image_path);

    primary_result
}

/// Update the lock-screen wallpaper. If the user has already switched the
/// lock screen to our Papery plugin, do nothing (the plugin reads the
/// current path itself). Otherwise update the org.kde.image plugin's Image
/// and FillMode keys.
fn set_lockscreen_kde(image_path: &Path, scaling: &str) -> Result<(), BackgroundError> {
    let current_plugin = std::process::Command::new("kreadconfig6")
        .args(["--file", "kscreenlockerrc", "--group", "Greeter", "--key", "WallpaperPlugin"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    if current_plugin == "dev.papery.wallpaper" {
        // Plugin already handles this — reads current_path live.
        return Ok(());
    }

    let uri = format!("file://{}", image_path.display());
    let fill = kde_fill_mode(scaling).to_string();

    let _ = std::process::Command::new("kwriteconfig6")
        .args(&[
            "--file", "kscreenlockerrc",
            "--group", "Greeter",
            "--key", "WallpaperPlugin",
            "org.kde.image",
        ])
        .output();
    let _ = std::process::Command::new("kwriteconfig6")
        .args(&[
            "--file", "kscreenlockerrc",
            "--group", "Greeter",
            "--group", "Wallpaper",
            "--group", "org.kde.image",
            "--group", "General",
            "--key", "Image",
            &uri,
        ])
        .output();
    let _ = std::process::Command::new("kwriteconfig6")
        .args(&[
            "--file", "kscreenlockerrc",
            "--group", "Greeter",
            "--group", "Wallpaper",
            "--group", "org.kde.image",
            "--group", "General",
            "--key", "FillMode",
            &fill,
        ])
        .output();
    Ok(())
}

/// Copy the current wallpaper to a system-readable path so the SDDM login
/// screen can render it. Requires a one-time root setup: the user runs
/// `sudo papery --setup-sddm` which installs a writable cache directory
/// (or a polkit rule). Without that setup, this is a no-op.
fn set_login_screen_kde(image_path: &Path) -> Result<(), BackgroundError> {
    let sddm_target = std::path::PathBuf::from("/var/lib/papery/current.jpg");
    if let Some(parent) = sddm_target.parent() {
        if !parent.exists() {
            // Setup hasn't been done — silent no-op.
            return Ok(());
        }
    }
    // Try to copy. If we lack permission, silently ignore.
    let _ = std::fs::copy(image_path, &sddm_target);
    Ok(())
}

/// Return true if any Plasma containment is currently using the Papery
/// wallpaper plugin. If so, we should NOT call plasma-apply-wallpaperimage
/// or write `wallpaperPlugin = 'org.kde.image'` because that overrides the
/// user's choice and the setting silently reverts on every rotation.
fn plasma_wallpaper_plugin_is_papery() -> bool {
    let Some(dirs) = directories::BaseDirs::new() else {
        return false;
    };
    let path = dirs
        .config_dir()
        .join("plasma-org.kde.plasma.desktop-appletsrc");
    let Ok(content) = std::fs::read_to_string(&path) else {
        return false;
    };
    // The active wallpaper plugin is recorded per Containment in lines like
    //   wallpaperplugin=dev.papery.wallpaper
    // (the key casing is lowercase in this config file).
    content.lines().any(|l| {
        let lower = l.trim().to_ascii_lowercase();
        lower == "wallpaperplugin=dev.papery.wallpaper"
    })
}

fn kde_fill_mode(scaling: &str) -> i32 {
    // Plasma org.kde.image FillMode values:
    //   0 = Stretch
    //   1 = Preserve aspect fit  (Fit)
    //   2 = Preserve aspect crop (Zoom)
    //   3 = Tiled
    //   4 = Centered
    //   5 = Pad
    match scaling {
        "stretch" => 0,
        "fit" => 1,
        _ => 2, // "zoom"
    }
}

fn apply_kde_scaling(scaling: &str) -> Result<(), BackgroundError> {
    let fill = kde_fill_mode(scaling);
    let script = format!(
        "var all = desktops();
         for (i = 0; i < all.length; i++) {{
             var d = all[i];
             d.wallpaperPlugin = 'org.kde.image';
             d.currentConfigGroup = ['Wallpaper', 'org.kde.image', 'General'];
             d.writeConfig('FillMode', {fill});
         }}"
    );
    let out = std::process::Command::new("qdbus6")
        .args([
            "org.kde.plasmashell",
            "/PlasmaShell",
            "org.kde.PlasmaShell.evaluateScript",
            &script,
        ])
        .output()
        .map_err(|e| BackgroundError::Command(e.to_string()))?;
    if !out.status.success() {
        tracing::warn!(
            "FillMode update failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(())
}

fn set_wallpaper_kde_qdbus(image_path: &Path, scaling: &str) -> Result<(), BackgroundError> {
    let fill = kde_fill_mode(scaling);
    let path = image_path.to_string_lossy().replace('\'', "\\'");
    let script = format!(
        "var all = desktops();
         for (i = 0; i < all.length; i++) {{
             var d = all[i];
             d.wallpaperPlugin = 'org.kde.image';
             d.currentConfigGroup = ['Wallpaper', 'org.kde.image', 'General'];
             d.writeConfig('Image', 'file://{path}');
             d.writeConfig('FillMode', {fill});
         }}"
    );
    let out = std::process::Command::new("qdbus6")
        .args([
            "org.kde.plasmashell",
            "/PlasmaShell",
            "org.kde.PlasmaShell.evaluateScript",
            &script,
        ])
        .output()
        .map_err(|e| BackgroundError::Command(e.to_string()))?;
    if !out.status.success() {
        return Err(BackgroundError::Command(format!(
            "qdbus6 evaluateScript failed: {}",
            String::from_utf8_lossy(&out.stderr)
        )));
    }
    tracing::info!("Wallpaper set (kde/qdbus): {}", image_path.display());
    Ok(())
}

fn set_wallpaper_gnome(image_path: &Path) -> Result<(), BackgroundError> {
    let uri = format!("file://{}", image_path.display());
    // Set both light and dark schemes so it works regardless of theme.
    for key in [
        "picture-uri",
        "picture-uri-dark",
    ] {
        let out = std::process::Command::new("gsettings")
            .args(["set", "org.gnome.desktop.background", key, &uri])
            .output()
            .map_err(|e| BackgroundError::Command(e.to_string()))?;
        if !out.status.success() {
            return Err(BackgroundError::Command(format!(
                "gsettings {key} failed: {}",
                String::from_utf8_lossy(&out.stderr)
            )));
        }
    }
    tracing::info!("Wallpaper set (gnome): {}", image_path.display());
    Ok(())
}
