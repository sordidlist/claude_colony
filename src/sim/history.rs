//! Rewind buffer.
//!
//! `History` is a ring of `Snapshot`s captured every `SNAPSHOT_INTERVAL_S`
//! seconds of *sim* time. Each snapshot clones every component used by
//! visible entities (ants + queens + scenery) plus the tile, exploration,
//! day/night, and dig-queue resources. Rewinding pops snapshots from the
//! tail and re-applies them — fast, since restoration is just memcpys and
//! batched spawns.
//!
//! Memory: at default resolution (~3 KB per ant × ~1k ants ≈ 80 KB ant data
//! + ~190 KB grids + 2 KB dig slots = ~270 KB per snapshot, ×60 snapshots ≈
//! 16 MB resident). That's a comfortable trade for a one-second rewind grid
//! over a full minute.

use std::collections::VecDeque;
use bevy_ecs::prelude::*;
use glam::Vec2;
use crate::config::*;
use crate::world::{TileGrid, ExploredGrid, DigJobs, dig_jobs::DigJobsSnapshot};
use super::components::*;
use super::scenery::{Decoration, DecorPos};
use super::{Time, TimeOfDay, Population};

#[derive(Clone)]
pub struct AntSnap {
    pub pos:    Position,
    pub vel:    Velocity,
    pub cargo:  Cargo,
    pub vis:    VisualState,
    pub hp:     Health,
    pub ant:    Ant,
    pub brain:  Option<WorkerBrain>,
    pub queen:  Option<QueenState>,
}

#[derive(Clone)]
pub struct DecorSnap {
    pub pos:   DecorPos,
    pub decor: Decoration,
}

#[derive(Clone)]
pub struct HostileSnap {
    pub pos:   Position,
    pub vel:   Velocity,
    pub vis:   VisualState,
    pub hp:    Health,
    pub kind:  HostileKind,
}

#[derive(Clone, Copy)]
pub enum HostileKind {
    Spider(Spider),
    Rival(RivalAnt),
}

#[derive(Clone)]
pub struct Snapshot {
    pub ants:        Vec<AntSnap>,
    pub decors:      Vec<DecorSnap>,
    pub hostiles:    Vec<HostileSnap>,
    pub tiles:       Vec<u8>,
    pub variants:    Vec<u8>,
    pub explored:    Vec<u8>,
    pub time_of_day: TimeOfDay,
    pub time_total:  f32,
    pub dig_jobs:    DigJobsSnapshot,
    pub population:  Population,
}

#[derive(Resource)]
pub struct History {
    pub buffer: VecDeque<Snapshot>,
    pub accum:  f32,
    pub max_seconds:    f32,
    pub interval:       f32,
}

impl Default for History {
    fn default() -> Self {
        let cap = (REWIND_HISTORY_SECONDS / SNAPSHOT_INTERVAL_S).ceil() as usize + 2;
        Self {
            buffer:      VecDeque::with_capacity(cap),
            accum:       0.0,
            max_seconds: REWIND_HISTORY_SECONDS,
            interval:    SNAPSHOT_INTERVAL_S,
        }
    }
}

impl History {
    pub fn capacity_snapshots(&self) -> usize {
        (self.max_seconds / self.interval).ceil() as usize + 1
    }
    pub fn seconds_buffered(&self) -> f32 {
        self.buffer.len() as f32 * self.interval
    }
}

// ── capture ─────────────────────────────────────────────────────────────────

