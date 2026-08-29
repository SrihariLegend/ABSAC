use crate::semantic::expression::SemanticExpression;
use crate::semantic::normalizer::NormalizationRule;
use sir_types::ConstantData;

/// Rewrite: `RotateRight(x, k)` → `Or(Shr(x, k), Shl(x, Sub(64, k)))`.
///
/// The mathematical identity behind the HD003 rotate-right proof. Valid for
/// 64-bit words; the width constant matches the definition's domain.
#[derive(Clone, Debug)]
pub struct RotateRightToShiftPair;

impl NormalizationRule for RotateRightToShiftPair {
    fn name(&self) -> &'static str {
        "RotateRightToShiftPair"
    }

    fn apply(&self, expr: &SemanticExpression) -> Option<SemanticExpression> {
        match expr {
            SemanticExpression::RotateRight(x, k) => {
                let width = SemanticExpression::Constant(ConstantData::u64(64));
                Some(SemanticExpression::BitwiseOr(
                    Box::new(SemanticExpression::ShiftRight(x.clone(), k.clone())),
                    Box::new(SemanticExpression::ShiftLeft(
                        x.clone(),
                        Box::new(SemanticExpression::Subtract(Box::new(width), k.clone())),
                    )),
                ))
            }
            _ => None,
        }
    }
}
