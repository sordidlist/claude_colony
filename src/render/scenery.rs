//! Above-ground decorations: barn, dog, trees, clouds, sun/moon. Drawn after
//! the tilemap and before the underground fog so above-ground scenery always
//! reads, but with the day/night tint applied so it integrates with the
//! lighting.

use macroquad::prelude::*;
use crate::config::*;
use crate::sim::scenery::{Decoration, DecorKind, DecorPos};
use crate::sim::TimeOfDay;
use super::{Atlas, Camera};

/// Draw the grass-blade overlay above the surface row. One thin
/// vertical strip per visible column whose `GrassField.length` is
/// non-zero — the lawn shaggies up between mower passes and gets
/// shaved back down whenever the mower rolls through.
pub fn draw_grass_blades(
    grass: &crate::world::GrassField,
    cam:   &Camera,
    tint:  Color,
) {
    let (x0, _, x1, _) = cam.visible_tile_rect();
    // Top edge of the grass row in screen-space — blades grow upward
    // from here, so we anchor at the row's top y.
    let blade_color = Color::new(0.36 * tint.r, 0.84 * tint.g, 0.32 * tint.b, 1.0);
    let blade_dark  = Color::new(0.22 * tint.r, 0.56 * tint.g, 0.18 * tint.b, 1.0);
    let pixel = (cam.zoom / 8.0).max(1.0);
    for x in x0.max(0)..x1.min(crate::config::WORLD_WIDTH) {
        let len = grass.at(x);
        if len == 0 { continue; }
        let blades_h = (len as f32) * pixel;
        let (sx, sy) = cam.world_to_screen(x as f32, SURFACE_ROW as f32);
        // Two blades per tile, slightly offset, so the lawn reads
        // as texture rather than a solid green strip.
        draw_rectangle(sx + pixel * 1.0,
                       sy - blades_h,
                       pixel, blades_h, blade_color);
        draw_rectangle(sx + pixel * 4.0,
                       sy - blades_h * 0.85,
                       pixel, blades_h * 0.85, blade_dark);
        draw_rectangle(sx + pixel * 6.0,
                       sy - blades_h * 0.7,
                       pixel, blades_h * 0.7, blade_color);
    }
}

pub fn draw_scenery(
    world: &mut bevy_ecs::world::World,
    atlas: &Atlas,
    cam:   &Camera,
    tint:  Color,
) {
    let nf = world.resource::<TimeOfDay>().night_factor;

    let mut q = world.query::<(&DecorPos, &Decoration)>();
    for (p, d) in q.iter(world) {
        match d.kind {
            DecorKind::Barn => {
                // 24×16 src rendered at 18×12 tiles → 6× pixel-art scale.
                draw_world_sprite(&atlas.texture, atlas.barn_rect(),
                                  p.x, p.y, 18.0, 12.0, false, tint, cam);
            }
            DecorKind::Tree => {
                // 24×48 src at 3×6 tiles → 1:1.
                draw_world_sprite(&atlas.texture, atlas.tree_rect(),
                                  p.x, p.y, 3.0, 6.0, false, tint, cam);
            }
            DecorKind::Dog => {
                // 8×8 src at 3×3 tiles → 3× pixel-art scale.
                draw_world_sprite(&atlas.texture, atlas.dog_rect(d.frame),
                                  p.x, p.y, 3.0, 3.0,
                                  d.flip_x, tint, cam);
            }
            DecorKind::Mower => {
                // 32×16 source painted at 4×2 tiles (1:1 source-to-screen).
                draw_world_sprite(&atlas.texture, atlas.mower_rect(d.frame),
                                  p.x, p.y, 4.0, 2.0,
                                  d.flip_x, tint, cam);
            }
            DecorKind::Cloud => {
                let cloud_tint = Color::new(
                    tint.r * (1.0 - 0.4 * nf),
                    tint.g * (1.0 - 0.4 * nf),
                    tint.b * (1.0 - 0.2 * nf),
                    0.86,
                );
                draw_world_sprite(&atlas.texture, atlas.cloud_rect(),
                                  p.x - 1.5, p.y,
                                  3.0, 1.0, false, cloud_tint, cam);
            }
            DecorKind::SunMoon => {
                draw_sun_moon(atlas, cam, nf);
            }
        }
    }
}

fn draw_world_sprite(
    tex: &Texture2D, src: Rect,
    wx: f32, wy: f32,
    dest_tiles_w: f32, dest_tiles_h: f32,
    flip_x: bool, tint: Color, cam: &Camera,
) {
    let (sx, sy) = cam.world_to_screen(wx, wy);
    draw_texture_ex(tex, sx, sy, tint, DrawTextureParams {
        dest_size: Some(Vec2::new(dest_tiles_w * cam.zoom,
                                   dest_tiles_h * cam.zoom)),
        source:    Some(src),
        flip_x,
        ..Default::default()
    });
}

/// Sun by day, moon by night. Position arcs across the sky based on time of
/// day. Always drawn at full daylight tint so it stays bright at night.
fn draw_sun_moon(atlas: &Atlas, cam: &Camera, night_factor: f32) {
    use std::f32::consts::PI;
    // Phase: noon (nf=0) → top centre. Midnight (nf=1) → top centre with moon.
    // Use a sweep: angle = pi * (some value) — for variety, compute from the
    // raw time-of-day seconds instead of nf so the sun moves L→R during day.
    // We don't have direct seconds here; approximate from nf with a sign
    // bit derived from a dummy phase counter — simplest approach: place the
    // sun in the upper-right when transitioning into night, upper-left when
    // exiting. nf alone is symmetric, so we just place at zenith for now.
    let zenith_x = WORLD_WIDTH as f32 * 0.5;
    let zenith_y = 4.0;
    let angle = (1.0 - night_factor) * PI;
    let arc_x = zenith_x + (WORLD_WIDTH as f32 * 0.35) * angle.cos();
    let arc_y = zenith_y + 4.0 * (1.0 - angle.sin());
    let (rect, w, h) = if night_factor < 0.5 {
        (atlas.sun_rect(),  8.0, 8.0)
    } else {
        (atlas.moon_rect(), 8.0, 8.0)
    };
    let (sx, sy) = cam.world_to_screen(arc_x, arc_y);
    let scale = cam.zoom / 8.0;
    draw_texture_ex(&atlas.texture, sx, sy, WHITE, DrawTextureParams {
        dest_size: Some(Vec2::new(w * scale, h * scale)),
        source:    Some(rect),
        ..Default::default()
    });
}
