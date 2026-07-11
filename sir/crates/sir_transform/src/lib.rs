//! SIR Transform — Transformation Contract v0.1
//!
//! Defines the immutable contract between program understanding and
//! program transformation. Contains only data types and invariants.
//! Contains no algorithms, analyses, or rewrite logic.
//!
//! This crate sits at the center of the architecture:
//!   Understanding → sir_transform ← Action

pub mod assumptions;
pub mod constraints;
pub mod context;
pub mod representation;
pub mod roles;
pub mod structures;
pub use context::TransformationContextDatabase;

pub mod ids;
pub use ids::*;
