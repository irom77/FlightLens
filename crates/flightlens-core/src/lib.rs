//! Offline analysis of immutable flight controller backup snapshots.
pub mod analysis;
pub mod compatibility;
pub mod export;
pub mod model;
pub mod parser;
pub mod source;
pub use model::*;
pub use parser::analyze;

pub mod vtx;
