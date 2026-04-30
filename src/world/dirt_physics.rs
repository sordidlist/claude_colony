//! Above-ground sand physics. Loose tiles (anything solid sitting above
//! `SURFACE_ROW`) settle each tick: they fall straight down into air, and
//! they slide diagonally off pile shoulders. Underground tiles are never
//! touched, so tunnels don't collapse.
//!
//! With this in place the ant AI can be deliberately *unstrategic* — drop a
//! tile anywhere outside the colony and let physics organise it into a
//! mound. The classic falling-sand model produces the cone shape for free.

use bevy_ecs::prelude::*;
use crate::config::SURFACE_ROW;
use super::tiles::{TileGrid, TileType};
use crate::sim::Time;

/// Settling cadence. 10 Hz looks like dirt actually slumping; faster
/// makes the mound twitch, slower makes it look frozen.
const SETTLE_INTERVAL: f32 = 0.10;

pub fn settle_above_ground(
    mut grid: ResMut<TileGrid>,
    time: Res<Time>,
    mut accum: Local<f32>,
    mut tick: Local<u32>,
) {
    *accum += time.dt;
    if *accum < SETTLE_INTERVAL { return; }
    *accum -= SETTLE_INTERVAL;
    *tick = tick.wrapping_add(1);

    // Alternate the slide-direction preference each tick so the pile
    // doesn't drift left/right over time.
    let dirs: [i32; 2] = if *tick & 1 == 0 { [-1, 1] } else { [1, -1] };

    // Iterate bottom-up so cascades resolve in a single tick: when row N
    // settles, row N-1 sees the freshly-vacated tile and can fall too.
    for y in (1..SURFACE_ROW).rev() {
        for x in 1..(grid.width - 1) {
            let t = grid.get(x, y);
            if !t.solid() { continue; }

            // (a) Fall straight down if below is empty.
            if grid.get(x, y + 1) == TileType::Air {
                grid.set(x, y, TileType::Air);
                grid.set(x, y + 1, t);
                continue;
            }

            // (b) Slide diagonally if both the side tile *and* the diagonal-
            // below tile are air. The two-tile clearance prevents grains
            // from tunnelling through neighbouring piles.
            for dx in dirs {
                let nx = x + dx;
                if grid.get(nx, y + 1) == TileType::Air
                    && grid.get(nx, y) == TileType::Air
                {
                    grid.set(x, y, TileType::Air);
                    grid.set(nx, y + 1, t);
                    break;
                }
            }
        }
    }
}
