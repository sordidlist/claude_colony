//! ECS world + schedule wiring.
//!
//! `App::new(seed)` builds a fully-populated world: tile grid, pheromones,
//! water, dig queue, spatial grid, time/population resources, and an initial
//! cohort of worker ants. The `Schedule` chains every per-frame system in
//! data-flow order — see comments inline.

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::{Schedule, ExecutorKind};
use glam::Vec2;
use rand::{Rng, SeedableRng, rngs::StdRng};

use crate::config::*;
use crate::sim::{self, *};
use crate::sim::history::{History, Snapshot, capture_snapshot, restore_snapshot};
use crate::world::{TileGrid, PheromoneGrid, WaterGrid, DigJobs, ExploredGrid, ReturnFlowField, GrassField, dig_jobs};

pub struct App {
    pub world:    World,
    pub schedule: Schedule,
}

impl App {
    pub fn new(seed: u64) -> Self {
        let mut world = World::new();

        // ── Resources ────────────────────────────────────────────────────
        let mut grid = TileGrid::new(WORLD_WIDTH, WORLD_HEIGHT);
        crate::world::procgen::generate(&mut grid, seed);

        let phero = PheromoneGrid::new(WORLD_WIDTH, WORLD_HEIGHT);
        let water = WaterGrid::new(WORLD_WIDTH, WORLD_HEIGHT);
        let jobs  = DigJobs::new(seed);
        let spatial = sim::SpatialGrid::new();
        let explored = ExploredGrid::new(WORLD_WIDTH, WORLD_HEIGHT);
        let mut flow = ReturnFlowField::new(WORLD_WIDTH, WORLD_HEIGHT);
        flow.rebuild(&grid);

        world.insert_resource(grid);
        world.insert_resource(phero);
        world.insert_resource(water);
        world.insert_resource(jobs);
        world.insert_resource(spatial);
        world.insert_resource(explored);
        world.insert_resource(flow);
        world.insert_resource(sim::Time::default());
        world.insert_resource(sim::Population::default());
        world.insert_resource(sim::EventLog::default());
        world.insert_resource(sim::TimeOfDay::default());
        world.insert_resource(sim::DigStats::default());
        world.insert_resource(History::default());
        world.insert_resource(sim::SurfaceFoodSpawner::new(seed));
        world.insert_resource(sim::ColonyStores::default());
        world.insert_resource(sim::scenery::MowerSchedule::default());
        world.insert_resource(GrassField::new(WORLD_WIDTH));
        world.insert_resource(sim::hostiles::InvaderSpawner::new(seed));
        world.insert_resource(sim::BalanceTunables::default());

        spawn_initial_ants(&mut world, seed);
        spawn_queen(&mut world);
        // Hostiles no longer spawn underground at start. They arrive
        // from off-screen above ground via the periodic invader
        // spawner (see `sim::hostiles::spawn_invaders`).
        sim::scenery::spawn_initial_scenery(&mut world, seed);

        // ── Schedule ─────────────────────────────────────────────────────
        let mut schedule = Schedule::default();
        schedule.set_executor_kind(ExecutorKind::SingleThreaded);
        schedule.add_systems((
            sim::day_night::advance_day_night,
            sim::spatial::rebuild_spatial,
            decay_pheromones,
            sim::ai_worker::worker_ai,
            sim::ai_worker::flush_pending_tile_ops,
            sim::movement::integrate_movement,
            sim::exploration::update_exploration,
            dig_jobs::director_update,
            sim::queen::queen_tick,
            sim::queen::queen_migration,
            sim::brood::mature_brood,
        ).chain());
        schedule.add_systems((
            sim::soldier::soldier_tick,
            sim::hostiles::spider_tick,
            sim::hostiles::rival_tick,
            sim::hostiles::hostile_alarm_emission,
            sim::hostiles::spawn_invaders,
            sim::combat::combat_step,
            sim::combat::corpse_decay,
            sim::food_spawn::spawn_surface_food,
            sim::foraging::pickup_and_deposit,
            sim::lifecycle::update_population,
            sim::scenery::animate_scenery,
            sim::scenery::mower_lifecycle,
            crate::world::dirt_physics::settle_above_ground,
            crate::world::grass::grow_grass,
            crate::world::flow_field::maintain_flow_field,
            milestone_events,
        ).chain().after(sim::brood::mature_brood));

        Self { world, schedule }
    }

