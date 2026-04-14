use std::io::Write;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let _ = tracing_log::LogTracer::init();
    tracing::info!("Starting Papery v{VERSION}");

    let bg_mode = std::env::args().any(|a| a == "--bg");

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
