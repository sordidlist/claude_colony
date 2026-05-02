//! Runtime-tunable balance knobs. Backing for the Shift-held debug
//! panel and the place AI/lifecycle systems read pacing values
//! from instead of `config.rs` constants. The constants in
//! `config.rs` still define the *defaults* (via `Default` below)
//! so the live game starts in the same shape it always did; the
//! resource just makes those values mutable at runtime.

use bevy_ecs::prelude::Resource;
use crate::config::*;

#[derive(Resource, Copy, Clone, Debug)]
pub struct BalanceTunables {
    /// Sim seconds between queen egg layings.
    pub queen_egg_interval: f32,
    /// Sim seconds between invader-wave spawns.
    pub invader_interval:   f32,
    /// Multiplier applied to the invader wave-composition counts so
    /// the player can dial *bigger waves* without changing cadence.
    pub invader_wave_mult:  f32,
    /// Tiles per second the lawn mower rolls at when respawned.
    pub mower_speed:        f32,
}

impl Default for BalanceTunables {
    fn default() -> Self {
        Self {
            queen_egg_interval: QUEEN_EGG_INTERVAL_S,
            invader_interval:   INVADER_SPAWN_INTERVAL_S,
            invader_wave_mult:  1.0,
            mower_speed:        MOWER_SPEED,
        }
    }
}
