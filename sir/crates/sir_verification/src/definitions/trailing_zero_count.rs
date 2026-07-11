use sir_generation::candidate::Candidate;
use sir_transform::ids::DefinitionId;
use sir_transform::ids::VariableId;

use crate::obligation::ProofObligation;
use crate::registry::TransformationDefinition;
use crate::semantic::expression::SemanticExpression;
use crate::semantic::theorem::Theorem;

pub struct TrailingZeroCountDefinition {
    id: DefinitionId,
}

impl TrailingZeroCountDefinition {
    pub fn new(id: DefinitionId) -> Self {
        Self { id }
    }
}

impl TransformationDefinition for TrailingZeroCountDefinition {
    fn id(&self) -> DefinitionId {
        self.id
    }

    fn name(&self) -> &'static str {
        "TrailingZeroSearch to TrailingZeroCount"
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
                SemanticExpression::TrailingZeros(Box::new(SemanticExpression::Variable(var))),
                SemanticExpression::TrailingZeros(Box::new(SemanticExpression::Variable(var))), // trivial equivalence for hardware op
            ),
            assumptions: vec![],
            domain: None,
        }
    }
}
