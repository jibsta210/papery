use std::path::Path;

/// Analyze the brightness of an image on a scale from 0.0 (dark) to 1.0 (light).
pub fn analyze_brightness(image_path: &Path) -> Result<f64, image::ImageError> {
    let img = image::open(image_path)?;
    let small = img.thumbnail(64, 64);
    let gray = small.to_luma8();
    let total: u64 = gray.pixels().map(|p| p.0[0] as u64).sum();
    let count = gray.pixels().count() as u64;
    if count == 0 {
        return Ok(0.5);
    }
    Ok(total as f64 / (count as f64 * 255.0))
}

/// Check if a wallpaper's brightness matches the desired theme filter.
pub fn matches_theme(brightness: f64, threshold: f64, filter: &str) -> bool {
    match filter {
        "light" => brightness > threshold,
        "dark" => brightness <= threshold,
        _ => true, // "any"
    }
}
