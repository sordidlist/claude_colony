//! ECS components. Kept small and `Copy` where possible so archetype storage
//! stays cache-friendly.

use bevy_ecs::prelude::{Component, Entity};
use glam::Vec2;
use std::collections::VecDeque;
use crate::world::{DigClaim, TileType};

#[derive(Component, Copy, Clone, Debug)]
pub struct Position(pub Vec2);

#[derive(Component, Copy, Clone, Debug)]
pub struct Velocity(pub Vec2);

#[derive(Component, Copy, Clone, Debug)]
pub struct Health {
    pub hp:     f32,
    pub max_hp: f32,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Faction { Colony, Predator, Rival }

#[derive(Component, Copy, Clone, Debug)]
pub struct FactionTag(pub Faction);

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum AntKind { Worker, Soldier, Queen }

#[derive(Component, Copy, Clone, Debug)]
pub struct Ant {
    pub kind: AntKind,
}

#[derive(Component, Copy, Clone, Debug, Default)]
pub struct Cargo {
    /// Food units carried (deposited at colony).
    pub amount: u8,
    /// Pebble of dug material being hauled to a dump site. Some ⇒ ant is
    /// loaded with debris and walking it out of the colony to drop.
    pub debris: Option<TileType>,
}

/// Worker AI mode. The worker switches between these on its replan tick.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum WorkerMode {
    Wander,
    SeekFood,
    ReturnHome,
    Dig,
    DepositDebris,
    FightBack,
}

#[derive(Component, Copy, Clone, Debug)]
pub struct WorkerBrain {
    pub mode:           WorkerMode,
    pub replan_in:      f32,
    pub dig_claim:      Option<DigClaim>,
    pub dig_target:     Option<(i32, i32)>,
    pub dig_phase:      DigPhase,
    /// While hauling dirt out of the colony, which way are we walking?
    /// 0 = unset (pick on next tick), -1 = left, +1 = right. Set on
    /// surfacing and cleared on drop.
    pub haul_direction: i8,
    /// How many tiles from the entrance this ant intends to walk before
    /// dropping. Picked at random when hauling starts; varying it across
    /// the population is what spreads drops out into a real mound shape
    /// rather than concentrating them on the first column past the corridor.
    pub haul_target_dist: i8,
    /// Wall-clock seconds the ant has been carrying dirt without making
    /// horizontal progress. Used so a stuck hauler tries the other
    /// direction (and ultimately drops *somewhere*) rather than walking
    /// into a wall forever.
    pub haul_stuck_time: f32,
    /// Last x position checked for progress, used to detect "stuck."
    pub haul_last_x:     i16,
    /// Number of direction-flip attempts the hauler has made on the
    /// current cycle. Each flip happens after ~1.5s of no x progress
    /// (see `step_deposit_debris`). Above `MAX_HAUL_ATTEMPTS` we
    /// stop oscillating and just drop the pebble at the worker's
    /// feet — otherwise tall mounds gridlock haulers permanently
    /// above ground and the colony stops functioning.
    pub haul_attempts:   u8,
    pub attack_target:  Option<Entity>,
    /// Number of completed dig → haul → deposit cycles this worker has
    /// finished, incremented by `step_deposit_debris` each time it
    /// actually drops the pebble. Saturates at u16::MAX. Used by
    /// scenario tests as an exact, per-worker cycle counter — no need
    /// to infer from world-state proxies that the mower or sand-
    /// physics could perturb.
    pub cycles_completed: u16,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum DigPhase { Approach, Mining }

impl Default for WorkerBrain {
    fn default() -> Self {
        Self {
            mode: WorkerMode::Wander,
            replan_in: 0.0,
            dig_claim: None,
            dig_target: None,
            dig_phase: DigPhase::Approach,
            haul_direction: 0,
            haul_target_dist: 0,
            haul_stuck_time: 0.0,
            haul_last_x:     0,
            haul_attempts:   0,
            attack_target: None,
            cycles_completed: 0,
        }
    }
}

/// Visible flag the renderer reads to swap the digging sprite frame.
#[derive(Component, Copy, Clone, Debug, Default)]
pub struct VisualState {
    pub digging:    bool,
    pub anim_frame: u8,
    pub anim_t:     f32,
    /// 1 = facing right (default sprite orientation), -1 = facing left.
    /// Renderer reads this to flip the sprite based on movement direction.
    pub facing:     i8,
    /// Per-agent idle pause clock — when > 0 the agent is briefly stationary,
    /// giving the population a less marching-in-lockstep feel.
    pub idle:       f32,
}

#[derive(Component, Copy, Clone, Debug)]
pub struct Food { pub value: u8 }

/// Queen-only state. Lives alongside `Ant { kind: Queen }`.
#[derive(Component, Copy, Clone, Debug, Default)]
pub struct QueenState {
    pub egg_timer: f32,
    pub eggs_laid: u32,
    /// Sim-seconds remaining before `queen_migration` re-evaluates
    /// whether a deeper chamber is now reachable. Hits zero on the
    /// next migration interval; reset to `QUEEN_MIGRATION_INTERVAL_S`
    /// after each check.
    pub migration_timer: f32,
    /// Total times this queen has relocated since founding. Surfaces
    /// in the inspector and gives a sense of how the colony has
    /// matured over a play session.
    pub migrations: u16,
}

/// Spider — predator faction. Lurks in deep tunnels, wanders, threatens.
#[derive(Component, Copy, Clone, Debug, Default)]
pub struct Spider {
    pub heading_timer: f32,
    /// When > 0, the spider has just made a kill and is "dragging
    /// its prey back to its lair" — really, walking away from the
    /// colony entrance at boosted speed for `SPIDER_RETREAT_AFTER_KILL_S`
    /// seconds. While retreating it ignores its random-walk heading
    /// and drives directly away from the colony entrance.
    pub retreat_timer: f32,
}

/// Rival ant — hostile faction. Patrols the surface near map edges.
#[derive(Component, Copy, Clone, Debug, Default)]
pub struct RivalAnt {
    pub heading_timer: f32,
    /// Worker (light, fast) vs Soldier (heavier, sturdier). Each tier
    /// gets distinct stats at spawn and a slightly different render
    /// scale so the player can read the threat at a glance.
    pub kind: RivalKind,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Default)]
