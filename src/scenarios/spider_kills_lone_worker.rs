//! Combat balance pin: a single spider should beat a single worker.
//!
//! With the buffed spider profile (HP 35, dmg 4.5, cooldown 1.0s)
//! against a worker (HP 14, dmg 2.2, cooldown 0.7s), 1v1 the worker
//! dies in roughly 3 s of combat while the spider still has most of
//! its HP. The fight is asymmetric on purpose — that's what makes a
//! spider a real threat the colony has to *swarm* rather than send
//! a lone worker against.

use bevy_ecs::prelude::*;
use crate::scenarios::{Scenario, ScenarioDef, builder::subject_alive};
use crate::config::*;
use crate::world::TileType;

pub fn def() -> ScenarioDef {
    ScenarioDef {
        name: "spider_kills_lone_worker",
        description: "1v1: a single spider kills a single worker — combat is intentionally asymmetric.",
        seed: 88,
        timeout_seconds: 12.0,
        setup,
        predicate,
        failure_hint: "spider too weak to win 1v1 — combat balance regression",
    }
}

fn setup(s: &mut Scenario) {
    s.clear_creatures();

    let cx = COLONY_X;
    let cy = COLONY_Y + 6;
    s.fill_rect(cx - 4, cy, cx + 4, cy + 2, TileType::Tunnel)
     .carve_vertical(COLONY_X, COLONY_Y, cy, 1)
     .mark_dirty()
     .rebuild_flow_field()
     .reveal_all();

    // Worker is the test subject — predicate watches for its death.
    s.spawn_worker_tagged(cx - 1, cy + 1, None, 0);
    // Spider stands one tile away so combat lands fast.
    s.spawn_spider_tagged(cx + 1, cy + 1, 1);
}

fn predicate(world: &World) -> bool {
    // Pass when the lone worker (subject id 0) is dead AND the
    // spider (id 1) is still alive.
    !subject_alive(world, 0) && subject_alive(world, 1)
}
