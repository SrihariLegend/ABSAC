use crate::semantic::expression::SemanticExpression;
use crate::semantic::normalizer::NormalizationRule;
use sir_types::ConstantData;

/// Rewrite: `ByteSwap(x)` → `Or(Shl(And(x, 0xFF), 8), And(Shr(x, 8), 0xFF))`.
///
/// The mathematical identity behind the HD004 byte-swap proof: swapping the
/// two low bytes of a 32-bit word is the masked shift pair
/// `((x & 0xFF) << 8) | ((x >> 8) & 0xFF)`. The recipe's instruction
/// (`bswap` + width alignment) is selected after this identity is proven.
#[derive(Clone, Debug)]
pub struct ByteSwapToMaskedShiftPair;

impl NormalizationRule for ByteSwapToMaskedShiftPair {
    fn name(&self) -> &'static str {
        "ByteSwapToMaskedShiftPair"
    }

    fn apply(&self, expr: &SemanticExpression) -> Option<SemanticExpression> {
        match expr {
            SemanticExpression::ByteSwap(x) => {
                let mask = SemanticExpression::Constant(ConstantData::u64(0xFF));
                let eight = SemanticExpression::Constant(ConstantData::u64(8));
                Some(SemanticExpression::BitwiseOr(
                    Box::new(SemanticExpression::ShiftLeft(
                        Box::new(SemanticExpression::BitwiseAnd(x.clone(), Box::new(mask.clone()))),
                        Box::new(eight.clone()),
                    )),
                    Box::new(SemanticExpression::BitwiseAnd(
                        Box::new(SemanticExpression::ShiftRight(x.clone(), Box::new(eight))),
                        Box::new(mask),
                    )),
                ))
            }
            _ => None,
        }
    }
}
