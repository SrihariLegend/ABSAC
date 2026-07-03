use serde::{Deserialize, Serialize};
use std::fmt;

/// A mathematical representation of a computation.
///
/// Representations are transformation-domain concepts, not inference concepts.
/// Inference predicts them; generation implements them; verification proves them;
/// rewrite applies them. All four phases use the same definition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Representation {
    /// A fixed-size set of boolean values representable as bits.
    BitSet,
}

impl fmt::Display for Representation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Representation::BitSet => write!(f, "BitSet"),
        }
    }
}
