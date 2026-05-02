//! Scaled-up haul cycle: 100 workers in a chamber, half of them must
//! each complete at least one full dig → haul → deposit cycle. Pins
//! the system's behaviour at population scale — a single-worker test
//! catches per-agent bugs, but a population test catches contention
//! issues (everyone fighting for the same dig job, the entrance
//! shaft becoming a traffic jam, claim-TTL recycling under load).
//!
//! Setup: a much larger chamber than the single-ant tests so 100
//! workers physically fit, with a wide shaft and a forest of
//! pre-planted dig jobs around the chamber walls. The natural
//! `director_update` loop also fires (pop ≥ 5) and keeps the queue
//! fed.
//!
//! Mower disabled for the test so its kill radius doesn't artificially
//! shrink the population.

use bevy_ecs::prelude::*;
use crate::scenarios::{Scenario, ScenarioDef};
use crate::config::*;
use crate::world::{TileGrid, TileType, DigJobs};
use crate::scenarios::TestSubject;
use crate::sim::components::{WorkerBrain, Ant, AntKind};
use crate::sim::Time;
use crate::sim::scenery::{MowerSchedule, MowerPhase};

const POPULATION:        u32 = 100;
const REQUIRED_FRACTION: u32 = 50;   // out of 100 — i.e. 50%

pub fn def() -> ScenarioDef {
    ScenarioDef {
        name: "fifty_of_hundred_complete_haul",
        description: "Of 100 workers, at least half each finish one full haul cycle within 90s.",
        seed: 4242,
        // Single-worker cycle ~7s; with 100 workers competing for
        // jobs and shaft bandwidth, give them 90s to all push
        // through. Plenty of slack — a passing run typically settles
        // well under the budget — but not so much that a 2×
        // regression slips by.
        timeout_seconds: 90.0,
        setup,
        predicate,
        failure_hint: "population-scale haul throughput collapsed — bottleneck or contention regression",
    }
}

fn setup(s: &mut Scenario) {
    s.clear_creatures();
    s.reveal_all();

    // Generous chamber: 30 cols wide × 8 rows tall so 100 workers
    // can spawn without colliding with the chamber walls. Wide
    // shaft (5 cols) so the bigger crowd can pass each other in
    // both directions without permanently jamming.
    let cx0 = COLONY_X - 15;
    let cx1 = COLONY_X + 15;
    let cy0 = COLONY_Y + 4;
    let cy1 = COLONY_Y + 11;
    s.fill_rect(cx0, cy0, cx1, cy1, TileType::Tunnel)
     .carve_vertical(COLONY_X - 2, COLONY_Y, cy0, 5)
     .mark_dirty();

    // Plant a ring of soil dig sites along the chamber's outer
    // walls. With 100 workers, a hundred-or-so jobs avoids
    // claim-thrash; the director will continue to top this up as
    // they get consumed.
    let mut soil_sites: Vec<(i32, i32)> = Vec::new();
    {
        let mut g = s.app.world.resource_mut::<TileGrid>();
        // Bottom row of dig sites
        for x in cx0..=cx1 {
            let y = cy1 + 1;
            g.set(x, y, TileType::Soil);
            soil_sites.push((x, y));
        }
        // Side walls — left and right vertical strips a few rows tall
        for y in cy0..=cy1 {
            for x in [cx0 - 1, cx1 + 1] {
                g.set(x, y, TileType::Soil);
                soil_sites.push((x, y));
            }
        }
        g.dirty = true;
    }
    s.rebuild_flow_field();

    {
        let mut jobs = s.app.world.resource_mut::<DigJobs>();
        for (x, y) in &soil_sites {
            jobs.push(*x, *y, TileType::Soil);
        }
    }

    // Park the mower for the duration of the test.
    s.app.world.resource_mut::<MowerSchedule>().phase =
        MowerPhase::Cooldown(9_999.0);

    // Spawn 100 test-subject workers spread across the chamber
    // floor. Tagged with rolling ids so the predicate can iterate
    // them without ambiguity. Each gets a slightly staggered
    // replan_in so they don't all decide at the same frame and
    // cause a synchronised stampede.
    let span_x   = (cx1 - cx0 - 1) as u32;          // available cols
    let rows: u32 = (cy1 - cy0).max(1) as u32;
    for i in 0..POPULATION {
        let dx = (i % span_x) as i32;
        let dy = ((i / span_x) % rows) as i32;
        let x  = cx0 + 1 + dx;
        let y  = cy0 + 1 + dy;
        // Use a u8-wrapping id; predicate doesn't actually need to
        // distinguish individuals, only count workers with
        // cycles_completed > 0.
        let id = (i & 0xFF) as u8;
        let e = s.spawn_worker_tagged(x, y, None, id);
        if let Some(mut b) = s.app.world.get_mut::<WorkerBrain>(e) {
            b.replan_in = (i as f32 % 17.0) * 0.05;
        }
    }

    // Same surface cleanup as the single-ant test — at population
    // scale, pebbles still pile up faster than physics can disperse
    // them, especially with the mower disabled. We're testing the
    // cycle, not pile-management.
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
    let mut completed: u32 = 0;
    for e in world.iter_entities() {
        if !e.contains::<TestSubject>() { continue; }
        let Some(ant) = e.get::<Ant>() else { continue; };
        if !matches!(ant.kind, AntKind::Worker) { continue; }
        let Some(b) = e.get::<WorkerBrain>() else { continue; };
        if b.cycles_completed >= 1 {
            completed += 1;
            if completed >= REQUIRED_FRACTION { return true; }
        }
    }
    false
}
