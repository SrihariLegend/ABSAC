use sir_generation::candidate::Candidate;
use sir_semantics::structure::StructuralDescription;
use sir_transform::ids::{DefinitionId, VariableId};
use sir_types::ConstantData;

use crate::definitions::trailing_zero_count::{scalar_and_width, sequence_domain};
use crate::obligation::{FiniteDomain, ProofObligation};
use crate::registry::{TransformationDefinition, VerificationStatus};
use crate::semantic::expression::SemanticExpression;
use crate::semantic::theorem::Theorem;

/// The leading-zero loop (`while (value & mask) == 0 { mask >>= 1; n += 1 }`
/// from the MSB) returns the number of high zero bits: exactly `FirstTrue`
/// over the REVERSED bit sequence, which the intrinsic computes as
/// `LeadingZeros(x)` (lzcnt convention for zero).
pub struct LeadingZeroCountDefinition {
    id: DefinitionId,
}

impl LeadingZeroCountDefinition {
    pub fn new(id: DefinitionId) -> Self {
        Self { id }
    }
}

impl TransformationDefinition for LeadingZeroCountDefinition {
    fn verification_status(&self) -> VerificationStatus {
        VerificationStatus::ConcreteSolverChecked
    }

    fn id(&self) -> DefinitionId {
        self.id
    }

    fn name(&self) -> &'static str {
        "LeadingZeroSearch to LeadingZeroCount"
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

impl LeadingZeroCountDefinition {
    fn bind(
        &self,
        candidate: &Candidate,
        function: &sir_nodes::Function,
        structural: &StructuralDescription,
    ) -> Option<ProofObligation> {
        let (scalar, width) = scalar_and_width(function, structural)?;
        let x = VariableId::new(scalar.as_u64());
        let lhs = SemanticExpression::LeadingZeros(Box::new(SemanticExpression::Variable(x)));
        let rhs = SemanticExpression::FirstTrue(Box::new(SemanticExpression::Reverse(Box::new(
            SemanticExpression::LogicalSequence { variable: x },
        ))));
        Some(self.obligation_with(
            candidate,
            Theorem::new(lhs, rhs),
            Some(sequence_domain(x, width)),
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
