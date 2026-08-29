use crate::semantic::expression::SemanticExpression;
use crate::semantic::normalizer::NormalizationRule;
use sir_types::ConstantData;

/// Rewrites `LowestSetBit(x)` to `x & (0 - x)`.
///
/// `x & -x` isolates the lowest set bit; the negative is expressed as
/// `Subtract(0, x)` so no dedicated negation node is needed.
pub struct LowestSetBitToBitwiseAnd;

impl NormalizationRule for LowestSetBitToBitwiseAnd {
    fn name(&self) -> &'static str {
        "LowestSetBitToBitwiseAnd"
    }

    fn apply(&self, expr: &SemanticExpression) -> Option<SemanticExpression> {
        if let SemanticExpression::LowestSetBit(inner) = expr {
            let zero = SemanticExpression::Constant(ConstantData::u64(0));
            return Some(SemanticExpression::BitwiseAnd(
                inner.clone(),
                Box::new(SemanticExpression::Subtract(
                    Box::new(zero),
                    inner.clone(),
                )),
            ));
        }
        None
    }
}
