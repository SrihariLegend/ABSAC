use std::fmt;

/// A mathematical representation of a computation.
///
/// These are concrete realizations of semantic concepts, not
/// machine instructions. v0.1 targets exactly one representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
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

/// Integer support score — no floating point in engine logic.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Support {
    pub positive: u16,
    pub negative: u16,
}

impl Support {
    /// Net score: positive minus negative.
    pub fn score(&self) -> i32 {
        self.positive as i32 - self.negative as i32
    }

    /// Ratio of positive support to total (for display only).
    pub fn ratio(&self) -> f32 {
        let total = self.positive as f32 + self.negative as f32;
        if total == 0.0 {
            0.0
        } else {
            self.positive as f32 / total
        }
    }

    /// Qualitative confidence label derived from net score.
    pub fn confidence_label(&self) -> &'static str {
        let net = self.score().abs();
        match net {
            0..=20 => "Weak",
            21..=50 => "Moderate",
            51..=79 => "Strong",
            _ => "Very Strong",
        }
    }
}

/// A hypothesis is a representation with accumulated support and evidence trace.
#[derive(Clone, Debug)]
pub struct Hypothesis {
    pub representation: Representation,
    pub support: Support,
    pub evidence: Vec<usize>, // indices into the engine's evidence list
}

/// A unique identifier for an evidence entry.
pub type EvidenceId = usize;
