//! Tile layer renderer.
//!
//! The whole tile grid is baked once into a single `Image` (one pixel per
//! tile-pixel) and uploaded as a `Texture2D`. Each frame we draw it as a
//! single textured quad, scaled by camera zoom. When tiles change (a dig
//! happens, water moves), `TileGrid::dirty` flips and we re-bake.
//!
//! This trades GPU bandwidth for one draw call instead of 16k per frame.

use macroquad::prelude::*;
use crate::config::*;
use crate::world::{TileGrid, TileType};
use super::atlas::Atlas;

pub struct TileMapRenderer {
    image:   Image,
    texture: Texture2D,
}

impl TileMapRenderer {
    pub fn new(grid: &TileGrid, atlas: &Atlas) -> Self {
        let w = (grid.width  as u16) * 8;
        let h = (grid.height as u16) * 8;
        // Init transparent so above-ground scenery + sky show through where
        // tiles are Air.
        let mut image = Image::gen_image_color(w, h, Color::new(0.0, 0.0, 0.0, 0.0));
        bake_full(&mut image, grid, atlas);
        let texture = Texture2D::from_image(&image);
        texture.set_filter(FilterMode::Nearest);
        Self { image, texture }
    }

    pub fn refresh_if_dirty(&mut self, grid: &mut TileGrid, atlas: &Atlas) {
        if !grid.dirty { return; }
        bake_full(&mut self.image, grid, atlas);
        self.texture.update(&self.image);
        grid.dirty = false;
    }

    pub fn draw(&self, cam: &super::Camera, tint: Color) {
        let (sx, sy) = cam.world_to_screen(0.0, 0.0);
        let dest_w = WORLD_WIDTH  as f32 * cam.zoom;
        let dest_h = WORLD_HEIGHT as f32 * cam.zoom;
        draw_texture_ex(&self.texture, sx, sy, tint, DrawTextureParams {
            dest_size: Some(Vec2::new(dest_w, dest_h)),
            ..Default::default()
        });
    }
}

fn bake_full(image: &mut Image, grid: &TileGrid, atlas: &Atlas) {
    for y in 0..grid.height {
        for x in 0..grid.width {
            let t = grid.get(x, y);
            let v = grid.variants[grid.idx(x, y)];
            paint_tile_into(image, x, y, t, v);
        }
    }
    let _ = atlas;
}

// Hand-painted per-tile renderer. Each tile type gets bespoke pixel logic
// driven by a stable per-tile hash so tiles look varied without re-rolling
// every frame. Light source is treated as upper-left throughout, so all
// textures get their highlights on top and shadows along the bottom-right.
fn paint_tile_into(img: &mut Image, tx: i32, ty: i32, t: TileType, variant: u8) {
    use TileType::*;
    let bx = (tx * 8) as u32;
    let by = (ty * 8) as u32;

    match t {
        Air => {
            let clear = Color::new(0.0, 0.0, 0.0, 0.0);
            for y in 0..8u32 { for x in 0..8u32 {
                img.set_pixel(bx + x, by + y, clear);
            }}
        }
        Tunnel | Chamber => {
            // Smooth dark interior with a faint per-tile mottle so big open
            // chambers don't read as one flat colour wash.
            let base    = if matches!(t, Tunnel) { rgb(22,12,8) } else { rgb(18,10,6) };
            let lighter = if matches!(t, Tunnel) { rgb(34,20,12) } else { rgb(28,18,10) };
            let darker  = if matches!(t, Tunnel) { rgb(14,8,4)   } else { rgb(12,6,4)   };
            let h = tile_hash(tx, ty, variant, 0);
            for y in 0..8u32 { for x in 0..8u32 {
                let v = (h.wrapping_add(x ^ y.wrapping_mul(7))) & 7;
                let c = if v == 0 { lighter }
                        else if v == 1 { darker }
                        else { base };
                img.set_pixel(bx + x, by + y, c);
            }}
        }
        Grass   => paint_grass(img, bx, by, tx, ty, variant),
        Rock    => paint_rock(img, bx, by, tx, ty, variant),
        Sand    => paint_dirt(img, bx, by, tx, ty, variant,
                              rgb(220,200,156), rgb(188,168,120),
                              rgb(160,140,92),  rgb(124,108,64)),
        Soil    => paint_dirt(img, bx, by, tx, ty, variant,
                              rgb(176,128,80),  rgb(140,100,58),
                              rgb(108,76,40),   rgb(80,54,26)),
        Dirt1   => paint_dirt(img, bx, by, tx, ty, variant,
                              rgb(140,100,58),  rgb(112,80,44),
                              rgb(88,60,30),    rgb(64,42,20)),
        Dirt2   => paint_dirt(img, bx, by, tx, ty, variant,
                              rgb(116,80,46),   rgb(92,62,34),
                              rgb(72,46,22),    rgb(50,30,14)),
        Dirt3   => paint_dirt(img, bx, by, tx, ty, variant,
                              rgb(92,60,34),    rgb(68,44,24),
                              rgb(50,32,16),    rgb(32,20,10)),
        Mud     => paint_dirt(img, bx, by, tx, ty, variant,
                              rgb(112,74,38),   rgb(80,52,24),
                              rgb(56,36,16),    rgb(36,22,10)),
        Fungus  => paint_fungus(img, bx, by, tx, ty, variant),
    }
}

