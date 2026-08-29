use sir_generation::candidate::Candidate;
use sir_transform::ids::{DefinitionId, VariableId};

use crate::obligation::ProofObligation;
use crate::registry::TransformationDefinition;
use crate::semantic::expression::SemanticExpression;
use crate::semantic::theorem::Theorem;

/// The SetLowestClearBit transformation: `x | (x + 1)` sets the lowest clear
/// (zero) bit of `x`.
///
/// Theorem:
///   SetLowestClearBit(x) ≡ BitwiseOr(x, Add(x, 1))
///
/// The semantic operation is target-independent; the recipe selects the
/// instruction composition (e.g. `x | blsmsk(~x)`) at the instruction
/// selection layer.
pub struct SetLowestClearBitDefinition {
    id: DefinitionId,
}

impl SetLowestClearBitDefinition {
    pub fn new(id: DefinitionId) -> Self {
        Self { id }
    }
}

impl TransformationDefinition for SetLowestClearBitDefinition {
    fn id(&self) -> DefinitionId {
        self.id
    }

    fn name(&self) -> &'static str {
        "Set Lowest Clear Bit"
    }

    fn applicability(&self, _candidate: &Candidate) -> bool {
        true
    }

    fn obligation(&self, candidate: &Candidate) -> ProofObligation {
        let x = SemanticExpression::Variable(VariableId::new(0));
        let one = SemanticExpression::Constant(sir_types::ConstantData::u64(1));

        // The candidate expression: `x | (x + 1)`.
        let candidate_expr = SemanticExpression::BitwiseOr(
            Box::new(x.clone()),
            Box::new(SemanticExpression::Add(Box::new(x.clone()), Box::new(one))),
        );

        ProofObligation {
            id: sir_transform::ids::ObligationId::new(0),
            region: candidate.region,
            definition: self.id,
            candidate: candidate.id,
            theorem: Theorem::new(
                SemanticExpression::SetLowestClearBit(Box::new(x.clone())),
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
