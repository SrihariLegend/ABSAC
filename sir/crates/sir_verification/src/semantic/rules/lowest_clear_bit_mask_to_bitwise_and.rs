use crate::semantic::expression::SemanticExpression;
use crate::semantic::normalizer::NormalizationRule;
use sir_types::ConstantData;

/// Rewrites `LowestClearBitMask(x)` to `~x & (x + 1)`.
///
/// The lowest clear bit of `x` is the lowest set bit of `~x`, which is
/// isolated by `~x & -~x` = `~x & (x + 1)` (two's complement).
pub struct LowestClearBitMaskToBitwiseAnd;

impl NormalizationRule for LowestClearBitMaskToBitwiseAnd {
    fn name(&self) -> &'static str {
        "LowestClearBitMaskToBitwiseAnd"
    }

    fn apply(&self, expr: &SemanticExpression) -> Option<SemanticExpression> {
        if let SemanticExpression::LowestClearBitMask(inner) = expr {
            let one = SemanticExpression::Constant(ConstantData::u64(1));
            return Some(SemanticExpression::BitwiseAnd(
                Box::new(SemanticExpression::BitwiseNot(inner.clone())),
                Box::new(SemanticExpression::Add(inner.clone(), Box::new(one))),
            ));
        }
        None
    }
}
