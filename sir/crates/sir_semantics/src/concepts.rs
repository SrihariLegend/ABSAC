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
    /// Operation: summing the values of elements in a collection
    SumReduction,
    /// Operation: checking if at least one element satisfies a condition
    DisjunctiveReduction,
    /// Operation: checking if all elements satisfy a condition
    ConjunctiveReduction,
    /// Operation: checking if an odd number of elements satisfy a condition (parity/xor)
    ExclusiveReduction,
    /// Operation: parity of the set bits of a scalar (odd number of set bits)
    Parity,
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
    /// Operation: isolating the lowest clear (zero) bit
    LowestClearBitMask,
    /// Operation: setting the lowest clear (zero) bit
    SetLowestClearBit,
    /// Operation: testing if a value is zero
    IsZero,

    // ── Added for Loop Iteration (Phase II.1) ───────────
    /// Property: a loop that terminates when a variable reaches zero
    LoopUntilZero,
    /// Operation: repeatedly applying an operation that monotonically removes one set bit
    BitsetIteration,

    // ── Added for Semantic Closure (Phase II.1) ───────────
    /// Operation: property of having at most one bit set (or being zero)
    AtMostOneBitSet,
    /// Operation: a boolean predicate evaluated over a collection
    PredicateMap,
    /// Structure: a sequence of elements originating from a collection
    ElementSequence,

    // ── Added for Bit Permutations (Phase II.2) ───────────
    /// Structure: a pair of opposite shifts of the same value — `(x << k) | (x >> (w - k))`.
    /// Left-rotating code shape; the physical evidence for a circular permutation.
    ShiftPairLeft,
    /// Structure: the mirror-image shift pair `(x >> k) | (x << (w - k))`.
    /// Right-rotating code shape; the physical evidence for a circular permutation.
    ShiftPairRight,
    /// Structure: an OR of two opposite masked shifts — `((x & m) << s) | ((x >> s) & m)`.
    /// Swaps the bit groups selected by `m` with their `s`-shifted counterparts.
    MaskedShiftSwap,
    /// Operation: a circular rotation of a word's bits (target-independent).
    CircularPermutation,
    /// Operation: a byte-order reversal of a word (e.g. a 16-bit byte swap).
    BytePermutation,
    /// Operation: an arbitrary fixed permutation of bit positions
    /// (e.g. full bit reversal).
    BitPermutation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ConceptKind {
    Structure,
    Operation,
    Property,
}

impl SemanticConcept {
    /// Every concept in the ontology. Kept in sync with the enum variants so
    /// downstream tooling (e.g. the benchmark report) can enumerate the
    /// ontology without re-typing the variant list.
    pub const ALL: &'static [SemanticConcept] = &[
        Self::LogicalSequence,
        Self::FiniteCollection,
        Self::MembershipTraversal,
        Self::CardinalityReduction,
        Self::SumReduction,
        Self::DisjunctiveReduction,
        Self::ConjunctiveReduction,
        Self::ExclusiveReduction,
        Self::Parity,
        Self::FindFirst,
        Self::SetIntersection,
        Self::ModuloPowerOfTwo,
        Self::MultiplyPowerOfTwo,
        Self::DividePowerOfTwo,
        Self::ShiftMask,
        Self::PositionSearch,
        Self::FirstOccurrence,
        Self::LastOccurrence,
        Self::TrailingZeroSearch,
        Self::LeadingZeroSearch,
        Self::FiniteSet,
        Self::SetMembership,
        Self::SetUnion,
        Self::SetDifference,
        Self::SetSymmetricDifference,
        Self::SetSubset,
        Self::SetEquality,
        Self::SetEmpty,
        Self::SetCardinality,
        Self::LowestSetBit,
        Self::ClearLowestSetBit,
        Self::LowestClearBitMask,
        Self::SetLowestClearBit,
        Self::IsZero,
        Self::LoopUntilZero,
        Self::BitsetIteration,
        Self::AtMostOneBitSet,
        Self::PredicateMap,
        Self::ElementSequence,
        Self::ShiftPairLeft,
        Self::ShiftPairRight,
        Self::MaskedShiftSwap,
        Self::CircularPermutation,
        Self::BytePermutation,
        Self::BitPermutation,
    ];

    pub fn kind(&self) -> ConceptKind {
        match self {
            // Structures
            SemanticConcept::LogicalSequence |
            SemanticConcept::FiniteCollection |
            SemanticConcept::FiniteSet |
            SemanticConcept::ElementSequence => ConceptKind::Structure,
            SemanticConcept::ShiftPairLeft |
            SemanticConcept::ShiftPairRight |
            SemanticConcept::MaskedShiftSwap => ConceptKind::Structure,
            
            // Properties
            SemanticConcept::IsZero |
            SemanticConcept::AtMostOneBitSet |
            SemanticConcept::SetEmpty |
            SemanticConcept::SetEquality |
            SemanticConcept::SetSubset |
            SemanticConcept::LoopUntilZero => ConceptKind::Property,

            // Operations
            SemanticConcept::CircularPermutation |
            SemanticConcept::BytePermutation |
            SemanticConcept::BitPermutation => ConceptKind::Operation,
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
            SemanticConcept::SumReduction => write!(f, "SumReduction"),
            SemanticConcept::DisjunctiveReduction => write!(f, "DisjunctiveReduction"),
            SemanticConcept::ConjunctiveReduction => write!(f, "ConjunctiveReduction"),
            SemanticConcept::ExclusiveReduction => write!(f, "ExclusiveReduction"),
            SemanticConcept::Parity => write!(f, "Parity"),
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
            SemanticConcept::LowestClearBitMask => write!(f, "LowestClearBitMask"),
            SemanticConcept::SetLowestClearBit => write!(f, "SetLowestClearBit"),
            SemanticConcept::IsZero => write!(f, "IsZero"),
            SemanticConcept::LoopUntilZero => write!(f, "LoopUntilZero"),
            SemanticConcept::BitsetIteration => write!(f, "BitsetIteration"),
            SemanticConcept::AtMostOneBitSet => write!(f, "AtMostOneBitSet"),
            SemanticConcept::PredicateMap => write!(f, "PredicateMap"),
            SemanticConcept::ElementSequence => write!(f, "ElementSequence"),
            SemanticConcept::ShiftPairLeft => write!(f, "ShiftPairLeft"),
            SemanticConcept::ShiftPairRight => write!(f, "ShiftPairRight"),
            SemanticConcept::MaskedShiftSwap => write!(f, "MaskedShiftSwap"),
            SemanticConcept::CircularPermutation => write!(f, "CircularPermutation"),
            SemanticConcept::BytePermutation => write!(f, "BytePermutation"),
            SemanticConcept::BitPermutation => write!(f, "BitPermutation"),
        }
    }
}
