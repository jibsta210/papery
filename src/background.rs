use cosmic_bg_config::{Entry, FilterMethod, SamplingMethod, ScalingMode, Source};
use std::path::Path;

#[derive(Debug)]
pub enum BackgroundError {
    Config(cosmic_config::Error),
}

impl std::fmt::Display for BackgroundError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Config(e) => write!(f, "Config error: {e}"),
        }
    }
}

impl From<cosmic_config::Error> for BackgroundError {
    fn from(e: cosmic_config::Error) -> Self {
        Self::Config(e)
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
    let context = cosmic_bg_config::context()?;

    let entry = Entry::new("all".to_string(), Source::Path(image_path.to_path_buf()))
        .filter_by_theme(false)
        .rotation_frequency(0)
        .filter_method(FilterMethod::Lanczos)
        .scaling_mode(scaling_mode_from_str(scaling))
        .sampling_method(SamplingMethod::Alphanumeric);

    let mut config = cosmic_bg_config::Config::load(&context)?;
    config.set_entry(&context, entry)?;

    tracing::info!("Wallpaper set to: {}", image_path.display());
    Ok(())
}
