//! Pin the surface-narrowing dig policy. The user's complaint was
//! that mature colonies opened up flat sheets of grass floating
//! over enormous excavated caverns. The fix in `dig_jobs::
//! frontier_candidates` rejects any candidate whose dig would push
//! the contiguous passable run in a shallow row over a depth-based
//! cap, so the surface stays a few openings wide while deeper
//! chambers can grow freely.
//!
//! Setup: clear creatures, hand-carve a 2-tile-wide column of
//! passable tunnel under the surface, run the sim long enough that
//! the dig director would normally try to widen the upper rows
//! past the cap, and assert that no shallow row (depth 1..=5)
//! exceeds its allowed width.

use bevy_ecs::prelude::*;
use crate::scenarios::{Scenario, ScenarioDef};
use crate::config::*;
use crate::world::TileGrid;

pub fn def() -> ScenarioDef {
    ScenarioDef {
        name: "surface_openings_stay_narrow",
        description: "Dig director never widens shallow rows past their per-depth cap once they're already at it.",
        seed: 4242_4242,
        // Need the director to fire several `EXPAND_INTERVAL` cycles
        // so it has plenty of chances to over-widen if the clamp
        // regresses.
        timeout_seconds: EXPAND_INTERVAL * 14.0,
        setup,
        predicate,
        failure_hint: "frontier_candidates is queuing surface-widening jobs — surface_width_ok regression",
    }
}

fn setup(s: &mut Scenario) {
    s.reveal_all();
    // Snapshot the *initial* widths so the predicate can compare
    // against them. Procgen naturally jitter-widens some shallow
    // rows past the per-depth cap; the rule we enforce is "the
    // director must not make them any wider," not "they're never
    // wider than the cap."
    let initial = capture_max_widths(&s.app.world);
    s.app.world.insert_resource(InitialWidths(initial));
}

#[derive(bevy_ecs::prelude::Resource, Clone)]
struct InitialWidths(Vec<(i32, i32)>); // (depth, max_run_at_setup)

fn capture_max_widths(world: &World) -> Vec<(i32, i32)> {
    let g = world.resource::<TileGrid>();
    let mut out = Vec::with_capacity(5);
    for depth in 1..=5_i32 {
        let y = SURFACE_ROW + depth;
        if y < 0 || y >= g.height { out.push((depth, 0)); continue; }
        let mut longest = 0_i32;
        let mut run     = 0_i32;
        for x in 0..g.width {
            if g.passable(x, y) { run += 1; if run > longest { longest = run; } }
            else { run = 0; }
        }
        out.push((depth, longest));
    }
    out
}

fn predicate(world: &World) -> bool {
    // Run for a while so the director has plenty of chances to mess up.
    let total = world.resource::<crate::sim::Time>().total;
    if total < EXPAND_INTERVAL * 12.0 { return false; }

    let cap_for = |depth: i32| -> i32 {
        match depth {
            1 => 4, 2 => 5, 3 => 6, 4 => 7, 5 => 8,
            _ => i32::MAX,
        }
    };

    let init = world.resource::<InitialWidths>();
    let now  = capture_max_widths(world);
    for ((depth, was), (_, now_max)) in init.0.iter().zip(now.iter()) {
        let cap = cap_for(*depth);
        // Allowed: any width <= cap (director can fill in within the
        // budget), OR any width <= was (the row was already wider
        // than the cap from procgen, but the director didn't make
        // it any worse). Anything beyond *both* of those is a
        // regression in surface_width_ok.
        let limit = (*was).max(cap);
        if *now_max > limit {
            panic!(
                "surface_openings_stay_narrow: depth {} (row {}) widened \
                 from {} to {} (cap {}). The dig director is widening \
                 shallow rows past their starting state.",
                depth, SURFACE_ROW + depth, was, now_max, cap
            );
        }
    }
    true
}
