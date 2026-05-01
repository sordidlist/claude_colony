//! Procedural SNES-ish atlas. Generated once on startup so we don't ship
//! binary art until the style is locked.
//!
//! Layout (8×8 cells):
//!   row 0 cols 0..12  — tile types (matches TileType enum order)
//!   row 1 cols 0..4   — worker ant frames (carrying)
//!   row 1 cols 4..8   — worker ant frames (empty)
//!   row 2 cols 0..1   — food pellet, dig marker

use macroquad::prelude::*;
use crate::world::TileType;

const CELL: u16 = 8;
const COLS: u16 = 16;
const ROWS: u16 = 16;
// Atlas is bigger than the cell grid because the barn / dog / tree are
// painted at 1:1 source-to-screen pixel ratio (matching the underground
// terrain). Those high-res sprites live below the cell-aligned region.
const ATLAS_W: u16 = 256;
const ATLAS_H: u16 = 256;
// High-res sprite anchor positions inside the atlas
const BARN_X: u16 = 0;
const BARN_Y: u16 = 128;
const BARN_W: u16 = 144;
const BARN_H: u16 = 96;
const DOG_X:  u16 = 144;
const DOG_Y:  u16 = 128;
const DOG_W:  u16 = 24;
const DOG_H:  u16 = 24;
const TREE_X: u16 = 144;
const TREE_Y: u16 = 152;
const TREE_W: u16 = 24;
const TREE_H: u16 = 48;
// Lawn mower — riding-mower silhouette, 32×16 per frame, 2 frames for
// rotating wheels. Sits in the free strip directly below the dog row
// (the dog occupies 144..240 × 128..152 across its 4 frames; placing
// the mower at 168..232 × 152..168 puts it in completely vacant atlas
// pixels). The previous home at y=128 overlapped dog frames 1-3 and
// caused the dog to visibly mutate into the mower mid-animation.
const MOWER_X: u16 = 168;
const MOWER_Y: u16 = 152;
const MOWER_W: u16 = 32;
const MOWER_H: u16 = 16;

pub struct Atlas {
    pub texture: Texture2D,
}

impl Atlas {
    pub fn build() -> Self {
        let w = ATLAS_W;
        let h = ATLAS_H;
        let _ = (COLS, ROWS); // legacy constants — kept so old `cell()` still works
        let mut img = Image::gen_image_color(w, h, Color::new(0.0, 0.0, 0.0, 0.0));

        // ─── tiles ─────────────────────────────────────────────────────────
        for (i, t) in [
            TileType::Air, TileType::Grass, TileType::Soil,
            TileType::Dirt1, TileType::Dirt2, TileType::Dirt3,
            TileType::Rock, TileType::Tunnel, TileType::Chamber,
            TileType::Sand, TileType::Fungus, TileType::Mud,
        ].iter().enumerate() {
            let (cx, cy) = (i as u16, 0u16);
            paint_tile(&mut img, cx, cy, *t);
        }

        // ─── ant frames ────────────────────────────────────────────────────
        // row 1: empty (cols 0..4) and food-carrying (cols 4..8)
        // row 2: pebble-hauling variants  (cols 0..4) for dig debris
        for f in 0..4 {
            paint_ant(&mut img, f, 1, false);
            paint_ant(&mut img, f + 4, 1, true);
            paint_ant_with_pebble(&mut img, f, 2);
        }

        // ─── food pellet ───────────────────────────────────────────────────
        paint_food(&mut img, 4, 2);
        // ─── dig marker (the × overlay) ────────────────────────────────────
        paint_dig_marker(&mut img, 5, 2);

        // ─── above-ground scenery ──────────────────────────────────────────
        // High-res barn / dog / tree live in the atlas region below the
        // 8×8 cell grid, painted at 1:1 source-to-screen pixel ratio.
        paint_barn_hi(&mut img);
        for f in 0..4 { paint_dog_hi(&mut img, f); }
        for f in 0..2 { paint_mower_hi(&mut img, f); }
        paint_tree_hi(&mut img);
        paint_sun(&mut img, 6, 5);
        paint_moon(&mut img, 7, 5);
        paint_cloud(&mut img, 8, 5);                 // 24×8, cells (8..11, 5)

        // ─── queen ─────────────────────────────────────────────────────────
        paint_queen(&mut img, 0, 8);                 // 16×16, cells (0..2, 8..10)

        // ─── spider ────────────────────────────────────────────────────────
        // Two animation frames at row 12, cells 0..4.
        paint_spider(&mut img, 0, 12, 0);            // (0,96, 16×16)
        paint_spider(&mut img, 2, 12, 1);            // (16,96, 16×16)

        // ─── rival ant frames ──────────────────────────────────────────────
        // Row 14: empty (cols 0..4) and food-carrying (cols 4..8).
        for f in 0..4 {
            paint_rival_ant(&mut img, f,     14, false);
            paint_rival_ant(&mut img, f + 4, 14, true);
        }

        // ─── soldier frames + brood + corpse ───────────────────────────────
        // Row 15: soldier 0..4, brood 4, corpse 5
        for f in 0..4 {
            paint_soldier_ant(&mut img, f, 15);
        }
        paint_brood(&mut img, 4, 15);
        paint_corpse(&mut img, 5, 15);

        let texture = Texture2D::from_image(&img);
        texture.set_filter(FilterMode::Nearest);
        Self { texture }
    }

    pub fn barn_rect(&self)  -> Rect {
        Rect::new(BARN_X as f32, BARN_Y as f32, BARN_W as f32, BARN_H as f32)
    }
    pub fn dog_rect(&self, frame: u8) -> Rect {
        Rect::new((DOG_X + (frame as u16 % 4) * DOG_W) as f32,
                  DOG_Y as f32, DOG_W as f32, DOG_H as f32)
    }
    pub fn tree_rect(&self)  -> Rect {
        Rect::new(TREE_X as f32, TREE_Y as f32, TREE_W as f32, TREE_H as f32)
    }
    pub fn mower_rect(&self, frame: u8) -> Rect {
        Rect::new(
            (MOWER_X + (frame as u16 % 2) * MOWER_W) as f32,
            MOWER_Y as f32,
            MOWER_W as f32, MOWER_H as f32,
        )
    }
    pub fn sun_rect(&self)   -> Rect { Rect::new(48.0, 40.0, 8.0,  8.0)  }
    pub fn moon_rect(&self)  -> Rect { Rect::new(56.0, 40.0, 8.0,  8.0)  }
    pub fn cloud_rect(&self) -> Rect { Rect::new(64.0, 40.0, 24.0, 8.0)  }
    pub fn queen_rect(&self) -> Rect { Rect::new(0.0,  64.0, 16.0, 16.0) }
    pub fn spider_rect(&self, frame: u8) -> Rect {
        Rect::new((frame as u16 % 2 * 16) as f32, 96.0, 16.0, 16.0)
    }
    pub fn rival_ant_cell(&self, frame: u8, carrying: bool) -> Rect {
        let col = if carrying { 4 + (frame as u16 % 4) } else { frame as u16 % 4 };
        self.cell(col, 14)
    }
    pub fn soldier_cell(&self, frame: u8) -> Rect {
        self.cell(frame as u16 % 4, 15)
    }
    pub fn brood_cell(&self) -> Rect { self.cell(4, 15) }
    pub fn corpse_cell(&self) -> Rect { self.cell(5, 15) }

    /// Source rect in atlas pixels for a given (col, row) cell.
    #[inline] pub fn cell(&self, col: u16, row: u16) -> Rect {
        Rect::new(
            (col * CELL) as f32,
            (row * CELL) as f32,
            CELL as f32, CELL as f32,
        )
    }

    #[allow(dead_code)] // used once tile rendering moves through atlas
    pub fn tile_cell(&self, t: TileType) -> Rect {
        self.cell(t as u16, 0)
    }
    pub fn ant_cell(&self, frame: u8, carrying: bool) -> Rect {
        let col = if carrying { 4 + (frame as u16 % 4) } else { frame as u16 % 4 };
        self.cell(col, 1)
    }
    pub fn ant_with_pebble_cell(&self, frame: u8) -> Rect {
        self.cell(frame as u16 % 4, 2)
    }
    pub fn food_cell(&self) -> Rect { self.cell(4, 2) }
    pub fn dig_marker_cell(&self) -> Rect { self.cell(5, 2) }
}

// ── pixel painters ──────────────────────────────────────────────────────────

fn px(img: &mut Image, x: u16, y: u16, c: Color) {
    img.set_pixel(x as u32, y as u32, c);
}

fn rect(img: &mut Image, cx: u16, cy: u16, w: u16, h: u16, c: Color) {
    for y in 0..h {
        for x in 0..w {
            px(img, cx*CELL + x, cy*CELL + y, c);
        }
    }
}

fn paint_tile(img: &mut Image, cx: u16, cy: u16, t: TileType) {
    use TileType::*;
    let (hi, mid, dark) = match t {
        Air     => (Color::new(0.0, 0.0, 0.0, 0.0), Color::new(0.0, 0.0, 0.0, 0.0), Color::new(0.0, 0.0, 0.0, 0.0)),
        Grass   => (rgb(72,200,56), rgb(52,164,40), rgb(40,128,28)),
        Soil    => (rgb(164,120,72), rgb(140,100,58), rgb(116,82,44)),
        Dirt1   => (rgb(136,96,56),  rgb(116,80,44),  rgb(96,64,32)),
        Dirt2   => (rgb(112,76,44),  rgb(92,62,34),   rgb(76,50,26)),
        Dirt3   => (rgb(88,56,32),   rgb(68,44,24),   rgb(52,32,16)),
        Rock    => (rgb(152,148,164),rgb(120,116,132),rgb(88,84,104)),
        Tunnel  => (rgb(34,20,12),   rgb(22,12,8),    rgb(14,8,4)),
        Chamber => (rgb(28,18,10),   rgb(18,10,6),    rgb(12,6,4)),
        Sand    => (rgb(212,192,148),rgb(188,168,120),rgb(160,140,92)),
        Fungus  => (rgb(96,200,80),  rgb(64,152,48),  rgb(40,108,28)),
        Mud     => (rgb(108,72,36),  rgb(80,52,24),   rgb(52,34,14)),
    };
    if matches!(t, Air) { return; }
    rect(img, cx, cy, CELL, CELL, mid);
    // Bayer 4x4 dithering between hi/mid/dark for SNES vibe
    const BAYER: [[u8; 4]; 4] = [
        [ 0,  8,  2, 10],
        [12,  4, 14,  6],
        [ 3, 11,  1,  9],
        [15,  7, 13,  5],
    ];
    for y in 0..CELL {
        for x in 0..CELL {
            let b = BAYER[(y % 4) as usize][(x % 4) as usize];
            let c = if b < 4 { hi }
                    else if b < 12 { mid }
                    else { dark };
            px(img, cx*CELL + x, cy*CELL + y, c);
        }
    }
    // Tunnel/chamber don't get dithered top-highlight
    if matches!(t, Tunnel | Chamber) {
        rect(img, cx, cy, CELL, CELL, mid);
    }
}

