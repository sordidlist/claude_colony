//! Sprite layer. One `draw_texture_ex` call per visible creature. macroquad
//! batches calls that share a texture, so this comes out as one GPU draw per
//! frame for the whole population.

use macroquad::prelude::*;
use colony::sim::components::*;
use super::{Atlas, Camera};

pub fn draw_sprites(
    world: &mut bevy_ecs::world::World,
    atlas: &Atlas,
    cam:   &Camera,
    tint:  Color,
) {
    let (x0, y0, x1, y1) = cam.visible_tile_rect();
    let shadow = Color::new(0.0, 0.0, 0.0, 0.32);

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

    // Spiders — 2×2 tiles, anchored centre-on-position.
    let mut qs = world.query::<(&Position, &VisualState, &Spider)>();
    for (p, v, _) in qs.iter(world) {
        let tx = p.0.x as i32;
        let ty = p.0.y as i32;
        if tx < x0 - 2 || tx > x1 + 2 || ty < y0 - 2 || ty > y1 + 2 { continue; }
        draw_ground_shadow(cam, p.0.x, p.0.y + 0.95, 1.4, 0.28, shadow);
        let (sx, sy) = cam.world_to_screen(p.0.x - 1.0, p.0.y - 1.0);
        draw_texture_ex(&atlas.texture, sx, sy, tint, DrawTextureParams {
            dest_size: Some(Vec2::new(2.0 * cam.zoom, 2.0 * cam.zoom)),
            source:    Some(atlas.spider_rect(v.anim_frame)),
            flip_x:    v.facing < 0,
            ..Default::default()
        });
    }

    // Rival ants — same size as workers, red sprite.
    let mut qr = world.query::<(&Position, &VisualState, &RivalAnt)>();
    for (p, v, _) in qr.iter(world) {
        let tx = p.0.x as i32;
        let ty = p.0.y as i32;
        if tx < x0 - 1 || tx > x1 + 1 || ty < y0 - 1 || ty > y1 + 1 { continue; }
        draw_ground_shadow(cam, p.0.x, p.0.y + 0.45, 0.7, 0.18, shadow);
        let (sx, sy) = cam.world_to_screen(p.0.x - 0.5, p.0.y - 0.5);
        draw_texture_ex(&atlas.texture, sx, sy, tint, DrawTextureParams {
            dest_size: Some(Vec2::new(cam.zoom, cam.zoom)),
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
