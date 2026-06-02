use std::io::Write;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let _ = tracing_log::LogTracer::init();
    tracing::info!("Starting Papery v{VERSION}");

    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--setup-sddm") {
        return setup_sddm();
    }
    let bg_mode = args.iter().any(|a| a == "--bg");

    // Single instance check via lock file
    let run_dir = directories::BaseDirs::new()
        .map(|d| d.runtime_dir().unwrap_or(d.cache_dir()).to_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"));
    let lock_path = run_dir.join("papery.lock");

    if let Ok(contents) = std::fs::read_to_string(&lock_path) {
        if let Ok(pid) = contents.trim().parse::<u32>() {
            if std::path::Path::new(&format!("/proc/{pid}")).exists() {
                if bg_mode {
                    // A GUI instance is running, it handles everything — just exit
                    std::process::exit(0);
                }
                // Kill the background daemon so GUI takes over
                unsafe { libc::kill(pid as i32, libc::SIGTERM); }
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
        }
    }
    let mut f = std::fs::File::create(&lock_path)?;
    write!(f, "{}", std::process::id())?;
    drop(f);

    if bg_mode {
        papery::daemon::run_background();
    } else {
        let settings = cosmic::app::Settings::default()
            .size(cosmic::iced::Size::new(900.0, 600.0));

        cosmic::app::run::<papery::app::Papery>(settings, papery::app::PaperyFlags)?;

        // GUI closed — spawn background daemon to keep rotating
        let _ = std::process::Command::new("papery")
            .arg("--bg")
            .spawn();
    }

    let _ = std::fs::remove_file(&lock_path);
    Ok(())
}

/// One-time root setup that wires the SDDM login screen to Papery's
/// current wallpaper. Creates /var/lib/papery/ (writable by the user)
/// and symlinks the current SDDM theme's background file to a stable
/// path inside it.
fn setup_sddm() -> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::PermissionsExt;

    if unsafe { libc::geteuid() } != 0 {
        eprintln!("--setup-sddm needs to run as root, e.g.:  sudo papery --setup-sddm");
        std::process::exit(1);
    }

    let real_user = std::env::var("SUDO_USER")
        .ok()
        .filter(|s| !s.is_empty())
        .ok_or("Could not determine the invoking user (set SUDO_USER)")?;

    // The cache dir owned by the user, writable from their session.
    let papery_state = std::path::PathBuf::from("/var/lib/papery");
    std::fs::create_dir_all(&papery_state)?;
    // Make it writable by the user via a passwd lookup
    let passwd_entry = std::fs::read_to_string("/etc/passwd")?;
    let uid_gid = passwd_entry
        .lines()
        .find_map(|l| {
            let f: Vec<&str> = l.split(':').collect();
            if f.first() == Some(&real_user.as_str()) {
                Some((f.get(2)?.parse::<u32>().ok()?, f.get(3)?.parse::<u32>().ok()?))
            } else {
                None
            }
        })
        .ok_or("User not found in /etc/passwd")?;
    unsafe {
        libc::chown(
            std::ffi::CString::new(papery_state.to_string_lossy().as_bytes())?.as_ptr(),
            uid_gid.0,
            uid_gid.1,
        );
    }
    let perms = std::fs::Permissions::from_mode(0o755);
    std::fs::set_permissions(&papery_state, perms)?;

    // Discover the current SDDM theme.
    let theme = std::fs::read_to_string("/etc/sddm.conf.d/kde_settings.conf")
        .or_else(|_| std::fs::read_to_string("/etc/sddm.conf"))
        .ok()
        .and_then(|s| {
            s.lines()
                .find_map(|l| l.strip_prefix("Current=").map(|v| v.trim().to_string()))
        })
        .unwrap_or_else(|| "breeze".to_string());

    let theme_dir = std::path::PathBuf::from(format!("/usr/share/sddm/themes/{theme}"));
    if !theme_dir.exists() {
        eprintln!("SDDM theme '{theme}' not found at {}", theme_dir.display());
        eprintln!(
            "Papery wrote the active wallpaper to /var/lib/papery/current.jpg,"
        );
        eprintln!(
            "but you'll need to manually point your theme's Background= setting at it."
        );
        return Ok(());
    }

    // Most themes have a Backgrounds/ subdir with a single image. Symlink
    // whichever file the theme's theme.conf points to.
    let theme_conf = theme_dir.join("theme.conf");
    let bg_rel = std::fs::read_to_string(&theme_conf)
        .ok()
        .and_then(|s| {
            s.lines().find_map(|l| {
                l.trim()
                    .strip_prefix("Background=")
                    .map(|v| v.trim().trim_matches('"').to_string())
            })
        });

    let target = match bg_rel {
        Some(rel) => theme_dir.join(rel),
        None => theme_dir.join("Backgrounds/desktop-wp.jpg"),
    };

    // Back up the original once, then replace with a symlink.
    let backup = target.with_extension("orig");
    if !backup.exists() && target.exists() {
        std::fs::rename(&target, &backup)?;
    } else if target.is_symlink() {
        std::fs::remove_file(&target)?;
    }
    std::os::unix::fs::symlink("/var/lib/papery/current.jpg", &target)?;

    println!("SDDM login background now follows Papery.");
    println!(
        "  Theme:       {theme}\n  Symlinked:   {}\n  Original backup: {}",
        target.display(),
        backup.display()
    );
    println!(
        "Papery will copy each new wallpaper to /var/lib/papery/current.jpg on rotation."
    );
    Ok(())
}
