use crate::semantic::expression::SemanticExpression;
use crate::semantic::normalizer::NormalizationRule;
use sir_types::ConstantData;

pub struct MultiplyToShift;

impl NormalizationRule for MultiplyToShift {
    fn name(&self) -> &'static str {
        "MultiplyToShift"
    }

    fn apply(&self, expr: &SemanticExpression) -> Option<SemanticExpression> {
        if let SemanticExpression::Multiply(lhs, rhs) = expr {
            if let SemanticExpression::Constant(c) = &**rhs {
                if let Some(v) = c.as_u64() {
                    if v.is_power_of_two() {
                        let shift_amt = v.trailing_zeros() as u64;
                        return Some(SemanticExpression::ShiftLeft(
                            lhs.clone(),
                            Box::new(SemanticExpression::Constant(ConstantData::u64(shift_amt))),
                        ));
                    }
                }
            }
        }
        None
    }
}
