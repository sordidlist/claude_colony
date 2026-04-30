# Colony

A SimAnt-inspired ant colony simulator built to put **thousands of creatures
on screen at once** in a SNES-style pixel-art world. The simulation runs on
an ECS, systems iterate component arrays in tight loops, and each render
layer collapses to a single instanced draw — so the on-screen population is
bounded by GPU bandwidth, not script overhead.

---

## Quick Start

```bash
cargo run --release          # default seed
cargo run --release -- --seed=7
```

A debug build is also runnable (`cargo run`); the `dev` profile uses
`opt-level=1` so debug builds remain interactive at several thousand agents.

**Controls**

| Input               | Action            |
|---------------------|-------------------|
| `WASD` / Arrows     | Pan camera        |
| Mouse scroll        | Zoom in / out     |
| `Space`             | Pause / resume    |
| `Q` / `Esc`         | Quit              |

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
   world, with all updates vectorised over slices. SIMD where the math
   allows.
7. **AI is data-driven, not planned per frame.** Workers run a small utility
   evaluator that reads the same observation buffer the renderer uses. Goal
   re-evaluation runs at most once per second per agent, staggered across
   the population.

If a feature can't be added without violating one of these, the feature
changes — not the rule.

---

## Crate Layout

```
colony/
├── Cargo.toml
├── assets/                  packed sprite + tile atlas (PNG, generated)
└── src/
    ├── main.rs              window + main loop (macroquad, render-only)
    ├── lib.rs               library face — sim/world reachable from tests
    ├── app.rs               schedule wiring, milestone events
    ├── config.rs            tunables: world size, AI rates, dig timings
    │
    ├── world/
    │   ├── tiles.rs         TileType, flat Vec<u8> grid, dig() API
    │   ├── procgen.rs       deterministic worldgen (seeded)
    │   ├── pheromones.rs    4-channel f32 grid, SIMD-friendly decay
    │   ├── water.rs         vectorised CA simulation
    │   └── dig_jobs.rs      slot-table claim queue + colony director
    │
    ├── sim/
    │   ├── components.rs    Position, Velocity, Health, Cargo, WorkerBrain
    │   ├── spatial.rs       uniform grid hash, rebuilt once per tick
    │   ├── movement.rs      velocity → position + gravity + tile collision
    │   ├── ai_worker.rs     observe → score → act (utility AI, hot path)
    │   ├── ai_predator.rs   spider / rival FSM
    │   ├── combat.rs        damage propagation, alarm pheromone deposit
    │   ├── lifecycle.rs     queen / brood / corpse handling, population stats
    │   ├── time.rs          dt + total time resources
    │   ├── day_night.rs     time-of-day cycle + dawn/dusk events
    │   └── event_log.rs     wall-clock alert ticker entries
    │
    └── render/
        ├── atlas.rs         procedural SNES atlas baked at startup
        ├── camera.rs        pan + scroll zoom, world↔screen transforms
        ├── tilemap.rs       tile layer baked to one Texture2D
        ├── sprites.rs       creature layer, instanced
        ├── overlays.rs      pheromone tints + dig markers
        └── ui.rs            bottom stats strip + alert ticker
```

The split mirrors the data flow:
**`world/` owns grids · `sim/` owns entities · `render/` owns GPU**, with one
direction of dependency: `render → sim → world`. Nothing in `world/` or
`sim/` imports a renderer type, so the simulation drives headless tests
without touching the GPU.

---

## Simulation Loop

Per frame:

1. **Input** — camera, pause, speed.
2. **Day/night** — advance time of day; push dawn/dusk/day-rollover events.
3. **Spatial rebuild** — clear and re-insert all positions into the grid hash.
4. **Pheromone decay** — single slice-wide multiply across all four channels.
5. **Worker AI** — utility evaluation + per-mode steering. Stages tile
   mutations and dig events in thread-local buffers.
6. **Tile-op flush** — single-writer system applies staged digs and pebble
   drops to the tile grid; pushes throttled milestone alerts.
7. **Movement** — integrate velocity, apply gravity when not surface-clinging,
   slide against tile collisions.
8. **Director** — frontier scan over the tile grid; queues new dig jobs.
9. **Predator AI / combat / lifecycle** — population accounting, future
   predator FSM and brood promotion.
10. **Render** — sky → tile layer (day/night-tinted) → overlays → sprite
    layer → UI panel + alert ticker.

Goal re-evaluation is **not** in the per-frame path. Workers carry a small
utility-AI state and only re-evaluate goals on a 1 Hz timer (jittered per
agent to avoid herd spikes). Dig jobs are claimed via a slot table that
rejects stale claims by generation counter, so a worker that abandons a job
mid-progress can never deadlock the queue — claims auto-expire after a TTL.

### Dig debris

When a worker finishes mining a tile, the tile becomes `Tunnel` **and the
worker picks up a pebble** of the original material as cargo. The worker
then switches to `DepositDebris` mode, walks to a random spot near the
surface entrance, and drops the pebble onto an air tile that has a solid
neighbour. Multiple drops accumulate into a real ant-hill above ground —
the world is conserved end-to-end, dirt doesn't vanish into the void.

### Day/night cycle

`TimeOfDay` advances with sim dt and exposes a smooth cosine `night_factor`
(0 = noon, 1 = midnight). The renderer multiplies tile and sprite tints by
the corresponding day-tint colour, and the sky background interpolates
between a clear blue and deep navy. Crossings push events to the ticker:
"Night falls…", "Dawn breaks", and "Day N begins" on each rollover.

### Alert ticker

The `EventLog` resource holds up to eight recent alerts. Crucially, the log
is aged by **wall-clock dt**, not sim dt — so messages stay visible the same
amount of real time whether the sim is paused, running normally, or fast
forwarded later. Events are pushed by:

- the day/night system (dawn/dusk/new day),
- the dig-flush system (every 25 tiles dug),
- the milestone system (every 100 workers gained or lost, every batch of
  newly surveyed dig jobs),
- the startup banner ("Colony founded — seed N").

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

`tests/sim_smoke.rs` drives `App` headless (no window, no GPU) and asserts
that:

- the initial spawn cohort populates correctly,
- ants actually complete dig jobs across a 60-second simulated run,
- the dig-job queue does not saturate with stuck claims (the slot-table
  invariant holds end-to-end).

These are guard rails against regressions in the dig pipeline, which is the
most-likely place for behavioural bugs to creep in unnoticed.
