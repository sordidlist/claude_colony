//! End-to-end haul cycle: a worker must
//!   (a) dig a tile from below ground,
//!   (b) deposit a soil pebble above ground, and
//!   (c) end up below ground again afterwards.
//!
//! Most existing dig/haul scenarios test one of those steps in
//! isolation; this one threads them together so a regression in any
//! single phase shows up here even when the targeted scenarios still
//! pass (e.g. the fight between dig completion and DepositDebris
//! handoff in `step_dig`, or the surface-snap that lifts a hauler
//! onto its destination column).
//!
//! Setup: a small chamber a few rows under the entrance, a vertical
//! shaft connecting the two, and a hand-planted dig job on a soil
//! tile bordering the chamber. We plant several follow-up dig jobs
//! deeper in the chamber so once the worker finishes the haul cycle
//! it has a reason to descend again — that's how step (c) gets
//! satisfied without a contrived "force the worker home" hack.

use bevy_ecs::prelude::*;
use crate::scenarios::{Scenario, ScenarioDef, builder::subject_pos};
use crate::config::*;
use crate::world::{TileGrid, TileType, DigJobs};

/// Two dig jobs stacked directly under the entrance shaft. The first
/// is reachable from inside the chamber (worker spawns there). The
/// second is initially unclaimable — its adjacent-passable side is
/// the soil at site #1 — but becomes claimable as soon as #1 is dug
/// out, which gives the post-deposit worker a job to chase whose
/// approach path runs straight through the open shaft. That's how
/// step (c) of the cycle lands: the worker direct-steers SW from the
/// surface, the trajectory crosses the wider shaft, and gravity
/// drops it into the chamber.
const ORIGINAL_SOIL_SITES: &[(i32, i32)] = &[
    (COLONY_X, COLONY_Y + 8),  // dug first; one row beneath the chamber floor
    (COLONY_X, COLONY_Y + 9),  // dug second after first becomes a Tunnel
];

pub fn def() -> ScenarioDef {
    ScenarioDef {
        name: "full_haul_cycle",
        description: "Worker digs underground, drops a pebble above ground, then descends again.",
        seed: 555,
        // The full cycle fires in about 7s sim time on the chosen
        // seed; 15s leaves comfortable slack so RNG-driven variance
        // (haul direction/distance, Wander pauses) doesn't trip the
        // test, while still flagging a real regression that doubles
        // the cycle duration.
        timeout_seconds: 15.0,
        setup,
        predicate,
        failure_hint: "dig → haul → deposit → descend cycle is broken — check choose_mode handoffs",
    }
}

fn setup(s: &mut Scenario) {
    s.clear_creatures();

    // Carve a small underground chamber and a 3-tile-wide vertical
    // shaft from the surface down to its top row. The wider shaft is
    // important: workers don't pathfind from the surface back to
    // underground (`step_dig` is direct steering, not BFS), so a
    // 1-tile shaft means a hauler that drops outside the corridor
    // can't easily re-enter the colony — it just walks west across
    // grass forever, anchored. With the shaft 3 columns wide the
    // worker eventually steps off the grass shoulder, hits the gap,
    // and falls into the chamber under gravity. That's how step (c)
    // of the cycle resolves without the AI having to know how to
    // re-enter.
    let cx0 = COLONY_X - 4;
    let cx1 = COLONY_X + 4;
    let cy0 = COLONY_Y + 5;
    let cy1 = COLONY_Y + 8;
    s.fill_rect(cx0, cy0, cx1, cy1, TileType::Tunnel)
     .carve_vertical(COLONY_X - 1, COLONY_Y, cy0, 3)
     .mark_dirty();

    // Force the two dig sites to Soil deterministically (procgen
    // might land any diggable type there). Also seal off the column
    // *below* the deepest dig site for a few rows — procgen now
    // sometimes leaves a Chamber tile directly under the test sites,
    // and `adjacent_passable` prefers the south neighbour first, so
    // the worker would try to approach the dig from below (which it
    // can't physically reach) instead of from the chamber above.
    {
        let mut g = s.app.world.resource_mut::<TileGrid>();
        for (x, y) in ORIGINAL_SOIL_SITES {
            g.set(*x, *y, TileType::Soil);
        }
        let deepest_y = ORIGINAL_SOIL_SITES.iter().map(|(_, y)| *y).max().unwrap_or(0);
        for y in (deepest_y + 1)..=(deepest_y + 4) {
            g.set(COLONY_X, y, TileType::Soil);
        }
        g.dirty = true;
    }
    s.rebuild_flow_field()
     .reveal_all();

    // Plant the dig jobs explicitly. `director_update` would
    // eventually find them on its own, but it requires
    // `pop.workers >= 5` and runs every `EXPAND_INTERVAL` seconds —
    // too slow for a focused single-worker test.
    {
        let mut jobs = s.app.world.resource_mut::<DigJobs>();
        for (x, y) in ORIGINAL_SOIL_SITES {
            jobs.push(*x, *y, TileType::Soil);
        }
    }

    // The lone test subject — spawns in the chamber so it has direct
    // line of sight to the dig target.
    s.spawn_worker(COLONY_X, cy1 - 1, None);
}

fn predicate(world: &World) -> bool {
    let g = world.resource::<TileGrid>();

    // (a) At least one of the originally-Soil sites is now Tunnel.
    //     We don't insist on a specific one because `claim_nearest`
    //     order is an implementation detail.
    let any_dug = ORIGINAL_SOIL_SITES.iter()
        .any(|(x, y)| matches!(g.get(*x, *y), TileType::Tunnel));
    if !any_dug { return false; }

    // (b) Some solid tile must exist *above* SURFACE_ROW outside
    //     the entrance corridor — the placed pebble. Scan a band
    //     extending a couple of haul-distances either side of the
    //     entrance.
    let mut placed_above = false;
    'scan: for y in 1..SURFACE_ROW {
        for dx in 3..=20 {
            for sign in [-1i32, 1] {
                let x = COLONY_X + sign * dx;
                if x < 1 || x >= g.width - 1 { continue; }
                if g.get(x, y).solid() {
                    placed_above = true;
                    break 'scan;
                }
            }
        }
    }
    if !placed_above { return false; }

    // (c) Worker is below ground at the moment we declare success
    //     — proving the full a → b → c arc rather than just
    //     stopping after the deposit.
    let Some(p) = subject_pos(world, 0) else { return false; };
    (p.y as i32) > SURFACE_ROW
}
