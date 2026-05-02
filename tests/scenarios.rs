//! Integration tests for all scenarios in `colony::scenarios::registry()`.
//!
//! Each `#[test]` builds the scenario via the library, runs it
//! headlessly until the predicate fires (or times out), then reports
//! Pass/fail with the elapsed sim seconds. The scenario's
//! `failure_hint` text comes through in the panic message so a CI
//! failure points at which subsystem to look at.

use colony::scenarios;

fn run(name: &str) {
    let def = scenarios::find(name)
        .unwrap_or_else(|| panic!("no scenario named '{}'", name));
    match def.run_headless() {
        Ok(t)  => println!("{}: passed in {:.2}s sim", def.name, t),
        Err(t) => panic!(
            "scenario '{}' timed out after {:.1}s — {}",
            def.name, t, def.failure_hint
        ),
    }
}

// One #[test] per registry entry. If you add a scenario, add a test
// here. The names match the scenario's `name` field.

#[test] fn escape_simple_chamber()      { run("escape_simple_chamber"); }
#[test] fn escape_winding_tunnel()      { run("escape_winding_tunnel"); }
#[test] fn hauler_drops_outside()       { run("hauler_drops_outside"); }
#[test] fn hauler_falls_off_pile()      { run("hauler_falls_off_pile"); }
#[test] fn swarm_kills_spider()         { run("swarm_kills_spider"); }
#[test] fn soldier_kills_lone_spider()  { run("soldier_kills_lone_spider"); }
#[test] fn queen_lays_egg()             { run("queen_lays_egg"); }
#[test] fn brood_hatches_to_worker()    { run("brood_hatches_to_worker"); }
#[test] fn forager_picks_up_food()      { run("forager_picks_up_food"); }
#[test] fn dirt_settles_into_slope()    { run("dirt_settles_into_slope"); }
#[test] fn rewind_doesnt_break_combat() { run("rewind_doesnt_break_combat"); }
#[test] fn mower_shaves_piles()         { run("mower_shaves_piles"); }
#[test] fn mower_retires_after_laps()   { run("mower_retires_after_laps"); }
#[test] fn mower_kills_workers()        { run("mower_kills_workers"); }
#[test] fn full_haul_cycle()              { run("full_haul_cycle"); }
#[test] fn single_ant_ten_haul_cycles()    { run("single_ant_ten_haul_cycles"); }
#[test] fn fifty_of_hundred_complete_haul(){ run("fifty_of_hundred_complete_haul"); }
#[test] fn queen_migrates_deeper()         { run("queen_migrates_deeper"); }
#[test] fn spider_kills_lone_worker()       { run("spider_kills_lone_worker"); }
#[test] fn four_workers_kill_spider()       { run("four_workers_kill_spider"); }
#[test] fn spider_triggers_alarm()          { run("spider_triggers_alarm"); }
#[test] fn worker_engages_nearby_spider()   { run("worker_engages_nearby_spider"); }
#[test] fn spider_hunts_nearby_ant()        { run("spider_hunts_nearby_ant"); }
#[test] fn hauler_unsticks_after_attempts() { run("hauler_unsticks_after_attempts"); }
#[test] fn grass_grows_over_time()             { run("grass_grows_over_time"); }
#[test] fn mower_shortens_grass()              { run("mower_shortens_grass"); }
#[test] fn worker_per_frame_hostile_reflex()   { run("worker_per_frame_hostile_reflex"); }
#[test] fn invaders_arrive_from_offscreen()    { run("invaders_arrive_from_offscreen"); }
#[test] fn invader_wave_escalation()           { run("invader_wave_escalation"); }
#[test] fn surface_openings_stay_narrow()      { run("surface_openings_stay_narrow"); }
#[test] fn surface_holes_keep_spacing()        { run("surface_holes_keep_spacing"); }