/// Stable per-tile hash for deterministic detail placement.
fn tile_hash(tx: i32, ty: i32, variant: u8, salt: u32) -> u32 {
    (tx as u32).wrapping_mul(73856093)
        ^ (ty as u32).wrapping_mul(19349663)
        ^ (variant as u32).wrapping_mul(83492791)
        ^ salt.wrapping_mul(2654435761)
}

/// Soil/dirt/sand/mud — base tone gradient + 1-2 organic clusters + a
/// small embedded detail (pebble, root tendril, glint, or nothing).
/// Colours: hi = upper-row highlight, mid = body, dark = lower-row shadow,
/// deep = strongest shadow used for clusters.
fn paint_dirt(img: &mut Image, bx: u32, by: u32,
              tx: i32, ty: i32, variant: u8,
              hi: Color, mid: Color, dark: Color, deep: Color) {
    // Vertical light gradient: top sun-touched, bulk mid, lower shadowed.
    for y in 0..8u32 {
        for x in 0..8u32 {
            let c = if y == 0 { hi }
                    else if y < 2 { hi_blend(hi, mid) }
                    else if y < 6 { mid }
                    else if y < 7 { dark }
                    else          { deep };
            img.set_pixel(bx + x, by + y, c);
        }
    }

    // Cluster A: dark soil clump — 4-7 contiguous pixels with curving edge
    let h = tile_hash(tx, ty, variant, 0);
    let cax = (h % 6) as u32;
    let cay = ((h / 6) % 5) as u32 + 1;
    let pat = (h / 30) % 5;
    let blob_a: &[(i32, i32)] = match pat {
        0 => &[(0,0),(1,0),(2,0),(0,1),(1,1)],          // wide hump
        1 => &[(0,0),(0,1),(0,2),(1,1),(1,2)],          // tall blob
        2 => &[(0,0),(1,0),(1,1),(2,1),(2,2)],          // S-curve
        3 => &[(0,0),(1,0),(2,0),(1,1),(0,1),(2,1)],    // round
        _ => &[(0,0),(1,0),(2,1),(1,1),(0,1)],          // crescent
    };
    for (dx, dy) in blob_a {
        let x = cax as i32 + *dx;
        let y = cay as i32 + *dy;
        if x >= 0 && x < 8 && y >= 0 && y < 8 {
            img.set_pixel(bx + x as u32, by + y as u32, dark);
        }
    }

    // Cluster B: lit highlight cluster — 2-3 bright pixels on the top edge
    let h2 = tile_hash(tx, ty, variant, 1);
    let cbx = (h2 % 6) as u32;
    let cby = ((h2 / 6) % 3) as u32;
    let blob_b: &[(i32, i32)] = match (h2 / 18) % 3 {
        0 => &[(0,0),(1,0)],
        1 => &[(0,0),(0,1)],
        _ => &[(0,0),(1,0),(0,1)],
    };
    for (dx, dy) in blob_b {
        let x = cbx as i32 + *dx;
        let y = cby as i32 + *dy;
        if x >= 0 && x < 8 && y >= 0 && y < 8 {
            img.set_pixel(bx + x as u32, by + y as u32, hi);
        }
    }

    // Embedded detail: pebble / root / glint / nothing
    let h3 = tile_hash(tx, ty, variant, 2);
    let dx = (h3 % 6) as u32 + 1;
    let dy = ((h3 / 6) % 5) as u32 + 2;
    match (h3 / 30) % 6 {
        0 => {
            // 2-pixel pebble in deep shadow
            img.set_pixel(bx + dx, by + dy, deep);
            if dx + 1 < 8 { img.set_pixel(bx + dx + 1, by + dy, deep); }
            if dy + 1 < 8 { img.set_pixel(bx + dx, by + dy + 1, deep); }
        }
        1 => {
            // Vertical root tendril
            for i in 0..3u32 {
                if dy + i < 8 { img.set_pixel(bx + dx, by + dy + i, deep); }
            }
        }
        2 => {
            // Diagonal mineral streak
            for i in 0..3u32 {
                let x = dx + i;
                let y = dy.saturating_sub(i);
                if x < 8 { img.set_pixel(bx + x, by + y, hi); }
            }
        }
        3 => {
            // Single bright glint
            img.set_pixel(bx + dx, by + dy, hi);
        }
        _ => {} // ~33% of tiles stay plain — uniformity isn't all bad
    }
}

