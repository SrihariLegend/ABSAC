//! SIR Analysis Framework (SAF) v0.1
//!
//! A **read-only** analysis layer for SIR graphs. SAF answers questions about
//! program meaning without modifying the IR. Every fact is stored outside
//! SIR nodes in a unified `FactDatabase`.

pub mod facts;
pub mod graph;
pub mod analysis;
pub mod manager;
pub mod cache;
pub mod dependency;

pub mod use_def;
pub mod dominance;
pub mod constants;
pub mod purity;
pub mod ranges;
pub mod alias;
pub mod escape;
pub mod loops;
pub mod value_numbering;

#[cfg(test)]
mod tests;
