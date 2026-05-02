//! A single worker has to complete ten full dig → haul → deposit
//! cycles in a row. Pins the *durability* of the cycle: a one-shot
//! pass is easy, but stringing ten in a row catches subtle leaks
//! (claim TTL not refreshing, haul-direction state not clearing,
//! mode handoffs that work once but desync over time).
//!
//! Layout: same chamber + 3-wide shaft as `full_haul_cycle`, with a
//! tall vertical stack of soil dig sites under the chamber. Each
//! cycle the worker mines the topmost remaining tile, hauls up the
//! shaft, drops a pebble, then claims the next site (which has by
//! then become claimable as the freshly-dug tile above it provides
//! its adjacent-passable approach).
//!
//! The mower is parked on a long cooldown for the duration so it
//! doesn't kill the test subject or eat its placed pebbles.

use bevy_ecs::prelude::*;
use crate::scenarios::{Scenario, ScenarioDef};
use crate::config::*;
use crate::world::{TileGrid, TileType, DigJobs};
use crate::scenarios::TestSubject;
use crate::sim::components::WorkerBrain;
use crate::sim::Time;
use crate::sim::scenery::{MowerSchedule, MowerPhase};

/// Stack of soil dig sites under the chamber. Twelve gives the
/// worker two cycles of slack past the ten the test demands.
const STACK_DEPTH: i32 = 12;
const STACK_TOP_Y: i32 = COLONY_Y + 8;
const TARGET_CYCLES: u16 = 10;

pub fn def() -> ScenarioDef {
    ScenarioDef {
        name: "single_ant_ten_haul_cycles",
        description: "One worker completes ten full dig → haul → deposit cycles back-to-back.",
        seed: 17,
        // Single-cycle measured ~7s sim. Ten cycles ≈ 70s; budget
        // 120s so RNG-driven variance plus deepening descents don't
        // trip the test, but a 2× regression still fails fast.
        timeout_seconds: 120.0,
        setup,
        predicate,
        failure_hint: "single-worker repeated cycle is leaking state — check choose_mode / step_dig handoffs",
    }
}

fn setup(s: &mut Scenario) {
    s.clear_creatures();
    s.reveal_all();

    // Chamber + 3-wide shaft. Same shape as `full_haul_cycle` so the
    // wider shaft lets the post-deposit worker fall back through.
    let cx0 = COLONY_X - 4;
    let cx1 = COLONY_X + 4;
    let cy0 = COLONY_Y + 5;
    let cy1 = COLONY_Y + 8;
    s.fill_rect(cx0, cy0, cx1, cy1, TileType::Tunnel)
     .carve_vertical(COLONY_X - 1, COLONY_Y, cy0, 3)
     .mark_dirty();

    // Vertical stack of soil dig sites directly under the chamber.
    {
        let mut g = s.app.world.resource_mut::<TileGrid>();
        for i in 0..STACK_DEPTH {
            g.set(COLONY_X, STACK_TOP_Y + i, TileType::Soil);
        }
        g.dirty = true;
    }
    s.rebuild_flow_field();

    // Plant all stack tiles as dig jobs. claim_nearest will favour
    // the topmost remaining tile each cycle (closest to the chamber
    // floor, then to the surface drop site).
    {
        let mut jobs = s.app.world.resource_mut::<DigJobs>();
        for i in 0..STACK_DEPTH {
            jobs.push(COLONY_X, STACK_TOP_Y + i, TileType::Soil);
        }
    }

    // Park the mower on a long cooldown so it neither kills the
    // worker mid-haul nor shaves the dropped pebbles.
    s.app.world.resource_mut::<MowerSchedule>().phase =
        MowerPhase::Cooldown(9_999.0);

    // The lone test subject.
    s.spawn_worker(COLONY_X, cy1 - 1, None);

    // Test-only cleanup: clear placed pebbles every half second so a
    // single worker can keep dropping new ones in the same surface
    // band. In the live game, accumulated piles get scattered by
    // many workers, sand-physics, and the mower; here we mimic
    // that by hand so the test isn't testing pile management — just
    // the cycle itself.
    s.app.schedule.add_systems(clear_surface_pebbles);
}

fn clear_surface_pebbles(
    mut grid: ResMut<TileGrid>,
    time:     Res<Time>,
    mut accum: Local<f32>,
) {
    *accum += time.dt;
    if *accum < 0.5 { return; }
    *accum = 0.0;
    for x in 1..(grid.width - 1) {
        if (x - COLONY_X).abs() < 3 { continue; }
        for y in 1..SURFACE_ROW {
            if grid.get(x, y).solid() {
                grid.set(x, y, TileType::Air);
            }
        }
    }
    grid.dirty = true;
}

fn predicate(world: &World) -> bool {
    for e in world.iter_entities() {
        if !e.contains::<TestSubject>() { continue; }
        if let Some(b) = e.get::<WorkerBrain>() {
            if b.cycles_completed >= TARGET_CYCLES { return true; }
        }
    }
    false
}
