# Colony

A SimAnt-inspired ant colony simulator built to put **thousands of creatures
on screen at once** in a SNES-style pixel-art world. The simulation runs on
an ECS, systems iterate component arrays in tight loops, and each render
layer collapses to a single instanced draw — so the on-screen population is
bounded by GPU bandwidth, not script overhead.

---

## Quick Start

```bash
cargo run --release                          # default seed
cargo run --release -- --seed=7              # specific seed

cargo run --release --bin scenario_viewer    # run scenario tests visually
cargo run --release --bin scenario_viewer -- --scenario=full_haul_cycle
cargo run --release --bin scenario_viewer -- --list

cargo test --release                         # full test suite (50+ tests)
```

A debug build is also runnable (`cargo run`); the `dev` profile uses
`opt-level=1` so debug builds remain interactive at several thousand agents.

**Game controls**

| Input               | Action                                    |
|---------------------|-------------------------------------------|
| `WASD` / Arrows     | Pan camera (sensitivity halved from raw)  |
| Mouse scroll        | Zoom in / out                             |
| `Space`             | Pause / resume                            |
| `]` / `[`           | Cycle fast-forward (1×, 2×, 4×, 10×, 100×) |
| Backspace (hold)    | Rewind through history buffer (~60s)      |
| Tab + hover         | Debug inspector for hovered creature      |
| Shift (held)        | Live-balance panel — adjust queen rate, invader rate, wave size, mower speed in real time |
| `Q` / `Esc`         | Quit                                      |

**Scenario viewer extras**

| Input    | Action                                    |
|----------|-------------------------------------------|
| `N` / `P`| Next / previous scenario                  |
| `R`      | Reset current scenario                    |
| `L`      | Toggle scenario list overlay              |

---

## Performance Targets

| Budget                | Target                                    |
|-----------------------|-------------------------------------------|
| Simulation tick       | ≤ 4 ms for 5,000 agents on M-class CPU    |
| Render frame          | ≤ 4 ms at 1280×800, single draw per layer |
| Steady FPS            | 60 with 5,000 agents, 120 with 1,000      |
| Memory per agent      | ≤ 256 B hot-path (position, vel, AI tag)  |

These constraints shape every system in the codebase.

### Architectural Rules

1. **ECS, not OOP.** All entities are `bevy_ecs` archetypes. No
   `Box<dyn Trait>` per-creature. Behaviour is a system that reads/writes
   components in bulk.
2. **SoA over AoS.** Positions, velocities, health, AI state live in their
   own component arrays. Iteration is cache-linear.
3. **No per-frame allocations on hot paths.** Pre-allocate work buffers in
   resources; reuse across frames.
4. **Rendering is one instanced draw per layer.** Tile layer, pheromone
   overlay, sprite layer, UI layer — each is a single draw call against the
   texture atlas. No per-entity `draw_*` calls outside of macroquad's
   internal batching.
5. **Spatial queries via grid hash, not per-pair scans.** A uniform spatial
   grid keyed by tile coordinate; ants query a 3×3 neighbourhood, not the
   full entity list.
6. **Pheromone / water / tile grids are flat `Vec<u8|f32>`** sized to the
   world, with all updates vectorised over slices.
7. **AI is data-driven, not planned per frame.** Workers run a small utility
   evaluator that re-evaluates at most once per second per agent, jittered
   across the population. Hot-path steering reads the resulting mode flag.

If a feature can't be added without violating one of these, the feature
changes — not the rule.

---

## Crate Layout

