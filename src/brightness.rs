use std::path::Path;

/// Analyze the brightness of an image on a scale from 0.0 (dark) to 1.0 (light).
pub fn analyze_brightness(image_path: &Path) -> Result<f64, image::ImageError> {
    // Guard against excessively large files (>100MB) that could OOM during decode
    if let Ok(meta) = std::fs::metadata(image_path) {
        if meta.len() > 100 * 1024 * 1024 {
            return Ok(0.5); // assume neutral brightness for huge files
        }
    }
    let img = image::open(image_path)?;
    // Thumbnail to 64x64 first to cap memory usage regardless of input resolution
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

/// Detect "broken" wallpapers — single-color, near-blank, or all-gray images
/// that often slip through wallpaper APIs (placeholder/error images).
///
/// Returns true if the image looks broken and should be skipped.
pub fn is_broken(image_path: &std::path::Path) -> bool {
    // Tiny files are almost certainly placeholders / error pages
    if let Ok(meta) = std::fs::metadata(image_path) {
        if meta.len() < 10 * 1024 {
            return true;
        }
    }

    let img = match image::open(image_path) {
        Ok(i) => i,
        Err(_) => return true,
    };

    // Sample a small thumbnail so we can do real work on large wallpapers
    let small = img.thumbnail(64, 64);
    let rgb = small.to_rgb8();
    let count = rgb.pixels().count() as u64;
    if count == 0 {
        return true;
    }

    // Compute per-channel mean and stddev
    let mut sum_r = 0u64;
    let mut sum_g = 0u64;
    let mut sum_b = 0u64;
    for p in rgb.pixels() {
        sum_r += p.0[0] as u64;
        sum_g += p.0[1] as u64;
        sum_b += p.0[2] as u64;
    }
    let mean_r = sum_r as f64 / count as f64;
    let mean_g = sum_g as f64 / count as f64;
    let mean_b = sum_b as f64 / count as f64;

    let mut var_r = 0.0;
    let mut var_g = 0.0;
    let mut var_b = 0.0;
    for p in rgb.pixels() {
        var_r += (p.0[0] as f64 - mean_r).powi(2);
        var_g += (p.0[1] as f64 - mean_g).powi(2);
        var_b += (p.0[2] as f64 - mean_b).powi(2);
    }
    let std_r = (var_r / count as f64).sqrt();
    let std_g = (var_g / count as f64).sqrt();
    let std_b = (var_b / count as f64).sqrt();
    let avg_std = (std_r + std_g + std_b) / 3.0;

    // Variance below ~8 (out of 255) means almost no detail — broken/placeholder.
    // Real photos easily exceed 30+.
    avg_std < 8.0
}
