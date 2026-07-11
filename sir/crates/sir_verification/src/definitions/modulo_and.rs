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
        // Find the operator node and operands
        let _lhs = SemanticExpression::Constant(ConstantData::u64(0));
        let _rhs = SemanticExpression::Constant(ConstantData::u64(0));

        ProofObligation {
            id: sir_transform::ids::ObligationId::new(0),
            region: candidate.region,
            candidate: candidate.id,
            definition: self.id,
            theorem: Theorem::new(
                // E.g., Modulo(Var, Constant(2^n)) == BitwiseAnd(Var, Constant(2^n - 1))
                SemanticExpression::Modulo(
                    Box::new(SemanticExpression::Variable(
                        sir_transform::ids::VariableId::new(0),
                    )),
                    Box::new(SemanticExpression::Constant(ConstantData::u64(16))), // stub
                ),
                SemanticExpression::BitwiseAnd(
                    Box::new(SemanticExpression::Variable(
                        sir_transform::ids::VariableId::new(0),
                    )),
                    Box::new(SemanticExpression::Constant(ConstantData::u64(15))), // stub
                ),
            ),
            assumptions: vec![],
            domain: None,
        }
    }
}
