use sir_generation::candidate::Candidate;
use sir_transform::ids::{DefinitionId, VariableId};

use crate::obligation::ProofObligation;
use crate::registry::TransformationDefinition;
use crate::semantic::expression::SemanticExpression;
use crate::semantic::theorem::Theorem;

use crate::semantic::rules::bit_reverse_to_stages::{stage1, stage2, stage3};

/// The BitReverse transformation: reversing the low 8 bits is the three-stage
/// swap pipeline of HD005.
///
/// Theorem (over an 8-bit slot):
///   BitReverse(x) ≡ S3(S2(S1(x)))
///
/// The semantic concept is the bit permutation; the recipe selects the `rbit`
/// instruction only after this identity is proven.
pub struct BitReverseDefinition {
    id: DefinitionId,
}

impl BitReverseDefinition {
    pub fn new(id: DefinitionId) -> Self {
        Self { id }
    }
}

impl TransformationDefinition for BitReverseDefinition {
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
        "Reverse Bits"
    }

    fn applicability(&self, _candidate: &Candidate) -> bool {
        true
    }

    fn obligation(&self, candidate: &Candidate) -> ProofObligation {
        let x = SemanticExpression::Variable(VariableId::new(0));
        let candidate_expr = stage3(stage2(stage1(x.clone())));

        ProofObligation {
            id: sir_transform::ids::ObligationId::new(0),
            region: candidate.region,
            definition: self.id,
            candidate: candidate.id,
            theorem: Theorem::new(SemanticExpression::BitReverse(Box::new(x)), candidate_expr),
            assumptions: vec![],
            domain: None,
        }
    }
}