struct AntPalette {
    outline: Color,
    body:    Color,
    body_hi: Color,
    leg:     Color,
    cargo:   Color,
}

const WORKER_PALETTE: AntPalette = AntPalette {
    // Deep lavender body — distinct from rival reds at a glance and reads
    // well against dirt, tunnels, and grass.
    outline: Color { r: 0.040, g: 0.020, b: 0.080, a: 1.0 },
    body:    Color { r: 0.282, g: 0.220, b: 0.486, a: 1.0 },
    body_hi: Color { r: 0.660, g: 0.580, b: 0.860, a: 1.0 },
    leg:     Color { r: 0.080, g: 0.054, b: 0.166, a: 1.0 },
    cargo:   Color { r: 0.518, g: 0.957, b: 0.314, a: 1.0 },
};

const RIVAL_PALETTE: AntPalette = AntPalette {
    outline: Color { r: 0.140, g: 0.014, b: 0.014, a: 1.0 },
    body:    Color { r: 0.816, g: 0.156, b: 0.110, a: 1.0 },
    body_hi: Color { r: 1.000, g: 0.470, b: 0.345, a: 1.0 },
    leg:     Color { r: 0.236, g: 0.046, b: 0.030, a: 1.0 },
    cargo:   Color { r: 0.518, g: 0.957, b: 0.314, a: 1.0 },
};

fn paint_ant(img: &mut Image, col: u16, row: u16, carrying: bool) {
    paint_ant_with_palette(img, col, row, carrying, &WORKER_PALETTE);
}

fn paint_rival_ant(img: &mut Image, col: u16, row: u16, carrying: bool) {
    paint_ant_with_palette(img, col, row, carrying, &RIVAL_PALETTE);
}

fn paint_ant_with_palette(img: &mut Image, col: u16, row: u16,
                          carrying: bool, p: &AntPalette) {
    let frame = (col % 4) as i32;
    let wig = if frame == 0 || frame == 2 { 0 } else { 1 };

    // Outline ring
    for &(x, y) in &[
        (1u16, 3u16), (2, 2), (3, 2), (4, 2), (5, 2), (6, 2), (7, 3),
        (1, 4),                                                  (7, 4),
        (2, 5), (3, 5), (4, 5), (5, 5), (6, 5),
    ] {
        px(img, col*CELL + x, row*CELL + y, p.outline);
    }
    // Body fill
    for x in 2..=6 {
        px(img, col*CELL + x, row*CELL + 3, p.body);
        px(img, col*CELL + x, row*CELL + 4, p.body);
    }
    // Highlight + head accent
    px(img, col*CELL + 3, row*CELL + 3, p.body_hi);
    px(img, col*CELL + 4, row*CELL + 3, p.body_hi);
    px(img, col*CELL + 6, row*CELL + 4, p.body_hi);

    // Animated legs
    px(img, col*CELL + 2, row*CELL + (5 + wig) as u16, p.leg);
    px(img, col*CELL + 4, row*CELL + (5 + wig) as u16, p.leg);
    px(img, col*CELL + 6, row*CELL + (5 + wig) as u16, p.leg);
    px(img, col*CELL + 3, row*CELL + (6 + (1 - wig)) as u16, p.leg);
    px(img, col*CELL + 5, row*CELL + (6 + (1 - wig)) as u16, p.leg);

    if carrying {
        px(img, col*CELL + 0, row*CELL + 3, p.outline);
        px(img, col*CELL + 0, row*CELL + 4, p.outline);
        px(img, col*CELL + 1, row*CELL + 2, p.cargo);
        px(img, col*CELL + 1, row*CELL + 3, p.cargo);
    }
}

fn paint_ant_with_pebble(img: &mut Image, col: u16, row: u16) {
    paint_ant(img, col, row, false);
    // Pebble: small grey-brown blob above the back of the ant
    let pebble_dark = rgb(80, 56, 32);
    let pebble_mid  = rgb(140, 100, 60);
    let pebble_hi   = rgb(196, 156, 96);
    px(img, col*CELL + 3, row*CELL + 0, pebble_dark);
    px(img, col*CELL + 4, row*CELL + 0, pebble_dark);
    px(img, col*CELL + 5, row*CELL + 0, pebble_dark);
    px(img, col*CELL + 2, row*CELL + 1, pebble_dark);
    px(img, col*CELL + 3, row*CELL + 1, pebble_mid);
    px(img, col*CELL + 4, row*CELL + 1, pebble_hi);
    px(img, col*CELL + 5, row*CELL + 1, pebble_mid);
    px(img, col*CELL + 6, row*CELL + 1, pebble_dark);
}

fn paint_food(img: &mut Image, col: u16, row: u16) {
    let hi  = rgb(72, 208, 52);
    let mid = rgb(52, 168, 36);
    px(img, col*CELL + 3, row*CELL + 3, hi);
    px(img, col*CELL + 4, row*CELL + 3, hi);
    px(img, col*CELL + 3, row*CELL + 4, mid);
    px(img, col*CELL + 4, row*CELL + 4, mid);
}

fn paint_dig_marker(img: &mut Image, col: u16, row: u16) {
    let c = rgb(200, 180, 100);
    for i in 0..6 {
        px(img, col*CELL + 1 + i, row*CELL + 1 + i, c);
        px(img, col*CELL + 1 + i, row*CELL + 6 - i, c);
    }
}

#[inline] fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0)
}

// ── high-res scenery painters (1:1 source-to-screen at zoom 8) ──────────────

