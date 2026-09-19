//! Minimal single-pass optimizer for the retained families.
//!
//! The first vertical slice is the fixed-extent sum reduction: recognize
//! the loop, check the monoid laws with `sir_mech`, build a straight-line
//! replacement chain, and splice it into SIR. Other retained families
//! (mapped sums, byte search, bit scans, reductions) port here next.

pub mod all;
pub mod any;
pub mod arithmetic;
pub mod count;
pub mod family;
pub mod optimize;
pub mod parity;
pub mod patch;
pub mod popcount;
pub mod prove;
pub mod runtime_sum;
pub mod search;
pub mod sum;

pub use all::optimize_all;
pub use any::optimize_any;
pub use arithmetic::optimize_arithmetic;
pub use count::optimize_count;
pub use optimize::optimize;
pub use parity::optimize_parity;
pub use popcount::optimize_popcount;
pub use runtime_sum::optimize_runtime_sum;
pub use search::{optimize_memchr, optimize_search};
pub use sum::{optimize_sum, optimize_sum_with_definition};
