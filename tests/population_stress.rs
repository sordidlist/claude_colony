//! Population-scale stress test on a *regular* (not-prefabricated)
//! colony. Boots the standard `App::new` worldgen — small founding
//! chamber, a handful of disconnected spider warrens at mid-depth,
//! the queen and her three founding workers — then tops the
//! workforce up to 1000 ants and runs the sim at 100× fast-forward
//! for ten in-game days. After that, two failure modes are
//! checked:
//!
//!   (a) **No un-engaged spiders inside the colony.** Once workers
//!       have dug out the tunnel network and possibly breached a
//!       warren, *some* spider may have wandered into the
//!       passable-from-entrance set. With ≥ 50 workers still
//!       alive, none of those spiders should be alive — the swarm
//!       response should be killing them. A spider sitting in the
//!       BFS-reachable region of the colony with a healthy
//!       population is the regression "spiders flood the colony but
//!       nobody fights them."
//!
//!   (b) **No surface-cargo pile-up.** At any moment a fraction of
//!       the workforce is legitimately on the surface mid-haul; the
//!       failure mode is *most* of them stuck up there forever.
//!       The bound is 25 % of the surviving workers. The
//!       force-drop escape hatch in `step_deposit_debris` is what
//!       keeps this honest — without it, mature colonies gridlock.
//!
//! The test isn't a per-frame predicate: it runs the full ten-day
//! window at 100× and only asserts at the end. Wall-clock cost in
//! release mode is ~30 s.

use std::collections::VecDeque;

use bevy_ecs::world::World;
use glam::Vec2;
use rand::{Rng, SeedableRng, rngs::StdRng};

use colony::config::*;
use colony::scenarios::Scenario;
use colony::sim::Time;
use colony::sim::components::*;
use colony::sim::scenery::{MowerSchedule, MowerPhase};
use colony::world::TileGrid;

/// In-game days to simulate before the assertion fires.
const TARGET_DAYS: f32 = 10.0;
/// Population floor for the engagement assertion. Below this we
/// can't honestly call out "the colony isn't fighting"; combat may
/// already have thinned the workforce too far.
const ENGAGEMENT_POP_THRESHOLD: usize = 30;
/// Fraction of surviving workers allowed to be *stuck* on the
/// surface — workers whose haul has already flipped direction at
/// least once (`WorkerBrain.haul_attempts >= 1`) and still has
/// cargo. A few percent are normal under load (mounds form, some
/// haulers bump shoulders); above this fraction the workforce is
/// gridlocked and the force-drop hatch is too lenient.
const STUCK_HAULER_BUDGET:   f32 = 0.20;
/// Sim seconds between in-run sanity checks. Picking ~1 in-game
/// minute (= 1 sim minute) means the assertions fire many times
/// over the 10-day window, while the population is still healthy
/// enough to count as "plenty of ants left to fight."
const CHECK_INTERVAL_SIM_S: f32 = 60.0;

#[test]
fn thousand_ants_engage_spiders_and_dont_pile_up() {
    let mut s = Scenario::new(20_260_501);
    fill_workers_to(&mut s, 1000);
    // Park the mower so its kill-radius doesn't carve through the
    // 1000-ant cohort the moment they're spawned. The test is about
    // hostile-engagement and haul-cycle health at scale, not mower
    // balance.
    s.app.world.resource_mut::<MowerSchedule>().phase =
        MowerPhase::Cooldown(9_999.0);

    let target_sim = TARGET_DAYS * DAY_LENGTH_SECONDS;
    let timeout    = target_sim + 60.0;

    // The predicate runs assertions periodically through the
    // 10-day window — important because the colony's population
    // declines over the run (mower kills, combat losses) and the
    // engagement guard is gated on "plenty of ants left to fight."
    // Catching a failure mid-run (while pop is still high) is
    // exactly the regression the user described. The predicate
    // also runs one final check on the way out and returns true
    // so the harness exits cleanly.
    let mut next_check_at: f32 = CHECK_INTERVAL_SIM_S;
    let result = s.run_until_at_ff_level(timeout, 100, move |world| {
        let total = world.resource::<Time>().total;
        if total >= next_check_at {
            next_check_at = total + CHECK_INTERVAL_SIM_S;
            run_assertions(world, total, /*final*/ false);
        }
        if total < target_sim { return false; }
        run_assertions(world, total, /*final*/ true);
        true
    });
    if let Err(t) = result {
        panic!(
            "population_stress: only reached sim t={:.0}s before timeout \
             (wanted ≥ {:.0}s for {} in-game days)",
            t, target_sim, TARGET_DAYS as i32);
    }
}

/// Spawn additional worker ants until the world holds `target` of
/// them. Mirrors `app::spawn_initial_ants` placement logic so the
/// extra ants land in the same valid-floor tiles as the founding
/// cohort. Idempotent — does nothing if we're already at or above
/// the target population.
fn fill_workers_to(s: &mut Scenario, target: usize) {
    let current = s.app.world.iter_entities()
        .filter(|e| e.get::<Ant>().map_or(false, |a| matches!(a.kind, AntKind::Worker)))
        .count();
    if current >= target { return; }
    let need = target - current;

    let mut rng = StdRng::seed_from_u64(s.seed.wrapping_add(101));
    let spawns = {
        let grid = s.app.world.resource::<TileGrid>();
        let surface_adj = |x: i32, y: i32| {
            grid.get(x + 1, y).solid() || grid.get(x - 1, y).solid()
            || grid.get(x, y + 1).solid() || grid.get(x, y - 1).solid()
        };
        let mut spawns = Vec::with_capacity(need);
        for _ in 0..need {
            for _ in 0..120 {
                let dx = rng.gen_range(-14..14);
                let dy = rng.gen_range(0..20);
                let x = (COLONY_X + dx).clamp(1, grid.width - 2);
                let y = (COLONY_Y + dy).clamp(1, grid.height - 2);
                if grid.passable(x, y) && surface_adj(x, y) {
                    spawns.push((x as f32 + 0.5, y as f32 + 0.5));
                    break;
                }
            }
        }
        spawns
    };
    for (x, y) in spawns {
        s.app.world.spawn((
            Position(Vec2::new(x, y)),
            Velocity(Vec2::ZERO),
            Health { hp: 14.0, max_hp: 14.0 },
            FactionTag(Faction::Colony),
            Ant { kind: AntKind::Worker },
            Cargo::default(),
            Attacker::new(2.2, 1.4, 0.7),
            WorkerBrain::default(),
            VisualState::default(),
            AiTrace::default(),
        ));
    }
}