/// Faceted rock with directional shading, internal cracks, and the
/// occasional crystalline sparkle.
fn paint_rock(img: &mut Image, bx: u32, by: u32,
              tx: i32, ty: i32, variant: u8) {
    let hi      = rgb(176, 172, 192);
    let mid     = rgb(120, 116, 132);
    let dark    = rgb(80, 76, 96);
    let crack   = rgb(40, 36, 60);
    let sparkle = rgb(232, 228, 252);

    // Diagonal lighting: bright top-left, dark bottom-right.
    for y in 0..8u32 {
        for x in 0..8u32 {
            let lit = (x as i32 + (7 - y as i32)) - 3;
            let c = if lit >= 7 { hi }
                    else if lit >= 2 { mid }
                    else { dark };
            img.set_pixel(bx + x, by + y, c);
        }
    }

    // One crisp highlight stripe on the lit edge — variant-driven
    let h = tile_hash(tx, ty, variant, 5);
    match variant & 3 {
        0 => for i in 0..3u32 { img.set_pixel(bx + i, by, hi); },
        1 => for i in 0..3u32 { img.set_pixel(bx, by + i, hi); },
        2 => for i in 0..3u32 { img.set_pixel(bx + i, by + i, hi); },
        _ => {
            img.set_pixel(bx + 1, by, hi);
            img.set_pixel(bx + 2, by + 1, hi);
        }
    }

    // Fissure / crack — 3-4 pixel line at a per-tile angle
    let cx = ((h / 4) % 4) as u32 + 2;
    let cy = ((h / 16) % 4) as u32 + 2;
    match h % 4 {
        0 => for i in 0..4u32 {
            if cx + i < 8 { img.set_pixel(bx + cx + i, by + cy, crack); }
        },
        1 => for i in 0..4u32 {
            if cy + i < 8 { img.set_pixel(bx + cx, by + cy + i, crack); }
        },
        2 => for i in 0..3u32 {
            if cx + i < 8 && cy + i < 8 {
                img.set_pixel(bx + cx + i, by + cy + i, crack);
            }
        },
        _ => {} // ~25% of rocks have no visible crack
    }

    // Crystalline sparkle on ~1 in 6 tiles
    if (h / 64) % 6 == 0 {
        let sx = ((h / 256) % 5) as u32 + 1;
        let sy = ((h / 1280) % 5) as u32 + 1;
        img.set_pixel(bx + sx, by + sy, sparkle);
    }
}

