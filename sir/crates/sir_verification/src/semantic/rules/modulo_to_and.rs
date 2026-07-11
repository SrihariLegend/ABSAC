use crate::semantic::expression::SemanticExpression;
use crate::semantic::normalizer::NormalizationRule;
use sir_types::ConstantData;

pub struct ModuloToAnd;

impl NormalizationRule for ModuloToAnd {
    fn name(&self) -> &'static str {
        "ModuloToAnd"
    }

    fn apply(&self, expr: &SemanticExpression) -> Option<SemanticExpression> {
        if let SemanticExpression::Modulo(lhs, rhs) = expr {
            if let SemanticExpression::Constant(c) = &**rhs {
                if let Some(v) = c.as_u64() {
                    if v.is_power_of_two() {
                        return Some(SemanticExpression::BitwiseAnd(
                            lhs.clone(),
                            Box::new(SemanticExpression::Constant(ConstantData::u64(v - 1))),
                        ));
                    }
                }
            }
        }
        None
    }
}
