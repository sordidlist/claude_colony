//! Behaviour-test scaffolding.
//!
//! A scenario is a hand-built mini world plus a goal: spawn the entities
//! we want, run the sim for a bounded number of frames, and assert that
//! some target state was reached. Scenarios are deterministic, fast (a
//! single ant for a few seconds of sim time), and they don't share state
//! with other tests.
//!
//! Each scenario file imports `crate::scenarios::Scenario` and uses the
//! builder to set up tiles, spawn entities, then `.run_until(...)` to
//! drive the App and check a predicate every frame.

use bevy_ecs::prelude::*;
use glam::Vec2;
use colony::app::App;
use colony::config::*;
use colony::world::{TileGrid, TileType, ExploredGrid};
use colony::sim::components::*;

/// Builder + harness for behavioural scenarios. Owns its own `App`, which
/// is initialised with default procgen and then carved/cleared/filled to
/// match the scenario.
pub struct Scenario {
    pub app: App,
    /// Fixed seed for reproducibility — every scenario runs identically.
    pub seed: u64,
}

impl Scenario {
    /// Spin up an empty (well: procgen-default) world with all the queen,
    /// initial workers, etc. that `App::new` produces, then optionally
    /// despawn everything so the scenario can hand-place its actors. Most
    /// scenarios call `.clear_creatures()` immediately after construction.
    pub fn new(seed: u64) -> Self {
        Self { app: App::new(seed), seed }
    }

    /// Despawn every dynamic creature (ants / hostiles / brood / decor /
    /// food). Resources stay. After this you can spawn a single test
    /// subject without thousands of others getting in the way.
    pub fn clear_creatures(&mut self) -> &mut Self {
        let dynamic: Vec<Entity> = self.app.world
            .iter_entities()
            .filter(|e| e.contains::<Ant>()
                     || e.contains::<Spider>()
                     || e.contains::<RivalAnt>()
                     || e.contains::<Brood>()
                     || e.contains::<Food>()
                     || e.contains::<Corpse>()
                     || e.contains::<colony::sim::scenery::Decoration>())
            .map(|e| e.id())
            .collect();
        for e in dynamic {
            self.app.world.despawn(e);
        }
        self
    }

    /// Fill an axis-aligned rectangle of tiles with `t`. Bounds are
    /// inclusive on both ends.
    pub fn fill_rect(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, t: TileType) -> &mut Self {
        let mut g = self.app.world.resource_mut::<TileGrid>();
        for y in y0..=y1 {
            for x in x0..=x1 {
                g.set(x, y, t);
            }
        }
        self
    }

    /// Carve a horizontal tunnel of height `h` between two columns at row
    /// `y` (top of tunnel). Tiles inside become `Tunnel`.
    pub fn carve_horizontal(&mut self, x0: i32, x1: i32, y: i32, h: i32) -> &mut Self {
        let mut g = self.app.world.resource_mut::<TileGrid>();
        for x in x0.min(x1)..=x0.max(x1) {
            for dy in 0..h {
                g.set(x, y + dy, TileType::Tunnel);
            }
        }
        self
    }

    /// Carve a vertical shaft of width `w` between two rows.
    pub fn carve_vertical(&mut self, x: i32, y0: i32, y1: i32, w: i32) -> &mut Self {
        let mut g = self.app.world.resource_mut::<TileGrid>();
        for y in y0.min(y1)..=y0.max(y1) {
            for dx in 0..w {
                g.set(x + dx, y, TileType::Tunnel);
            }
        }
        self
    }

    /// Reveal everything so the renderer-side fog wouldn't hide test
    /// state if you ever inspect it visually. Doesn't affect sim.
    pub fn reveal_all(&mut self) -> &mut Self {
        let mut e = self.app.world.resource_mut::<ExploredGrid>();
        for v in e.data.iter_mut() { *v = 255; }
        e.dirty = true;
        self
    }

    /// Mark the tile grid dirty so any cached state (e.g. water mask)
    /// rebuilds. Cheap to call after carving.
    pub fn mark_dirty(&mut self) -> &mut Self {
        self.app.world.resource_mut::<TileGrid>().dirty = true;
        self
    }

    /// Force the return flow field to rebuild against the current tile
    /// state. Scenarios carve their own world after `App::new`, so the
    /// initial flow field doesn't reflect the test layout — call this
    /// after carving so workers can navigate the test geometry from the
    /// first frame instead of waiting for the periodic rebuild.
    pub fn rebuild_flow_field(&mut self) -> &mut Self {
        self.app.world.resource_scope::<colony::world::ReturnFlowField, _>(|world, mut field| {
            let grid = world.resource::<TileGrid>();
            field.rebuild(grid);
        });
        self
    }

    /// Spawn a worker ant at a tile centre with optional cargo.
    pub fn spawn_worker(&mut self, x: i32, y: i32, debris: Option<TileType>) -> Entity {
        let pos = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
        let mut cargo = Cargo::default();
        cargo.debris = debris;
        self.app.world.spawn((
            Position(pos),
            Velocity(Vec2::ZERO),
            Health { hp: 14.0, max_hp: 14.0 },
            FactionTag(Faction::Colony),
            Ant { kind: AntKind::Worker },
            cargo,
            Attacker::new(1.5, 1.4, 0.9),
            WorkerBrain::default(),
            VisualState::default(),
        )).id()
    }

    /// Spawn a single FoodPellet at a tile centre.
    pub fn spawn_food(&mut self, x: i32, y: i32) -> Entity {
        self.app.world.spawn((
            Position(Vec2::new(x as f32 + 0.5, y as f32 + 0.5)),
            Velocity(Vec2::ZERO),
            Food { value: 1 },
        )).id()
    }

    /// Step `app` until `predicate(world)` returns true, or until
    /// `max_seconds` of sim time elapses (1/60 fixed timestep).
    /// Returns Ok(elapsed_seconds) on success, Err(elapsed_seconds) on
    /// timeout — the elapsed time is useful for "how fast did it
    /// happen?" assertions.
    pub fn run_until<F>(&mut self, max_seconds: f32, mut predicate: F) -> Result<f32, f32>
    where
        F: FnMut(&World) -> bool,
    {
        let dt = 1.0 / 60.0;
        let max_frames = (max_seconds / dt).ceil() as usize;
        for frame in 0..max_frames {
            self.app.step(dt);
            if predicate(&self.app.world) {
                return Ok(frame as f32 * dt);
            }
        }
        Err(max_frames as f32 * dt)
    }

    /// Convenience: get an entity's current Position, or panic if it was
    /// despawned (typically a sign that the scenario broke unexpectedly).
    pub fn position_of(&self, e: Entity) -> Vec2 {
        self.app.world.get::<Position>(e)
            .expect("entity has been despawned").0
    }

    /// Convenience: did `e` reach within `radius` tiles of `(x, y)`?
    pub fn within(&self, e: Entity, x: i32, y: i32, radius: f32) -> bool {
        let p = self.app.world.get::<Position>(e);
        let Some(p) = p else { return false; };
        let dx = p.0.x - (x as f32 + 0.5);
        let dy = p.0.y - (y as f32 + 0.5);
        (dx*dx + dy*dy).sqrt() <= radius
    }
}

// Individual scenarios live in submodules.
pub mod escape_simple_chamber;
pub mod escape_winding_tunnel;
pub mod hauler_drops_outside;
pub mod swarm_kills_spider;
