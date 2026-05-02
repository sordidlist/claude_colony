//! Sprite layer. One `draw_texture_ex` call per visible creature. macroquad
//! batches calls that share a texture, so this comes out as one GPU draw per
//! frame for the whole population.

use macroquad::prelude::*;
use crate::sim::components::*;
use super::{Atlas, Camera};

pub fn draw_sprites(
    world: &mut bevy_ecs::world::World,
    atlas: &Atlas,
    cam:   &Camera,
    tint:  Color,
) {
    let (x0, y0, x1, y1) = cam.visible_tile_rect();
    let shadow = Color::new(0.0, 0.0, 0.0, 0.32);

    // Queen royal-glow halo — drawn FIRST so it sits behind every other
    // sprite. Pulses in size and alpha on a slow ~2 Hz cycle, in deep
    // royal purple. Helps the player locate the queen anywhere in the
    // tunnel network even when the camera is elsewhere or fog covers
    // her chamber.
    let total = world.resource::<crate::sim::Time>().total;
    let mut qglow = world.query::<(&Position, &Ant)>();
    for (p, a) in qglow.iter(world) {
        if !matches!(a.kind, AntKind::Queen) { continue; }
        let tx = p.0.x as i32;
        let ty = p.0.y as i32;
        if tx < x0 - 5 || tx > x1 + 5 || ty < y0 - 5 || ty > y1 + 5 { continue; }
        let phase = (total * 2.0).sin() * 0.5 + 0.5;        // 0..1
        let radius_tiles = 2.5 + phase * 1.0;                // 2.5..3.5
        let (sx, sy) = cam.world_to_screen(p.0.x, p.0.y);
        // Outer soft halo
        draw_circle(sx, sy, radius_tiles * cam.zoom,
            Color::new(0.55, 0.18, 0.78, 0.18 + phase * 0.10));
        // Inner brighter ring
        draw_circle(sx, sy, (radius_tiles - 0.8) * cam.zoom,
            Color::new(0.78, 0.36, 0.94, 0.22 + phase * 0.14));
        // Tight bright core
        draw_circle(sx, sy, (radius_tiles - 1.6) * cam.zoom,
            Color::new(0.96, 0.66, 1.0,  0.20 + phase * 0.10));
    }

    // Workers + queen + soldiers
    let mut q = world.query::<(&Position, &VisualState, &Cargo, &Ant)>();
    for (p, v, c, a) in q.iter(world) {
        let tx = p.0.x as i32;
        let ty = p.0.y as i32;
        if tx < x0 - 2 || tx > x1 + 2 || ty < y0 - 2 || ty > y1 + 2 { continue; }
        let flip = v.facing < 0;
        match a.kind {
            AntKind::Worker => {
                draw_ground_shadow(cam, p.0.x, p.0.y + 0.45, 0.7, 0.18, shadow);
                let (sx, sy) = cam.world_to_screen(p.0.x - 0.5, p.0.y - 0.5);
                let src = if c.debris.is_some() {
                    atlas.ant_with_pebble_cell(v.anim_frame)
                } else {
                    atlas.ant_cell(v.anim_frame, c.amount > 0)
                };
                draw_texture_ex(&atlas.texture, sx, sy, tint, DrawTextureParams {
                    dest_size: Some(Vec2::new(cam.zoom, cam.zoom)),
                    source:    Some(src),
                    flip_x:    flip,
                    ..Default::default()
                });
            }
            AntKind::Soldier => {
                draw_ground_shadow(cam, p.0.x, p.0.y + 0.45, 0.9, 0.22, shadow);
                let (sx, sy) = cam.world_to_screen(p.0.x - 0.5, p.0.y - 0.5);
                draw_texture_ex(&atlas.texture, sx, sy, tint, DrawTextureParams {
                    dest_size: Some(Vec2::new(cam.zoom, cam.zoom)),
                    source:    Some(atlas.soldier_cell(v.anim_frame)),
                    flip_x:    flip,
                    ..Default::default()
                });
            }
            AntKind::Queen => {
                draw_ground_shadow(cam, p.0.x, p.0.y + 0.95, 1.4, 0.30, shadow);
                let (sx, sy) = cam.world_to_screen(p.0.x - 1.0, p.0.y - 1.0);
                draw_texture_ex(&atlas.texture, sx, sy, tint, DrawTextureParams {
                    dest_size: Some(Vec2::new(2.0 * cam.zoom, 2.0 * cam.zoom)),
                    source:    Some(atlas.queen_rect()),
                    flip_x:    flip,
                    ..Default::default()
                });
            }
        }
    }

    // Brood entities (eggs / larvae)
    let mut qb = world.query::<(&Position, &Brood)>();
    for (p, _) in qb.iter(world) {
        let tx = p.0.x as i32;
        let ty = p.0.y as i32;
        if tx < x0 - 1 || tx > x1 + 1 || ty < y0 - 1 || ty > y1 + 1 { continue; }
        let (sx, sy) = cam.world_to_screen(p.0.x - 0.5, p.0.y - 0.5);
        draw_texture_ex(&atlas.texture, sx, sy, tint, DrawTextureParams {
            dest_size: Some(Vec2::new(cam.zoom, cam.zoom)),
            source:    Some(atlas.brood_cell()),
            ..Default::default()
        });
    }

    // Corpses
    let mut qc = world.query::<(&Position, &Corpse)>();
    for (p, _) in qc.iter(world) {
        let tx = p.0.x as i32;
        let ty = p.0.y as i32;
        if tx < x0 - 1 || tx > x1 + 1 || ty < y0 - 1 || ty > y1 + 1 { continue; }
        let (sx, sy) = cam.world_to_screen(p.0.x - 0.5, p.0.y - 0.5);
        draw_texture_ex(&atlas.texture, sx, sy, tint, DrawTextureParams {
            dest_size: Some(Vec2::new(cam.zoom, cam.zoom)),
            source:    Some(atlas.corpse_cell()),
            ..Default::default()
        });
    }

    // Spiders — drawn at 2.6×2.6 tiles so they read as a real
    // threat next to a worker (a worker is 1×1; the previous 2×2
    // size made spiders only modestly bigger). Sprite is anchored
    // centre-on-position so the larger render expands outward
    // around the actual collision point, not below it.
    const SPIDER_DRAW_TILES: f32 = 2.6;
    let mut qs = world.query::<(&Position, &VisualState, &Spider)>();
    for (p, v, _) in qs.iter(world) {
        let tx = p.0.x as i32;
        let ty = p.0.y as i32;
        if tx < x0 - 3 || tx > x1 + 3 || ty < y0 - 3 || ty > y1 + 3 { continue; }
        draw_ground_shadow(cam, p.0.x, p.0.y + 1.05, 1.8, 0.32, shadow);
        let half = SPIDER_DRAW_TILES * 0.5;
        let (sx, sy) = cam.world_to_screen(p.0.x - half, p.0.y - half);
        draw_texture_ex(&atlas.texture, sx, sy, tint, DrawTextureParams {
            dest_size: Some(Vec2::new(SPIDER_DRAW_TILES * cam.zoom,
                                       SPIDER_DRAW_TILES * cam.zoom)),
            source:    Some(atlas.spider_rect(v.anim_frame)),
            flip_x:    v.facing < 0,
            ..Default::default()
        });
    }

    // Rival ants — workers at 1× tile, soldiers at 1.4× so the
    // sturdier tier reads as a different threat at any zoom.
    let mut qr = world.query::<(&Position, &VisualState, &RivalAnt)>();
    for (p, v, r) in qr.iter(world) {
        let tx = p.0.x as i32;
        let ty = p.0.y as i32;
        if tx < x0 - 2 || tx > x1 + 2 || ty < y0 - 2 || ty > y1 + 2 { continue; }
        let scale = match r.kind {
            RivalKind::Worker  => 1.0,
            RivalKind::Soldier => 1.4,
        };
        let half = scale * 0.5;
        draw_ground_shadow(cam, p.0.x, p.0.y + 0.45, scale * 0.7, 0.18 * scale, shadow);
        let (sx, sy) = cam.world_to_screen(p.0.x - half, p.0.y - half);
        draw_texture_ex(&atlas.texture, sx, sy, tint, DrawTextureParams {
            dest_size: Some(Vec2::new(scale * cam.zoom, scale * cam.zoom)),
            source:    Some(atlas.rival_ant_cell(v.anim_frame, false)),
            flip_x:    v.facing < 0,
            ..Default::default()
        });
    }

    // Food pellets
    let mut qf = world.query::<(&Position, &Food)>();
    for (p, _) in qf.iter(world) {
        let tx = p.0.x as i32;
        let ty = p.0.y as i32;
        if tx < x0 || tx > x1 || ty < y0 || ty > y1 { continue; }
        let (sx, sy) = cam.world_to_screen(p.0.x - 0.5, p.0.y - 0.5);
        let src = atlas.food_cell();
        draw_texture_ex(&atlas.texture, sx, sy, tint, DrawTextureParams {
            dest_size: Some(Vec2::new(cam.zoom, cam.zoom)),
            source:    Some(src),
            ..Default::default()
        });
    }
}

/// Flat oval shadow on the ground — classic SNES sprite anchoring trick that
/// stops creatures from looking like they're floating against the tilemap.
fn draw_ground_shadow(cam: &Camera, wx: f32, wy: f32,
                      tile_w: f32, tile_h: f32, color: Color) {
    let (cx, cy) = cam.world_to_screen(wx, wy);
    let w = tile_w * cam.zoom;
    let h = tile_h * cam.zoom;
    draw_ellipse(cx, cy, w * 0.5, h * 0.5, 0.0, color);
}
