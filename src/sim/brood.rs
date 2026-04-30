//! Brood maturation: when a brood entity's timer hits zero it hatches into
//! either a worker or a soldier ant. The brood entity itself is despawned.

use bevy_ecs::prelude::*;
use glam::Vec2;
use super::components::*;
use super::{Time, EventLog};

pub fn mature_brood(
    time: Res<Time>,
    mut log: ResMut<EventLog>,
    mut q: Query<(Entity, &Position, &mut Brood)>,
    mut commands: Commands,
) {
    if time.dt <= 0.0 { return; }
    for (e, pos, mut brood) in q.iter_mut() {
        brood.timer -= time.dt;
        if brood.timer > 0.0 { continue; }
        commands.entity(e).despawn();
        if brood.will_be_soldier {
            commands.spawn((
                Position(pos.0),
                Velocity(Vec2::ZERO),
                Health { hp: 35.0, max_hp: 35.0 },
                FactionTag(Faction::Colony),
                Ant { kind: AntKind::Soldier },
                Cargo::default(),
                Attacker::new(4.0, 1.6, 0.8),
                SoldierAi::default(),
                VisualState::default(),
            ));
            log.push("A new soldier hatched!", [1.0, 0.78, 0.36, 1.0]);
        } else {
            commands.spawn((
                Position(pos.0),
                Velocity(Vec2::ZERO),
                Health { hp: 14.0, max_hp: 14.0 },
                FactionTag(Faction::Colony),
                Ant { kind: AntKind::Worker },
                Cargo::default(),
                Attacker::new(1.5, 1.4, 0.9),
                WorkerBrain::default(),
                VisualState::default(),
            ));
        }
    }
}
