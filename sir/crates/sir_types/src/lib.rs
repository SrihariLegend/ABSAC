//! SIR Types — Foundational type system for Semantic IR.
//!
//! This crate provides the core type definitions, identifiers, effects,
//! source spans, metadata, and constant values used throughout SIR.

pub mod node_id;
pub mod types;
pub mod effects;
pub mod span;
pub mod metadata;
pub mod constant;
pub mod region_id;
pub mod region_map;
pub mod cost_profile;

pub use node_id::*;
pub use types::*;
pub use effects::*;
pub use span::*;
pub use metadata::*;
pub use constant::*;
pub use region_id::*;
pub use region_map::RegionMap;
pub use cost_profile::*;
