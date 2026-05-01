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
use super::scenery::{Decoration, DecorPos, MowerSchedule};
use super::{Time, TimeOfDay, Population};

#[derive(Clone)]
pub struct AntSnap {
    pub pos:      Position,
    pub vel:      Velocity,
    pub cargo:    Cargo,
    pub vis:      VisualState,
    pub hp:       Health,
    pub ant:      Ant,
    pub brain:    Option<WorkerBrain>,
    pub queen:    Option<QueenState>,
    /// Combat capability — present on workers and soldiers, absent on
    /// the queen. Without this, ants restored from history were
    /// silently stripped of `Attacker`, which removed them from the
    /// `combat_step` query and caused all combat to freeze the moment
    /// the player rewound — including hostiles, which became
    /// immortal zombies that piled up at the entrance forever.
    pub attacker: Option<Attacker>,
    /// Soldier-only patrol/chase state. Without this, a soldier
    /// restored from history kept its `Ant.kind == Soldier` tag but
    /// lost its `SoldierAi` component, so `soldier_tick` skipped it
    /// and it stood still doing nothing.
    pub soldier:  Option<SoldierAi>,
}

#[derive(Clone)]
pub struct DecorSnap {
    pub pos:   DecorPos,
    pub decor: Decoration,
}

#[derive(Clone)]
pub struct HostileSnap {
    pub pos:      Position,
    pub vel:      Velocity,
    pub vis:      VisualState,
    pub hp:       Health,
    pub kind:     HostileKind,
    /// Same Attacker-loss bug as AntSnap: hostiles restored without
    /// this component dropped out of the combat query and became
    /// unkillable.
    pub attacker: Option<Attacker>,
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
    /// Mower lifecycle phase at capture time. Without this, a rewind
    /// can leave the schedule out of sync with the world (e.g. mower
    /// entity restored but schedule still says Cooldown).
    pub mower:       MowerSchedule,
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
            let pos      = entity.get::<Position>().copied().unwrap_or(Position(Vec2::ZERO));
            let vel      = entity.get::<Velocity>().copied().unwrap_or(Velocity(Vec2::ZERO));
            let cargo    = entity.get::<Cargo>().copied().unwrap_or_default();
            let vis      = entity.get::<VisualState>().copied().unwrap_or_default();
            let hp       = entity.get::<Health>().copied().unwrap_or(
                Health { hp: 1.0, max_hp: 1.0 });
            let brain    = entity.get::<WorkerBrain>().copied();
            let queen    = entity.get::<QueenState>().copied();
            let attacker = entity.get::<Attacker>().copied();
            let soldier  = entity.get::<SoldierAi>().copied();
            ants.push(AntSnap { pos, vel, cargo, vis, hp, ant: *ant,
                                brain, queen, attacker, soldier });
        } else if let Some(spider) = entity.get::<Spider>() {
            let pos      = entity.get::<Position>().copied().unwrap_or(Position(Vec2::ZERO));
            let vel      = entity.get::<Velocity>().copied().unwrap_or(Velocity(Vec2::ZERO));
            let vis      = entity.get::<VisualState>().copied().unwrap_or_default();
            let hp       = entity.get::<Health>().copied().unwrap_or(
                Health { hp: 1.0, max_hp: 1.0 });
            let attacker = entity.get::<Attacker>().copied();
            hostiles.push(HostileSnap { pos, vel, vis, hp,
                                         kind: HostileKind::Spider(*spider), attacker });
        } else if let Some(rival) = entity.get::<RivalAnt>() {
            let pos      = entity.get::<Position>().copied().unwrap_or(Position(Vec2::ZERO));
            let vel      = entity.get::<Velocity>().copied().unwrap_or(Velocity(Vec2::ZERO));
            let vis      = entity.get::<VisualState>().copied().unwrap_or_default();
            let hp       = entity.get::<Health>().copied().unwrap_or(
                Health { hp: 1.0, max_hp: 1.0 });
            let attacker = entity.get::<Attacker>().copied();
            hostiles.push(HostileSnap { pos, vel, vis, hp,
                                         kind: HostileKind::Rival(*rival), attacker });
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
        mower:       *world.resource::<MowerSchedule>(),
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
    *world.resource_mut::<Population>()    = snap.population;
    *world.resource_mut::<MowerSchedule>() = snap.mower;
    world.resource_mut::<DigJobs>().restore(&snap.dig_jobs);

    // Re-spawn ants. Dispatch on the snapshotted `Ant.kind` so a
    // soldier is restored as a soldier (with SoldierAi + Attacker), a
    // worker as a worker (with WorkerBrain + Attacker), and the queen
    // as a queen (with QueenState, no Attacker). This matches how
    // each is spawned in `app.rs` and `brood.rs`. The previous
    // implementation dispatched on `(brain, queen)` Options, which
    // mapped soldiers to a default-brained worker — wiping their AI
    // and turning them into idle entities that couldn't fight.
    for a in &snap.ants {
        let attacker = a.attacker.unwrap_or_else(|| match a.ant.kind {
            AntKind::Worker  => Attacker::new(2.2, 1.4, 0.7),
            AntKind::Soldier => Attacker::new(4.0, 1.6, 0.8),
            AntKind::Queen   => Attacker::new(0.0, 0.0, 1.0),
        });
        match a.ant.kind {
            AntKind::Worker => {
                let brain = a.brain.unwrap_or_default();
                world.spawn((
                    a.pos, a.vel, a.cargo, a.vis, a.hp,
                    FactionTag(Faction::Colony),
                    a.ant, brain, attacker,
                ));
            }
            AntKind::Soldier => {
                let soldier_ai = a.soldier.unwrap_or_default();
                world.spawn((
                    a.pos, a.vel, a.cargo, a.vis, a.hp,
                    FactionTag(Faction::Colony),
                    a.ant, soldier_ai, attacker,
                ));
            }
            AntKind::Queen => {
                let queen = a.queen.unwrap_or_default();
                world.spawn((
                    a.pos, a.vel, a.cargo, a.vis, a.hp,
                    FactionTag(Faction::Colony),
                    a.ant, queen,
                ));
            }
        }
    }
    // Re-spawn decorations
    for d in &snap.decors {
        world.spawn((d.pos, d.decor));
    }
    // Re-spawn hostiles. As with ants, restore the `Attacker`
    // component too, otherwise the entity drops out of the combat
    // query and becomes unkillable.
    for h in &snap.hostiles {
        let attacker = h.attacker.unwrap_or_else(|| match h.kind {
            HostileKind::Spider(_) => Attacker::new(3.0, 1.5, 1.2),
            HostileKind::Rival(_)  => Attacker::new(2.0, 1.3, 0.7),
        });
        match h.kind {
            HostileKind::Spider(s) => {
                world.spawn((
                    h.pos, h.vel, h.vis, h.hp,
                    FactionTag(Faction::Predator),
                    s, attacker,
                ));
            }
            HostileKind::Rival(r) => {
                world.spawn((
                    h.pos, h.vel, h.vis, h.hp,
                    FactionTag(Faction::Rival),
                    r, attacker,
                ));
            }
        }
    }
}