fn paint_barn_hi(img: &mut Image) {
    // 144×96 traditional gambrel-roofed American barn. The previous
    // simple-triangle roof was reading as a church spire; replacing it
    // with a two-pitch gambrel (steep upper, shallow lower with a knee)
    // is the silhouette that says "barn" instantly. The X-braced double
    // doors also got swapped for plank-and-strap sliding doors with a
    // track header — the other big "barn vs. church" cue.
    let plot = |img: &mut Image, x: i32, y: i32, c: Color| {
        if x >= 0 && (x as u16) < BARN_W && y >= 0 && (y as u16) < BARN_H {
            img.set_pixel(BARN_X as u32 + x as u32,
                          BARN_Y as u32 + y as u32, c);
        }
    };

    // Weathered wood-shingle gray — every reference photo of an
    // American barn has the roof in this palette, never barn-red.
    let shingle_dark = rgb(40, 36, 32);
    let shingle_dk   = rgb(72, 68, 60);
    let shingle_mid  = rgb(108, 100, 88);
    let shingle_mid2 = rgb(140, 132, 116);
    let shingle_hi   = rgb(176, 168, 148);
    let shingle_aged = rgb(96, 80, 56);
    let trim_mid     = rgb(216, 200, 156);
    let trim_dk      = rgb(160, 144, 104);
    let trim_shadow  = rgb(96, 84, 60);
    let plank_dk     = rgb(96, 18, 12);
    let plank_mid    = rgb(168, 36, 24);
    let plank_hi     = rgb(212, 60, 42);
    let plank_grain  = rgb(132, 28, 20);
    let plank_seam   = rgb(36, 8, 4);
    let knot_dk      = rgb(40, 10, 6);
    let knot_mid     = rgb(72, 14, 8);
    let weather      = rgb(120, 76, 52);
    let glass_dk     = rgb(28, 60, 116);
    let glass_mid    = rgb(72, 132, 200);
    let glass_hi     = rgb(184, 220, 240);
    let door_dk      = rgb(28, 14, 6);
    let door_mid     = rgb(80, 48, 24);
    let door_hi      = rgb(124, 84, 48);
    let door_grain   = rgb(52, 28, 14);
    let iron_dk      = rgb(20, 18, 28);
    let iron_mid     = rgb(48, 44, 60);
    let iron_hi      = rgb(80, 76, 96);
    let rivet        = rgb(140, 132, 156);
    let brass        = rgb(248, 200, 96);
    let brass_dk     = rgb(168, 124, 36);
    let brass_hi     = rgb(255, 232, 144);
    let stone_lit    = rgb(200, 196, 220);
    let stone_hi     = rgb(168, 164, 188);
    let stone_mid    = rgb(124, 120, 140);
    let stone_dk     = rgb(76, 72, 96);
    let stone_shadow = rgb(48, 44, 64);
    let mortar       = rgb(36, 32, 48);
    let moss         = rgb(72, 100, 40);

    // ── Cupola (the iconic barn ventilator tower on the peak) ─────
    // This is the single most barn-recognisable feature — a small
    // square tower with a louvered vent, its own little roof, and the
    // weathervane on top. Churches don't have these; barns almost
    // always do. The flat ridge of the gambrel sits beneath it.
    let cup_l = 66i32;
    let cup_r = 77i32;            // 12 wide
    let cup_wall_top = 6i32;
    let cup_wall_bot = 9i32;
    let cup_base_y   = 10i32;
    let cupola_white = rgb(232, 220, 188);
    let cupola_shadow = rgb(168, 152, 116);
    let louver_dk = rgb(28, 18, 10);
    let louver_mid = rgb(64, 44, 24);

    // Cupola pyramidal mini-roof (rows 3-5)
    for y in 3..=5 {
        let inset = 5 - y;            // 2 at top, 0 at bottom
        for x in (cup_l + inset)..=(cup_r - inset) {
            let c = if y == 5 { shingle_dark }
                    else if y == 3 { shingle_dk }
                    else { shingle_mid };
            plot(img, x, y, c);
        }
    }
    // Cupola walls — white-painted siding
    for y in cup_wall_top..=cup_wall_bot {
        for x in cup_l..=cup_r {
            let c = if y == cup_wall_bot { cupola_shadow }
                    else { cupola_white };
            plot(img, x, y, c);
        }
    }
    // Louvered vent slats — the iconic horizontal bars on a barn cupola
    for y in cup_wall_top..=cup_wall_bot {
        if y % 2 == 0 {
            for x in (cup_l + 2)..=(cup_r - 2) {
                plot(img, x, y, louver_mid);
            }
            plot(img, cup_l + 2, y, louver_dk);
            plot(img, cup_r - 2, y, louver_dk);
        }
    }
    // Cupola base trim band
    for x in (cup_l - 1)..=(cup_r + 1) {
        plot(img, x, cup_base_y, trim_dk);
    }

    // ── Weathervane on top of the cupola ──────────────────────────
    // Short post + arrow, mounted on the cupola peak — much shorter
    // than the previous skyward post that was reading as a steeple.
    plot(img, 71, 0, iron_dk);
    plot(img, 72, 0, iron_hi);
    plot(img, 71, 1, iron_dk);
    plot(img, 72, 1, iron_hi);
    plot(img, 71, 2, iron_dk);
    plot(img, 72, 2, iron_hi);
    // Compact arrow at row 1
    for x in 66..72 { plot(img, x, 1, brass); }
    for x in 73..79 { plot(img, x, 1, brass); }
    plot(img, 65, 1, brass_dk);
    plot(img, 79, 1, brass_dk);
    // Tail/tip feathers
    plot(img, 64, 0, brass_dk);
    plot(img, 64, 2, brass_dk);
    plot(img, 80, 0, brass_dk);
    plot(img, 80, 2, brass_dk);

    // ── Gambrel roof ───────────────────────────────────────────────
    // VERY dramatic gambrel — upper nearly vertical, lower nearly
    // horizontal, with a hard trim line at the knee so the bend reads
    // unmistakably. Anything less extreme and the roof reads as a
    // single straight pitch.
    // Tuned to match the reference-photo gambrel: upper pitch ~55-60°
    // from horizontal, lower pitch ~25-30°. Going more extreme than
    // this read as "siloey" rather than "barny."
    let ridge_y  = 11i32;
    let peak_y   = 13i32;
    let knee_y   = 26i32;
    let eaves_y  = 44i32;
    let upper_w  = 14i32;
    let knee_w   = 22i32;          // moderate widening — upper pitch ~60° from horizontal
    let eaves_w  = 64i32;          // big widening below — lower pitch ~25° from horizontal
    let centre_x = 72i32;

    // Flat ridge cap (rows 11-12), wider than the cupola so it
    // overhangs visibly
    for x in (centre_x - upper_w)..=(centre_x + upper_w) {
        plot(img, x, ridge_y,     shingle_dk);
        plot(img, x, ridge_y + 1, shingle_mid);
    }

    for y in peak_y..=eaves_y {
        let half_w = if y <= knee_y {
            let t = (y - peak_y) as f32 / (knee_y - peak_y) as f32;
            (upper_w as f32 + (knee_w - upper_w) as f32 * t).round() as i32
        } else {
            let t = (y - knee_y) as f32 / (eaves_y - knee_y) as f32;
            (knee_w as f32 + (eaves_w - knee_w) as f32 * t).round() as i32
        };
        let l = centre_x - half_w;
        let r = centre_x + half_w;
        let shingle_row  = (y - peak_y) / 3;
        let shingle_y_in = (y - peak_y) % 3;
        let stagger = if shingle_row & 1 == 0 { 0 } else { 3 };
        for x in l..=r {
            let on_edge = x == l || x == r || x == l + 1 || x == r - 1;
            let shingle_x_in = (x + stagger).rem_euclid(6);
            let shingle_idx  = (x + stagger).div_euclid(6);
            let aged = ((shingle_idx.wrapping_mul(13)
                       ^ shingle_row.wrapping_mul(31)) & 7) == 0;
            let c = if on_edge          { shingle_dk }
                    else if shingle_y_in == 0 { shingle_hi }
                    else if shingle_y_in == 2 { shingle_dark }
                    else if shingle_x_in == 0 { shingle_dark }
                    else if aged              { shingle_aged }
                    else {
                        let h = ((shingle_idx * 7) ^ (shingle_row * 11)) & 3;
                        match h { 0 => shingle_mid2, _ => shingle_mid }
                    };
            plot(img, x, y, c);
        }
    }

    // ── Knee trim band ────────────────────────────────────────────
    // Hard horizontal cream trim where the two pitches meet. This is
    // what every real gambrel barn has — a fascia board at the knee
    // — and it's the single most decisive visual cue that turns "is
    // this even a gambrel?" into "obviously a barn." Spans the full
    // width of the roof at the knee row.
    let knee_half = knee_w + 1;
    let knee_l = centre_x - knee_half;
    let knee_r = centre_x + knee_half;
    for x in knee_l..=knee_r {
        plot(img, x, knee_y - 1, trim_dk);
        plot(img, x, knee_y,     trim_mid);
        plot(img, x, knee_y + 1, trim_dk);
    }

    // ── Eave trim band ─────────────────────────────────────────────
    for x in 0..(BARN_W as i32) {
        plot(img, x, 45, trim_dk);
        plot(img, x, 46, trim_mid);
        plot(img, x, 47, trim_dk);
        plot(img, x, 48, plank_seam);                // hard shadow under eaves
        plot(img, x, 49, plank_seam);
    }

    // ── Walls ──────────────────────────────────────────────────────
    let wall_l = 4i32;
    let wall_r = 139i32;
    let wall_t = 50i32;
    let wall_b = 79i32;
    let seams = [wall_l, 28, 50, 72, 94, 116, wall_r];
    for y in wall_t..=wall_b {
        for x in wall_l..=wall_r {
            let body = if y == wall_t || y == wall_b { plank_dk }
                       else if y % 2 == 0           { plank_mid }
                       else                         { plank_hi };
            plot(img, x, y, body);
        }
        for x in wall_l..=wall_r {
            if (x ^ y) & 7 == 0 {
                plot(img, x, y, plank_grain);
            }
        }
    }
    for &x in &seams {
        for y in wall_t..=wall_b {
            plot(img, x, y, plank_seam);
        }
    }
    // Knots
    let knots = [
        (10i32, 56i32), (32, 65), (62, 53), (88, 70),
        (108, 60), (130, 73), (15, 71), (45, 60),
        (75, 75), (115, 55),
    ];
    for &(kx, ky) in &knots {
        plot(img, kx,     ky,     knot_dk);
        plot(img, kx + 1, ky,     knot_mid);
        plot(img, kx,     ky + 1, knot_mid);
        plot(img, kx + 1, ky + 1, knot_dk);
    }
    // Vertical rain streaks
    for y in 52..78 { plot(img, 7,   y, weather); plot(img, 8,   y, plank_dk); }
    for y in 54..76 { plot(img, 135, y, weather); plot(img, 136, y, plank_dk); }
    for y in 55..72 { plot(img, 134, y, weather); }

    // ── Hayloft door (top of wall, just below eaves) ──────────────
    // Small two-leaf loading door for hauling hay bales up. Smaller
    // than the main door, no glass — barn lofts have a wood door.
    let hl_l = 60i32; let hl_r = 84i32;
    let hl_t = 50i32; let hl_b = 60i32;
    // Lintel + frame
    for x in (hl_l - 1)..=(hl_r + 1) { plot(img, x, hl_t, trim_dk); }
    for y in hl_t..=hl_b {
        plot(img, hl_l - 1, y, trim_dk);
        plot(img, hl_r + 1, y, trim_dk);
    }
    for y in (hl_t + 1)..=hl_b {
        for x in hl_l..=hl_r {
            let plank_in = ((x - hl_l) % 4 + 4) % 4;
            let body = if plank_in == 0 { door_dk }
                       else if plank_in == 1 { door_mid }
                       else if plank_in == 2 { door_hi }
                       else { door_mid };
            plot(img, x, y, body);
        }
    }
    // Iron straps top + bottom
    for &hy in &[hl_t + 1, hl_b - 1] {
        for x in hl_l..=hl_r { plot(img, x, hy, iron_dk); }
    }
    plot(img, hl_l, hl_t + 1, iron_hi);
    plot(img, hl_r, hl_t + 1, iron_hi);
    plot(img, hl_l, hl_b - 1, iron_hi);
    plot(img, hl_r, hl_b - 1, iron_hi);
    // Centre split
    let hl_split = (hl_l + hl_r) / 2;
    for y in (hl_t + 1)..=hl_b { plot(img, hl_split, y, door_dk); }
    // Pulley hook above the hayloft door — for raising bales
    for y in 47..50 { plot(img, 71, y, iron_dk); plot(img, 72, y, iron_dk); }
    plot(img, 70, 49, iron_dk);
    plot(img, 73, 49, iron_dk);

    // ── Big main barn door (sliding two-leaf, takes most of bottom wall) ──
    let dl = 38i32; let dr = 105i32;
    let dt = 62i32; let db = 79i32;

    // Sliding-door track header — long iron bar above the door with
    // brass track wheels on each leaf. THE single biggest "this is a
    // working barn, not a chapel" visual cue.
    for x in (dl - 4)..=(dr + 4) {
        plot(img, x, dt - 2, iron_dk);
        plot(img, x, dt - 1, iron_mid);
    }
    plot(img, dl - 5, dt - 2, iron_dk); plot(img, dr + 5, dt - 2, iron_dk);
    plot(img, dl - 5, dt - 1, iron_mid); plot(img, dr + 5, dt - 1, iron_mid);
    // Track wheels
    for &wx in &[dl + 4, dr - 4] {
        plot(img, wx, dt - 2, brass_dk);
        plot(img, wx, dt - 1, brass);
    }

    // Door body — vertical planks
    for y in dt..=db {
        for x in dl..=dr {
            let plank_in = ((x - dl) % 5 + 5) % 5;
            let body = if plank_in == 0 { door_dk }
                       else if plank_in == 1 { door_mid }
                       else if plank_in == 2 { door_hi }
                       else if plank_in == 3 { door_mid }
                       else                  { door_grain };
            plot(img, x, y, body);
        }
        for x in dl..=dr {
            if (x ^ y) & 5 == 0 {
                plot(img, x, y, door_grain);
            }
        }
    }
    // Door frame outline (sides + bottom; top is the track header)
    for y in dt..=db {
        plot(img, dl - 1, y, door_dk);
        plot(img, dr + 1, y, door_dk);
    }
    for x in (dl - 1)..=(dr + 1) {
        plot(img, x, db + 1, door_dk);
    }
    // Centre split for the two leaves
    let split = (dl + dr) / 2;
    for y in dt..=db {
        plot(img, split,     y, door_dk);
        plot(img, split + 1, y, door_dk);
    }
    // White X-trim on each door leaf — the iconic barn-door visual
    // from the reference photo. Each X spans corner-to-corner of its
    // leaf, in cream trim against the dark plank background.
    let plot_thick_line = |img: &mut Image, x0: i32, y0: i32, x1: i32, y1: i32, c: Color| {
        let steps = (x1 - x0).abs().max((y1 - y0).abs());
        if steps == 0 { return; }
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let x = (x0 as f32 + (x1 - x0) as f32 * t).round() as i32;
            let y = (y0 as f32 + (y1 - y0) as f32 * t).round() as i32;
            for dx in -1..=0 {
                plot(img, x + dx, y, c);
            }
        }
    };
    // Left leaf X
    plot_thick_line(img, dl + 1, dt + 1, split - 1, db - 1, trim_mid);
    plot_thick_line(img, split - 1, dt + 1, dl + 1, db - 1, trim_mid);
    // Right leaf X
    plot_thick_line(img, split + 2, dt + 1, dr - 1, db - 1, trim_mid);
    plot_thick_line(img, dr - 1, dt + 1, split + 2, db - 1, trim_mid);
    // White trim border around each leaf
    for x in dl..=split - 1 {
        plot(img, x, dt, trim_mid);
        plot(img, x, db, trim_mid);
    }
    for x in (split + 2)..=dr {
        plot(img, x, dt, trim_mid);
        plot(img, x, db, trim_mid);
    }
    for y in dt..=db {
        plot(img, dl,        y, trim_mid);
        plot(img, split - 1, y, trim_mid);
        plot(img, split + 2, y, trim_mid);
        plot(img, dr,        y, trim_mid);
    }
    // Brass D-handles, one per leaf — sliding-door pull, mounted on the
    // outer edge of each leaf
    let handle_y_mid = (dt + db) / 2;
    for &hx in &[split - 6, split + 7] {
        for dy_ in -3..=3 {
            plot(img, hx, handle_y_mid + dy_, brass);
        }
        plot(img, hx - 1, handle_y_mid - 3, brass_dk);
        plot(img, hx + 1, handle_y_mid - 3, brass_dk);
        plot(img, hx - 1, handle_y_mid + 3, brass_dk);
        plot(img, hx + 1, handle_y_mid + 3, brass_dk);
        plot(img, hx, handle_y_mid - 1, brass_hi);
    }
    let _ = (rivet, iron_dk, iron_mid, iron_hi);

    // ── Two side windows ──────────────────────────────────────────
    for &xs in &[14i32, 110] {
        let xe = xs + 14;
        let yt = 55i32; let yb = 70i32;
        for x in xs..=xe { plot(img, x, yt, trim_dk); plot(img, x, yb, trim_dk); }
        for y in yt..=yb { plot(img, xs, y, trim_dk); plot(img, xe, y, trim_dk); }
        for y in (yt + 1)..yb {
            for x in (xs + 1)..xe {
                let lx = x - xs - 1;
                let ly = y - yt - 1;
                let on_v = (lx + 1) % 6 == 0 && lx + 1 < 14;
                let on_h = (ly + 1) % 7 == 0 && ly + 1 < 14;
                let c = if on_v || on_h { trim_mid }
                        else if ly < 3 && lx < 4 { glass_hi }
                        else if ly < 6 { glass_mid }
                        else { glass_dk };
                plot(img, x, y, c);
            }
        }
        for x in (xs - 1)..=(xe + 1) { plot(img, x, yb + 1, trim_shadow); }
    }

    // ── Stone foundation ──────────────────────────────────────────
    let course1: [(i32, i32); 9] = [
        (0, 14), (15, 30), (31, 47), (48, 65), (66, 79),
        (80, 96), (97, 112), (113, 128), (129, 143),
    ];
    let course2: [(i32, i32); 8] = [
        (0, 7), (8, 24), (25, 42), (43, 60), (61, 80),
        (81, 100), (101, 120), (121, 143),
    ];
    for &(l, r) in &course1 {
        for x in l..=r {
            for y in 80..86 {
                let c = if y == 80 { stone_lit }
                        else if y == 85 { stone_dk }
                        else if x == l { stone_hi }
                        else if x == r { stone_dk }
                        else if (x ^ y) & 3 == 0 { stone_hi }
                        else if (x ^ (y + 1)) & 5 == 0 { stone_dk }
                        else { stone_mid };
                plot(img, x, y, c);
            }
        }
    }
    for &(_, r) in &course1 {
        if r + 1 < 144 {
            for y in 80..86 { plot(img, r + 1, y, mortar); }
        }
    }
    for x in 0..144 { plot(img, x, 86, mortar); }
    for &(l, r) in &course2 {
        for x in l..=r {
            for y in 87..96 {
                let c = if y == 87 { stone_lit }
                        else if y == 95 { stone_shadow }
                        else if x == l { stone_hi }
                        else if x == r { stone_dk }
                        else if (x ^ y) & 3 == 0 { stone_hi }
                        else if (x ^ (y + 1)) & 5 == 0 { stone_dk }
                        else { stone_mid };
                plot(img, x, y, c);
            }
        }
    }
    for &(_, r) in &course2 {
        if r + 1 < 144 {
            for y in 87..96 { plot(img, r + 1, y, mortar); }
        }
    }
    for x in (15..130).step_by(20) {
        plot(img, x,     80, moss);
        plot(img, x + 1, 80, moss);
    }
}

