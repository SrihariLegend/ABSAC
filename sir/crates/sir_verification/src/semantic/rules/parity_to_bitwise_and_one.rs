use crate::semantic::expression::SemanticExpression;
use crate::semantic::normalizer::NormalizationRule;

/// Rewrite: Parity(BooleanArray(v)) → BitwiseAndOne(Popcount(Pack(BooleanArray(v))))
///
/// This is the mathematical identity that powers the BS004 proof.
#[derive(Clone, Debug)]
pub struct ParityToBitwiseAndOne;

impl NormalizationRule for ParityToBitwiseAndOne {
    fn name(&self) -> &'static str {
        "ParityToBitwiseAndOne"
    }

    fn apply(&self, expr: &SemanticExpression) -> Option<SemanticExpression> {
        match expr {
            SemanticExpression::Parity(inner) => match inner.as_ref() {
                SemanticExpression::BooleanArray { variable } => {
                    Some(SemanticExpression::BitwiseAndOne(Box::new(
                        SemanticExpression::Popcount(Box::new(
                            SemanticExpression::Pack(Box::new(
                                SemanticExpression::BooleanArray {
                                    variable: *variable,
                                },
                            )),
                        ))
                    )))
                }
                _ => None,
            },
            _ => None,
        }
    }
}
