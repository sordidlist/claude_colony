//! World-level state: tile grids, pheromone field, dig-job queue,
//! flow-field navigation, and the procgen that initialises them.
//! Nothing in here knows about ECS systems or rendering; this layer
//! is pure data + pure transformations on data.

pub mod tiles;
pub mod procgen;
pub mod pheromones;
pub mod water;
pub mod dig_jobs;
pub mod exploration;
pub mod dirt_physics;
pub mod flow_field;
pub mod grass;

pub use tiles::{TileType, TileGrid};
pub use pheromones::{PheromoneGrid, PheromoneChannel};
pub use water::WaterGrid;
pub use dig_jobs::{DigJobs, DigClaim};
pub use exploration::ExploredGrid;
pub use flow_field::ReturnFlowField;
pub use grass::GrassField;