fn paint_dog_hi(img: &mut Image, frame: u8) {
    // 24×24 — re-painted from scratch at 1:1 resolution. Tan-and-cream
    // shepherd-ish dog facing right with animated legs and tail.
    let frame_off_x = DOG_X + (frame as u16 % 4) * DOG_W;
    let plot = |img: &mut Image, x: i32, y: i32, c: Color| {
        if x >= 0 && (x as u16) < DOG_W && y >= 0 && (y as u16) < DOG_H {
            img.set_pixel(frame_off_x as u32 + x as u32,
                          DOG_Y as u32 + y as u32, c);
        }
    };
    let body     = rgb(212, 168, 92);
    let body_dk  = rgb(160, 120, 60);
    let belly    = rgb(232, 196, 132);
    let outline  = rgb(40, 24, 12);
    let nose_eye = rgb(20, 12, 4);
    let collar   = rgb(184, 36, 28);
    let buckle   = rgb(248, 200, 96);
    let tongue   = rgb(232, 92, 92);

    // Body (cols 3-17, rows 9-15)
    for y in 9..=15 {
        for x in 3..=17 { plot(img, x, y, body); }
    }
    // Belly underside lighter
    for y in 14..=16 {
        for x in 4..=15 { plot(img, x, y, belly); }
    }
    // Body outline
    for x in 3..=17 { plot(img, x, 8, outline); plot(img, x, 16, outline); }
    for y in 9..=16 { plot(img, 2, y, outline); plot(img, 18, y, outline); }
    // Body shading (back)
    for x in 4..=16 { plot(img, x, 9, body_dk); }
    plot(img, 17, 10, body_dk);
    plot(img, 17, 11, body_dk);

    // Head (cols 14-22, rows 4-12)
    for y in 5..=11 { for x in 14..=21 { plot(img, x, y, body); } }
    // Head outline
    for x in 14..=21 { plot(img, x, 4, outline); plot(img, x, 12, outline); }
    for y in 5..=12 { plot(img, 13, y, outline); plot(img, 22, y, outline); }
    // Snout sticking forward
    plot(img, 22, 8, body); plot(img, 22, 9, body);
    plot(img, 23, 8, outline); plot(img, 23, 9, outline);
    // Nose
    plot(img, 22, 7, nose_eye);
    plot(img, 23, 7, nose_eye);
    // Eye + brow
    plot(img, 19, 7, nose_eye);
    plot(img, 19, 6, body_dk);
    // Ear (drop ear, dark)
    for y in 3..=5 { plot(img, 14, y, outline); plot(img, 15, y, body_dk); }
    plot(img, 16, 4, body_dk);
    // Mouth slightly open with tongue
    plot(img, 21, 11, outline);
    plot(img, 22, 11, tongue);

    // Tail wagging — frame-dependent vertical offset
    let tail_off = match frame & 3 { 0 => 0i32, 1 => -1, 2 => 0, _ => 1 };
    let tail_pixels: [(i32, i32, Color); 6] = [
        (1, 6 + tail_off, outline),
        (2, 6 + tail_off, body),
        (1, 7 + tail_off, body),
        (2, 7 + tail_off, body_dk),
        (0, 7 + tail_off, outline),
        (1, 8 + tail_off, outline),
    ];
    for (x, y, c) in tail_pixels { plot(img, x, y, c); }

    // Collar
    for x in 13..=17 { plot(img, x, 12, collar); }
    plot(img, 15, 13, buckle);

    // Legs animated by frame — 4 legs total, alternating gait
    let (front_off, back_off) = match frame & 3 {
        0 => (0i32, 1i32),
        1 => (1, 0),
        2 => (0, 1),
        _ => (1, 0),
    };
    for &(lx, off) in &[(4i32, back_off), (8, back_off),
                        (13, front_off), (16, front_off)] {
        for y in 17..=21 {
            plot(img, lx,     y, body_dk);
            plot(img, lx + 1, y, body);
        }
        // Paw
        plot(img, lx,     21 + off, outline);
        plot(img, lx + 1, 21 + off, outline);
        plot(img, lx + 2, 21 + off, outline);
    }
}

