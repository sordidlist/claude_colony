//! Above-ground sand physics. Loose tiles (anything solid sitting above
//! `SURFACE_ROW`) settle each tick: they fall straight down into air, and
//! they slide diagonally off pile shoulders. Underground tiles are never
//! touched, so tunnels don't collapse.
//!
//! With this in place the ant AI can be deliberately *unstrategic* — drop a
//! tile anywhere outside the colony and let physics organise it into a
//! mound. The classic falling-sand model produces the cone shape for free.

use bevy_ecs::prelude::*;
use crate::config::{SURFACE_ROW, COLONY_X};
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

    // Reserve a 2-tile corridor on each side of the entrance shaft as a
    // strict no-fly zone: real ants keep their entrance clear, and
    // computationally we can't let cascading slides fill the column
    // above the shaft (which would seal the colony in).
    const ENTRANCE_KEEP: i32 = 2;
    let entrance_dx = |x: i32| (x - COLONY_X).abs();

    // Iterate bottom-up so cascades resolve in a single tick: when row N
    // settles, row N-1 sees the freshly-vacated tile and can fall too.
    for y in (1..SURFACE_ROW).rev() {
        for x in 1..(grid.width - 1) {
            let t = grid.get(x, y);
            if !t.solid() { continue; }

            // Tiles already inside the keep-clear zone get pushed
            // outward — soil that slid in here previously, or a stray
            // drop, gets bumped back out by one column per tick.
            if entrance_dx(x) <= ENTRANCE_KEEP {
                let outward = if x < COLONY_X { -1 } else { 1 };
                let nx = x + outward;
                if entrance_dx(nx) > ENTRANCE_KEEP
                    && grid.get(nx, y + 1) == TileType::Air
                    && grid.get(nx, y)     == TileType::Air
                {
                    grid.set(x,  y,     TileType::Air);
                    grid.set(nx, y + 1, t);
                }
                continue;
            }

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
                // Refuse to slide INTO the entrance keep-clear corridor.
                if entrance_dx(nx) <= ENTRANCE_KEEP { continue; }
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
