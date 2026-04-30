pub mod atlas;
pub mod camera;
pub mod tilemap;
pub mod sprites;
pub mod overlays;
pub mod ui;
pub mod fog;
pub mod scenery;
pub mod sky;

pub use atlas::Atlas;
pub use camera::Camera;
pub use tilemap::TileMapRenderer;
pub use fog::FogRenderer;
pub use sky::SkyRenderer;

use macroquad::prelude::Color;

/// Map a 0..1 night factor to a colour-multiplication tint. 0 = full daylight
/// (white), 1 = midnight (deep blue). Used by tile + sprite layers so the
/// world reads as time-of-day without a per-pixel post pass.
pub fn day_tint(night_factor: f32) -> Color {
    let nf = night_factor.clamp(0.0, 1.0);
    Color::new(
        1.0 - nf * 0.70,
        1.0 - nf * 0.60,
        1.0 - nf * 0.30,
        1.0,
    )
}

/// Sky background colour at the current night factor.
pub fn sky_color(night_factor: f32) -> Color {
    let nf = night_factor.clamp(0.0, 1.0);
    let day   = (0.42, 0.65, 0.92);
    let night = (0.04, 0.04, 0.10);
    Color::new(
        day.0 * (1.0 - nf) + night.0 * nf,
        day.1 * (1.0 - nf) + night.1 * nf,
        day.2 * (1.0 - nf) + night.2 * nf,
        1.0,
    )
}
