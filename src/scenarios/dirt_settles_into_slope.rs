//! A vertical column of soil tiles dropped above ground (without
//! anything supporting it laterally) should slump into a wider, lower
//! pile via the falling-sand physics. The pile's footprint must
//! spread to multiple columns — that's what gives drops a triangular
//! mound rather than a tower.

use bevy_ecs::prelude::*;
use crate::scenarios::{Scenario, ScenarioDef};
use crate::config::*;
use crate::world::{TileGrid, TileType};

pub fn def() -> ScenarioDef {
    ScenarioDef {
        name: "dirt_settles_into_slope",
        description: "A 6-tall column of soil dropped above ground spreads into >= 3 columns of pile.",
        seed: 222,
        timeout_seconds: 6.0,
        setup,
        predicate,
        failure_hint: "settle_above_ground isn't sliding tiles diagonally — sand-physics regression",
    }
}

fn setup(s: &mut Scenario) {
    s.clear_creatures();
    s.reveal_all();

    // Stack 6 soil tiles in a single column well clear of the entrance
    // corridor. Below them: empty air, then grass at SURFACE_ROW. The
    // bottom tile will settle onto grass; subsequent ones will pile
    // and the higher tiles will slide diagonally off the shoulder.
    let cx = COLONY_X + 18;
    {
        let mut g = s.app.world.resource_mut::<TileGrid>();
        for dy in 0..6 {
            // Stack starts a few rows up so we can watch them fall.
            g.set(cx, SURFACE_ROW - 3 - dy, TileType::Soil);
        }
    }
    s.mark_dirty();
}

fn predicate(world: &World) -> bool {
    let g = world.resource::<TileGrid>();
    // Count how many distinct columns near the drop site contain a
    // solid soil tile above ground (y < SURFACE_ROW). Triangle pile
    // formation needs at least 3 columns to be a slope, not a tower.
    let centre_x = COLONY_X + 18;
    let mut filled_columns = 0;
    for dx in -4..=4 {
        let x = centre_x + dx;
        let mut has = false;
        for y in 1..SURFACE_ROW {
            if g.get(x, y).solid() { has = true; break; }
        }
        if has { filled_columns += 1; }
    }
    filled_columns >= 3
}
