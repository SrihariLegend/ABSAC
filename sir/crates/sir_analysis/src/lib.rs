//! SIR analysis: loop facts and the graph utilities used by the
//! semantic pipeline. The former multi-pass framework was removed;
//! `FactDatabase` now carries exactly the analysis the recognizers,
//! binding and rewrite engine consume.

pub mod facts;
pub mod graph;
pub mod loop_shape;
pub mod loops;
pub mod manager;
