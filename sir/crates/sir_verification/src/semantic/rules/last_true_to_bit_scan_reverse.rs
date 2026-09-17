use crate::semantic::expression::SemanticExpression;
use crate::semantic::normalizer::NormalizationRule;

/// `LastTrue(seq)` is the highest set index of `Pack(seq)` (with the
/// width sentinel for an all-false sequence) — i.e. `BitScanReverse`,
/// NOT `LeadingZeros`. The historical rule rewrote it to `LeadingZeros`
/// and codified a false equation (MSB-set mask: LastTrue = 63 vs
/// clz = 0).
pub struct LastTrueToBitScanReverse;

impl NormalizationRule for LastTrueToBitScanReverse {
    fn name(&self) -> &'static str {
        "LastTrueToBitScanReverse"
    }

    fn apply(&self, expr: &SemanticExpression) -> Option<SemanticExpression> {
        if let SemanticExpression::LastTrue(inner) = expr {
            return Some(SemanticExpression::BitScanReverse(Box::new(
                SemanticExpression::Pack(inner.clone()),
            )));
        }
        None
    }
}