pub enum RivalKind {
    #[default]
    Worker,
    Soldier,
}

/// Soldier marker. Lives alongside `Ant { kind: Soldier }`. Soldiers patrol
/// near the colony entrance and engage hostiles in melee.
#[derive(Component, Copy, Clone, Debug, Default)]
pub struct SoldierAi {
    pub patrol_target: Option<(f32, f32)>,
    pub patrol_timer:  f32,
}

/// Brood — pre-adult colony entity. Matures into a worker or soldier when
/// `timer` reaches zero.
#[derive(Component, Copy, Clone, Debug)]
pub struct Brood {
    pub timer:           f32,
    pub will_be_soldier: bool,
}

/// Combat capability — present on workers, soldiers, spiders, rivals.
#[derive(Component, Copy, Clone, Debug)]
pub struct Attacker {
    pub damage:   f32,
    pub range:    f32,
    pub cooldown: f32,
    pub timer:    f32,
}

impl Attacker {
    pub fn new(damage: f32, range: f32, cooldown: f32) -> Self {
        Self { damage, range, cooldown, timer: 0.0 }
    }
}

/// Corpse — short-lived food entity dropped on death. Decays over time so
/// the world doesn't accumulate them indefinitely.
#[derive(Component, Copy, Clone, Debug)]
pub struct Corpse {
    pub decay: f32,
}

/// Number of past decisions an `AiTrace` keeps. Newest entries push out
/// the oldest. 10 is enough to read a recent behavioural history at a
/// glance without consuming much memory.
pub const AI_TRACE_CAPACITY: usize = 10;

/// Per-creature ring buffer of recent AI decisions. Each AI system
/// (`worker_ai`, `soldier_tick`, `spider_tick`, `rival_tick`,
/// `queen_tick`) calls `record` when its agent transitions state or
/// makes a notable choice. The debug inspector reads this to show a
/// "what was this thing thinking lately" log.
///
/// Keeps `String` text instead of an enum to give each AI system free
/// rein over how it describes the event — a worker might log "claimed
/// dig at (50, 80)" while a spider logs "new heading: NE". The cost
/// is one allocation per logged event; cap-trimmed so memory bounded.
#[derive(Component, Clone, Debug, Default)]
pub struct AiTrace {
    pub entries: VecDeque<TraceEntry>,
}

#[derive(Clone, Debug)]
pub struct TraceEntry {
    /// Sim-clock seconds at the time of the event.
    pub time: f32,
    /// Free-form description; kept short so the inspector renders it
    /// on a single line.
    pub text: String,
}

impl AiTrace {
    pub fn record(&mut self, time: f32, text: impl Into<String>) {
        self.entries.push_back(TraceEntry { time, text: text.into() });
        while self.entries.len() > AI_TRACE_CAPACITY {
            self.entries.pop_front();
        }
    }
}
