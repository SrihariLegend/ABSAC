//! SemanticExpression — the mathematical language for expressing program semantics.

use sir_transform::ids::VariableId;
use sir_types::ConstantData;

/// The mathematical language for expressing program semantics.
///
/// Intentionally minimal — only variants needed for BS001 exist.
/// Closed enum: exhaustiveness is a feature, not a limitation.
///
/// Design rule: Every new variant must justify itself by enabling
/// the proof of at least one new transformation theorem.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SemanticExpression {
    /// A variable referring to an input (e.g., the board parameter).
    Variable(VariableId),

    /// A compile-time constant value.
    Constant(ConstantData),

    /// A fixed-size array of boolean values.
    /// Length is obtained from the domain/environment at evaluation time.
    LogicalSequence { variable: VariableId },

    /// Pack a sequence of booleans into a single bitvector.
    Pack(Box<SemanticExpression>),

    /// Filter elements of a collection by a predicate.
    Filter {
        input: Box<SemanticExpression>,
        predicate: Predicate,
    },

    /// Count the number of elements in a collection.
    Count(Box<SemanticExpression>),

    /// Count the number of set bits in a bitvector.
    Popcount(Box<SemanticExpression>),

    // ── Added for Boolean Reductions (Phase 0016) ───────────
    /// True if at least one element in the boolean array is true.
    Exists(Box<SemanticExpression>),

    /// True if all elements in the boolean array are true.
    All(Box<SemanticExpression>),

    /// True if an odd number of elements in the boolean array are true.
    Parity(Box<SemanticExpression>),

    /// True if the bitvector is not zero.
    NotEqualZero(Box<SemanticExpression>),

    /// True if the bitvector has all bits set (equal to full mask).
    EqualFullMask(Box<SemanticExpression>),

    /// Bitwise AND with 1 (extract the lowest bit).
    BitwiseAndOne(Box<SemanticExpression>),

    // ── Added for Bitwise Arithmetic (Phase 0017) ───────────
    /// Integer modulo `(x % y)`.
    Modulo(Box<SemanticExpression>, Box<SemanticExpression>),

    /// Bitwise AND `(x & y)`.
    BitwiseAnd(Box<SemanticExpression>, Box<SemanticExpression>),

    /// Integer division `(x / y)`.
    Divide(Box<SemanticExpression>, Box<SemanticExpression>),

    /// Bitwise shift right `(x >> y)`.
    ShiftRight(Box<SemanticExpression>, Box<SemanticExpression>),

    /// Integer multiplication `(x * y)`.
    Multiply(Box<SemanticExpression>, Box<SemanticExpression>),

    /// Bitwise shift left `(x << y)`.
    ShiftLeft(Box<SemanticExpression>, Box<SemanticExpression>),

    // ── Added for Positional Search (Phase 0018) ────────────
    /// Index of the first true element in a boolean array. Returns the length if none found.
    FirstTrue(Box<SemanticExpression>),

    /// Index of the last true element in a boolean array. Returns -1 (or sentinel) if none found.
    LastTrue(Box<SemanticExpression>),

    /// Count of trailing zeros in a bitvector (equal to BitScanForward).
    TrailingZeros(Box<SemanticExpression>),

    /// Count of leading zeros in a bitvector (equal to BitScanReverse).
    LeadingZeros(Box<SemanticExpression>),
}

/// A predicate for filtering collections.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Predicate {
    /// All elements pass (identity filter).
    True,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construct_bs001_theorem_expressions() {
        let board = VariableId::new(0);
        // Count(Filter(BooleanArray(board), True))
        let lhs = SemanticExpression::Count(Box::new(SemanticExpression::Filter {
            input: Box::new(SemanticExpression::LogicalSequence { variable: board }),
            predicate: Predicate::True,
        }));
        // Popcount(Pack(BooleanArray(board)))
        let rhs = SemanticExpression::Popcount(Box::new(SemanticExpression::Pack(Box::new(
            SemanticExpression::LogicalSequence { variable: board },
        ))));
        // Verify they are not equal (different structure)
        assert_ne!(lhs, rhs);
    }
}
