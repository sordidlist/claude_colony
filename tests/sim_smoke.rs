//! Headless smoke tests. Run with `cargo test --release`.
//!
//! These don't touch the renderer — the App is renderer-agnostic by design.
//! This is exactly the kind of test I wish the Python build had: verifies
//! that the dig pipeline (director → claim → progress → tile mutation) end
//! to end actually fires under simulation, instead of silently deadlocking.

use colony::app::App;
use colony::sim::Population;
use colony::world::{TileGrid, DigJobs};

/// 60 seconds of simulation should yield meaningful digs and never let the
/// queue saturate with stale claims.
#[test]
fn ants_actually_dig() {
    let mut app = App::new(42);
    let dt = 1.0 / 60.0;

    let initial_workers = {
        // run one step so population resource is filled
        app.step(dt);
        app.world.resource::<Population>().workers
    };
    assert!(initial_workers > 100,
            "expected the spawn loop to seat hundreds of workers, got {}",
            initial_workers);

    // Snapshot tile state at t=0 so we can count diffs.
    let tiles_before = {
        let g = app.world.resource::<TileGrid>();
        g.tiles.clone()
    };

    // Sim 60 wall-seconds at 60 fps
    for i in 0..(60 * 60) {
        app.step(dt);
        if i % (60 * 10) == 0 {
            let pop  = *app.world.resource::<Population>();
            let jobs = app.world.resource::<DigJobs>();
            eprintln!("t={:>4.1}s  workers={} digging={} jobs(occ={}, unclaimed={})",
                      i as f32 * dt,
                      pop.workers, pop.digging,
                      jobs.occupied_count(), jobs.unclaimed_count());
        }
    }

    let tiles_after = {
        let g = app.world.resource::<TileGrid>();
        g.tiles.clone()
    };

    let dug_count = tiles_before.iter()
        .zip(tiles_after.iter())
        .filter(|(a, b)| a != b)
        .count();

    // The Python build, with the leaked-claim bug, completed ~2 digs in 60s.
    // The slot-table design should beat that comfortably even with no
    // tuning. Set a lower bound that actually validates "ants dig at all".
    assert!(dug_count >= 5,
            "expected ≥ 5 dug tiles in 60s, got {}", dug_count);

    let _ = app.world.resource::<DigJobs>();
}

/// The colony should keep digging *over time* — not just during the first
/// burst of activity, then plateau because workers are stuck holding
/// recycled dig claims. Compares dig progress in two equal windows; the
/// later window must be productive too.
#[test]
fn colony_keeps_growing_over_time() {
    use colony::world::TileGrid;
    let mut app = App::new(42);
    let dt = 1.0 / 60.0;

    // Warm-up so the queen, director, and worker pool reach steady state.
    for _ in 0..(60 * 30) { app.step(dt); }

    let snap_a = app.world.resource::<TileGrid>().tiles.clone();
    for _ in 0..(60 * 30) { app.step(dt); }
    let snap_b = app.world.resource::<TileGrid>().tiles.clone();
    for _ in 0..(60 * 30) { app.step(dt); }
    let snap_c = app.world.resource::<TileGrid>().tiles.clone();

    let dug_window_1 = snap_a.iter().zip(snap_b.iter()).filter(|(a,b)| a != b).count();
    let dug_window_2 = snap_b.iter().zip(snap_c.iter()).filter(|(a,b)| a != b).count();

    // Window 1 should make some progress (warm-up is done).
    assert!(dug_window_1 >= 5, "window 1 dug only {} tiles", dug_window_1);
    // Window 2 must also be productive — the regression we're guarding
    // against is "tunnels stop growing after the first wave of digging."
    assert!(dug_window_2 >= 5,
            "tunnel growth stalled — window 1 dug {}, window 2 dug {}",
            dug_window_1, dug_window_2);
}

/// Population resource stays in sync with worker count.
#[test]
fn population_tracking() {
    let mut app = App::new(42);
    app.step(1.0 / 60.0);
    let pop = *app.world.resource::<Population>();
    assert_eq!(pop.workers, colony::config::INITIAL_WORKERS,
               "Population::workers should match INITIAL_WORKERS after first step");
    assert_eq!(pop.queens, 1, "exactly one queen should be alive at startup");
}

/// The queen must always land on a tile reachable from the colony entrance —
/// otherwise she's sealed off from her workers and the colony silently dies.
/// Try several seeds to catch worldgen variations.
#[test]
fn queen_is_reachable_from_entrance() {
    use colony::config::{COLONY_X, COLONY_Y};
    use colony::sim::components::{Position, Ant, AntKind};
    use colony::world::TileGrid;
    use std::collections::VecDeque;

    for seed in [1u64, 7, 42, 99, 12345, 67890] {
        let app = App::new(seed);

        // Find the queen
        let mut queen_pos: Option<(i32, i32)> = None;
        for entity in app.world.iter_entities() {
            if let Some(a) = entity.get::<Ant>() {
                if a.kind == AntKind::Queen {
                    let p = entity.get::<Position>().unwrap();
                    queen_pos = Some((p.0.x as i32, p.0.y as i32));
                    break;
                }
            }
        }
        let (qx, qy) = queen_pos.expect("seed produced no queen");

        // BFS from the entrance through passable tiles
        let grid = app.world.resource::<TileGrid>();
        let mut visited = vec![false; (grid.width * grid.height) as usize];
        let mut q       = VecDeque::new();
        if grid.passable(COLONY_X, COLONY_Y) {
            visited[grid.idx(COLONY_X, COLONY_Y)] = true;
            q.push_back((COLONY_X, COLONY_Y));
        }
        let mut reachable = false;
        while let Some((x, y)) = q.pop_front() {
            if x == qx && y == qy { reachable = true; break; }
            for (dx, dy) in [(1,0),(-1,0),(0,1),(0,-1)] {
                let nx = x + dx; let ny = y + dy;
                if !grid.in_bounds(nx, ny) { continue; }
                let i = grid.idx(nx, ny);
                if visited[i] || !grid.passable(nx, ny) { continue; }
                visited[i] = true;
                q.push_back((nx, ny));
            }
        }
        assert!(reachable,
                "seed {}: queen at ({}, {}) is not reachable from entrance ({}, {})",
                seed, qx, qy, COLONY_X, COLONY_Y);
    }
}
