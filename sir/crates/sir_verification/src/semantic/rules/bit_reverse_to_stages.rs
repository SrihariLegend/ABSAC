use crate::semantic::expression::SemanticExpression;
use crate::semantic::normalizer::NormalizationRule;
use sir_types::ConstantData;

/// Rewrite: `BitReverse(x)` → the three-stage swap chain of HD005.
///
/// `BitReverse(x)` over an 8-bit slot expands to the classic
/// swap-adjacent-bits / swap-pairs / swap-nibbles pipeline:
///
///   S1(x) = Or(Shl(And(x, 0x55), 1), And(Shr(x, 1), 0x55))
///   S2    = Or(Shl(And(S1, 0x33), 2), And(Shr(S1, 2), 0x33))
///   S3    = Or(Shl(And(S2, 0x0F), 4), And(Shr(S2, 4), 0x0F))
///
/// The recipe's `rbit` instruction is selected only after this identity is
/// proven.
#[derive(Clone, Debug)]
pub struct BitReverseToStages;

impl NormalizationRule for BitReverseToStages {
    fn name(&self) -> &'static str {
        "BitReverseToStages"
    }

    fn apply(&self, expr: &SemanticExpression) -> Option<SemanticExpression> {
        match expr {
            SemanticExpression::BitReverse(x) => Some(stage3(stage2(stage1((**x).clone())))),
            _ => None,
        }
    }
}

fn mask(v: u64) -> SemanticExpression {
    SemanticExpression::Constant(ConstantData::u64(v))
}

pub fn stage1(x: SemanticExpression) -> SemanticExpression {
    swap_stage(x, 0x55, 1)
}

pub fn stage2(x: SemanticExpression) -> SemanticExpression {
    swap_stage(x, 0x33, 2)
}

pub fn stage3(x: SemanticExpression) -> SemanticExpression {
    swap_stage(x, 0x0F, 4)
}

/// One swap stage: `Or(Shl(And(x, m), s), And(Shr(x, s), m))`.
fn swap_stage(x: SemanticExpression, m: u64, s: u64) -> SemanticExpression {
    let mask_e = mask(m);
    let shift_e = mask(s);
    SemanticExpression::BitwiseOr(
        Box::new(SemanticExpression::ShiftLeft(
            Box::new(SemanticExpression::BitwiseAnd(Box::new(x.clone()), Box::new(mask_e.clone()))),
            Box::new(shift_e.clone()),
        )),
        Box::new(SemanticExpression::BitwiseAnd(
            Box::new(SemanticExpression::ShiftRight(Box::new(x), Box::new(shift_e))),
            Box::new(mask_e),
        )),
    )
}
