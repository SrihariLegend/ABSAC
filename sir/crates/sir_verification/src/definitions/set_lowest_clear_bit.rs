use sir_generation::candidate::Candidate;
use sir_transform::ids::{DefinitionId, VariableId};
use sir_types::ConstantData;

use crate::definitions::mask_bind::{bind_mask, MaskPattern};
use crate::obligation::{FiniteDomain, ProofObligation, VariableKind, VariableSpec};
use crate::registry::{TransformationDefinition, VerificationStatus};
use crate::semantic::expression::SemanticExpression;
use crate::semantic::theorem::Theorem;

/// `x | (x + 1)` sets the lowest clear bit.
pub struct SetLowestClearBitDefinition {
    id: DefinitionId,
}

impl SetLowestClearBitDefinition {
    pub fn new(id: DefinitionId) -> Self {
        Self { id }
    }
}

impl TransformationDefinition for SetLowestClearBitDefinition {
    fn verification_status(&self) -> VerificationStatus {
        // UPGRADED (2026-09-17): bound to the actual `x | (x + 1)`
        // pattern and discharged by the concrete solver.
        VerificationStatus::ConcreteSolverChecked
    }

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

impl SetLowestClearBitDefinition {
    fn bind(
        &self,
        candidate: &Candidate,
        function: &sir_nodes::Function,
    ) -> Option<ProofObligation> {
        let bound = bind_mask(
            function,
            &candidate.authorization.region_nodes,
            MaskPattern::SetLowestClearBit,
        )?;
        let x = VariableId::new(bound.operand.as_u64());
        let one = SemanticExpression::Constant(ConstantData::u64(1));
        let lhs = SemanticExpression::SetLowestClearBit(Box::new(
            SemanticExpression::Variable(x),
        ));
        let rhs = SemanticExpression::BitwiseOr(
            Box::new(SemanticExpression::Variable(x)),
            Box::new(SemanticExpression::Add(
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
