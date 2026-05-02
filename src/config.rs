//! Compile-time tunables. Anything that changes simulation behaviour or world
//! shape lives here so the rest of the code reads as logic, not magic numbers.

pub const TILE_SIZE:    f32 = 8.0;
pub const WORLD_WIDTH:  i32 = 320;
pub const WORLD_HEIGHT: i32 = 200;
pub const SURFACE_ROW:  i32 = 28;

pub const SCREEN_WIDTH:  i32 = 1280;
pub const SCREEN_HEIGHT: i32 = 800;

// Initial population. The colony starts modest — three workers and one
// queen — so the player can watch the colony grow from a real founding.
// The queen and the dig director will scale things up from there over
// the first few minutes of play.
pub const INITIAL_WORKERS: usize = 3;

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

/// Minimum horizontal distance (in tiles) the dig director will allow
/// between two surface holes. Without this, the workforce eventually
/// pock-marks the lawn into a single wide chasm; with it, additional
/// tunnels surface as discrete openings spaced like real ant-hill
/// entrances. Each hill above ground sits over its own narrow shaft
/// instead of a flat sheet of grass over a cavern.
pub const MIN_SURFACE_HOLE_SPACING: i32 = 14;
/// Maximum width of a single surface opening (the grass row). The
/// director won't queue a dig that would push the contiguous
/// passable run on row `SURFACE_ROW` past this. A 3-tile entrance
/// is wide enough for two-way ant traffic, narrow enough that a
/// dirt-pile hill above it reads correctly.
pub const MAX_SURFACE_HOLE_WIDTH: i32 = 3;

// Spatial grid cell size, in tiles
pub const SPATIAL_CELL: i32 = 4;

// Water sim cadence
pub const WATER_STEP_INTERVAL_S: f32 = 0.04;

// Day/night
pub const DAY_LENGTH_SECONDS: f32 = 240.0;

// Queen reproduction
pub const QUEEN_EGG_INTERVAL_S: f32 = 8.0;
pub const BROOD_MATURE_S:       f32 = 18.0;

// Queen migration: as workers extend the colony, the queen periodically
// re-evaluates where she lives and moves to the deepest reachable
// chamber from the entrance. "Deeper" is a proxy for "safer" — further
// from the surface where rivals enter, with more tunnel between her and
// any predator that finds the entrance shaft.
//
// The check fires every `QUEEN_MIGRATION_INTERVAL_S` of sim time. She
// only relocates if the new deepest spot is at least
// `QUEEN_MIGRATION_MIN_DEPTH_GAIN` rows below her current position, so
// she doesn't twitch back and forth on equivalent chambers.
pub const QUEEN_MIGRATION_INTERVAL_S:    f32 = 30.0;
pub const QUEEN_MIGRATION_MIN_DEPTH_GAIN: i32 = 3;

// Surface food
pub const FOOD_SPAWN_INTERVAL_S: f32 = 4.0;
pub const FOOD_SPAWN_MAX:        usize = 80;
pub const FOOD_PHEROMONE_BURST:  f32 = 220.0;

// Combat
pub const ATTACK_DAMAGE_RADIUS_TILES: f32 = 1.6;
pub const ALARM_PHEROMONE_BURST:      f32 = 240.0;
pub const ALARM_TRIGGER_LEVEL:        f32 = 12.0;
pub const CORPSE_DECAY_S:             f32 = 80.0;

// Spider combat profile. Tuned so a single spider reliably beats a
// single worker, two workers and a spider often trade kills (spider
// gets one before going down), three workers usually wins with a
// casualty, and four-or-more workers wins cleanly. Combined with the
// spider's post-kill retreat (below), a spider's appearance is a
// genuine threat to the colony — hauling off the worker it killed
// instead of leaving a corpse for the colony to scavenge.
pub const SPIDER_HP:               f32 = 35.0;
pub const SPIDER_ATTACK_DAMAGE:    f32 = 4.5;
pub const SPIDER_ATTACK_RANGE:     f32 = 1.5;
pub const SPIDER_ATTACK_COOLDOWN:  f32 = 1.0;