```
colony/
├── Cargo.toml
├── assets/                  packed sprite + tile atlas (PNG, generated)
├── tests/
│   ├── scenarios.rs         drives every scenario in src/scenarios/
│   ├── sim_smoke.rs         end-to-end "ants actually dig" headless guards
│   └── speed_invariance.rs  every scenario × every fast-forward level
└── src/
    ├── main.rs              window + main loop (macroquad-driven)
    ├── lib.rs               library face — sim/world/render/scenarios
    ├── app.rs               world boot + ECS schedule wiring
    ├── config.rs            tunable constants (one place to balance)
    ├── bin/
    │   └── scenario_viewer.rs
    │
    ├── world/               flat-grid state + worldgen
    │   ├── tiles.rs         TileType, flat Vec<u8> grid, dig_for_depth
    │   ├── procgen.rs       deterministic worldgen, spider warrens
    │   ├── pheromones.rs    4-channel f32 grid, decay, alarm diffusion
    │   ├── water.rs         vectorised CA placeholder
    │   ├── exploration.rs   fog-of-war ExploredGrid resource
    │   ├── dig_jobs.rs      slot-table claim queue + colony director
    │   ├── dirt_physics.rs  above-ground sand-physics (mound formation)
    │   └── flow_field.rs    BFS return-to-entrance navigation
    │
    ├── sim/                 entities, components, AI/lifecycle systems
    │   ├── components.rs    Position, Velocity, Health, Cargo, brains, ...
    │   ├── time.rs          dt + total time + Population resources
    │   ├── event_log.rs     wall-clock-aged alert ticker
    │   ├── day_night.rs     time-of-day cycle + phase events
    │   ├── history.rs       ring-buffer snapshots for rewind
    │   ├── spatial.rs       uniform grid hash, rebuilt once per tick
    │   ├── movement.rs      velocity → position + sub-stepped physics
    │   ├── exploration.rs   fog reveal system
    │   ├── ai_worker.rs     observe → score → act, mode-based steering
    │   ├── soldier.rs       patrol + chase + engage AI
    │   ├── hostiles.rs      spider/rival movement, hunting, alarm emission
    │   ├── queen.rs         egg-laying + migration to deeper chambers
    │   ├── brood.rs         egg → worker / soldier maturation
    │   ├── combat.rs        damage propagation, alarm bursts, kill handling
    │   ├── lifecycle.rs     Population accounting
    │   ├── food_spawn.rs    surface food spawner
    │   ├── foraging.rs      pickup + deposit at colony entrance
    │   └── scenery.rs       barn, dog, mower (incl. lifecycle), trees, sun
    │
    ├── render/              GPU layer; one draw per visible layer
    │   ├── atlas.rs         procedural SNES atlas baked at startup
    │   ├── camera.rs        pan + scroll zoom, world↔screen transforms
    │   ├── sky.rs           sky gradient + parallax hills
    │   ├── tilemap.rs       tile layer baked to one Texture2D
    │   ├── fog.rs           fog-of-war overlay
    │   ├── scenery.rs       barn/dog/mower/trees/cloud sprite renderer
    │   ├── sprites.rs       creature layer, instanced
    │   ├── overlays.rs      pheromone tints + dig markers
    │   ├── ui.rs            bottom stats strip + alert banners
    │   └── inspector.rs     Tab-held debug popup (any creature)
    │
    └── scenarios/           reusable test/inspection scenarios
        ├── builder.rs       Scenario harness used by tests + viewer
        └── *.rs             one file per scenario; see `registry()`
```

The split mirrors the data flow:
**`world/` owns grids · `sim/` owns entities · `render/` owns GPU**, with one
direction of dependency: `render → sim → world`. Nothing in `world/` or
`sim/` imports a renderer type, so the simulation drives headless tests
without touching the GPU.

---

## Simulation Loop

The schedule runs every system in two chained groups (see `app.rs`):

1. Day/night → spatial rebuild → pheromone decay → worker AI → tile-op flush
   → movement → fog reveal → dig director → queen / queen migration → brood maturation.
2. Soldier AI → spider/rival movement → hostile alarm emission →
   alarm diffusion → combat → corpse decay → food spawn → foraging
   → population stats → scenery animation → mower lifecycle →
   above-ground dirt physics → flow-field rebuild → milestone events.

Goal re-evaluation is **not** in the per-frame path. Workers carry a small
utility-AI state and only re-evaluate goals on a 1 Hz timer (jittered per
agent to avoid herd spikes). Dig jobs are claimed via a slot table that
rejects stale claims by generation counter, so a worker that abandons a job
mid-progress can never deadlock the queue — claims auto-expire after a TTL.

### Hostile detection (worker side)

Workers detect spiders three ways, in priority order:

1. **Direct sight** — any hostile within `WORKER_THREAT_RADIUS` flips the
   worker into `FightBack` with a locked attack target. Drops cargo and
   dig claim. Closest reflex.
2. **Alarm pheromone gradient** — `hostile_alarm_emission` stamps a 9×9
   square of alarm around each spider, and `diffuse_alarm_system` mixes
   that field outward through 4-connected passable tiles each
   `ALARM_DIFFUSE_INTERVAL`, so the gradient propagates through the
   tunnel network to workers far from the spider.
3. **Combat fallout** — every successful spider/rival attack stamps an
   extra alarm burst at the attacker's tile.

### Spider hunting

Spiders (and rivals) actively hunt: each frame `spider_tick` scans for
the nearest colony ant within `SPIDER_HUNT_RADIUS` and steers directly at
it. After a kill, the spider enters a brief retreat state — driving away
from the entrance at boosted speed — to model "dragging the prey back to
its lair." No corpse is left behind for the colony to scavenge from a
predator kill.

### Dig debris

