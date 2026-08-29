use sir_generation::candidate::Candidate;
use sir_transform::ids::{DefinitionId, VariableId};

use crate::obligation::ProofObligation;
use crate::registry::TransformationDefinition;
use crate::semantic::expression::SemanticExpression;
use crate::semantic::theorem::Theorem;

/// The IsolateLowestSetBit transformation: `x & -x` isolates the lowest set bit.
///
/// Theorem:
///   LowestSetBit(x) ≡ BitwiseAnd(x, Subtract(0, x))
///
/// Recognized as the `LowestSetBit` semantic operation; the rewrite recipe
/// selects the `blsi` intrinsic at the instruction-selection layer.
pub struct IsolateLowestSetBitDefinition {
    id: DefinitionId,
}

impl IsolateLowestSetBitDefinition {
    pub fn new(id: DefinitionId) -> Self {
        Self { id }
    }
}

impl TransformationDefinition for IsolateLowestSetBitDefinition {
    fn id(&self) -> DefinitionId {
        self.id
    }

    fn name(&self) -> &'static str {
        "Isolate Lowest Set Bit"
    }

    fn applicability(&self, _candidate: &Candidate) -> bool {
        true
    }

    fn obligation(&self, candidate: &Candidate) -> ProofObligation {
        let x = SemanticExpression::Variable(VariableId::new(0));
        let zero = SemanticExpression::Constant(sir_types::ConstantData::u64(0));

        // The candidate expression: `x & (0 - x)` == `x & -x`.
        let candidate_expr = SemanticExpression::BitwiseAnd(
            Box::new(x.clone()),
            Box::new(SemanticExpression::Subtract(Box::new(zero), Box::new(x.clone()))),
        );

        ProofObligation {
            id: sir_transform::ids::ObligationId::new(0),
            region: candidate.region,
            definition: self.id,
            candidate: candidate.id,
            theorem: Theorem::new(
                SemanticExpression::LowestSetBit(Box::new(x.clone())),
                candidate_expr,
            ),
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
