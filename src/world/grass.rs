//! Per-column grass length on the surface row. Grows on a timer,
//! gets cropped by the mower as it rolls across each column.
//!
//! One byte per world column is enough — the field is a flat
//! `Vec<u8>` indexed by `x`. No per-tile state, no archetype churn.

use bevy_ecs::prelude::*;
use crate::config::*;
use crate::sim::Time;

#[derive(Resource)]
pub struct GrassField {
    /// `length[x]` is the current grass length at world column `x`,
    /// 0..=`GRASS_LENGTH_MAX`. The renderer reads this directly to
    /// paint the lawn.
    pub length: Vec<u8>,
    /// Sim-seconds-since-last-grow accumulator. Grass ticks once
    /// every `GRASS_GROW_INTERVAL_S`; in between, this just counts
    /// up.
    pub accum: f32,
}

impl GrassField {
    pub fn new(width: i32) -> Self {
        Self {
            length: vec![1; width as usize],
            accum:  0.0,
        }
    }

    /// Reset the grass at column `x` to 0 (just mowed). Out-of-range
    /// `x` is a no-op so callers can pass tile coords without a
    /// bounds check.
    #[inline] pub fn mow(&mut self, x: i32) {
        if x < 0 { return; }
        let i = x as usize;
        if i < self.length.len() {
            self.length[i] = 0;
        }
    }

    /// Read `length` at column `x`. Returns 0 outside the grid.
    #[inline] pub fn at(&self, x: i32) -> u8 {
        if x < 0 { return 0; }
        let i = x as usize;
        if i < self.length.len() { self.length[i] } else { 0 }
    }
}

/// System: tick the grass-growth timer. Every
/// `GRASS_GROW_INTERVAL_S` of sim time, every column's grass length
/// goes up by one (capped at `GRASS_LENGTH_MAX`).
pub fn grow_grass(time: Res<Time>, mut grass: ResMut<GrassField>) {
    if time.dt <= 0.0 { return; }
    grass.accum += time.dt;
    if grass.accum < GRASS_GROW_INTERVAL_S { return; }
    grass.accum = 0.0;
    for cell in grass.length.iter_mut() {
        if *cell < GRASS_LENGTH_MAX {
            *cell += 1;
        }
    }
}
