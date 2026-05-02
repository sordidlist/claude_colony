//! Speed-invariance tests.
//!
//! The live game can run at five fast-forward levels: 1×, 2×, 4×, 10×,
//! and 100×. Internally each is implemented as a multi-pass step
//! model — `actual_passes = min(ff, 10)` calls of `App::step` per
//! wall-clock frame, with `dt` scaled up when the requested speed
//! exceeds the 10-pass cap. That stretched-`dt` regime is where
//! speed-related divergence tends to creep in (sub-step physics
//! integration, RNG seeding from time, claim TTLs, deferred
//! commands, etc.).
//!
//! The headless runs in `tests/scenarios.rs` exercise the sim at a
//! constant `dt = 1/60`; they don't catch behaviour that only
//! breaks at higher speed levels — which is exactly the class of
//! bug the user has been hitting in visual mode (a scenario passes
//! headless but breaks the moment it's watched at 100×).
//!
//! These tests run a handful of representative scenarios at every
//! FF level. The predicate must succeed within a generous sim-time
//! budget regardless of the speed level — failure means the
//! multi-pass model is changing the outcome.
//!
//! Wall-clock cost is small; each scenario runs in milliseconds in
//! release mode even at the largest budget.

use colony::scenarios;
use colony::config::FF_LEVELS;

fn run_scenario_at_ff(name: &str, ff: u32, max_sim_seconds: f32) {
    let def = scenarios::find(name)
        .unwrap_or_else(|| panic!("no scenario named '{}'", name));
    let mut s = def.build();
    match s.run_until_at_ff_level(max_sim_seconds, ff, def.predicate) {
        Ok(t)  => println!("{} @ {}× : passed at sim {:.2}s", name, ff, t),
        Err(t) => panic!(
            "scenario '{}' failed at {}× speed (sim elapsed {:.1}s) — {}",
            name, ff, t, def.failure_hint),
    }
}

// ── full_haul_cycle: exercises AI mode handoffs, dig progress,
//    sand-physics, hauler movement, and gravity. A good canary for
//    speed-related drift — the cycle's success depends on every
//    sub-system staying behaviourally identical at any FF level.
//
//    Sim budget: 30s (the headless run finishes in ~7s; doubling
//    that gives slack for RNG variance under stretched dt).
mod full_haul_cycle {
    use super::*;
    const NAME: &str = "full_haul_cycle";
    const MAX_SIM: f32 = 30.0;

    #[test] fn at_1x()   { run_scenario_at_ff(NAME, 1,   MAX_SIM); }
    #[test] fn at_2x()   { run_scenario_at_ff(NAME, 2,   MAX_SIM); }
    #[test] fn at_4x()   { run_scenario_at_ff(NAME, 4,   MAX_SIM); }
    #[test] fn at_10x()  { run_scenario_at_ff(NAME, 10,  MAX_SIM); }
    #[test] fn at_100x() { run_scenario_at_ff(NAME, 100, MAX_SIM); }
    #[test] fn at_500x() { run_scenario_at_ff(NAME, 500, MAX_SIM); }
}

// ── swarm_kills_spider: combat-side stress test. Multiple
//    attackers, snapshot+apply pattern, alarm pheromone, and the
//    deferred-despawn sequence after a kill — all things that have
//    historically broken at higher speeds (the sub-step movement
//    fix and the rewind-zombie fix both surfaced as "spiders
//    accumulate at high FF").
mod swarm_kills_spider {
    use super::*;
    const NAME: &str = "swarm_kills_spider";
    const MAX_SIM: f32 = 30.0;

    #[test] fn at_1x()   { run_scenario_at_ff(NAME, 1,   MAX_SIM); }
    #[test] fn at_2x()   { run_scenario_at_ff(NAME, 2,   MAX_SIM); }
    #[test] fn at_4x()   { run_scenario_at_ff(NAME, 4,   MAX_SIM); }
    #[test] fn at_10x()  { run_scenario_at_ff(NAME, 10,  MAX_SIM); }
    #[test] fn at_100x() { run_scenario_at_ff(NAME, 100, MAX_SIM); }
    #[test] fn at_500x() { run_scenario_at_ff(NAME, 500, MAX_SIM); }
}

// ── escape_winding_tunnel: pure-movement test through a non-trivial
//    geometry. The flow-field navigation + auto-climb has to keep
//    behaving the same when each physics sub-step's dt grows.
mod escape_winding_tunnel {
    use super::*;
    const NAME: &str = "escape_winding_tunnel";
    const MAX_SIM: f32 = 30.0;

    #[test] fn at_1x()   { run_scenario_at_ff(NAME, 1,   MAX_SIM); }
    #[test] fn at_2x()   { run_scenario_at_ff(NAME, 2,   MAX_SIM); }
    #[test] fn at_4x()   { run_scenario_at_ff(NAME, 4,   MAX_SIM); }
    #[test] fn at_10x()  { run_scenario_at_ff(NAME, 10,  MAX_SIM); }
    #[test] fn at_100x() { run_scenario_at_ff(NAME, 100, MAX_SIM); }
    #[test] fn at_500x() { run_scenario_at_ff(NAME, 500, MAX_SIM); }
}

// ── queen_lays_egg: pure timer-driven behaviour. If multi-pass
//    stepping miscounts wall-clock-vs-sim-time relationships (e.g.
//    egg_timer ticking at the wrong rate), this is where it shows.
mod queen_lays_egg {
    use super::*;
    const NAME: &str = "queen_lays_egg";
    const MAX_SIM: f32 = 20.0;

    #[test] fn at_1x()   { run_scenario_at_ff(NAME, 1,   MAX_SIM); }
    #[test] fn at_2x()   { run_scenario_at_ff(NAME, 2,   MAX_SIM); }
    #[test] fn at_4x()   { run_scenario_at_ff(NAME, 4,   MAX_SIM); }
    #[test] fn at_10x()  { run_scenario_at_ff(NAME, 10,  MAX_SIM); }
    #[test] fn at_100x() { run_scenario_at_ff(NAME, 100, MAX_SIM); }
    #[test] fn at_500x() { run_scenario_at_ff(NAME, 500, MAX_SIM); }
}

// ── coverage check: assert this file actually covers every level
//    advertised in `FF_LEVELS`. If `config.rs` adds another speed
//    later, this test reminds us to add modules above for it.
#[test]
fn covers_every_ff_level() {
    use std::collections::HashSet;
    let configured: HashSet<u32> = FF_LEVELS.iter().copied().collect();
    let tested:     HashSet<u32> = [1, 2, 4, 10, 100, 500].into_iter().collect();
    assert_eq!(
        configured, tested,
        "FF_LEVELS in config.rs has changed; update the speed-invariance \
         tests in tests/speed_invariance.rs to cover every advertised level."
    );
}
