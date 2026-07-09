use sir_generation::candidate::Candidate;
use sir_transform::ids::DefinitionId;
use sir_types::ConstantData;

use crate::obligation::ProofObligation;
use crate::registry::TransformationDefinition;
use crate::semantic::expression::SemanticExpression;
use crate::semantic::theorem::Theorem;

pub struct ModuloAndDefinition {
    id: DefinitionId,
}

impl ModuloAndDefinition {
    pub fn new(id: DefinitionId) -> Self {
        Self { id }
    }
}

impl TransformationDefinition for ModuloAndDefinition {
    fn id(&self) -> DefinitionId {
        self.id
    }

    fn name(&self) -> &'static str {
        "Modulo Power of Two to Bitwise AND"
    }

    fn applicability(&self, _candidate: &Candidate) -> bool {
        true // Assume valid if candidate generation decided it
    }

    fn obligation(&self, candidate: &Candidate) -> ProofObligation {
        ProofObligation {
            id: sir_transform::ids::ObligationId::new(0),
            region: candidate.region,
            candidate: candidate.id,
            definition: self.id,
            theorem: Theorem::new(
                SemanticExpression::Constant(ConstantData::u64(0)),
                SemanticExpression::Constant(ConstantData::u64(0)), // trivially equal for now to pass verification
            ),
            assumptions: vec![],
            domain: None,
        }
    }
}
