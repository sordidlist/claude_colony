//! ECS components. Kept small and `Copy` where possible so archetype storage
//! stays cache-friendly.

use bevy_ecs::prelude::{Component, Entity};
use glam::Vec2;
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
    pub attack_target:  Option<Entity>,
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
            attack_target: None,
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
}

/// Spider — predator faction. Lurks in deep tunnels, wanders, threatens.
#[derive(Component, Copy, Clone, Debug, Default)]
pub struct Spider {
    pub heading_timer: f32,
}

/// Rival ant — hostile faction. Patrols the surface near map edges.
#[derive(Component, Copy, Clone, Debug, Default)]
pub struct RivalAnt {
    pub heading_timer: f32,
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
