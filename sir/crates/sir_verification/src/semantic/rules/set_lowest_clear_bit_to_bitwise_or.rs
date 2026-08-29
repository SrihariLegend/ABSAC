use crate::semantic::expression::SemanticExpression;
use crate::semantic::normalizer::NormalizationRule;
use sir_types::ConstantData;

/// Rewrites `SetLowestClearBit(x)` to `x | (x + 1)`.
///
/// The lowest clear bit of `x` is the lowest zero bit; `x + 1` flips it to
/// one (carrying past any trailing ones), so the OR sets it.
pub struct SetLowestClearBitToBitwiseOr;

impl NormalizationRule for SetLowestClearBitToBitwiseOr {
    fn name(&self) -> &'static str {
        "SetLowestClearBitToBitwiseOr"
    }

    fn apply(&self, expr: &SemanticExpression) -> Option<SemanticExpression> {
        if let SemanticExpression::SetLowestClearBit(inner) = expr {
            let one = SemanticExpression::Constant(ConstantData::u64(1));
            return Some(SemanticExpression::BitwiseOr(
                inner.clone(),
                Box::new(SemanticExpression::Add(inner.clone(), Box::new(one))),
            ));
        }
        None
    }
}
