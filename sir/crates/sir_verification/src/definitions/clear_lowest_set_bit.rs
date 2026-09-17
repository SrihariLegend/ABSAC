use sir_generation::candidate::Candidate;
use sir_transform::ids::{DefinitionId, VariableId};
use sir_types::ConstantData;

use crate::definitions::mask_bind::{bind_mask, MaskPattern};
use crate::obligation::{FiniteDomain, ProofObligation, VariableKind, VariableSpec};
use crate::registry::{TransformationDefinition, VerificationStatus};
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
    fn verification_status(&self) -> VerificationStatus {
        // UPGRADED (2026-09-17): the obligation binds the candidate's
        // actual `x & (x - 1)` pattern (operand and width) and the
        // concrete solver proves `ClearLowestSetBit(x) == x & (x-1)`.
        VerificationStatus::ConcreteSolverChecked
    }

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
        self.unbound_obligation(candidate)
    }

    fn obligation_bound(
        &self,
        candidate: &Candidate,
        function: &sir_nodes::Function,
    ) -> ProofObligation {
        self.bind(candidate, function)
            .unwrap_or_else(|| self.unbound_obligation(candidate))
    }
}

impl ClearLowestSetBitDefinition {
    fn bind(
        &self,
        candidate: &Candidate,
        function: &sir_nodes::Function,
    ) -> Option<ProofObligation> {
        let bound = bind_mask(
            function,
            &candidate.authorization.region_nodes,
            MaskPattern::ClearLowestSetBit,
        )?;
        let x = VariableId::new(bound.operand.as_u64());
        let one = SemanticExpression::Constant(ConstantData::u64(1));
        let lhs = SemanticExpression::ClearLowestSetBit(Box::new(
            SemanticExpression::Variable(x),
        ));
        let rhs = SemanticExpression::BitwiseAnd(
            Box::new(SemanticExpression::Variable(x)),
            Box::new(SemanticExpression::Subtract(
                Box::new(SemanticExpression::Variable(x)),
                Box::new(one),
            )),
        );
        Some(self.obligation_with(
            candidate,
            Theorem::new(lhs, rhs),
            Some(domain(x, bound.width)),
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

fn domain(id: VariableId, width: usize) -> FiniteDomain {
    FiniteDomain {
        variables: vec![VariableSpec {
            id,
            kind: VariableKind::BitVector { width },
        }],
    }
}