When a worker finishes mining a tile, the tile becomes `Tunnel` **and the
worker picks up a pebble** of the original material as cargo. The worker
walks to the surface, picks an outward direction + random distance, and
drops the pebble onto an air tile outside the entrance corridor. Multiple
drops accumulate into a real ant-hill above ground — the world is conserved
end-to-end. After `MAX_HAUL_ATTEMPTS` failed direction-flips a worker
force-drops in place rather than gridlocking on tall mounds.

### Lawn mower

A lawn mower scenery agent drives across the surface every few minutes,
running over piled dirt and any unlucky workers in its path. The blade
shaves above-ground tiles in each column it crosses; the wheels kill any
creature inside `MOWER_KILL_RADIUS`. After `MOWER_LAPS_PER_VISIT` traversals
it retires for `MOWER_COOLDOWN_SECONDS` before returning. State is owned by
the `MowerSchedule` resource and rides through history snapshots.

### Queen migration

The queen lays an egg every `QUEEN_EGG_INTERVAL_S`. Every
`QUEEN_MIGRATION_INTERVAL_S` she runs the same flood-fill that placed her
at startup; if the deepest reachable spot has dropped by at least
`QUEEN_MIGRATION_MIN_DEPTH_GAIN` rows, she relocates there. Workers dig
deeper → queen retreats further from the surface as the colony matures.

### Day/night cycle

`TimeOfDay` advances with sim dt and exposes a smooth cosine `night_factor`
(0 = noon, 1 = midnight). The renderer multiplies tile and sprite tints by
the corresponding day-tint colour, and the sky background interpolates
between a clear blue and deep navy. Crossings push events to the ticker:
"Night falls…", "Dawn breaks", and "Day N begins" on each rollover.

### Alert ticker

The `EventLog` resource holds a small ring of recent alerts. The log is
aged by **wall-clock dt**, not sim dt — so messages stay visible the same
amount of real time whether the sim is paused, running normally, or fast
forwarded.

### Debug inspector

Hold `Tab` and hover any creature in the live game (or the scenario
viewer): the inspector shows position / velocity / HP bar, cargo, AI mode
plus its current target, attacker stats, and the last 10 entries of the
creature's `AiTrace` ring buffer. Each AI system records a one-line note
on every notable transition (mode change, heading reset, egg laid,
"hunting", post-kill retreat, etc.) so the trace reads as a recent decision
log. Works on workers, soldiers, queens, spiders, rivals, brood, and the
mower.

---

## Art Pipeline

The renderer expects exactly one texture atlas, generated at startup by
`render::atlas` from procedural pixel data so we don't ship binary art in
the repo until the style is locked. SNES constraints are enforced in code:

- 8×8 tile grid for terrain, 8×8 sprites for creatures.
- Per-sprite palette of ≤ 16 entries from a fixed master palette.
- Dithered shading only (Bayer 4×4), no gradients.
- One palette swap per time-of-day phase (warm noon → cool dusk → cold
  night) applied as a tint at draw time.

When real artwork lands, it replaces the generator output with the same
atlas layout — no engine changes needed.

---

## Testing

```bash
cargo test --release
```

Three test targets, run together by `cargo test`:

* **`tests/sim_smoke.rs`** — end-to-end headless guards. Asserts the spawn
  cohort populates correctly, the queen lands somewhere reachable from the
  entrance for several seeds, dig jobs progress over a 60-second sim
  window, the queue doesn't saturate with stale claims, and **at least one
  spider spawns into the warrens on each fresh seed** (this last test was
  added to catch a silent regression where procgen left no deep passable
  tiles and predators never appeared in the live game).

* **`tests/scenarios.rs`** — runs every scenario in `src/scenarios/`
  through `ScenarioDef::run_headless`. Each scenario is a hand-built
  mini-world plus a goal predicate — escape a chamber, drop a pebble
  outside the corridor, swarm a spider, complete ten haul cycles in a row,
  spider hunts a nearby ant, alarm pheromone diffuses through tunnels,
  queen migrates to a deeper chamber, mower kills a worker, etc. The same
  `ScenarioDef` registry feeds the `scenario_viewer` binary so any test
  can be opened and watched at 1× wall-clock when its setup needs visual
  inspection.

* **`tests/speed_invariance.rs`** — runs a representative subset of
  scenarios at every advertised fast-forward level (1×, 2×, 4×, 10×,
  100×). Catches behaviour that only breaks under stretched-`dt`
  multi-pass stepping (sub-step movement integration, RNG seeded from
  `time.total`, etc.). Includes a coverage check that fails if
  `FF_LEVELS` in `config.rs` ever grows without somebody adding the
  matching tests.

The full suite finishes in well under 10 seconds in release mode.