/// Tile radius within which a spider actively hunts the nearest
/// colony ant (steers toward it instead of random-walking). Without
/// this, spiders that wander into the colony just pace around while
/// workers ignore them. With this, the spider becomes a genuine
/// predator that closes on the nearest meal.
pub const SPIDER_HUNT_RADIUS:        f32 = 10.0;
/// Tile radius within which a worker switches to FightBack mode on
/// direct sight, independent of the alarm pheromone. The pheromone
/// path still works for distant alerts; this is the close-range
/// "I see a spider, drop everything" reflex.
pub const WORKER_THREAT_RADIUS:      f32 = 8.0;
/// Half-width of the square area over which `hostile_alarm_emission`
/// stamps alarm pheromone each frame. 1 = 3×3 (original), 2 = 5×5,
/// 4 = 9×9. Larger values pull more distant workers into the fight.
/// Combined with the alarm-diffusion system below, the gradient
/// reaches workers many tiles away through the tunnel network.
pub const ALARM_EMISSION_HALF_WIDTH: i32 = 4;

/// After this many failed direction-flip attempts on a haul cycle, a
/// worker stops walking back and forth in the dirt-mound trap and
/// just drops the pebble at its feet (or clears cargo if no Air tile
/// is nearby). Without this, mature colonies grow tall mounds that
/// gridlock haulers above ground forever, and the workforce never
/// returns underground to encounter the spiders that have wandered
/// in.
pub const MAX_HAUL_ATTEMPTS:      u8  = 4;

/// Sim seconds the spider spends retreating after killing a colony
/// member. During retreat the spider runs away from the colony
/// entrance at a higher speed, dragging its prey "back to its lair"
/// — flavour-only behaviour that visually motivates why no corpse
/// is left at the kill site.
pub const SPIDER_RETREAT_AFTER_KILL_S: f32 = 5.0;
pub const SPIDER_RETREAT_SPEED_MULT:   f32 = 1.5;

/// Periodic off-screen invader spawning. Hostiles no longer start
/// underground; they walk in from the world edges at the surface
/// row at random intervals. Tuned so a fresh colony has a few minutes
/// to grow before the first visitor.
pub const INVADER_FIRST_SPAWN_S:    f32 = 90.0;
pub const INVADER_SPAWN_INTERVAL_S: f32 = 75.0;
pub const INVADER_SPAWN_JITTER_S:   f32 = 45.0;
/// Probability that a single spawn picks a spider; otherwise it's a
/// rival ant. Spiders are the rarer, scarier visitor.
pub const INVADER_SPIDER_PROBABILITY: f32 = 0.35;

// Soldier patrol
pub const SOLDIER_PATROL_RADIUS:  f32 = 22.0;
pub const SOLDIER_SENSE_RADIUS_T: i32 = 8;

// Time control / history (rewind)
pub const REWIND_HISTORY_SECONDS: f32 = 60.0;
pub const SNAPSHOT_INTERVAL_S:    f32 = 1.0;
pub const FF_LEVELS: &[u32] = &[1, 2, 4, 10, 100, 500];

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

// ── Grass field (above-ground decoration) ──────────────────────────
//
// Grass blades on every surface column have a `length` value that
// grows slowly over time and gets reset to 0 when the mower drives
// through that column. Renders as 0..MAX vertical pixels above the
// grass tile, so the lawn visibly shaggies up between mower visits.

/// Maximum grass length, expressed as pixels of overlay above the
/// grass row. Renderer interprets each unit as one screen pixel
/// per zoom-1 tile.
pub const GRASS_LENGTH_MAX:      u8  = 6;
/// Sim seconds between grass growth ticks. At 1× speed the lawn
/// reaches full length in roughly `GRASS_LENGTH_MAX × interval`
/// seconds; at 100× FF that's ~30s of wall-clock for full shag.
pub const GRASS_GROW_INTERVAL_S: f32 = 5.0;