fn paint_mower_hi(img: &mut Image, frame: u8) {
    // 32×16 riding mower facing right. Two frames cycle the wheel
    // spokes so it reads as rolling. Yellow body, red engine cowl,
    // gray blade housing slung underneath, black wheels.
    let frame_off_x = MOWER_X + (frame as u16 % 2) * MOWER_W;
    let plot = |img: &mut Image, x: i32, y: i32, c: Color| {
        if x >= 0 && (x as u16) < MOWER_W && y >= 0 && (y as u16) < MOWER_H {
            img.set_pixel(frame_off_x as u32 + x as u32,
                          MOWER_Y  as u32 + y as u32, c);
        }
    };
    let body    = rgb(248, 196, 60);   // mower yellow
    let body_dk = rgb(184, 136, 28);
    let body_hi = rgb(255, 232, 120);
    let cowl    = rgb(196, 48, 36);    // engine red
    let cowl_dk = rgb(132, 28, 20);
    let blade   = rgb(120, 124, 132);
    let blade_dk= rgb(72, 76, 84);
    let seat    = rgb(48, 28, 20);
    let outline = rgb(24, 16, 8);
    let tire    = rgb(16, 12, 10);
    let hub     = rgb(160, 156, 148);
    let spoke   = rgb(96, 92, 88);
    let exhaust = rgb(96, 96, 96);

    // Blade housing: slim deck slung along the bottom, x=4..28, y=10..12.
    for y in 10..=11 { for x in 4..=27 { plot(img, x, y, blade); } }
    for x in 4..=27 { plot(img, x, 12, blade_dk); }
    plot(img, 3, 11, blade_dk); plot(img, 28, 11, blade_dk);

    // Main chassis body (yellow), top of deck.
    for y in 6..=9 { for x in 5..=24 { plot(img, x, y, body); } }
    // Highlight stripe along the top.
    for x in 6..=23 { plot(img, x, 6, body_hi); }
    // Bottom-shadow line on chassis.
    for x in 5..=24 { plot(img, x, 9, body_dk); }
    // Chassis outline.
    for x in 5..=24 { plot(img, x, 5, outline); }
    plot(img, 4, 6, outline); plot(img, 4, 7, outline); plot(img, 4, 8, outline); plot(img, 4, 9, outline);
    plot(img, 25, 6, outline); plot(img, 25, 7, outline); plot(img, 25, 8, outline); plot(img, 25, 9, outline);

    // Engine cowl (red box at the front-right).
    for y in 4..=7 { for x in 19..=24 { plot(img, x, y, cowl); } }
    for x in 19..=24 { plot(img, x, 3, outline); }
    for y in 4..=7   { plot(img, 25, y, outline); }
    plot(img, 18, 4, outline); plot(img, 18, 5, outline); plot(img, 18, 6, outline); plot(img, 18, 7, outline);
    for x in 20..=23 { plot(img, x, 4, cowl_dk); }
    plot(img, 22, 5, body_hi); // chrome glint

    // Exhaust pipe poking up from the cowl.
    plot(img, 23, 2, outline); plot(img, 23, 1, outline); plot(img, 23, 0, outline);
    plot(img, 24, 2, exhaust); plot(img, 24, 1, exhaust);

    // Driver seat (small bucket near the back-left of the chassis).
    for y in 3..=5 { for x in 9..=12 { plot(img, x, y, seat); } }
    plot(img, 8, 4, outline); plot(img, 8, 5, outline);
    plot(img, 13, 4, outline); plot(img, 13, 5, outline);
    for x in 9..=12 { plot(img, x, 2, outline); }

    // Steering column from seat-front diagonally up to a wheel.
    plot(img, 14, 5, outline);
    plot(img, 15, 4, outline);
    plot(img, 16, 3, outline);
    plot(img, 16, 2, outline);
    // Tiny steering wheel
    plot(img, 17, 2, outline);
    plot(img, 17, 3, outline);

    // Front grille bars on the cowl.
    plot(img, 25, 5, outline);
    plot(img, 25, 6, outline);

    // ── Wheels ─────────────────────────────────────────────────
    // Big rear wheel at x=5..9, y=10..14.  Small front at x=22..25, y=11..14.
    let rear  = (7, 12);   // centre
    let front = (24, 13);

    // Rear wheel disc
    for dy in -2..=2 {
        for dx in -2..=2 {
            if dx*dx + dy*dy <= 5 {
                plot(img, rear.0 + dx, rear.1 + dy, tire);
            }
        }
    }
    plot(img, rear.0, rear.1, hub);
    // Spoke pattern alternates by frame for rolling animation.
    let s = match frame & 1 {
        0 => [(-1, 0i32), (1, 0), (0, -1), (0, 1)],
        _ => [(-1, -1),   (1, 1), (-1, 1), (1, -1)],
    };
    for (dx, dy) in s.iter() {
        plot(img, rear.0 + dx, rear.1 + dy, spoke);
    }

    // Front wheel disc (smaller)
    for dy in -1..=1 {
        for dx in -1..=1 {
            if dx*dx + dy*dy <= 2 {
                plot(img, front.0 + dx, front.1 + dy, tire);
            }
        }
    }
    plot(img, front.0, front.1, hub);
    let fs = match frame & 1 {
        0 => [(-1, 0i32), (1, 0)],
        _ => [(0, -1),    (0, 1)],
    };
    for (dx, dy) in fs.iter() {
        plot(img, front.0 + dx, front.1 + dy, spoke);
    }
}

fn paint_tree_hi(img: &mut Image) {
    // 24×48 — tall oak with a layered foliage crown, knobbly trunk, root
    // flare at the base, and a few hidden apples.
    let plot = |img: &mut Image, x: i32, y: i32, c: Color| {
        if x >= 0 && (x as u16) < TREE_W && y >= 0 && (y as u16) < TREE_H {
            img.set_pixel(TREE_X as u32 + x as u32,
                          TREE_Y as u32 + y as u32, c);
        }
    };
    let trunk    = rgb(108, 72, 36);
    let trunk_dk = rgb(72, 44, 22);
    let trunk_hi = rgb(140, 96, 52);
    let bark     = rgb(48, 30, 14);
    let foli_dk  = rgb(36, 92, 32);
    let foli_mid = rgb(60, 140, 48);
    let foli_hi  = rgb(120, 200, 80);
    let foli_glow = rgb(176, 232, 96);
    let outline  = rgb(20, 48, 16);
    let apple    = rgb(220, 60, 40);
    let apple_hi = rgb(248, 120, 80);

    // Trunk: rows 30-47, cols 10-13
    for y in 30..48 {
        plot(img, 10, y, trunk_dk);
        plot(img, 11, y, trunk);
        plot(img, 12, y, trunk_hi);
        plot(img, 13, y, trunk_dk);
    }
    // Bark rings
    for &y in &[33, 38, 42, 45] {
        plot(img, 11, y, bark);
        plot(img, 12, y, bark);
    }
    // Root flare at base
    plot(img,  9, 46, trunk_dk);
    plot(img,  9, 47, trunk_dk);
    plot(img, 14, 46, trunk_dk);
    plot(img, 14, 47, trunk_dk);
    plot(img,  8, 47, bark);
    plot(img, 15, 47, bark);
    // Branch nubs poking out near top of trunk
    plot(img,  9, 30, trunk_dk);
    plot(img, 14, 32, trunk_dk);

    // Crown — circle around (12, 14) with radius ~12, with procedural lumps
    for y in 0..32 {
        for x in 0..24 {
            let dx = (x - 12) as f32;
            let dy = (y - 14) as f32 * 0.85;
            let r = (dx * dx + dy * dy).sqrt();
            // Bumpy edge from a hash
            let lump = (((x * 7) ^ (y * 11)) & 7) as f32 * 0.30 - 0.9;
            let r_eff = r + lump;
            if r_eff > 12.0 { continue; }
            let inner_h = ((x * 13) ^ (y * 17)) & 7;
            let c = if r_eff > 11.0 { outline }
                    else if r_eff > 10.0 { foli_dk }
                    else if r_eff > 7.5 {
                        match inner_h { 0 | 1 => foli_dk, _ => foli_mid }
                    } else if r_eff > 4.0 {
                        match inner_h {
                            0 => foli_glow,
                            1 | 2 => foli_hi,
                            3 | 4 | 5 => foli_mid,
                            _ => foli_dk,
                        }
                    } else if r_eff > 1.5 {
                        match inner_h { 0 | 1 => foli_glow, _ => foli_hi }
                    } else { foli_glow };
            plot(img, x, y, c);
        }
    }
    // Hand-placed lit highlights on upper-left of crown
    for &(x, y) in &[(7i32,4i32),(8,4),(9,4),(7,5),(8,5),(7,6),(8,7),(9,7)] {
        plot(img, x, y, foli_glow);
    }
    // Hidden apples
    for &(ax, ay) in &[(17i32,11i32),(7,18),(18,21),(11,7)] {
        plot(img, ax,     ay,     apple);
        plot(img, ax + 1, ay,     apple_hi);
        plot(img, ax,     ay + 1, apple);
    }
}

// ── small (cell-grid) scenery painters — sun, moon, cloud ───────────────────

