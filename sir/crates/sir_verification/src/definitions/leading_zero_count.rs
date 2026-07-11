use sir_generation::candidate::Candidate;
use sir_transform::ids::DefinitionId;
use sir_transform::ids::VariableId;
use sir_types::ConstantData;

use crate::obligation::ProofObligation;
use crate::registry::TransformationDefinition;
use crate::semantic::expression::SemanticExpression;
use crate::semantic::theorem::Theorem;

pub struct LeadingZeroCountDefinition {
    id: DefinitionId,
}

impl LeadingZeroCountDefinition {
    pub fn new(id: DefinitionId) -> Self {
        Self { id }
    }
}

impl TransformationDefinition for LeadingZeroCountDefinition {
    fn id(&self) -> DefinitionId {
        self.id
    }

    fn name(&self) -> &'static str {
        "LeadingZeroSearch to LeadingZeroCount"
    }

    fn applicability(&self, _candidate: &Candidate) -> bool {
        true
    }

    fn obligation(&self, candidate: &Candidate) -> ProofObligation {
        let var = VariableId::new(0); // stub
        ProofObligation {
            id: sir_transform::ids::ObligationId::new(0),
            region: candidate.region,
            candidate: candidate.id,
            definition: self.id,
            theorem: Theorem::new(
                SemanticExpression::LeadingZeros(Box::new(SemanticExpression::Variable(var))),
                SemanticExpression::LeadingZeros(Box::new(SemanticExpression::Variable(var))),
            ),
            assumptions: vec![],
            domain: None,
        }
    }
}
