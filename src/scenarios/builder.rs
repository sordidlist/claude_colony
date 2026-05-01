//! Scenario builder + harness. Owns an `App` (full ECS world + schedule)
//! and provides chainable carve / spawn / run helpers.

use bevy_ecs::prelude::*;
use glam::Vec2;
use crate::app::App;
use crate::world::{TileGrid, TileType, ExploredGrid};
use crate::sim::components::*;

/// Marker component a scenario stamps on its key entity (the worker
/// being tested, the spider being killed, etc.). Predicates can `.query()`
/// for `(Position, &TestSubject)` without round-tripping through Entity
/// IDs that aren't visible from a `fn(&World) -> bool` predicate.
///
/// `id` lets a scenario tag multiple distinct subjects (0, 1, 2…).
#[derive(Component, Copy, Clone, Debug, Default)]
pub struct TestSubject {
    pub id: u8,
}

/// Builder + harness. Each scenario owns one of these, mutates it via
/// the chainable carve / spawn helpers, then drives it with `run_until`.
pub struct Scenario {
    pub app:  App,
    pub seed: u64,
}

impl Scenario {
    /// Spin up a fully-populated world, ready for the scenario to clear
    /// and re-shape. Most scenarios immediately call `.clear_creatures()`
    /// so they can hand-place a single test subject.
    pub fn new(seed: u64) -> Self {
        Self { app: App::new(seed), seed }
    }

    /// Despawn every dynamic creature (ants, hostiles, brood, decor,
    /// food, corpses). Resources stay. After this, the scenario can
    /// spawn one or two actors without thousands of others competing.
    pub fn clear_creatures(&mut self) -> &mut Self {
        let dynamic: Vec<Entity> = self.app.world
            .iter_entities()
            .filter(|e| e.contains::<Ant>()
                     || e.contains::<Spider>()
                     || e.contains::<RivalAnt>()
                     || e.contains::<Brood>()
                     || e.contains::<Food>()
                     || e.contains::<Corpse>()
                     || e.contains::<crate::sim::scenery::Decoration>())
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

    /// Carve a horizontal tunnel `h` tiles tall between two columns
    /// (rows `y..y+h`).
    pub fn carve_horizontal(&mut self, x0: i32, x1: i32, y: i32, h: i32) -> &mut Self {
        let mut g = self.app.world.resource_mut::<TileGrid>();
        for x in x0.min(x1)..=x0.max(x1) {
            for dy in 0..h {
                g.set(x, y + dy, TileType::Tunnel);
            }
        }
        self
    }

    /// Carve a vertical shaft `w` tiles wide between two rows.
    pub fn carve_vertical(&mut self, x: i32, y0: i32, y1: i32, w: i32) -> &mut Self {
        let mut g = self.app.world.resource_mut::<TileGrid>();
        for y in y0.min(y1)..=y0.max(y1) {
            for dx in 0..w {
                g.set(x + dx, y, TileType::Tunnel);
            }
        }
        self
    }

    /// Reveal the whole map so the renderer-side fog wouldn't hide
    /// scenario state when viewing live. No effect on simulation.
    pub fn reveal_all(&mut self) -> &mut Self {
        let mut e = self.app.world.resource_mut::<ExploredGrid>();
        for v in e.data.iter_mut() { *v = 255; }
        e.dirty = true;
        self
    }

    /// Mark the tile grid dirty so cached state (fog, water mask)
    /// rebuilds. Cheap to call after any carve.
    pub fn mark_dirty(&mut self) -> &mut Self {
        self.app.world.resource_mut::<TileGrid>().dirty = true;
        self
    }

    /// Force the return flow field to rebuild against current tiles.
    /// Scenarios carve their own world after `App::new`, so the initial
    /// flow field doesn't reflect the test layout — call this after
    /// carving so workers can navigate the test geometry from frame 1
    /// instead of waiting for the periodic background rebuild.
    pub fn rebuild_flow_field(&mut self) -> &mut Self {
        self.app.world.resource_scope::<crate::world::ReturnFlowField, _>(|world, mut field| {
            let grid = world.resource::<TileGrid>();
            field.rebuild(grid);
        });
        self
    }

    /// Spawn a worker at a tile centre, optionally pre-loaded with a
    /// debris pebble. Tags the worker with `TestSubject { id: 0 }` so
    /// scenarios can find it from the predicate without entity IDs.
    pub fn spawn_worker(&mut self, x: i32, y: i32, debris: Option<TileType>) -> Entity {
        self.spawn_worker_tagged(x, y, debris, 0)
    }

    /// Spawn a worker tagged with a specific `TestSubject.id` for
    /// scenarios that need to distinguish multiple test subjects.
    pub fn spawn_worker_tagged(&mut self, x: i32, y: i32,
                                debris: Option<TileType>, id: u8) -> Entity {
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
            Attacker::new(2.2, 1.4, 0.7),
            WorkerBrain::default(),
            VisualState::default(),
            TestSubject { id },
        )).id()
    }

    /// Spawn a soldier at a tile centre, tagged with `TestSubject.id`.
    pub fn spawn_soldier_tagged(&mut self, x: i32, y: i32, id: u8) -> Entity {
        let pos = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
        self.app.world.spawn((
            Position(pos),
            Velocity(Vec2::ZERO),
            Health { hp: 35.0, max_hp: 35.0 },
            FactionTag(Faction::Colony),
            Ant { kind: AntKind::Soldier },
            Cargo::default(),
            Attacker::new(4.0, 1.6, 0.8),
            SoldierAi::default(),
            VisualState::default(),
            TestSubject { id },
        )).id()
    }

    /// Spawn a spider at a tile centre, tagged with `TestSubject.id`.
    pub fn spawn_spider_tagged(&mut self, x: i32, y: i32, id: u8) -> Entity {
        let pos = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
        self.app.world.spawn((
            Position(pos),
            Velocity(Vec2::ZERO),
            Health { hp: 22.0, max_hp: 22.0 },
            FactionTag(Faction::Predator),
            Spider::default(),
            Attacker::new(3.0, 1.5, 1.2),
            VisualState::default(),
            TestSubject { id },
        )).id()
    }

    /// Spawn a queen at a tile centre, tagged with `TestSubject.id`.
    pub fn spawn_queen_tagged(&mut self, x: i32, y: i32, id: u8) -> Entity {
        let pos = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
        self.app.world.spawn((
            Position(pos),
            Velocity(Vec2::ZERO),
            Health { hp: 60.0, max_hp: 60.0 },
            FactionTag(Faction::Colony),
            Ant { kind: AntKind::Queen },
            Cargo::default(),
            QueenState::default(),
            VisualState::default(),
            TestSubject { id },
        )).id()
    }

    /// Spawn a `Brood` entity. `mature_in` is how many seconds before
    /// it hatches; `to_soldier` selects the caste.
    pub fn spawn_brood(&mut self, x: i32, y: i32, mature_in: f32,
                       to_soldier: bool) -> Entity {
        let pos = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
        self.app.world.spawn((
            Position(pos),
            Velocity(Vec2::ZERO),
            Brood { timer: mature_in, will_be_soldier: to_soldier },
            TestSubject::default(),
        )).id()
    }

    /// Spawn a single food pellet at a tile centre.
    pub fn spawn_food(&mut self, x: i32, y: i32) -> Entity {
        self.app.world.spawn((
            Position(Vec2::new(x as f32 + 0.5, y as f32 + 0.5)),
            Velocity(Vec2::ZERO),
            Food { value: 1 },
        )).id()
    }

    /// Step the App until `predicate(world)` returns true, or until
    /// `max_seconds` of sim time elapses (1/60 s fixed timestep).
    /// Returns `Ok(elapsed)` on success, `Err(elapsed)` on timeout.
    pub fn run_until<F>(&mut self, max_seconds: f32, mut predicate: F) -> Result<f32, f32>
    where F: FnMut(&World) -> bool,
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

    /// Convenience: did `e` reach within `radius` tiles of `(x, y)`?
    pub fn within(&self, e: Entity, x: i32, y: i32, radius: f32) -> bool {
        let p = self.app.world.get::<Position>(e);
        let Some(p) = p else { return false; };
        let dx = p.0.x - (x as f32 + 0.5);
        let dy = p.0.y - (y as f32 + 0.5);
        (dx*dx + dy*dy).sqrt() <= radius
    }
}

