//! SIR Types — Foundational type system for Semantic IR.
//!
//! This crate provides the core type definitions, identifiers, effects,
//! source spans, metadata, and constant values used throughout SIR.

pub mod constant;
pub mod cost_profile;
pub mod effects;
pub mod metadata;
pub mod node_id;
pub mod region_id;
pub mod region_map;
pub mod span;
pub mod types;

pub use constant::*;
pub use cost_profile::*;
pub use effects::*;
pub use metadata::*;
pub use node_id::*;
pub use region_id::*;
pub use region_map::RegionMap;
pub use span::*;
pub use types::*;