/// Grass — soil base on the lower half with a varied band of individual
/// blades poking up. The blade row reads as hand-drawn because each
/// column gets its own height and shade picked from the per-tile hash.
fn paint_grass(img: &mut Image, bx: u32, by: u32,
               tx: i32, ty: i32, variant: u8) {
    let blade_hi   = rgb(124, 220, 92);
    let blade_mid  = rgb(76, 184, 56);
    let blade_dark = rgb(48, 144, 36);
    let blade_root = rgb(36, 96, 24);
    let soil_hi    = rgb(124, 88, 50);
    let soil_mid   = rgb(96, 64, 30);
    let soil_dk    = rgb(72, 44, 22);

    // Soil rows 4..8
    for y in 4..8u32 {
        for x in 0..8u32 {
            let c = if y == 4 { soil_hi }
                    else if y == 7 { soil_dk }
                    else { soil_mid };
            img.set_pixel(bx + x, by + y, c);
        }
    }
    // Sky / above-grass band starts as transparent so blades sit on the sky
    let clear = Color::new(0.0, 0.0, 0.0, 0.0);
    for y in 0..4u32 { for x in 0..8u32 {
        img.set_pixel(bx + x, by + y, clear);
    }}

    // Dark ground line under the blades for definition
    for x in 0..8u32 {
        img.set_pixel(bx + x, by + 4, blade_root);
    }

    // Per-column blades: height 1..4, colour from hash
    let h = tile_hash(tx, ty, variant, 0);
    for x in 0..8u32 {
        let cell = h.wrapping_add(x.wrapping_mul(2654435761));
        let height = (cell % 4) as u32 + 1; // 1..=4
        let color = match (cell / 4) % 5 {
            0 => blade_hi,
            1 | 2 => blade_mid,
            3 => blade_dark,
            _ => blade_root,
        };
        for j in 0..height {
            let y = 3i32 - j as i32;
            if y >= 0 {
                img.set_pixel(bx + x, by + y as u32, color);
            }
        }
    }

    // Occasional tiny flower / dew dot on the very top of one blade
    if (h / 256) % 8 == 0 {
        let petal = rgb(248, 248, 200);
        let fx = ((h / 2048) % 7) as u32;
        img.set_pixel(bx + fx, by, petal);
    }
}

/// Glowing fungus tile — bright pulse-spotted underground food source.
fn paint_fungus(img: &mut Image, bx: u32, by: u32,
                tx: i32, ty: i32, variant: u8) {
    let body_dk = rgb(28, 80, 22);
    let body    = rgb(60, 152, 48);
    let body_hi = rgb(112, 220, 92);
    let glow    = rgb(220, 252, 180);

    for y in 0..8u32 {
        for x in 0..8u32 {
            let c = if y == 0 { body_hi }
                    else if y < 4 { body }
                    else { body_dk };
            img.set_pixel(bx + x, by + y, c);
        }
    }
    // Glowing spore dots in a cluster
    let h = tile_hash(tx, ty, variant, 7);
    let spots: [(u32, u32); 4] = [
        ((h % 6) + 1, ((h / 6) % 4) + 2),
        (((h / 24) % 6) + 1, ((h / 144) % 4) + 2),
        (((h / 576) % 6) + 1, ((h / 3456) % 4) + 2),
        (((h / 20736) % 6) + 1, ((h / 124416) % 4) + 2),
    ];
    for &(sx, sy) in &spots {
        if sx < 8 && sy < 8 {
            img.set_pixel(bx + sx, by + sy, glow);
        }
    }
}

#[inline] fn hi_blend(a: Color, b: Color) -> Color {
    Color::new((a.r + b.r) * 0.5, (a.g + b.g) * 0.5, (a.b + b.b) * 0.5, 1.0)
}

#[inline] fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0)
}