// ── Predicate helpers ──────────────────────────────────────────────
//
// Predicates take `&World`. They can't capture entity IDs because the
// `ScenarioDef` predicate is a plain `fn` pointer. Instead they query
// for `TestSubject` markers attached at setup time.

/// Find the position of the test subject with the given id, if any
/// entity with that id is still alive.
///
/// Predicates run with a `&World`, which means we can't use the
/// `World::query::<>()` API (which mutates internal state to cache the
/// query). Instead we iterate entities and use `EntityRef::get::<T>()`
/// — no caching, but predicate-call-rate is low enough not to matter.
pub fn subject_pos(world: &World, id: u8) -> Option<Vec2> {
    for e in world.iter_entities() {
        let Some(t) = e.get::<TestSubject>() else { continue; };
        if t.id != id { continue; }
        if let Some(p) = e.get::<Position>() {
            return Some(p.0);
        }
    }
    None
}

/// True iff *any* test subject with the given id is alive.
pub fn subject_alive(world: &World, id: u8) -> bool {
    subject_pos(world, id).is_some()
}

/// Read the cargo on the test-subject worker with the given id. Useful
/// for "did the forager pick up food?" predicates.
pub fn subject_cargo(world: &World, id: u8) -> Option<Cargo> {
    for e in world.iter_entities() {
        let Some(t) = e.get::<TestSubject>() else { continue; };
        if t.id != id { continue; }
        if let Some(c) = e.get::<Cargo>() {
            return Some(*c);
        }
    }
    None
}
