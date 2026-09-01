use sir_generation::candidate::Candidate;
use sir_transform::ids::{DefinitionId, VariableId};

use crate::obligation::ProofObligation;
use crate::registry::TransformationDefinition;
use crate::semantic::expression::SemanticExpression;
use crate::semantic::theorem::Theorem;

/// The IsolateLowestClearBit transformation: `~x & (x + 1)` isolates the
/// lowest clear (zero) bit of `x`.
///
/// Theorem:
///   LowestClearBitMask(x) ≡ BitwiseAnd(BitwiseNot(x), Add(x, 1))
///
/// The semantic operation is target-independent; the recipe selects the
/// instruction composition (e.g. `blsi` applied to `~x`) at the
/// instruction-selection layer.
pub struct IsolateLowestClearBitDefinition {
    id: DefinitionId,
}

impl IsolateLowestClearBitDefinition {
    pub fn new(id: DefinitionId) -> Self {
        Self { id }
    }
}

impl TransformationDefinition for IsolateLowestClearBitDefinition {
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
        "Isolate Lowest Clear Bit"
    }

    fn applicability(&self, _candidate: &Candidate) -> bool {
        true
    }

    fn obligation(&self, candidate: &Candidate) -> ProofObligation {
        let x = SemanticExpression::Variable(VariableId::new(0));
        let one = SemanticExpression::Constant(sir_types::ConstantData::u64(1));

        // The candidate expression: `~x & (x + 1)`.
        let candidate_expr = SemanticExpression::BitwiseAnd(
            Box::new(SemanticExpression::BitwiseNot(Box::new(x.clone()))),
            Box::new(SemanticExpression::Add(Box::new(x.clone()), Box::new(one))),
        );

        ProofObligation {
            id: sir_transform::ids::ObligationId::new(0),
            region: candidate.region,
            definition: self.id,
            candidate: candidate.id,
            theorem: Theorem::new(
                SemanticExpression::LowestClearBitMask(Box::new(x.clone())),
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