#[allow(dead_code)]
fn paint_barn(img: &mut Image, col: u16, row: u16) {
    // 24×16 source rendered at 6× pixel-art scale on screen (18×12 tiles),
    // so every source pixel becomes a 6×6 block. Hand-placed per-pixel to
    // match the per-tile detail level of the underground: weathered roof
    // shingles in five tones, wall planks with knots and grain, framed
    // windows with reflected sky, iron-strapped double doors with brass
    // ring, mortared stone foundation.
    let bx = col * CELL;
    let by = row * CELL;

    // ── Palette ───────────────────────────────────────────────────────
    let shingle_dark = rgb(36, 12, 10);
    let shingle_mid  = rgb(96, 36, 24);
    let shingle_mid2 = rgb(124, 52, 36);
    let shingle_hi   = rgb(176, 88, 64);
    let shingle_aged = rgb(80, 56, 40);          // weathered brown-red
    let trim_hi      = rgb(252, 244, 212);
    let trim_mid     = rgb(220, 204, 160);
    let trim_dk      = rgb(160, 144, 104);
    let plank_seam   = rgb(40, 8, 6);
    let plank_dk     = rgb(112, 24, 16);
    let plank_mid    = rgb(168, 36, 24);
    let plank_hi     = rgb(212, 60, 42);
    let plank_grain  = rgb(140, 30, 22);
    let knot         = rgb(56, 12, 8);
    let weather      = rgb(132, 84, 56);          // rain streak brown
    let glass_dk     = rgb(28, 60, 116);
    let glass_mid    = rgb(72, 132, 200);
    let glass_hi     = rgb(184, 220, 240);        // sky reflection
    let door_dk      = rgb(36, 18, 8);
    let door_mid     = rgb(80, 48, 24);
    let door_hi      = rgb(124, 84, 48);
    let door_grain   = rgb(56, 30, 14);
    let iron_dk      = rgb(20, 18, 28);
    let iron_hi      = rgb(76, 72, 92);
    let brass        = rgb(248, 200, 96);
    let brass_dk     = rgb(168, 124, 36);
    let stone_hi     = rgb(176, 172, 192);
    let stone_mid    = rgb(124, 120, 140);
    let stone_dk     = rgb(76, 72, 96);
    let mortar       = rgb(36, 32, 48);

    let plot = |img: &mut Image, x: i32, y: i32, c: Color| {
        if x >= 0 && x < 24 && y >= 0 && y < 16 {
            img.set_pixel((bx as i32 + x) as u32,
                          (by as i32 + y) as u32, c);
        }
    };

    // ── Weathervane ───────────────────────────────────────────────────
    // Tall iron post above the peak with a brass arrow
    plot(img, 11, 0, iron_dk);
    plot(img, 12, 0, iron_dk);
    plot(img, 11, 1, iron_hi);
    plot(img, 12, 1, iron_hi);
    // Arrow wings spread horizontally
    plot(img,  9, 1, brass_dk);
    plot(img, 10, 1, brass);
    plot(img, 13, 1, brass);
    plot(img, 14, 1, brass_dk);
    // Tail feather + arrow tip flourish
    plot(img,  8, 1, brass_dk);
    plot(img, 15, 1, brass_dk);

    // ── Roof ──────────────────────────────────────────────────────────
    // Five-row triangular peak with weathered shingles. Each row gets
    // varied tone with edge darkening and a top-row highlight strip.
    let roof_rows: [(i32, i32, i32); 5] = [
        (2, 10, 13),
        (3,  7, 16),
        (4,  4, 19),
        (5,  1, 22),
        (6,  0, 23),
    ];
    for (y, l, r) in roof_rows {
        for x in l..=r {
            // Edge of each row darkens
            let on_edge = x == l || x == r;
            // Hash-style weathering: roughly 1 in 8 shingles is aged
            let aged = ((x.wrapping_mul(13) ^ y.wrapping_mul(31)) & 7) == 0;
            // Position-based shingle tone
            let stripe = ((x - l) + y * 3) % 5;
            let c = if on_edge   { shingle_dark }
                    else if aged { shingle_aged }
                    else if stripe == 0 { shingle_hi }
                    else if stripe == 1 { shingle_mid2 }
                    else if stripe < 4  { shingle_mid }
                    else                { shingle_dark };
            plot(img, x, y, c);
        }
    }
    // Top-of-row highlight strip on the lit edge of each shingle band —
    // gives the roof a "lit from upper-left" feel without re-lighting
    // every pixel.
    for &(y, l, _) in &roof_rows {
        plot(img, l + 1, y, shingle_hi);
        plot(img, l + 2, y, shingle_hi);
    }

    // ── Eave trim ─────────────────────────────────────────────────────
    // 2-pixel band: bright cream above, slight shadow line below
    for x in 0..24 {
        let c = if x == 0 || x == 23 { trim_dk }
                else if x % 5 == 0   { trim_hi }
                else                 { trim_mid };
        plot(img, x, 7, c);
    }
    // Cast shadow under the eaves onto the wall
    for x in 0..24 { plot(img, x, 8, plank_seam); }

    // ── Walls ─────────────────────────────────────────────────────────
    // Six rows of plank wall (rows 9..14). Plank seams every 5 cols. Body
    // alternates two tones row-by-row with grain flecks; deliberate knots
    // and rain-streak weathering at hand-picked positions.
    let seams = [0i32, 5, 10, 15, 20, 23];
    for y in 9..14 {
        for x in 0..24 {
            // Body tone
            let body = if y == 9 || y == 13 { plank_dk }
                       else if y % 2 == 0  { plank_mid }
                       else                { plank_hi };
            plot(img, x, y, body);
        }
        // Grain flecks every other column
        for x in (1..23).step_by(2) {
            if (x ^ y as i32) & 3 == 0 {
                plot(img, x, y, plank_grain);
            }
        }
    }
    // Plank seams: vertical dark lines
    for &x in &seams {
        for y in 9..14 {
            plot(img, x, y, plank_seam);
        }
    }
    // Knots — hand-placed along the planks
    let knots = [(2i32, 9i32), (8, 11), (16, 10), (21, 12), (3, 13), (18, 13)];
    for &(kx, ky) in &knots {
        plot(img, kx, ky, knot);
        // Halo darker around knot
        if kx + 1 < 24 && (kx + 1) % 5 != 0 { plot(img, kx + 1, ky, plank_dk); }
    }
    // Vertical rain-streak weathering — three short streaks
    for y in 10..14 { plot(img, 1, y, weather); }
    for y in 10..13 { plot(img, 22, y, weather); }
    for y in 11..14 { plot(img, 13, y, weather); }

    // ── Hayloft window (high centred) ─────────────────────────────────
    // 4×3 panes of glass with a cream cross mullion and corner reflections
    for y in 9..12 {
        for x in 10..14 {
            // Sky reflection in the upper-right pane
            let c = if x >= 12 && y == 9 { glass_hi }
                    else if y == 9 { glass_mid }
                    else if y == 10 { glass_mid }
                    else { glass_dk };
            plot(img, x, y, c);
        }
    }
    // Frame
    for x in 10..14 {
        plot(img, x, 9, trim_mid);
        plot(img, x, 11, trim_dk);
    }
    for y in 9..12 {
        plot(img, 10, y, trim_mid);
        plot(img, 13, y, trim_dk);
    }
    // Cross mullion
    plot(img, 11, 10, trim_hi);
    plot(img, 12, 10, trim_hi);
    plot(img, 11, 9,  trim_hi);
    plot(img, 12, 11, trim_dk);

    // ── Two side windows with full mullions and reflections ───────────
    for &xs in &[2i32, 18] {
        for y in 10..13 {
            for x in xs..(xs + 4) {
                let c = if x == xs && y == 10 { glass_hi }   // corner reflection
                        else if y == 10 { glass_mid }
                        else if y == 11 { glass_mid }
                        else { glass_dk };
                plot(img, x, y, c);
            }
        }
        // Frames (top + bottom + sides)
        for x in xs..(xs + 4) {
            plot(img, x, 10, trim_mid);
            plot(img, x, 12, trim_dk);
        }
        for y in 10..13 {
            plot(img, xs,     y, trim_mid);
            plot(img, xs + 3, y, trim_dk);
        }
        // Cross mullions
        plot(img, xs + 1, 11, trim_hi);
        plot(img, xs + 2, 11, trim_hi);
        // Tiny sill shadow below
        for x in xs - 1 ..= xs + 4 {
            if x >= 0 && x < 24 { plot(img, x, 13, plank_seam); }
        }
    }

    // ── Double doors ──────────────────────────────────────────────────
    // 7-wide × 4-tall double-leaf doors with iron strap hinges, vertical
    // plank grain, brass ring handle.
    for y in 9..14 {
        for x in 8..16 {
            // Vertical plank tone
            let c = if (x - 8) % 2 == 0 { door_mid } else { door_hi };
            plot(img, x, y, c);
        }
    }
    // Plank seams within door
    for y in 9..14 {
        plot(img,  9, y, door_grain);
        plot(img, 11, y, door_grain);
        plot(img, 14, y, door_grain);
    }
    // Door split down the centre
    for y in 9..14 {
        plot(img, 12, y, door_dk);
    }
    // Iron strap hinges (top + bottom, both leaves)
    for x in 8..12 {
        plot(img, x, 9,  iron_dk);
        plot(img, x, 13, iron_dk);
    }
    for x in 13..16 {
        plot(img, x, 9,  iron_dk);
        plot(img, x, 13, iron_dk);
    }
    // Hinge highlights
    plot(img,  8, 9,  iron_hi);
    plot(img, 11, 9,  iron_hi);
    plot(img, 13, 9,  iron_hi);
    plot(img, 15, 9,  iron_hi);
    // Door frame outline
    for y in 9..14 {
        plot(img,  8, y, door_dk);
        plot(img, 15, y, door_dk);
    }
    plot(img,  8, 9, iron_dk);
    plot(img, 15, 9, iron_dk);
    plot(img,  8, 13, iron_dk);
    plot(img, 15, 13, iron_dk);
    // Brass ring handles
    plot(img, 11, 11, brass);
    plot(img, 13, 11, brass);
    plot(img, 11, 12, brass_dk);
    plot(img, 13, 12, brass_dk);

    // ── Stone foundation ──────────────────────────────────────────────
    // Two-row band of irregular stones with mortar between. Each stone
    // gets a small lit highlight on its top-left and shadow on its
    // bottom-right; mortar lines run between.
    // Define the stone footprints as (x0, x1, row): each stone spans
    // columns [x0..x1] on `row`.
    let foundation_top: [(i32, i32); 8] = [
        (0, 2), (3, 5), (6, 8), (9, 12), (13, 15), (16, 18), (19, 21), (22, 23),
    ];
    let foundation_bot: [(i32, i32); 8] = [
        (0, 1), (2, 4), (5, 7), (8, 10), (11, 14), (15, 17), (18, 20), (21, 23),
    ];
    for &(l, r) in &foundation_top {
        for x in l..=r {
            let body = if (x ^ 14) & 1 == 0 { stone_mid } else { stone_dk };
            plot(img, x, 14, body);
        }
        plot(img, l, 14, stone_hi);              // lit corner
        if r > l { plot(img, r, 14, stone_dk); } // shadow corner
    }
    for &(l, r) in &foundation_bot {
        for x in l..=r {
            let body = if (x ^ 15) & 1 == 0 { stone_dk } else { stone_mid };
            plot(img, x, 15, body);
        }
        plot(img, l, 15, stone_mid);
        if r > l { plot(img, r, 15, stone_dk); }
    }
    // Mortar lines: where the stone segments end, set mortar pixels
    for &(_, r) in &foundation_top {
        if r + 1 < 24 { plot(img, r + 1, 14, mortar); }
    }
    for &(_, r) in &foundation_bot {
        if r + 1 < 24 { plot(img, r + 1, 15, mortar); }
    }
    // Foundation top edge — thin shadow of the wall sitting on stone
    for x in 0..24 {
        if x % 3 == 0 { plot(img, x, 13, plank_seam); }
    }
}

