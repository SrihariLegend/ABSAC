use sir_generation::candidate::Candidate;
use sir_semantics::structure::StructuralDescription;
use sir_transform::ids::{DefinitionId, VariableId};
use sir_types::ConstantData;

use crate::definitions::bitscan_forward::{
    collection_id, position_collection_length, sequence_domain,
};
use crate::obligation::{FiniteDomain, ProofObligation};
use crate::registry::{TransformationDefinition, VerificationStatus};
use crate::semantic::expression::SemanticExpression;
use crate::semantic::theorem::Theorem;

/// A reverse position search returns the index of the last true element:
/// `LastTrue(seq) == clz(Pack(seq))` under the lzcnt convention.
pub struct BitScanReverseDefinition {
    id: DefinitionId,
}

impl BitScanReverseDefinition {
    pub fn new(id: DefinitionId) -> Self {
        Self { id }
    }
}

impl TransformationDefinition for BitScanReverseDefinition {
    fn verification_status(&self) -> VerificationStatus {
        // HELD STUB (2026-09-17): the historical obligation
        // `LastTrue(seq) == LeadingZeros(Pack(seq))` is FALSE (the solver
        // returns the counterexample MSB-set: lhs 63 vs rhs 0), and the
        // recipe equally emits LeadingZeros — a latent miscompile the
        // quarantine hid. A correct lift needs a bit-scan-reverse
        // intrinsic (highest set index, width sentinel for zero) plus
        // reverse counted-loop trip-count support in the binding.
        VerificationStatus::Stub
    }

    fn id(&self) -> DefinitionId {
        self.id
    }

    fn name(&self) -> &'static str {
        "LastTrue to LeadingZeroCount"
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
        _function: &sir_nodes::Function,
        _structural: &StructuralDescription,
    ) -> ProofObligation {
        self.unbound_obligation(candidate)
    }
}

impl BitScanReverseDefinition {
    #[allow(dead_code)]
    fn bind(
        &self,
        candidate: &Candidate,
        function: &sir_nodes::Function,
        structural: &StructuralDescription,
    ) -> Option<ProofObligation> {
        let length = position_collection_length(function, structural)?;
        let v = VariableId::new(collection_id(structural)?.as_u64());
        let seq = SemanticExpression::LogicalSequence { variable: v };
        let lhs = SemanticExpression::LastTrue(Box::new(seq.clone()));
        let rhs = SemanticExpression::LeadingZeros(Box::new(SemanticExpression::Pack(
            Box::new(seq),
        )));
        Some(self.obligation_with(
            candidate,
            Theorem::new(lhs, rhs),
            Some(sequence_domain(v, length)),
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
