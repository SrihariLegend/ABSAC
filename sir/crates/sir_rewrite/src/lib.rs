//! SIR Rewrite — Verified Graph Rewriting Engine v0.1
//!
//! Executes proven transformations by constructing replacement SIR
//! in a detached arena, then performing transactional graph surgery.
//! Never discovers, never analyses, never proves — only executes.

pub mod builder;
pub mod detached_arena;
pub mod engine;
pub mod error;
pub mod local_id;
pub mod patch;
pub mod plan;
pub mod recipe;
pub mod recipes;
pub mod region;
pub mod registry;
pub mod result;
pub mod subgraph_builder;
