//! SIR Builder — Type-safe construction API for SIR functions.
//!
//! The Builder provides methods for constructing SIR graphs with
//! automatic type checking and effect computation.

pub mod builder;
pub mod error;

pub use builder::*;
pub use error::*;
