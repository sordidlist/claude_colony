//! Combat: every frame, every Attacker tries to hit the nearest enemy
//! within range. Damage is applied to the target's Health; when a target
//! drops to ≤ 0, it dies, drops a corpse (a long-lived high-value Food
//! pellet), and burns alarm pheromone at its tile so nearby colony ants
//! switch into FightBack mode.

use bevy_ecs::prelude::*;
use glam::Vec2;
use crate::config::*;
use crate::world::{PheromoneGrid, PheromoneChannel};
use super::components::*;
use super::{Time, EventLog};

pub fn combat_step(
    time: Res<Time>,
    mut phero: ResMut<PheromoneGrid>,
    mut log:   ResMut<EventLog>,
    mut commands: Commands,
    mut combatants: Query<(Entity, &Position, &FactionTag, &mut Attacker, &mut Health), Without<Brood>>,
) {
    let dt = time.dt;
    if dt <= 0.0 { return; }

    // Snapshot pass — read every combatant's relevant state into a Vec.
    // We'll do all decisions off the snapshot, then apply mutations in one
    // pass at the end. Avoids the two-query Health conflict.
    #[derive(Copy, Clone)]
    struct Combatant {
        e:        Entity,
        pos:      Vec2,
        fac:      Faction,
        hp:       f32,
        damage:   f32,
        range:    f32,
        cooldown: f32,
        timer:    f32,
    }
    let snap: Vec<Combatant> = combatants.iter()
        .map(|(e, p, f, a, h)| Combatant {
            e, pos: p.0, fac: f.0, hp: h.hp,
            damage: a.damage, range: a.range, cooldown: a.cooldown, timer: a.timer,
        })
        .collect();

    // Per-entity new timer + accumulated incoming damage.
    let mut new_timers: std::collections::HashMap<Entity, f32> =
        std::collections::HashMap::with_capacity(snap.len());
    let mut damage_in: std::collections::HashMap<Entity, f32> =
        std::collections::HashMap::with_capacity(snap.len());

    for atk in &snap {
        let next_timer = (atk.timer - dt).max(0.0);
        if next_timer > 0.0 {
            new_timers.insert(atk.e, next_timer);
            continue;
        }
        // Pick the nearest enemy in range
        let r2 = atk.range * atk.range;
        let mut best: Option<(Entity, f32)> = None;
        for other in &snap {
            if other.e == atk.e { continue; }
            if !is_enemy(atk.fac, other.fac) { continue; }
            if other.hp <= 0.0 { continue; }
            let dx = other.pos.x - atk.pos.x;
            let dy = other.pos.y - atk.pos.y;
            let d2 = dx*dx + dy*dy;
            if d2 > r2 { continue; }
            if best.map_or(true, |(_, b)| d2 < b) { best = Some((other.e, d2)); }
        }
        if let Some((target, _)) = best {
            new_timers.insert(atk.e, atk.cooldown);
            *damage_in.entry(target).or_insert(0.0) += atk.damage;
            // Attackers from hostile factions stir up alarm pheromone.
            if matches!(atk.fac, Faction::Predator | Faction::Rival) {
                phero.deposit(atk.pos.x as i32, atk.pos.y as i32,
                              PheromoneChannel::Alarm, ALARM_PHEROMONE_BURST * 0.4);
            }
        } else {
            new_timers.insert(atk.e, 0.0);
        }
    }

    // Apply: timer resets, hp deltas, and gather kills in one mutable pass.
    let mut killed: Vec<(Entity, Vec2, Faction)> = Vec::new();
    for (e, p, f, mut atk, mut h) in combatants.iter_mut() {
        if let Some(&t) = new_timers.get(&e) {
            atk.timer = t;
        } else {
            atk.timer = (atk.timer - dt).max(0.0);
        }
        if let Some(&dmg) = damage_in.get(&e) {
            h.hp -= dmg;
            if h.hp <= 0.0 {
                killed.push((e, p.0, f.0));
            }
        }
    }

    // Process kills: alarm pheromone + corpse spawn + log + despawn.
    for (e, p, fac) in killed {
        commands.entity(e).despawn();
        // Loud alarm at the death site.
        phero.deposit(p.x as i32, p.y as i32,
                      PheromoneChannel::Alarm, ALARM_PHEROMONE_BURST);
        // Corpse: long-lived food pellet on the ground.
        let value: u8 = match fac {
            Faction::Predator => 8,
            Faction::Rival    => 3,
            Faction::Colony   => 3,
        };
        commands.spawn((
            Position(p),
            Velocity(Vec2::ZERO),
            Food { value },
            Corpse { decay: CORPSE_DECAY_S },
        ));
        match fac {
            Faction::Colony => {
                log.push("A colony ant fell in battle!", [1.0, 0.36, 0.20, 1.0]);
            }
            Faction::Rival => {
                log.push("A rival ant was slain", [0.96, 0.66, 0.36, 1.0]);
            }
            Faction::Predator => {
                log.push("A spider was killed!", [0.86, 0.50, 0.96, 1.0]);
            }
        }
    }
}

#[inline]
fn is_enemy(a: Faction, b: Faction) -> bool {
    match (a, b) {
        (Faction::Colony,   Faction::Colony)   => false,
        (Faction::Predator, Faction::Predator) => false,
        (Faction::Rival,    Faction::Rival)    => false,
        _ => true,
    }
}

/// Decay corpses on a separate cadence so they don't accumulate forever.
pub fn corpse_decay(
    time: Res<Time>,
    mut q: Query<(Entity, &mut Corpse)>,
    mut commands: Commands,
) {
    if time.dt <= 0.0 { return; }
    for (e, mut c) in q.iter_mut() {
        c.decay -= time.dt;
        if c.decay <= 0.0 {
            commands.entity(e).despawn();
        }
    }
}