#[allow(dead_code)]
fn paint_dog(img: &mut Image, col: u16, row: u16, frame: u8) {
    // 8×8 stocky dog. Tan body, dark muzzle, animated leg pairs.
    let body  = rgb(212, 168, 92);
    let body2 = rgb(180, 132, 64);
    let dark  = rgb(72, 44, 18);
    let nose  = rgb(28, 16, 8);
    let eye   = rgb(28, 16, 8);
    let bx = col * CELL;
    let by = row * CELL;
    // Body
    for y in 3..6 {
        for x in 1..7 {
            img.set_pixel((bx + x) as u32, (by + y) as u32, body);
        }
    }
    // Belly shading
    for x in 1..7 {
        img.set_pixel((bx + x) as u32, (by + 5) as u32, body2);
    }
    // Head (front-right)
    img.set_pixel((bx + 6) as u32, (by + 2) as u32, body);
    img.set_pixel((bx + 7) as u32, (by + 2) as u32, body);
    img.set_pixel((bx + 6) as u32, (by + 3) as u32, body);
    img.set_pixel((bx + 7) as u32, (by + 3) as u32, body);
    img.set_pixel((bx + 7) as u32, (by + 4) as u32, nose);
    img.set_pixel((bx + 6) as u32, (by + 1) as u32, dark); // ear
    // Eye
    img.set_pixel((bx + 6) as u32, (by + 3) as u32, eye);
    // Tail (back-left)
    img.set_pixel((bx + 0) as u32, (by + 2) as u32, body);
    img.set_pixel((bx + 0) as u32, (by + 3) as u32, body);
    // Legs animate
    let (front_y, back_y) = match frame & 3 {
        0 => (6, 7),
        1 => (7, 6),
        2 => (6, 7),
        _ => (7, 6),
    };
    img.set_pixel((bx + 1) as u32, (by + back_y) as u32, dark);
    img.set_pixel((bx + 2) as u32, (by + back_y) as u32, dark);
    img.set_pixel((bx + 5) as u32, (by + front_y) as u32, dark);
    img.set_pixel((bx + 6) as u32, (by + front_y) as u32, dark);
}

#[allow(dead_code)]
fn paint_tree(img: &mut Image, col: u16, row: u16) {
    // 16×16 little oak. Brown trunk, layered foliage in two greens.
    let trunk_dark = rgb(72, 44, 22);
    let trunk      = rgb(108, 70, 36);
    let foliage_dark = rgb(36, 92, 32);
    let foliage_mid  = rgb(60, 140, 48);
    let foliage_hi   = rgb(112, 188, 72);
    let bx = col * CELL;
    let by = row * CELL;
    // Trunk (lower 6 rows, narrow)
    for y in 10..16 {
        img.set_pixel((bx + 7) as u32, (by + y) as u32, trunk);
        img.set_pixel((bx + 8) as u32, (by + y) as u32, trunk_dark);
    }
    // Crown (rough disc)
    let crown = [
        // (x, y) offsets relative to top-left of 16×16
        (5,1),(6,1),(7,1),(8,1),(9,1),(10,1),
        (3,2),(4,2),(5,2),(6,2),(7,2),(8,2),(9,2),(10,2),(11,2),(12,2),
        (2,3),(3,3),(4,3),(5,3),(6,3),(7,3),(8,3),(9,3),(10,3),(11,3),(12,3),(13,3),
        (2,4),(3,4),(4,4),(5,4),(6,4),(7,4),(8,4),(9,4),(10,4),(11,4),(12,4),(13,4),
        (3,5),(4,5),(5,5),(6,5),(7,5),(8,5),(9,5),(10,5),(11,5),(12,5),
        (4,6),(5,6),(6,6),(7,6),(8,6),(9,6),(10,6),(11,6),
        (5,7),(6,7),(7,7),(8,7),(9,7),(10,7),
        (6,8),(7,8),(8,8),(9,8),
    ];
    for (x, y) in crown {
        img.set_pixel((bx + x as u16) as u32, (by + y as u16) as u32, foliage_mid);
    }
    // Highlights (top-left lighter)
    for &(x, y) in &[(5u16,2u16),(6,2),(7,2),(4,3),(5,3),(6,3),(5,4)] {
        img.set_pixel((bx + x) as u32, (by + y) as u32, foliage_hi);
    }
    // Shadow (bottom-right darker)
    for &(x, y) in &[(11u16,5u16),(12,5),(11,6),(10,7),(11,7),(9,8)] {
        img.set_pixel((bx + x) as u32, (by + y) as u32, foliage_dark);
    }
}

fn paint_sun(img: &mut Image, col: u16, row: u16) {
    let core = rgb(255, 232, 120);
    let edge = rgb(248, 188, 64);
    let bx = col * CELL;
    let by = row * CELL;
    let pixels = [
        (3,1),(4,1),
        (2,2),(3,2),(4,2),(5,2),
        (1,3),(2,3),(3,3),(4,3),(5,3),(6,3),
        (1,4),(2,4),(3,4),(4,4),(5,4),(6,4),
        (2,5),(3,5),(4,5),(5,5),
        (3,6),(4,6),
    ];
    for (x, y) in pixels {
        let c = if x == 1 || x == 6 || y == 1 || y == 6 { edge } else { core };
        img.set_pixel((bx + x as u16) as u32, (by + y as u16) as u32, c);
    }
}

fn paint_moon(img: &mut Image, col: u16, row: u16) {
    let core   = rgb(232, 232, 248);
    let shadow = rgb(140, 140, 168);
    let crater = rgb(96, 96, 132);
    let bx = col * CELL;
    let by = row * CELL;
    let body = [
        (3,1),(4,1),
        (2,2),(3,2),(4,2),(5,2),
        (2,3),(3,3),(4,3),(5,3),(6,3),
        (2,4),(3,4),(4,4),(5,4),(6,4),
        (2,5),(3,5),(4,5),(5,5),
        (3,6),(4,6),
    ];
    for (x, y) in body {
        img.set_pixel((bx + x as u16) as u32, (by + y as u16) as u32, core);
    }
    // Right-side shadow + crater dots
    for &(x,y) in &[(5u16,2u16),(6,3),(6,4),(5,5)] {
        img.set_pixel((bx + x) as u32, (by + y) as u32, shadow);
    }
    img.set_pixel((bx + 3) as u32, (by + 3) as u32, crater);
    img.set_pixel((bx + 4) as u32, (by + 5) as u32, crater);
}

fn paint_soldier_ant(img: &mut Image, col: u16, row: u16) {
    // Larger, darker, more armoured silhouette than worker. Iron-grey
    // chitin with black outline and faint red highlights — visibly a
    // different caste at any zoom.
    let outline = rgb(4, 2, 6);
    let body    = rgb(48, 36, 56);
    let body_hi = rgb(112, 92, 124);
    let armor   = rgb(176, 64, 48);
    let leg     = rgb(16, 12, 18);
    let frame = (col % 4) as i32;
    let wig = if frame == 0 || frame == 2 { 0 } else { 1 };

    // Outline ring (slightly fatter than worker)
    for &(x, y) in &[
        (1u16, 2u16), (2, 1), (3, 1), (4, 1), (5, 1), (6, 1), (7, 2),
        (1, 3), (1, 4), (1, 5),                 (7, 3), (7, 4), (7, 5),
        (2, 6), (3, 6), (4, 6), (5, 6), (6, 6),
    ] {
        px(img, col*CELL + x, row*CELL + y, outline);
    }
    // Body fill (taller than worker — fills 2..6)
    for y in 2..=5 {
        for x in 2..=6 {
            px(img, col*CELL + x, row*CELL + y, body);
        }
    }
    // Highlight strip
    px(img, col*CELL + 3, row*CELL + 2, body_hi);
    px(img, col*CELL + 4, row*CELL + 2, body_hi);
    px(img, col*CELL + 5, row*CELL + 2, body_hi);
    // Red shoulder armour band
    px(img, col*CELL + 2, row*CELL + 3, armor);
    px(img, col*CELL + 6, row*CELL + 3, armor);
    px(img, col*CELL + 6, row*CELL + 4, armor);
    // Mandibles / head accent
    px(img, col*CELL + 6, row*CELL + 5, body_hi);

    // Legs
    px(img, col*CELL + 2, row*CELL + (6 + wig) as u16, leg);
    px(img, col*CELL + 4, row*CELL + (6 + wig) as u16, leg);
    px(img, col*CELL + 6, row*CELL + (6 + wig) as u16, leg);
    px(img, col*CELL + 3, row*CELL + (7 - wig) as u16, leg);
    px(img, col*CELL + 5, row*CELL + (7 - wig) as u16, leg);
}