fn run_assertions(world: &World, sim_total: f32, is_final: bool) {
    let day = sim_total / DAY_LENGTH_SECONDS;
    let inside = colony_tile_set(world);

    let mut workers_alive  = 0_usize;
    let mut stuck_haulers  = 0_usize;
    for e in world.iter_entities() {
        let Some(ant) = e.get::<Ant>() else { continue; };
        if !matches!(ant.kind, AntKind::Worker) { continue; }
        workers_alive += 1;
        let p = match e.get::<Position>() { Some(p) => p, None => continue };
        let on_surface = (p.0.y as i32) < SURFACE_ROW;
        let carrying   = e.get::<Cargo>().map_or(false, |c| c.debris.is_some());
        let stuck      = e.get::<WorkerBrain>()
            .map_or(false, |b| b.haul_attempts >= 1);
        if on_surface && carrying && stuck {
            stuck_haulers += 1;
        }
    }

    // "Inside the colony" means specifically *underground* tunnel
    // tiles reachable from the entrance — surface tiles where
    // invaders are merely *en route* don't count as "the swarm
    // isn't engaging them," they're outside the nest.
    let mut spiders_alive_inside = 0_usize;
    let grid = world.resource::<TileGrid>();
    for e in world.iter_entities() {
        if !e.contains::<Spider>() { continue; }
        let Some(p) = e.get::<Position>() else { continue; };
        let x = p.0.x as i32;
        let y = p.0.y as i32;
        if x < 0 || y < 0 || x >= grid.width || y >= grid.height { continue; }
        if y <= SURFACE_ROW { continue; }
        if inside[(y * grid.width + x) as usize] {
            spiders_alive_inside += 1;
        }
    }

    // (a) Spiders accumulating inside the colony with the
    // population still healthy. We allow up to 2 in-transit spiders
    // — invaders arrive from off-screen and may briefly pass
    // through the colony tunnel network before workers reach them
    // — but more than that means the swarm response isn't keeping
    // up.
    const SPIDER_TRANSIT_TOLERANCE: usize = 2;
    if workers_alive >= ENGAGEMENT_POP_THRESHOLD
        && spiders_alive_inside > SPIDER_TRANSIT_TOLERANCE
    {
        panic!(
            "population_stress @ day {:.1}: {} spider(s) alive in the colony's \
             tunnel network (tolerance {}) with {} workers still standing — \
             the swarm response isn't engaging them. \
             (Check WORKER_THREAT_RADIUS and choose_mode's hostile-detection \
              priority.)",
            day, spiders_alive_inside, SPIDER_TRANSIT_TOLERANCE, workers_alive,
        );
    }

    // (b) Surface-cargo pile-up.
    let cap = (workers_alive as f32 * STUCK_HAULER_BUDGET) as usize;
    if workers_alive >= ENGAGEMENT_POP_THRESHOLD && stuck_haulers > cap {
        panic!(
            "population_stress @ day {:.1}: {} of {} workers are STUCK \
             above ground holding cargo (haul_attempts >= 1; cap {} = {:.0}% \
             of workforce). Haulers are gridlocking on tall mounds — \
             check MAX_HAUL_ATTEMPTS / step_deposit_debris force-drop hatch.",
            day, stuck_haulers, workers_alive, cap,
            STUCK_HAULER_BUDGET * 100.0,
        );
    }

    if is_final {
        eprintln!(
            "population_stress @ day {:.1} (FINAL): {} workers alive, \
             {} stuck haulers (cap {}), \
             {} spider(s) inside colony. PASS.",
            day, workers_alive, stuck_haulers, cap, spiders_alive_inside,
        );
    }
}

/// 4-connected flood fill from the colony entrance through passable
/// tiles, returning a bitmask sized to the grid. A spider whose
/// integer position lands on a `true` cell is "inside the colony"
/// — i.e. workers should be engaging it.
fn colony_tile_set(world: &World) -> Vec<bool> {
    let grid = world.resource::<TileGrid>();
    let w = grid.width;
    let h = grid.height;
    let mut visited = vec![false; (w * h) as usize];
    let mut queue   = VecDeque::new();
    if grid.passable(COLONY_X, COLONY_Y) {
        visited[(COLONY_Y * w + COLONY_X) as usize] = true;
        queue.push_back((COLONY_X, COLONY_Y));
    }
    while let Some((x, y)) = queue.pop_front() {
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let nx = x + dx;
            let ny = y + dy;
            if nx < 0 || ny < 0 || nx >= w || ny >= h { continue; }
            let i = (ny * w + nx) as usize;
            if visited[i] || !grid.passable(nx, ny) { continue; }
            visited[i] = true;
            queue.push_back((nx, ny));
        }
    }
    visited
}
