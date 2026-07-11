//! SIR Printer — Human-readable and JSON output for SIR graphs.
//!
//! Supports compact and detailed text formats, plus JSON serialization
//! for roundtrip preservation.

pub mod json;
pub mod text;

pub use json::*;
pub use text::*;
