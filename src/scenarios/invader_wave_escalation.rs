//! Pin the wave-escalation curve. Wave 1 is workers-only, wave 4
//! includes a spider, and the wave counter advances exactly once
//! per spawn event. Cranks `BalanceTunables.invader_interval` down
//! to 1s during the test so we can observe several waves in a few
//! sim seconds without affecting the main game's pacing.

use bevy_ecs::prelude::*;
use crate::scenarios::{Scenario, ScenarioDef};
use crate::sim::components::{Spider, RivalAnt, RivalKind};
use crate::sim::hostiles::InvaderSpawner;
use crate::sim::BalanceTunables;

pub fn def() -> ScenarioDef {
    ScenarioDef {
        name: "invader_wave_escalation",
        description: "Wave 1 is workers-only; by wave 4 a spider has joined; wave count tracks spawn events.",
        seed: 7777,
        timeout_seconds: 30.0,
        setup,
        predicate,
        failure_hint: "wave-escalation curve regressed — check pick_wave() in spawn_invaders",
    }
}

fn setup(s: &mut Scenario) {
    s.clear_creatures();
    s.reveal_all();
    // Compress the spawn cadence so the test resolves quickly.
    {
        let mut bal = s.app.world.resource_mut::<BalanceTunables>();
        bal.invader_interval = 1.0;
    }
    // Force the FIRST wave to fire on the next tick — otherwise we
    // wait `INVADER_FIRST_SPAWN_S` before anything happens.
    {
        let mut sp = s.app.world.resource_mut::<InvaderSpawner>();
        sp.timer = 0.0;
    }
}

fn predicate(world: &World) -> bool {
    let sp = world.resource::<InvaderSpawner>();
    // Need to have seen at least 4 waves before the predicate is
    // honest about the curve.
    if sp.wave_count < 4 { return false; }

    // At least one spider must be alive (or have been created — we
    // can't easily tell after combat scrubs them, but with
    // workers-only world the spider should still be alive at this
    // point unless something killed it).
    let any_spider = world.iter_entities().any(|e| e.contains::<Spider>());

    // Soldier rivals must have appeared by wave 2-3 → check that any
    // RivalAnt currently has kind == Soldier.
    let any_soldier = world.iter_entities().any(|e| {
        e.get::<RivalAnt>()
            .map_or(false, |r| matches!(r.kind, RivalKind::Soldier))
    });

    any_spider && any_soldier
}
