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
    carve_spider_warrens(grid, &mut rng);
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
    // The starting colony is deliberately tiny — a shallow shaft and a
    // single small chamber at the bottom for the queen and her three
    // founding workers. Everything beyond that is dug by the workers
    // over the course of a play session, and the queen migrates
    // deeper as new chambers open up. See `queen::queen_migration`.
    let cx = COLONY_X;
    let cy = COLONY_Y;
    let shaft_depth = rng.gen_range(6..10);
    carve_tunnel(grid, cx, cy, cx, cy + shaft_depth, rng);

    // One small founding chamber at the shaft's base.
    let ccx = cx;
    let ccy = (cy + shaft_depth).clamp(SURFACE_ROW + 2, grid.height - 3);
    carve_chamber(grid, ccx, ccy, 4, 2);
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

/// Carve a handful of isolated spider warren pockets at mid-depth.
/// These are the homes the predators spawn into — without them the
/// new shallow colony procgen would leave the deep underground
/// completely sealed and `spawn_spiders` would find no passable
/// candidate tiles at all (resulting in zero spiders for the entire
/// run). The warrens are deliberately disconnected from the colony
/// shaft: workers have to dig their way over to them, which is
/// what turns "there are spiders somewhere underground" into
/// "spiders are now a threat to the nest."
fn carve_spider_warrens(grid: &mut TileGrid, rng: &mut StdRng) {
    let count = 5;
    let mut placed: Vec<(i32, i32)> = Vec::new();
    let mut tries = 0;
    while placed.len() < count && tries < 60 {
        tries += 1;
        // Mid-depth band: deep enough that workers have to mean it,
        // shallow enough that they reach it within a play session.
        let cy = rng.gen_range(SURFACE_ROW + 14..SURFACE_ROW + 40);
        let cx = rng.gen_range(20..grid.width - 20);
        // Don't drop a warren right under the entrance — keep it at
        // least ~40 tiles horizontal offset so it's a navigation goal,
        // not an immediate breach point. (Also keeps warrens clear
        // of the wide chambers used by population-scale test scenarios,
        // which set up their own ~30-tile-wide chambers around
        // COLONY_X.)
        if (cx - COLONY_X).abs() < 40 { continue; }
        // Spread warrens out so they don't overlap.
        if placed.iter().any(|(px, py)| (px - cx).abs() < 24 && (py - cy).abs() < 8) {
            continue;
        }
        placed.push((cx, cy));
        let rw = rng.gen_range(3..=5);
        let rh = rng.gen_range(2..=3);
        carve_chamber(grid, cx, cy, rw, rh);
        // A short stub-tunnel sideways from the warren so a wandering
        // spider has somewhere to go before bumping into solid dirt.
        let stub_dx = if rng.gen::<bool>() { rw + 3 } else { -rw - 3 };
        carve_tunnel(grid, cx, cy, cx + stub_dx, cy, rng);
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