pub fn capture_snapshot(world: &World) -> Snapshot {
    let mut ants     = Vec::new();
    let mut decors   = Vec::new();
    let mut hostiles = Vec::new();

    for entity in world.iter_entities() {
        if let Some(ant) = entity.get::<Ant>() {
            let pos   = entity.get::<Position>().copied().unwrap_or(Position(Vec2::ZERO));
            let vel   = entity.get::<Velocity>().copied().unwrap_or(Velocity(Vec2::ZERO));
            let cargo = entity.get::<Cargo>().copied().unwrap_or_default();
            let vis   = entity.get::<VisualState>().copied().unwrap_or_default();
            let hp    = entity.get::<Health>().copied().unwrap_or(
                Health { hp: 1.0, max_hp: 1.0 });
            let brain = entity.get::<WorkerBrain>().copied();
            let queen = entity.get::<QueenState>().copied();
            ants.push(AntSnap { pos, vel, cargo, vis, hp, ant: *ant, brain, queen });
        } else if let Some(spider) = entity.get::<Spider>() {
            let pos = entity.get::<Position>().copied().unwrap_or(Position(Vec2::ZERO));
            let vel = entity.get::<Velocity>().copied().unwrap_or(Velocity(Vec2::ZERO));
            let vis = entity.get::<VisualState>().copied().unwrap_or_default();
            let hp  = entity.get::<Health>().copied().unwrap_or(
                Health { hp: 1.0, max_hp: 1.0 });
            hostiles.push(HostileSnap { pos, vel, vis, hp, kind: HostileKind::Spider(*spider) });
        } else if let Some(rival) = entity.get::<RivalAnt>() {
            let pos = entity.get::<Position>().copied().unwrap_or(Position(Vec2::ZERO));
            let vel = entity.get::<Velocity>().copied().unwrap_or(Velocity(Vec2::ZERO));
            let vis = entity.get::<VisualState>().copied().unwrap_or_default();
            let hp  = entity.get::<Health>().copied().unwrap_or(
                Health { hp: 1.0, max_hp: 1.0 });
            hostiles.push(HostileSnap { pos, vel, vis, hp, kind: HostileKind::Rival(*rival) });
        } else if let Some(decor) = entity.get::<Decoration>() {
            let pos = entity.get::<DecorPos>().copied()
                .unwrap_or(DecorPos { x: 0.0, y: 0.0 });
            decors.push(DecorSnap { pos, decor: *decor });
        }
    }

    let g  = world.resource::<TileGrid>();
    let e  = world.resource::<ExploredGrid>();
    let dj = world.resource::<DigJobs>();
    let tt = world.resource::<Time>().total;
    Snapshot {
        ants, decors, hostiles,
        tiles:    g.tiles.clone(),
        variants: g.variants.clone(),
        explored: e.data.clone(),
        time_of_day: *world.resource::<TimeOfDay>(),
        time_total:  tt,
        dig_jobs:    dj.snapshot(),
        population:  *world.resource::<Population>(),
    }
}

// ── restore ─────────────────────────────────────────────────────────────────

pub fn restore_snapshot(world: &mut World, snap: &Snapshot) {
    // Despawn every dynamic entity (ants, hostiles, decorations). Resources stay.
    let dynamic: Vec<Entity> = world
        .iter_entities()
        .filter(|e| e.contains::<Ant>()
                 || e.contains::<Decoration>()
                 || e.contains::<Spider>()
                 || e.contains::<RivalAnt>())
        .map(|e| e.id())
        .collect();
    for id in dynamic {
        world.despawn(id);
    }

    // Restore grids
    {
        let mut g = world.resource_mut::<TileGrid>();
        g.tiles.copy_from_slice(&snap.tiles);
        g.variants.copy_from_slice(&snap.variants);
        g.dirty = true;
    }
    {
        let mut e = world.resource_mut::<ExploredGrid>();
        e.data.copy_from_slice(&snap.explored);
        e.dirty = true;
    }
    *world.resource_mut::<TimeOfDay>() = snap.time_of_day;
    world.resource_mut::<Time>().total = snap.time_total;
    *world.resource_mut::<Population>() = snap.population;
    world.resource_mut::<DigJobs>().restore(&snap.dig_jobs);

    // Re-spawn ants
    for a in &snap.ants {
        match (a.brain, a.queen) {
            (Some(b), None) => {
                world.spawn((
                    a.pos, a.vel, a.cargo, a.vis, a.hp,
                    FactionTag(Faction::Colony),
                    a.ant, b,
                ));
            }
            (None, Some(q)) => {
                world.spawn((
                    a.pos, a.vel, a.cargo, a.vis, a.hp,
                    FactionTag(Faction::Colony),
                    a.ant, q,
                ));
            }
            (Some(b), Some(q)) => {
                world.spawn((
                    a.pos, a.vel, a.cargo, a.vis, a.hp,
                    FactionTag(Faction::Colony),
                    a.ant, b, q,
                ));
            }
            (None, None) => {
                // Edge case: ant with neither brain nor queen state. Spawn
                // with a default brain so the AI system still progresses it.
                world.spawn((
                    a.pos, a.vel, a.cargo, a.vis, a.hp,
                    FactionTag(Faction::Colony),
                    a.ant, WorkerBrain::default(),
                ));
            }
        }
    }
    // Re-spawn decorations
    for d in &snap.decors {
        world.spawn((d.pos, d.decor));
    }
    // Re-spawn hostiles
    for h in &snap.hostiles {
        match h.kind {
            HostileKind::Spider(s) => {
                world.spawn((
                    h.pos, h.vel, h.vis, h.hp,
                    FactionTag(Faction::Predator),
                    s,
                ));
            }
            HostileKind::Rival(r) => {
                world.spawn((
                    h.pos, h.vel, h.vis, h.hp,
                    FactionTag(Faction::Rival),
                    r,
                ));
            }
        }
    }
}
