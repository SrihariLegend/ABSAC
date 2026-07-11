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

    // ── Added for Set Algebra (Phase 0020) ───────────
    /// Data: a finite mathematical set abstraction
    FiniteSet,
    /// Operation: testing if an item is present in a set
    SetMembership,
    /// Operation: elements present in either of two sets
    SetUnion,
    /// Operation: elements present in one set but not the other
    SetDifference,
    /// Operation: elements present in exactly one of two sets
    SetSymmetricDifference,
    /// Operation: testing if all elements of one set are present in another
    SetSubset,
    /// Operation: testing if two sets contain exactly the same elements
    SetEquality,
    /// Operation: testing if a set contains zero elements
    SetEmpty,
    /// Operation: counting the number of elements in a set
    SetCardinality,

    // ── Added for Mask Algebra (Phase II.1) ───────────
    /// Operation: isolating the lowest set bit
    LowestSetBit,
    /// Operation: clearing the lowest set bit
    ClearLowestSetBit,
    /// Operation: testing if a value is zero
    IsZero,

    // ── Added for Semantic Closure (Phase II.1) ───────────
    /// Operation: property of having at most one bit set (or being zero)
    AtMostOneBitSet,
    /// Operation: a boolean predicate evaluated over a collection
    PredicateMap,
    /// Structure: a sequence of elements originating from a collection
    ElementSequence,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ConceptKind {
    Structure,
    Operation,
    Property,
}

impl SemanticConcept {
    pub fn kind(&self) -> ConceptKind {
        match self {
            // Structures
            SemanticConcept::LogicalSequence |
            SemanticConcept::FiniteCollection |
            SemanticConcept::FiniteSet |
            SemanticConcept::ElementSequence => ConceptKind::Structure,
            
            // Properties
            SemanticConcept::IsZero |
            SemanticConcept::AtMostOneBitSet |
            SemanticConcept::SetEmpty |
            SemanticConcept::SetEquality |
            SemanticConcept::SetSubset => ConceptKind::Property,

            // Operations
            _ => ConceptKind::Operation,
        }
    }
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
            SemanticConcept::FiniteSet => write!(f, "FiniteSet"),
            SemanticConcept::SetMembership => write!(f, "SetMembership"),
            SemanticConcept::SetUnion => write!(f, "SetUnion"),
            SemanticConcept::SetDifference => write!(f, "SetDifference"),
            SemanticConcept::SetSymmetricDifference => write!(f, "SetSymmetricDifference"),
            SemanticConcept::SetSubset => write!(f, "SetSubset"),
            SemanticConcept::SetEquality => write!(f, "SetEquality"),
            SemanticConcept::SetEmpty => write!(f, "SetEmpty"),
            SemanticConcept::SetCardinality => write!(f, "SetCardinality"),
            SemanticConcept::LowestSetBit => write!(f, "LowestSetBit"),
            SemanticConcept::ClearLowestSetBit => write!(f, "ClearLowestSetBit"),
            SemanticConcept::IsZero => write!(f, "IsZero"),
            SemanticConcept::AtMostOneBitSet => write!(f, "AtMostOneBitSet"),
            SemanticConcept::PredicateMap => write!(f, "PredicateMap"),
            SemanticConcept::ElementSequence => write!(f, "ElementSequence"),
        }
    }
}
