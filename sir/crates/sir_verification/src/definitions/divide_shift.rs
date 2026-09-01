use sir_generation::candidate::Candidate;
use sir_transform::ids::DefinitionId;
use sir_types::ConstantData;

use crate::obligation::ProofObligation;
use crate::registry::TransformationDefinition;
use crate::semantic::expression::SemanticExpression;
use crate::semantic::theorem::Theorem;

pub struct DivideShiftDefinition {
    id: DefinitionId,
}

impl DivideShiftDefinition {
    pub fn new(id: DefinitionId) -> Self {
        Self { id }
    }
}

impl TransformationDefinition for DivideShiftDefinition {
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
        "Divide Power of Two to Bitwise Shift Right"
    }

    fn applicability(&self, _candidate: &Candidate) -> bool {
        true
    }

    fn obligation(&self, candidate: &Candidate) -> ProofObligation {
        ProofObligation {
            id: sir_transform::ids::ObligationId::new(0),
            region: candidate.region,
            candidate: candidate.id,
            definition: self.id,
            theorem: Theorem::new(
                SemanticExpression::Constant(ConstantData::u64(0)),
                SemanticExpression::Constant(ConstantData::u64(0)), // trivially equal stub
            ),
            assumptions: vec![],
            domain: None,
        }
    }
}
