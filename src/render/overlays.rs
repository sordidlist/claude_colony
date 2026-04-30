//! Pheromone + dig-marker overlays. Drawn after tiles, before sprites.

use macroquad::prelude::*;
use colony::world::{PheromoneGrid, PheromoneChannel, DigJobs};
use super::{Atlas, Camera};

pub fn draw_pheromones(phero: &PheromoneGrid, cam: &Camera) {
    let (x0, y0, x1, y1) = cam.visible_tile_rect();
    let z = cam.zoom;
    for y in y0.max(0)..y1.min(phero.height) {
        for x in x0.max(0)..x1.min(phero.width) {
            let f = phero.level(x, y, PheromoneChannel::Food);
            let r = phero.level(x, y, PheromoneChannel::Return);
            let e = phero.level(x, y, PheromoneChannel::Explore);
            let a = phero.level(x, y, PheromoneChannel::Alarm);
            // Dominant channel wins. We render at low alpha so tiles show through.
            let max_v = f.max(r).max(e).max(a);
            if max_v < 8.0 { continue; }
            let alpha = (max_v / 80.0).min(0.6);
            let c = if a == max_v {
                Color::new(1.0, 0.15, 0.15, alpha)
            } else if f == max_v {
                Color::new(1.0, 0.86, 0.0, alpha)
            } else if r == max_v {
                Color::new(0.0, 0.78, 1.0, alpha)
            } else {
                Color::new(0.7, 0.31, 1.0, alpha)
            };
            let (sx, sy) = cam.world_to_screen(x as f32, y as f32);
            draw_rectangle(sx, sy, z, z, c);
        }
    }
}

pub fn draw_dig_markers(jobs: &DigJobs, atlas: &Atlas, cam: &Camera) {
    let (x0, y0, x1, y1) = cam.visible_tile_rect();
    for (tx, ty, progress, claimed) in jobs.iter_jobs() {
        if tx < x0 || tx > x1 || ty < y0 || ty > y1 { continue; }
        let (sx, sy) = cam.world_to_screen(tx as f32, ty as f32);
        let src = atlas.dig_marker_cell();
        let tint = if claimed {
            Color::new(1.0, 1.0, 1.0, 1.0)
        } else {
            Color::new(0.85, 0.85, 0.85, 0.85)
        };
        draw_texture_ex(&atlas.texture, sx, sy, tint, DrawTextureParams {
            dest_size: Some(Vec2::new(cam.zoom, cam.zoom)),
            source:    Some(src),
            ..Default::default()
        });
        // Progress bar under the marker
        if progress > 0.01 {
            let w = cam.zoom;
            draw_rectangle(sx, sy + cam.zoom * 0.85,
                           w * progress.clamp(0.0f32, 1.0),
                           cam.zoom * 0.12,
                           Color::new(1.0, 0.78, 0.2, 0.9));
        }
    }
}
