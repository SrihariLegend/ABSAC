use crate::semantic::expression::SemanticExpression;
use crate::semantic::normalizer::NormalizationRule;
use sir_types::ConstantData;

pub struct DivideToShift;

impl NormalizationRule for DivideToShift {
    fn name(&self) -> &'static str {
        "DivideToShift"
    }

    fn apply(&self, expr: &SemanticExpression) -> Option<SemanticExpression> {
        if let SemanticExpression::Divide(lhs, rhs) = expr {
            if let SemanticExpression::Constant(c) = &**rhs {
                if let Some(v) = c.as_u64() {
                    if v.is_power_of_two() {
                        let shift_amt = v.trailing_zeros() as u64;
                        return Some(SemanticExpression::ShiftRight(
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
