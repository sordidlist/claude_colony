//! Simulation layer — entities, components, and the systems that drive
//! them. Owns no rendering state and no tile storage; reads from
//! `world::*` and writes to `Position`/`Velocity`/etc. so headless
//! tests can exercise the full game loop without a GPU.

// Data — components and per-tick world resources.
pub mod components;
pub mod time;
pub mod event_log;
pub mod day_night;
pub mod history;
pub mod balance;

// Spatial / motion / pathfinding helpers.
pub mod spatial;
pub mod movement;
pub mod exploration;

// AI systems.
pub mod ai_worker;
pub mod soldier;
pub mod hostiles;

// Lifecycle systems — birth, growth, death, scavenging.
pub mod queen;
pub mod brood;
pub mod combat;
pub mod lifecycle;

// World interaction — picking up food, finding it, surface flavour.
pub mod food_spawn;
pub mod foraging;
pub mod scenery;

pub use components::*;
pub use spatial::SpatialGrid;
pub use time::{Time, Population};
pub use event_log::EventLog;
pub use day_night::TimeOfDay;
pub use ai_worker::DigStats;
pub use food_spawn::SurfaceFoodSpawner;
pub use foraging::ColonyStores;
pub use balance::BalanceTunables;
