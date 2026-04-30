//! Deterministic worldgen. Seeded `StdRng` so the same seed yields the same
//! map. Operates entirely on the flat `TileGrid` — no entity awareness.

use rand::{SeedableRng, Rng, rngs::StdRng};
use crate::config::*;
use super::tiles::{TileGrid, TileType, dirt_for_depth};

pub fn generate(grid: &mut TileGrid, seed: u64) {
    let mut rng = StdRng::seed_from_u64(seed);
    fill_base(grid);
    scatter_rocks(grid, &mut rng);
    scatter_sand(grid, &mut rng);
    carve_colony(grid, &mut rng);
    assign_variants(grid, &mut rng);
    grid.dirty = true;
}

fn fill_base(grid: &mut TileGrid) {
    let sr = SURFACE_ROW;
    for y in 0..grid.height {
        for x in 0..grid.width {
            let t = if y < sr        { TileType::Air }
                    else if y == sr  { TileType::Grass }
                    else             { dirt_for_depth(y, sr) };
            let i = grid.idx(x, y);
            grid.tiles[i] = t as u8;
        }
    }
}

fn scatter_rocks(grid: &mut TileGrid, rng: &mut StdRng) {
    let sr = SURFACE_ROW;
    let bands = [
        (sr +  8, sr + 30,  40, 2,  6),
        (sr + 28, sr + 70,  70, 3,  9),
        (sr + 60, grid.height - 1, 100, 4, 13),
    ];
    for (y_min, y_max, count, smin, smax) in bands {
        let y_min = y_min.max(sr + 1);
        let y_max = y_max.min(grid.height - 2).max(y_min + 1);
        for _ in 0..count {
            let cx = rng.gen_range(1..grid.width  - 1);
            let cy = rng.gen_range(y_min..y_max);
            let r  = rng.gen_range(smin..smax);
            blob(grid, cx, cy, r, TileType::Rock, rng);
        }
    }
    // Deep stratum
    let band = sr + ((grid.height - sr) as f32 * 0.85) as i32;
    for x in 0..grid.width {
        for dy in -2..=2 {
            let ny = band + dy + rng.gen_range(-2..3);
            if ny > sr && ny < grid.height && rng.gen::<f32>() < 0.16 {
                let i = grid.idx(x, ny);
                grid.tiles[i] = TileType::Rock as u8;
            }
        }
    }
}

fn scatter_sand(grid: &mut TileGrid, rng: &mut StdRng) {
    for _ in 0..18 {
        let cx = rng.gen_range(5..grid.width - 5);
        let cy = rng.gen_range(SURFACE_ROW + 2..SURFACE_ROW + 22);
        let r  = rng.gen_range(2..7);
        for dy in -r..=r {
            for dx in -r..=r {
                if dx*dx + dy*dy <= r*r {
                    let nx = cx + dx; let ny = cy + dy;
                    if grid.in_bounds(nx, ny) {
                        let i = grid.idx(nx, ny);
                        let t = TileType::from_u8(grid.tiles[i]);
                        if !matches!(t, TileType::Air | TileType::Grass | TileType::Rock) {
                            grid.tiles[i] = TileType::Sand as u8;
                        }
                    }
                }
            }
        }
    }
}

fn blob(grid: &mut TileGrid, cx: i32, cy: i32, r: i32, t: TileType, rng: &mut StdRng) {
    for dy in -r..=r {
        for dx in -r..=r {
            let d2 = dx*dx + dy*dy;
            if d2 <= r*r {
                let nx = cx + dx; let ny = cy + dy;
                if grid.in_bounds(nx, ny) && rng.gen::<f32>() < 0.93 {
                    let i = grid.idx(nx, ny);
                    if grid.tiles[i] != TileType::Air as u8 {
                        grid.tiles[i] = t as u8;
                    }
                }
            }
        }
    }
}

fn carve_colony(grid: &mut TileGrid, rng: &mut StdRng) {
    let cx = COLONY_X;
    let cy = COLONY_Y;
    let shaft_depth = rng.gen_range(20..32);
    carve_tunnel(grid, cx, cy, cx, cy + shaft_depth, rng);

    let chamber_specs = [
        (cx + rng.gen_range(-10..10), cy + shaft_depth,      6, 3),
        (cx + rng.gen_range(-18..-6), cy + shaft_depth - 7,  5, 3),
        (cx + rng.gen_range(6..18),   cy + shaft_depth - 5,  5, 3),
    ];
    for (ccx, ccy, rw, rh) in chamber_specs {
        let ccx = ccx.clamp(2, grid.width - 3);
        let ccy = ccy.clamp(SURFACE_ROW + 2, grid.height - 3);
        carve_chamber(grid, ccx, ccy, rw, rh);
        carve_tunnel(grid, cx, cy + shaft_depth, ccx, ccy, rng);
    }

    for _ in 0..10 {
        let sx = (cx + rng.gen_range(-30..30)).clamp(2, grid.width - 3);
        let sy = (cy + rng.gen_range(6..shaft_depth)).clamp(SURFACE_ROW + 2, grid.height - 3);
        let ex = (sx + rng.gen_range(-20..20)).clamp(2, grid.width - 3);
        let ey = (sy + rng.gen_range(0..10)).clamp(SURFACE_ROW + 2, grid.height - 3);
        carve_tunnel(grid, sx, sy, ex, ey, rng);
    }
}

fn carve_tunnel(grid: &mut TileGrid, x0: i32, y0: i32, x1: i32, y1: i32, rng: &mut StdRng) {
    let dx = x1 - x0; let dy = y1 - y0;
    let dist = ((dx*dx + dy*dy) as f32).sqrt();
    if dist == 0.0 { return; }
    let steps = (dist * 2.0) as i32;
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let nx = (x0 as f32 + dx as f32 * t + rng.gen_range(-1.0..1.0)) as i32;
        let ny = (y0 as f32 + dy as f32 * t + rng.gen_range(-0.5..0.5)) as i32;
        for ox in -1i32..=1 {
            for oy in -1i32..=1 {
                if ox.abs() + oy.abs() <= 1 {
                    let tx = nx + ox; let ty = ny + oy;
                    if grid.in_bounds(tx, ty) && ty >= SURFACE_ROW {
                        let i = grid.idx(tx, ty);
                        if grid.tiles[i] != TileType::Rock as u8 {
                            grid.tiles[i] = TileType::Tunnel as u8;
                        }
                    }
                }
            }
        }
    }
}

fn carve_chamber(grid: &mut TileGrid, cx: i32, cy: i32, rw: i32, rh: i32) {
    for dy in -rh..=rh {
        for dx in -rw..=rw {
            let nx = cx + dx; let ny = cy + dy;
            let inside = (dx*dx) as f32 / (rw*rw) as f32
                       + (dy*dy) as f32 / (rh*rh) as f32 <= 1.0;
            if inside && grid.in_bounds(nx, ny) && ny > SURFACE_ROW {
                let i = grid.idx(nx, ny);
                if grid.tiles[i] != TileType::Rock as u8 {
                    grid.tiles[i] = TileType::Chamber as u8;
                }
            }
        }
    }
}

fn assign_variants(grid: &mut TileGrid, rng: &mut StdRng) {
    for v in grid.variants.iter_mut() {
        *v = rng.gen_range(0..4);
    }
}
