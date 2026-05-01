//! A single soldier alone in a chamber with one spider should win the
//! 1v1. Soldiers have higher hp and damage than workers; the swarm
//! scenario shows numerical advantage, this one shows that the soldier
//! caste is actually meaningfully stronger individually.

use bevy_ecs::prelude::*;
use crate::scenarios::{Scenario, ScenarioDef, builder::subject_alive};
use crate::config::*;
use crate::world::TileType;

pub fn def() -> ScenarioDef {
    ScenarioDef {
        name: "soldier_kills_lone_spider",
        description: "One soldier kills one spider in a chamber within 25s — caste matters.",
        seed: 47,
        timeout_seconds: 25.0,
        setup,
        predicate,
        failure_hint: "soldier caste isn't strong enough to win 1v1 vs a spider — combat regression",
    }
}

fn setup(s: &mut Scenario) {
    s.clear_creatures();

    let cx = COLONY_X;
    let cy = COLONY_Y + 8;
    s.fill_rect(cx - 6, cy, cx + 6, cy + 3, TileType::Tunnel)
     .carve_vertical(COLONY_X, COLONY_Y, cy, 1)
     .mark_dirty()
     .rebuild_flow_field()
     .reveal_all();

    // Spider is subject id 0 — death is the win condition.
    s.spawn_spider_tagged(cx + 4, cy + 2, 0);
    // Soldier is subject id 1.
    s.spawn_soldier_tagged(cx - 4, cy + 2, 1);
}

fn predicate(world: &World) -> bool {
    !subject_alive(world, 0)
}
