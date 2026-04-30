//! A spider in a chamber surrounded by workers should die within a
//! reasonable time. Pins the combat balance: workers are individually
//! weaker than spiders (the user wants spiders to feel formidable), but
//! a numerical advantage backed by alarm-pheromone swarming should
//! consistently win. If this fails, the colony can't defend itself
//! against deep-tunnel predators.

use crate::scenarios::Scenario;
use colony::config::*;
use colony::world::TileType;
use colony::sim::components::*;
use glam::Vec2;

#[test]
fn worker_swarm_takes_down_a_spider() {
    let mut s = Scenario::new(91);
    s.clear_creatures();

    // 16x4 underground chamber a few rows below the surface.
    let cx = COLONY_X;
    let cy = COLONY_Y + 8;
    s.fill_rect(cx - 8, cy, cx + 8, cy + 3, TileType::Tunnel)
     .carve_vertical(COLONY_X, COLONY_Y, cy, 1)
     .mark_dirty()
     .rebuild_flow_field()
     .reveal_all();

    // Spawn a spider in the centre of the chamber.
    let spider_e = s.app.world.spawn((
        Position(Vec2::new(cx as f32 + 0.5, (cy + 2) as f32 + 0.5)),
        Velocity(Vec2::ZERO),
        Health { hp: 22.0, max_hp: 22.0 },
        FactionTag(Faction::Predator),
        Spider::default(),
        Attacker::new(3.0, 1.5, 1.2),
        VisualState::default(),
    )).id();

    // Six workers placed around the spider — enough numerical advantage
    // to win, even though each one is weaker individually.
    for dx in &[-3i32, -2, -1, 1, 2, 3] {
        s.spawn_worker(cx + dx, cy + 2, None);
    }

    let elapsed = s.run_until(20.0, |world| {
        // Goal: spider is despawned (killed) before 20s
        world.get::<Position>(spider_e).is_none()
    });

    match elapsed {
        Ok(t)  => println!("worker_swarm_takes_down_a_spider: spider down in {:.2}s", t),
        Err(t) => panic!("six workers couldn't kill one spider in {:.1}s — \
                          combat balance broken or workers don't engage", t),
    }
}
