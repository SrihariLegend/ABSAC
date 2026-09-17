use sir_generation::candidate::Candidate;
use sir_semantics::structure::StructuralDescription;
use sir_transform::ids::{DefinitionId, VariableId};
use sir_types::ConstantData;

use crate::definitions::rotate_bind::{bind_rotate, RotateDir};
use crate::obligation::{FiniteDomain, ProofObligation, VariableKind, VariableSpec};
use crate::registry::{TransformationDefinition, VerificationStatus};
use crate::semantic::expression::SemanticExpression;
use crate::semantic::theorem::Theorem;

/// `(x >> k) | (x << (W - k))` is a right rotation by a CONSTANT `k`.
pub struct RotateRightDefinition {
    id: DefinitionId,
}

impl RotateRightDefinition {
    pub fn new(id: DefinitionId) -> Self {
        Self { id }
    }
}

impl TransformationDefinition for RotateRightDefinition {
    fn verification_status(&self) -> VerificationStatus {
        // HOLD (2026-09-17): see RotateLeftDefinition — concrete binding,
        // held Stub until CircularPermutation candidates are generated.
        VerificationStatus::Stub
    }

    fn id(&self) -> DefinitionId {
        self.id
    }

    fn name(&self) -> &'static str {
        "Rotate Right"
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

impl RotateRightDefinition {
    fn bind(
        &self,
        candidate: &Candidate,
        function: &sir_nodes::Function,
        structural: &StructuralDescription,
    ) -> Option<ProofObligation> {
        let bound = bind_rotate(
            function,
            &candidate.authorization.region_nodes,
            structural,
            RotateDir::Right,
        )?;
        let x = VariableId::new(bound.operand.as_u64());
        let k = ConstantData::u64(bound.amount);
        let complement = ConstantData::u64(bound.width as u64 - bound.amount);
        let lhs = SemanticExpression::RotateRight(
            Box::new(SemanticExpression::Variable(x)),
            Box::new(SemanticExpression::Constant(k.clone())),
        );
        let rhs = SemanticExpression::BitwiseOr(
            Box::new(SemanticExpression::ShiftRight(
                Box::new(SemanticExpression::Variable(x)),
                Box::new(SemanticExpression::Constant(k)),
            )),
            Box::new(SemanticExpression::ShiftLeft(
                Box::new(SemanticExpression::Variable(x)),
                Box::new(SemanticExpression::Constant(complement)),
            )),
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
