use std::fmt;

/// A semantic concept describing what a computation is doing.
///
/// Concepts are organized into two groups:
/// - **Data concepts:** describe the data being operated on
/// - **Operation concepts:** describe what the computation does with the data
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SemanticConcept {
    /// Data: sequence of boolean values, whether physical (array) or virtual (predicates)
    LogicalSequence,
    /// Data: collection with a statically known bound
    FiniteCollection,
    /// Operation: iterating over elements and testing membership
    MembershipTraversal,
    /// Operation: counting how many elements satisfy a condition
    CardinalityReduction,
    /// Operation: checking if at least one element satisfies a condition
    DisjunctiveReduction,
    /// Operation: checking if all elements satisfy a condition
    ConjunctiveReduction,
    /// Operation: checking if an odd number of elements satisfy a condition (parity/xor)
    ExclusiveReduction,
    /// Operation: finding the first element that satisfies a condition
    FindFirst,
    /// Operation: checking if elements are present in two collections simultaneously
    SetIntersection,
    /// Operation: integer modulo by a power of two
    ModuloPowerOfTwo,
    /// Operation: integer multiplication by a power of two
    MultiplyPowerOfTwo,
    /// Operation: integer division by a power of two
    DividePowerOfTwo,
    /// Operation: shift left followed by shift right, extracting a mask
    ShiftMask,

    // ── Added for Positional Search (Phase 0018) ───────────
    /// Operation: algorithmic search for a position based on a condition
    PositionSearch,
    /// Operation: finding the first element/bit that satisfies a condition
    FirstOccurrence,
    /// Operation: finding the last element/bit that satisfies a condition
    LastOccurrence,
    /// Operation: counting trailing zeroes
    TrailingZeroSearch,
    /// Operation: counting leading zeroes
    LeadingZeroSearch,
}

impl fmt::Display for SemanticConcept {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SemanticConcept::LogicalSequence => write!(f, "LogicalSequence"),
            SemanticConcept::FiniteCollection => write!(f, "FiniteCollection"),
            SemanticConcept::MembershipTraversal => write!(f, "MembershipTraversal"),
            SemanticConcept::CardinalityReduction => write!(f, "CardinalityReduction"),
            SemanticConcept::DisjunctiveReduction => write!(f, "DisjunctiveReduction"),
            SemanticConcept::ConjunctiveReduction => write!(f, "ConjunctiveReduction"),
            SemanticConcept::ExclusiveReduction => write!(f, "ExclusiveReduction"),
            SemanticConcept::FindFirst => write!(f, "FindFirst"),
            SemanticConcept::SetIntersection => write!(f, "SetIntersection"),
            SemanticConcept::ModuloPowerOfTwo => write!(f, "ModuloPowerOfTwo"),
            SemanticConcept::MultiplyPowerOfTwo => write!(f, "MultiplyPowerOfTwo"),
            SemanticConcept::DividePowerOfTwo => write!(f, "DividePowerOfTwo"),
            SemanticConcept::ShiftMask => write!(f, "ShiftMask"),
            SemanticConcept::PositionSearch => write!(f, "PositionSearch"),
            SemanticConcept::FirstOccurrence => write!(f, "FirstOccurrence"),
            SemanticConcept::LastOccurrence => write!(f, "LastOccurrence"),
            SemanticConcept::TrailingZeroSearch => write!(f, "TrailingZeroSearch"),
            SemanticConcept::LeadingZeroSearch => write!(f, "LeadingZeroSearch"),
        }
    }
}
