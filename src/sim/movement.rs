//! Velocity → position integration with tile collision + (gentle) gravity.
//!
//! Ants are agile climbers. They cling whenever there's terrain anywhere in
//! their 3×3 neighbourhood, and even when there isn't they get a brief
//! "coyote time" allowance if a floor is two tiles below — the kind of leap
//! a real ant scoots across without falling. Gravity itself is also tuned
//! down so the rare actual fall is a gentle settle, not a plunge.

use bevy_ecs::prelude::*;
use crate::world::TileGrid;
use super::components::{Position, Velocity};

const GRAVITY:        f32 = 12.0;  // tiles/s² — softer than before
const TERMINAL_FALL:  f32 = 8.0;   // tiles/s

pub fn integrate_movement(
    grid: Res<TileGrid>,
    time: Res<super::Time>,
    mut q: Query<(&mut Position, &mut Velocity)>,
) {
    let dt = time.dt;
    for (mut p, mut v) in q.iter_mut() {
        let tx = p.0.x as i32;
        let ty = p.0.y as i32;
        // Only gate gravity. Don't zero an existing y velocity here — that
        // would also kill AI-driven downward motion (e.g. an ant deliberately
        // descending a vertical shaft). The slide-collision logic below
        // handles cleanup when an ant actually hits something.
        if !surface_anchor(&grid, tx, ty) {
            v.0.y += GRAVITY * dt;
            if v.0.y > TERMINAL_FALL { v.0.y = TERMINAL_FALL; }
        }

        let nx = p.0.x + v.0.x * dt;
        let ny = p.0.y + v.0.y * dt;
        // Axis-aligned slide with one-tile auto-climb. The auto-climb is
        // the cheap "ants are tiny climbing creatures" trick: when a step
        // is blocked horizontally and the tile up-and-forward is passable,
        // hop up. It handles dirt mounds, lips between tunnels, the edge
        // of a chamber — nearly every place a 1-tile vertical step matters
        // — without any AI special-casing. Once entities can climb 1-tile
        // bumps natively, the dump/forage/wander code stays simple.
        if grid.passable(nx as i32, ny as i32) {
            p.0.x = nx; p.0.y = ny;
        } else if grid.passable(nx as i32, p.0.y as i32) {
            p.0.x = nx;
            v.0.y = 0.0;
        } else if grid.passable(p.0.x as i32, ny as i32) {
            p.0.y = ny;
            v.0.x = 0.0;
        } else {
            // Two-step auto-climb: only valid if the *path* (up, then over)
            // is clear. Without checking the tile directly above the
            // entity, the climb would pop entities straight through any
            // 1-tile wall whose far side happens to be air above. That's
            // how spiders ended up wedged inside the colony entrance and
            // ants ended up sealed inside solid rock pockets.
            let up_x   = nx as i32;
            let up_y   = (p.0.y - 1.0) as i32;
            let stay_x = p.0.x as i32;
            if up_y >= 0
                && grid.passable(stay_x, up_y)
                && grid.passable(up_x,   up_y)
            {
                p.0.x = nx;
                p.0.y = up_y as f32 + 0.5;
                v.0.y = 0.0;
            } else {
                v.0.x = 0.0; v.0.y = 0.0;
            }
        }

        // Soft world clamp
        if p.0.x < 0.5 { p.0.x = 0.5; }
        if p.0.y < 0.5 { p.0.y = 0.5; }
        if p.0.x > grid.width  as f32 - 1.5 { p.0.x = grid.width  as f32 - 1.5; }
        if p.0.y > grid.height as f32 - 1.5 { p.0.y = grid.height as f32 - 1.5; }
    }
}

/// "Is this ant clinging to terrain?" — generous so ants only fall when
/// genuinely in open air.
///
///   • Any solid tile in the 3×3 neighbourhood (8 directions) counts as a
///     grip surface, including diagonals.
///   • A solid tile up to two rows directly below counts as a coyote-time
///     allowance: ants can hop over a one-tile gap or scoot across the lip
///     of a tunnel without instantly plunging.
#[inline]
fn surface_anchor(grid: &TileGrid, tx: i32, ty: i32) -> bool {
    for dy in -1..=1 {
        for dx in -1..=1 {
            if dx == 0 && dy == 0 { continue; }
            if grid.get(tx + dx, ty + dy).solid() { return true; }
        }
    }
    grid.get(tx, ty + 2).solid()
}
