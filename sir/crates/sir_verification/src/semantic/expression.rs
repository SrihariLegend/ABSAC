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
    BooleanArray { variable: VariableId },

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
        let lhs = SemanticExpression::Count(Box::new(
            SemanticExpression::Filter {
                input: Box::new(SemanticExpression::BooleanArray { variable: board }),
                predicate: Predicate::True,
            },
        ));
        // Popcount(Pack(BooleanArray(board)))
        let rhs = SemanticExpression::Popcount(Box::new(
            SemanticExpression::Pack(Box::new(
                SemanticExpression::BooleanArray { variable: board },
            )),
        ));
        // Verify they are not equal (different structure)
        assert_ne!(lhs, rhs);
    }
}
