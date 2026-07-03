//! Theorem — a mathematical equivalence statement.

use crate::semantic::expression::SemanticExpression;

/// A mathematical statement: lhs ≡ rhs under the stated assumptions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Theorem {
    pub lhs: SemanticExpression,
    pub rhs: SemanticExpression,
}

impl Theorem {
    pub fn new(lhs: SemanticExpression, rhs: SemanticExpression) -> Self {
        Self { lhs, rhs }
    }
}
