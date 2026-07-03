//! SIR Rewrite — Verified Graph Rewriting Engine v0.1
//!
//! Executes proven transformations by constructing replacement SIR
//! in a detached arena, then performing transactional graph surgery.
//! Never discovers, never analyses, never proves — only executes.

pub mod local_id;
pub mod detached_arena;
