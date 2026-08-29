use sir_generation::candidate::Candidate;
use sir_transform::ids::{DefinitionId, VariableId};

use crate::obligation::ProofObligation;
use crate::registry::TransformationDefinition;
use crate::semantic::expression::SemanticExpression;
use crate::semantic::theorem::Theorem;

pub struct ClearLowestSetBitDefinition {
    id: DefinitionId,
}

impl ClearLowestSetBitDefinition {
    pub fn new(id: DefinitionId) -> Self {
        Self { id }
    }
}

impl TransformationDefinition for ClearLowestSetBitDefinition {
    fn id(&self) -> DefinitionId {
        self.id
    }

    fn name(&self) -> &'static str {
        "Clear Lowest Set Bit"
    }

    fn applicability(&self, _candidate: &Candidate) -> bool {
        true
    }

    fn obligation(&self, candidate: &Candidate) -> ProofObligation {
        let x = SemanticExpression::Variable(VariableId::new(0));
        let one = SemanticExpression::Constant(sir_types::ConstantData::u64(1));

        let lhs = SemanticExpression::ClearLowestSetBit(Box::new(x.clone()));

        // The candidate expression from Generation strategy
        let candidate_expr = SemanticExpression::BitwiseAnd(
            Box::new(x.clone()),
            Box::new(SemanticExpression::Subtract(Box::new(x), Box::new(one))),
        );

        ProofObligation {
            id: sir_transform::ids::ObligationId::new(0),
            region: candidate.region,
            definition: self.id,
            candidate: candidate.id,
            theorem: Theorem::new(lhs, candidate_expr),
            assumptions: vec![],
            domain: Some(crate::obligation::FiniteDomain {
                variables: vec![crate::obligation::VariableSpec {
                    id: VariableId::new(0),
                    kind: crate::obligation::VariableKind::BitVector { width: 64 },
                }],
            }),
        }
    }
}
