//! The lawn mower walking across a hand-built pile must shrink it.
//! Pins both halves of the design:
//!   1. The mower walks the surface and rolls over piles (no zone
//!      check — it cuts whatever its wheels are on).
//!   2. The blade only shaves *above-ground* dirt, never the natural
//!      grass row.
//!
//! Setup: build a 5-tall soil column to the left of where we drop the
//! mower, point the mower toward it, run a few seconds. Predicate
//! checks that the pile is shorter than its starting height AND that
//! the natural grass row is still solid.

use bevy_ecs::prelude::*;
use glam::Vec2;
use crate::scenarios::{Scenario, ScenarioDef};
use crate::config::*;
use crate::world::{TileGrid, TileType};
use crate::sim::scenery::{Decoration, DecorKind, DecorPos};

const PILE_X:        i32 = 200;
const PILE_HEIGHT:   i32 = 5;
const PILE_TOP_ROW:  i32 = SURFACE_ROW - PILE_HEIGHT;

pub fn def() -> ScenarioDef {
    ScenarioDef {
        name: "mower_shaves_piles",
        description: "The lawn mower rolling over a 5-tall pile shrinks it without carving grass.",
        seed: 731,
        // The mower moves at 1 tile/s. Place it ~6 tiles from the
        // pile so it's on top of it within a couple seconds; budget
        // 18s so it has time to roll fully over and clip multiple
        // tiles even with the per-column shave roll.
        timeout_seconds: 18.0,
        setup,
        predicate,
        failure_hint: "mower isn't shaving above-ground dirt — animate_scenery / blade hookup broken",
    }
}

fn setup(s: &mut Scenario) {
    s.clear_creatures();
    s.reveal_all();

    // Hand-built pile.
    {
        let mut g = s.app.world.resource_mut::<TileGrid>();
        for y in PILE_TOP_ROW..SURFACE_ROW {
            g.set(PILE_X, y, TileType::Soil);
        }
        g.dirty = true;
    }

    // Drop the mower a handful of tiles to the right of the pile,
    // moving leftward. By spawning it close, we don't have to wait
    // for a full traversal of the world.
    s.app.world.spawn((
        DecorPos { x: (PILE_X + 6) as f32, y: (SURFACE_ROW - 2) as f32 },
        Decoration { kind: DecorKind::Mower, frame: 0, anim_t: 0.0, vx: -1.0,
                     flip_x: true, last_col: -1 },
    ));
    let _ = Vec2::ZERO; // keep glam import live in case the test grows
}

fn predicate(world: &World) -> bool {
    let g = world.resource::<TileGrid>();
    // Pile must have shrunk (at least one of the original soil tiles
    // turned to Air) AND the natural grass row at PILE_X must still
    // be solid (mower doesn't carve into untouched ground).
    let any_above_pile_cleared = (PILE_TOP_ROW..SURFACE_ROW)
        .any(|y| g.get(PILE_X, y) == TileType::Air);
    let grass_row_intact = matches!(
        g.get(PILE_X, SURFACE_ROW),
        TileType::Grass
    );
    any_above_pile_cleared && grass_row_intact
}
