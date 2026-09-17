use sir_generation::candidate::Candidate;
use sir_semantics::structure::StructuralDescription;
use sir_transform::ids::{DefinitionId, VariableId};
use sir_types::ConstantData;

use crate::definitions::pow2_bind::{bind_pow2, Pow2Op};
use crate::obligation::{FiniteDomain, ProofObligation, VariableKind, VariableSpec};
use crate::registry::{TransformationDefinition, VerificationStatus};
use crate::semantic::expression::SemanticExpression;
use crate::semantic::theorem::Theorem;

/// Unsigned `x / C` for a constant power-of-two `C` is `x >> log2(C)`.
/// Signed operands are refused (rounding toward zero is not a logical
/// shift).
pub struct DivideShiftDefinition {
    id: DefinitionId,
}

impl DivideShiftDefinition {
    pub fn new(id: DefinitionId) -> Self {
        Self { id }
    }
}

impl TransformationDefinition for DivideShiftDefinition {
    fn verification_status(&self) -> VerificationStatus {
        VerificationStatus::ConcreteSolverChecked
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
        self.unbound_obligation(candidate)
    }

    fn obligation_bound(
        &self,
        candidate: &Candidate,
        _function: &sir_nodes::Function,
    ) -> ProofObligation {
        self.unbound_obligation(candidate)
    }

    fn obligation_with_roles(
        &self,
        candidate: &Candidate,
        function: &sir_nodes::Function,
        structural: &StructuralDescription,
    ) -> ProofObligation {
        self.bind(candidate, function, structural)
            .unwrap_or_else(|| self.unbound_obligation(candidate))
    }
}

impl DivideShiftDefinition {
    fn bind(
        &self,
        candidate: &Candidate,
        function: &sir_nodes::Function,
        structural: &StructuralDescription,
    ) -> Option<ProofObligation> {
        let bound = bind_pow2(function, structural, Pow2Op::Div)?;
        let x = VariableId::new(bound.dynamic.as_u64());
        let lhs = SemanticExpression::Divide(
            Box::new(SemanticExpression::Variable(x)),
            Box::new(SemanticExpression::Constant(ConstantData::u64(bound.constant))),
        );
        let rhs = SemanticExpression::ShiftRight(
            Box::new(SemanticExpression::Variable(x)),
            Box::new(SemanticExpression::Constant(ConstantData::u64(
                bound.constant.trailing_zeros() as u64,
            ))),
        );
        Some(self.obligation_with(
            candidate,
            Theorem::new(lhs, rhs),
            Some(FiniteDomain {
                variables: vec![VariableSpec {
                    id: x,
                    kind: VariableKind::BitVector {
                        width: bound.width,
                    },
                }],
            }),
        ))
    }

    fn unbound_obligation(&self, candidate: &Candidate) -> ProofObligation {
        let v = VariableId::new(u64::MAX);
        self.obligation_with(
            candidate,
            Theorem::new(
                SemanticExpression::Variable(v),
                SemanticExpression::Constant(ConstantData::u64(0)),
            ),
            None,
        )
    }

    fn obligation_with(
        &self,
        candidate: &Candidate,
        theorem: Theorem,
        domain: Option<FiniteDomain>,
    ) -> ProofObligation {
        ProofObligation {
            id: sir_transform::ids::ObligationId::new(0),
            region: candidate.region,
            candidate: candidate.id,
            definition: self.id,
            theorem,
            assumptions: vec![],
            domain,
        }
    }
}
