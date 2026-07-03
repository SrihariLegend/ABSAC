//! Evidence sources — one module per representation type.
//!
//! Each source is a pure function that inspects a region's concepts
//! and returns evidence entries. The engine owns the registry and
//! calls each source during inference.

pub mod bitset_evidence;