fn paint_brood(img: &mut Image, col: u16, row: u16) {
    // Cream-coloured oval larva.
    let outline = rgb(108, 92, 56);
    let body    = rgb(232, 216, 180);
    let body_hi = rgb(248, 240, 220);
    let shadow  = rgb(176, 152, 108);
    let pixels: [(u16, u16, Color); 18] = [
        (3,2,outline), (4,2,outline),
        (2,3,outline), (3,3,body_hi), (4,3,body_hi), (5,3,outline),
        (2,4,body_hi), (3,4,body),    (4,4,body),    (5,4,body),
        (2,5,body),    (3,5,body),    (4,5,body),    (5,5,shadow),
        (2,6,outline), (3,6,shadow),  (4,6,shadow),  (5,6,outline),
    ];
    for (x, y, c) in pixels {
        px(img, col*CELL + x, row*CELL + y, c);
    }
}

fn paint_corpse(img: &mut Image, col: u16, row: u16) {
    // Curled-up dark silhouette to read as a body, not a food pellet.
    let dark = rgb(28, 18, 12);
    let mid  = rgb(56, 36, 22);
    let pixels: [(u16, u16, Color); 14] = [
        (2,4,dark), (3,4,dark), (4,4,mid),  (5,4,dark),
        (1,5,dark), (2,5,mid),  (3,5,mid),  (4,5,mid),  (5,5,mid),  (6,5,dark),
        (2,6,dark), (3,6,dark), (4,6,dark), (5,6,dark),
    ];
    for (x, y, c) in pixels {
        px(img, col*CELL + x, row*CELL + y, c);
    }
}

fn paint_spider(img: &mut Image, col: u16, row: u16, frame: u8) {
    // 16×16 spider — round dark body, glowing red eyes, eight legs.
    let body_dk = rgb(14, 12, 18);
    let body    = rgb(38, 34, 48);
    let body_hi = rgb(82, 76, 96);
    let leg     = rgb(8, 6, 12);
    let eye     = rgb(255, 32, 28);
    let bx = col * CELL;
    let by = row * CELL;

    let plot = |img: &mut Image, x: i32, y: i32, c: Color| {
        if x >= 0 && y >= 0 && x < 16 && y < 16 {
            img.set_pixel((bx as i32 + x) as u32, (by as i32 + y) as u32, c);
        }
    };

    // Body — rounded oval, pixels (5..11, 5..11)
    let body_cells: [(i32, i32); 28] = [
        (6,5),(7,5),(8,5),(9,5),
        (5,6),(6,6),(7,6),(8,6),(9,6),(10,6),
        (5,7),(6,7),(7,7),(8,7),(9,7),(10,7),
        (5,8),(6,8),(7,8),(8,8),(9,8),(10,8),
        (5,9),(6,9),(7,9),(8,9),(9,9),(10,9),
    ];
    for (x, y) in body_cells { plot(img, x, y, body); }
    // Top highlight
    for &(x, y) in &[(7i32,5i32),(8,5),(6,6),(7,6)] { plot(img, x, y, body_hi); }
    // Bottom shadow
    for &(x, y) in &[(5i32,9i32),(6,9),(9,9),(10,9)] { plot(img, x, y, body_dk); }
    // Eyes
    plot(img, 6, 7, eye);
    plot(img, 9, 7, eye);

    // Legs — 4 per side, animated by `frame`. Each leg is a 3-pixel diagonal
    // line. Frame 0 has legs spread "high"; frame 1 has them "low" — the
    // alternation reads as scuttling motion.
    let wig = if frame == 0 { 0 } else { 1 };

    // Left side legs (out from x=4..0)
    let left_legs: [(i32, i32, i32, i32, i32, i32); 4] = [
        // (knee_x, knee_y, foot_x, foot_y, mid_x, mid_y) — uses wig offset
        (4, 5 + wig, 1, 4 + wig, 3, 5 + wig),
        (4, 6 + wig, 1, 6 + wig, 3, 6 + wig),
        (4, 8 - wig, 1, 9 - wig, 3, 8 - wig),
        (4, 9 - wig, 1, 11 - wig, 3, 10 - wig),
    ];
    for (kx, ky, fx, fy, mx, my) in left_legs {
        plot(img, mx, my, leg);
        plot(img, kx, ky, leg);
        plot(img, fx, fy, leg);
    }
    // Right side legs — mirrored
    let right_legs: [(i32, i32, i32, i32, i32, i32); 4] = [
        (11, 5 + wig, 14, 4 + wig, 12, 5 + wig),
        (11, 6 + wig, 14, 6 + wig, 12, 6 + wig),
        (11, 8 - wig, 14, 9 - wig, 12, 8 - wig),
        (11, 9 - wig, 14, 11 - wig, 12, 10 - wig),
    ];
    for (kx, ky, fx, fy, mx, my) in right_legs {
        plot(img, mx, my, leg);
        plot(img, kx, ky, leg);
        plot(img, fx, fy, leg);
    }
}

fn paint_queen(img: &mut Image, col: u16, row: u16) {
    // 16×16 royal ant: deep purple body, lavender highlights, gold crown.
    let outline = rgb(20, 4, 28);
    let body    = rgb(120, 60, 168);
    let body_hi = rgb(180, 116, 220);
    let body_dk = rgb(72, 28, 110);
    let crown_hi  = rgb(252, 224, 96);
    let crown_dk  = rgb(176, 132, 32);
    let leg     = rgb(36, 12, 48);
    let bx = col * CELL;
    let by = row * CELL;

    let plot = |img: &mut Image, x: u16, y: u16, c: Color| {
        img.set_pixel((bx + x) as u32, (by + y) as u32, c);
    };

    // Body — three blobs (abdomen, thorax, head) outlined in dark purple.
    let body_cells: [(u16, u16); 40] = [
        // abdomen (rear, big)
        (2,6),(3,6),(4,6),(5,6),
        (1,7),(2,7),(3,7),(4,7),(5,7),(6,7),
        (1,8),(2,8),(3,8),(4,8),(5,8),(6,8),
        (2,9),(3,9),(4,9),(5,9),
        // thorax
        (7,7),(8,7),(9,7),
        (7,8),(8,8),(9,8),
        // head
        (10,7),(11,7),(12,7),
        (10,8),(11,8),(12,8),
        // long royal legs
        (3,10),(5,10),(8,10),(11,10),
        (4,11),(6,11),(9,11),(11,11),
    ];
    for &(x, y) in &body_cells { plot(img, x, y, body); }

    // Outline around abdomen
    for &(x, y) in &[(0u16,7u16),(0,8),(1,6),(2,5),(3,5),(4,5),(5,5),(6,6),(7,8),(7,7)] {
        plot(img, x, y, outline);
    }
    // Highlights
    for &(x, y) in &[(2u16,7u16),(3,7),(8,7),(11,7)] { plot(img, x, y, body_hi); }
    // Shadows
    for &(x, y) in &[(5u16,9u16),(6,8),(9,8),(12,8)] { plot(img, x, y, body_dk); }

    // Legs
    for &(x, y) in &[(3u16,10u16),(5,10),(8,10),(11,10),
                     (4,11),(6,11),(9,11),(11,11)] {
        plot(img, x, y, leg);
    }

    // Crown — three points above the head
    plot(img, 10, 5, crown_dk);
    plot(img, 11, 5, crown_dk);
    plot(img, 12, 5, crown_dk);
    plot(img, 10, 6, crown_hi);
    plot(img, 11, 6, crown_hi);
    plot(img, 12, 6, crown_hi);
    plot(img, 10, 4, crown_hi);
    plot(img, 12, 4, crown_hi);
    plot(img, 11, 3, crown_hi);

    // Faint wing shimmer behind thorax
    let wing = Color::new(0.85, 0.78, 1.0, 0.4);
    plot(img, 7, 4, wing);
    plot(img, 8, 4, wing);
    plot(img, 9, 4, wing);
    plot(img, 7, 5, wing);
    plot(img, 8, 5, wing);
    plot(img, 9, 5, wing);
    plot(img, 6, 5, wing);
}

fn paint_cloud(img: &mut Image, col: u16, row: u16) {
    let core = rgb(244, 244, 252);
    let edge = rgb(196, 204, 224);
    let bx = col * CELL;
    let by = row * CELL;
    // 24×8 puffy cloud, rounded edges
    let body = [
        (4,1),(5,1),(6,1),(13,1),(14,1),
        (3,2),(4,2),(5,2),(6,2),(7,2),(8,2),(11,2),(12,2),(13,2),(14,2),(15,2),(16,2),(17,2),
        (2,3),(3,3),(4,3),(5,3),(6,3),(7,3),(8,3),(9,3),(10,3),(11,3),(12,3),(13,3),(14,3),(15,3),(16,3),(17,3),(18,3),(19,3),(20,3),
        (2,4),(3,4),(4,4),(5,4),(6,4),(7,4),(8,4),(9,4),(10,4),(11,4),(12,4),(13,4),(14,4),(15,4),(16,4),(17,4),(18,4),(19,4),(20,4),(21,4),
        (3,5),(4,5),(5,5),(6,5),(7,5),(8,5),(9,5),(10,5),(11,5),(12,5),(13,5),(14,5),(15,5),(16,5),(17,5),(18,5),(19,5),(20,5),
    ];
    for (x, y) in body {
        img.set_pixel((bx + x as u16) as u32, (by + y as u16) as u32, core);
    }
    // Edges
    for &(x, y) in &[(2u16,3u16),(2,4),(21,4),(20,5),(3,5),(4,1),(13,1)] {
        img.set_pixel((bx + x) as u32, (by + y) as u32, edge);
    }
}
