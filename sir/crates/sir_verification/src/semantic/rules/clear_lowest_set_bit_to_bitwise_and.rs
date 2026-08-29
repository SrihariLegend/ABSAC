use crate::semantic::expression::SemanticExpression;
use crate::semantic::normalizer::NormalizationRule;
use sir_types::ConstantData;

pub struct ClearLowestSetBitToBitwiseAnd;

impl NormalizationRule for ClearLowestSetBitToBitwiseAnd {
    fn name(&self) -> &'static str {
        "ClearLowestSetBitToBitwiseAnd"
    }

    fn apply(&self, expr: &SemanticExpression) -> Option<SemanticExpression> {
        if let SemanticExpression::ClearLowestSetBit(inner) = expr {
            let one = SemanticExpression::Constant(ConstantData::u64(1));
            return Some(SemanticExpression::BitwiseAnd(
                inner.clone(),
                Box::new(SemanticExpression::Subtract(
                    inner.clone(),
                    Box::new(one),
                )),
            ));
        }
        None
    }
}
