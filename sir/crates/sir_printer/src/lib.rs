//! SIR Printer — Human-readable and JSON output for SIR graphs.
//!
//! Supports compact and detailed text formats, plus JSON serialization
//! for roundtrip preservation.

pub mod text;
pub mod json;

pub use text::*;
pub use json::*;
