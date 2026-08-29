use crate::semantic::expression::SemanticExpression;
use crate::semantic::normalizer::NormalizationRule;
use sir_types::ConstantData;

/// Rewrite: `RotateLeft(x, k)` → `Or(Shl(x, k), Shr(x, Sub(64, k)))`.
///
/// The mathematical identity behind the HD003 rotate-left proof. Valid for
/// 64-bit words; the width constant matches the definition's domain.
#[derive(Clone, Debug)]
pub struct RotateLeftToShiftPair;

impl NormalizationRule for RotateLeftToShiftPair {
    fn name(&self) -> &'static str {
        "RotateLeftToShiftPair"
    }

    fn apply(&self, expr: &SemanticExpression) -> Option<SemanticExpression> {
        match expr {
            SemanticExpression::RotateLeft(x, k) => {
                let width = SemanticExpression::Constant(ConstantData::u64(64));
                Some(SemanticExpression::BitwiseOr(
                    Box::new(SemanticExpression::ShiftLeft(x.clone(), k.clone())),
                    Box::new(SemanticExpression::ShiftRight(
                        x.clone(),
                        Box::new(SemanticExpression::Subtract(Box::new(width), k.clone())),
                    )),
                ))
            }
            _ => None,
        }
    }
}
