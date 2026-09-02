use sir_generation::candidate::Candidate;
use sir_transform::ids::{DefinitionId, VariableId};
use sir_types::ConstantData;

use crate::obligation::ProofObligation;
use crate::registry::TransformationDefinition;
use crate::semantic::expression::SemanticExpression;
use crate::semantic::theorem::Theorem;

/// The RotateRight transformation: `(x >> k) | (x << (64 - k))` is a right
/// rotation of `x` by `k`.
///
/// Theorem (over 64-bit words):
///   RotateRight(x, k) ≡ Or(Shr(x, k), Shl(x, Sub(64, k)))
///
/// The semantic concept is the circular permutation; the recipe selects the
/// `ror` instruction only after this identity is proven.
pub struct RotateRightDefinition {
    id: DefinitionId,
}

impl RotateRightDefinition {
    pub fn new(id: DefinitionId) -> Self {
        Self { id }
    }
}

impl TransformationDefinition for RotateRightDefinition {
    fn verification_status(&self) -> crate::registry::VerificationStatus {
        // STUB (quarantined, advisor P0 audit): the obligation is a
        // hardcoded or tautological theorem template that never binds
        // the actual source/candidate operands — changing the region's
        // constant, swapping operands, or changing widths cannot cause
        // rejection. This definition may not authorize a rewrite until
        // its obligation is built from the actual pair and discharged
        // concretely.
        crate::registry::VerificationStatus::Stub
    }

    fn id(&self) -> DefinitionId {
        self.id
    }

    fn name(&self) -> &'static str {
        "Rotate Right"
    }

    fn applicability(&self, _candidate: &Candidate) -> bool {
        true
    }

    fn obligation(&self, candidate: &Candidate) -> ProofObligation {
        let x = SemanticExpression::Variable(VariableId::new(0));
        let k = SemanticExpression::Variable(VariableId::new(1));
        let width = SemanticExpression::Constant(ConstantData::u64(64));

        let candidate_expr = SemanticExpression::BitwiseOr(
            Box::new(SemanticExpression::ShiftRight(
                Box::new(x.clone()),
                Box::new(k.clone()),
            )),
            Box::new(SemanticExpression::ShiftLeft(
                Box::new(x.clone()),
                Box::new(SemanticExpression::Subtract(
                    Box::new(width),
                    Box::new(k.clone()),
                )),
            )),
        );

        ProofObligation {
            id: sir_transform::ids::ObligationId::new(0),
            region: candidate.region,
            definition: self.id,
            candidate: candidate.id,
            theorem: Theorem::new(
                SemanticExpression::RotateRight(Box::new(x), Box::new(k)),
                candidate_expr,
            ),
            assumptions: vec![],
            domain: None,
        }
    }
}
