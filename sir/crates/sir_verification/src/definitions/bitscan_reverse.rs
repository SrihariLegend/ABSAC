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
/// `LastTrue(seq) == BitScanReverse(Pack(seq))` (highest set index, width
/// sentinel for zero). The historical obligation claimed
/// `LastTrue(seq) == LeadingZeros(Pack(seq))`, which is FALSE.
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
        // HELD STUB (2026-09-17): the theorem is now CORRECT
        // (`BitScanReverse`, proved by the concrete solver), but the
        // definition cannot be lifted until the remaining application
        // blockers are gone: (a) the recipe still emits `LeadingZeros`
        // — a latent miscompile — so SIR needs a bit-scan-reverse
        // intrinsic (highest set index, width sentinel for zero) and
        // the recipe must emit it; (b) the reduction binding's
        // trip-count contract is forward-only while the reverse search
        // iterates downward; (c) the PS002 kernel itself is unsound
        // (unsigned `i >= 0` underflows when nothing matches, so it
        // never terminates) and a sound reverse kernel is required.
        VerificationStatus::Stub
    }

    fn id(&self) -> DefinitionId {
        self.id
    }

    fn name(&self) -> &'static str {
        "LastTrue to BitScanReverse"
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

impl BitScanReverseDefinition {
    fn bind(
        &self,
        candidate: &Candidate,
        function: &sir_nodes::Function,
        structural: &StructuralDescription,
    ) -> Option<ProofObligation> {
        let length = position_collection_length(function, structural)?;
        let v = VariableId::new(collection_id(structural)?.as_u64());
        let seq = SemanticExpression::LogicalSequence { variable: v };
        // Corrected target: the highest set index of the packed mask
        // (width sentinel for an all-false sequence), NOT clz.
        let lhs = SemanticExpression::BitScanReverse(Box::new(SemanticExpression::Pack(
            Box::new(seq.clone()),
        )));
        let rhs = SemanticExpression::LastTrue(Box::new(seq));
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
