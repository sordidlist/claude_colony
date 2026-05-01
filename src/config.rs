//! Compile-time tunables. Anything that changes simulation behaviour or world
//! shape lives here so the rest of the code reads as logic, not magic numbers.

pub const TILE_SIZE:    f32 = 8.0;
pub const WORLD_WIDTH:  i32 = 320;
pub const WORLD_HEIGHT: i32 = 200;
pub const SURFACE_ROW:  i32 = 28;

pub const SCREEN_WIDTH:  i32 = 1280;
pub const SCREEN_HEIGHT: i32 = 800;

// Initial population. The Rust build is supposed to handle thousands; start
// at 1000 for the default seed and let the queen grow it from there.
pub const INITIAL_WORKERS: usize = 1000;

pub const ANT_SPEED:        f32 = 6.0;   // tiles / s
pub const ANT_SENSE_RADIUS: f32 = 6.0;
pub const ANT_REPLAN_HZ:    f32 = 1.0;   // per-agent goal re-evaluation rate

// Pheromones — 4 channels: FOOD, RETURN, EXPLORE, ALARM
pub const PHEROMONE_CHANNELS: usize = 4;
pub const PHERO_DECAY_PER_S:  f32   = 0.4;   // exponential decay rate
pub const PHERO_MAX:          f32   = 255.0;

// Dig system
pub const EXPAND_INTERVAL:     f32 = 5.0;
pub const EXPAND_JOBS_PER_PASS: usize = 24;
pub const EXPAND_MAX_QUEUE:    usize = 200;

// Claim TTL: a worker that doesn't make dig progress within this window has
// its claim auto-expired by the slot table. Structurally prevents the leaked-
// claim deadlock that bit the Python build.
pub const DIG_CLAIM_TTL_S: f32 = 6.0;

pub const DIG_TIME_SOIL:  f32 = 1.5;
pub const DIG_TIME_SAND:  f32 = 0.8;
pub const DIG_TIME_DIRT1: f32 = 3.5;
pub const DIG_TIME_DIRT2: f32 = 7.5;
pub const DIG_TIME_DIRT3: f32 = 14.0;

// Colony entrance (centre of map at surface)
pub const COLONY_X: i32 = WORLD_WIDTH / 2;
pub const COLONY_Y: i32 = SURFACE_ROW;

// Spatial grid cell size, in tiles
pub const SPATIAL_CELL: i32 = 4;

// Water sim cadence
pub const WATER_STEP_INTERVAL_S: f32 = 0.04;

// Day/night
pub const DAY_LENGTH_SECONDS: f32 = 240.0;

// Queen reproduction
pub const QUEEN_EGG_INTERVAL_S: f32 = 8.0;
pub const BROOD_MATURE_S:       f32 = 18.0;

// Surface food
pub const FOOD_SPAWN_INTERVAL_S: f32 = 4.0;
pub const FOOD_SPAWN_MAX:        usize = 80;
pub const FOOD_PHEROMONE_BURST:  f32 = 220.0;

// Combat
pub const ATTACK_DAMAGE_RADIUS_TILES: f32 = 1.6;
pub const ALARM_PHEROMONE_BURST:      f32 = 240.0;
pub const ALARM_TRIGGER_LEVEL:        f32 = 12.0;
pub const CORPSE_DECAY_S:             f32 = 80.0;

// Soldier patrol
pub const SOLDIER_PATROL_RADIUS:  f32 = 22.0;
pub const SOLDIER_SENSE_RADIUS_T: i32 = 8;

// Time control / history (rewind)
pub const REWIND_HISTORY_SECONDS: f32 = 60.0;
pub const SNAPSHOT_INTERVAL_S:    f32 = 1.0;
pub const FF_LEVELS: &[u32] = &[1, 2, 4, 10, 100];

// ── Surface scenery — dog and lawn mower ─────────────────────────────
// All numbers here are tunable knobs. They live together at the bottom
// of config.rs so balance changes (mower destructiveness, lap count,
// cooldown, speeds, etc.) can be made in one file without grepping.

/// Tiles per second the barn dog walks at.
pub const DOG_SPEED:                 f32 = 2.4;
/// Dog patrol box, expressed as offsets from `COLONY_X`. Bounds are
/// chosen so the dog stays in the barn yard and never overlaps the
/// mower's range — otherwise the two surface-anchored sprites
/// composite into one creature.
pub const DOG_PATROL_WEST_OFFSET:    i32 = -80;
pub const DOG_PATROL_EAST_OFFSET:    i32 = -30;

/// Tiles per second the lawn mower rolls at. Deliberately lower than
/// `DOG_SPEED` so passes feel infrequent.
pub const MOWER_SPEED:               f32 = 1.0;
/// Mower patrol bounds. West is an offset from `COLONY_X`; east is a
/// margin from the right edge of the world. Together they cover the
/// entire pile-prone region around the entrance plus the eastern half
/// of the map.
pub const MOWER_PATROL_WEST_OFFSET:  i32 = -25;
pub const MOWER_PATROL_RIGHT_MARGIN: f32 = 6.0;
/// Where the mower spawns each visit, as an offset from `COLONY_X`.
pub const MOWER_SPAWN_X_OFFSET:      i32 = 60;

/// Probability that the mower clears one above-ground tile per
/// shave-roll. Each column transition rolls
/// `MOWER_TILES_PER_COLUMN` independent attempts at this chance.
pub const MOWER_SHAVE_CHANCE:        f32 = 0.65;
/// Tiles eligible to be shaved per column transition. Average tiles
/// removed per column = `MOWER_TILES_PER_COLUMN * MOWER_SHAVE_CHANCE`
/// — bump this to make the mower more destructive.
pub const MOWER_TILES_PER_COLUMN:    u32 = 2;

/// Number of patrol-bound traversals the mower completes per visit
/// before retiring. One traversal = one bound-to-bound trip across
/// the patrol range, so 3 = west→east→west, ending on the same side
/// it started from.
pub const MOWER_LAPS_PER_VISIT:      u32 = 3;
/// Sim seconds the mower stays away after finishing its laps. At 1×
/// speed this is wall-clock seconds.
pub const MOWER_COOLDOWN_SECONDS:    f32 = 300.0;

/// Tile radius around the mower's centre where any creature with a
/// `Health` component is killed and turned into a corpse. The mower's
/// chassis is 4 tiles wide × 2 tall; this radius covers the deck and
/// a bit of slack on either side so ants don't slip past the blade.
pub const MOWER_KILL_RADIUS:         f32 = 2.5;
/// Food value placed in the corpse the mower drops on each kill.
/// Small but non-zero so the colony can scavenge what's left.
pub const MOWER_KILL_FOOD_VALUE:     u8  = 2;