    pub fn step(&mut self, dt: f32) {
        {
            let mut t = self.world.resource_mut::<sim::Time>();
            t.dt    = dt;
            t.total += dt;
        }
        self.schedule.run(&mut self.world);
        self.maybe_snapshot(dt);
    }

    fn maybe_snapshot(&mut self, dt: f32) {
        if dt <= 0.0 { return; }
        let due = {
            let mut h = self.world.resource_mut::<History>();
            h.accum += dt;
            if h.accum >= h.interval {
                h.accum -= h.interval;
                true
            } else { false }
        };
        if !due { return; }
        let snap = capture_snapshot(&self.world);
        let mut h = self.world.resource_mut::<History>();
        h.buffer.push_back(snap);
        let cap = h.capacity_snapshots();
        while h.buffer.len() > cap { h.buffer.pop_front(); }
    }

    /// Pop the most recent snapshot and apply it. Returns true if the world
    /// was rewound; false if the buffer is empty.
    pub fn rewind_one_step(&mut self) -> bool {
        let snap: Option<Snapshot> = {
            let mut h = self.world.resource_mut::<History>();
            h.buffer.pop_back()
        };
        let Some(s) = snap else { return false; };
        restore_snapshot(&mut self.world, &s);
        true
    }
}

// `spawn_spiders` / `spawn_rivals` used to seed an initial
// underground/edge population. Hostiles now arrive from off-screen
// over time via `sim::hostiles::spawn_invaders` instead — easier to
// pace, no surprise lurkers underground at t=0.

fn spawn_queen(world: &mut World) {
    use glam::Vec2;
    let (qx, qy) = {
        let g = world.resource::<TileGrid>();
        sim::queen::find_queen_spot(&g)
    };
    world.spawn((
        Position(Vec2::new(qx, qy)),
        Velocity(Vec2::ZERO),
        Health { hp: 60.0, max_hp: 60.0 },
        FactionTag(Faction::Colony),
        Ant { kind: AntKind::Queen },
        Cargo::default(),
        QueenState::default(),
        VisualState::default(),
        AiTrace::default(),
    ));
}

// `find_queen_spot` lives in `sim::queen` so the migration system can
// share it.

fn spawn_initial_ants(world: &mut World, seed: u64) {
    let mut rng = StdRng::seed_from_u64(seed.wrapping_add(7));
    let spawns = {
        let grid = world.resource::<TileGrid>();
        let mut spawns = Vec::with_capacity(INITIAL_WORKERS);
        let (ex, ey) = (COLONY_X, COLONY_Y);
        let surface_adj = |x: i32, y: i32| {
            grid.get(x+1, y).solid() || grid.get(x-1, y).solid()
            || grid.get(x, y+1).solid() || grid.get(x, y-1).solid()
        };
        for _ in 0..INITIAL_WORKERS {
            for _ in 0..80 {
                let dx = rng.gen_range(-14..14);
                let dy = rng.gen_range(0..20);
                let x = (ex + dx).clamp(1, grid.width  - 2);
                let y = (ey + dy).clamp(1, grid.height - 2);
                // Spawn only on a passable tile that has a solid neighbour —
                // otherwise the ant would immediately fall through the world.
                if grid.passable(x, y) && surface_adj(x, y) {
                    spawns.push((x as f32 + 0.5, y as f32 + 0.5));
                    break;
                }
            }
        }
        spawns
    };

    for (x, y) in spawns {
        world.spawn((
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

fn decay_pheromones(time: Res<sim::Time>, mut p: ResMut<PheromoneGrid>) {
    p.decay(time.dt);
}

#[derive(Resource, Default)]
struct MilestoneTracker {
    last_pop_bucket: usize,
}

fn milestone_events(
    pop:   Res<sim::Population>,
    mut log: ResMut<sim::EventLog>,
    mut tracker: Local<MilestoneTracker>,
) {
    let bucket = pop.workers / 100;
    if tracker.last_pop_bucket == 0 && bucket > 0 {
        tracker.last_pop_bucket = bucket;
    } else if bucket > tracker.last_pop_bucket {
        log.push(format!("Colony grows — {} workers", pop.workers),
                 [0.46, 0.92, 0.42, 1.0]);
        tracker.last_pop_bucket = bucket;
    } else if bucket + 1 < tracker.last_pop_bucket {
        log.push(format!("Colony falters — {} workers", pop.workers),
                 [0.96, 0.48, 0.32, 1.0]);
        tracker.last_pop_bucket = bucket;
    }
}
