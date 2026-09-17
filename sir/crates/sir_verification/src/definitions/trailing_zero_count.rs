use sir_generation::candidate::Candidate;
use sir_semantics::structure::StructuralDescription;
use sir_transform::ids::{DefinitionId, VariableId};
use sir_transform::roles::RegionRoles;
use sir_types::{ConstantData, Type};

use crate::obligation::{FiniteDomain, ProofObligation, VariableKind, VariableSpec};
use crate::registry::{TransformationDefinition, VerificationStatus};
use crate::semantic::expression::SemanticExpression;
use crate::semantic::theorem::Theorem;

/// The trailing-zero loop (`while (x & 1) == 0 { x >>= 1; n += 1 }`)
/// returns the index of the lowest set bit, or the width when none:
/// exactly `FirstTrue` over the value's bit sequence, which the intrinsic
/// computes as `TrailingZeros(x)` (tzcnt convention for zero). The
/// obligation binds the loop's scalar and width from the recognized role
/// and the solver proves the two scan constructions agree.
pub struct TrailingZeroCountDefinition {
    id: DefinitionId,
}

impl TrailingZeroCountDefinition {
    pub fn new(id: DefinitionId) -> Self {
        Self { id }
    }
}

impl TransformationDefinition for TrailingZeroCountDefinition {
    fn verification_status(&self) -> VerificationStatus {
        VerificationStatus::ConcreteSolverChecked
    }

    fn id(&self) -> DefinitionId {
        self.id
    }

    fn name(&self) -> &'static str {
        "TrailingZeroSearch to TrailingZeroCount"
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

impl TrailingZeroCountDefinition {
    fn bind(
        &self,
        candidate: &Candidate,
        function: &sir_nodes::Function,
        structural: &StructuralDescription,
    ) -> Option<ProofObligation> {
        let (scalar, width) = scalar_and_width(function, structural)?;
        let x = VariableId::new(scalar.as_u64());
        let lhs = SemanticExpression::TrailingZeros(Box::new(SemanticExpression::Variable(x)));
        let rhs = SemanticExpression::FirstTrue(Box::new(SemanticExpression::LogicalSequence {
            variable: x,
        }));
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

/// The loop's scalar value and its unsigned integer width, from the
/// recognized PositionSearch (or SetIteration) role.
pub(crate) fn scalar_and_width(
    function: &sir_nodes::Function,
    structural: &StructuralDescription,
) -> Option<(sir_types::NodeId, usize)> {
    for role in &structural.roles {
        let scalar = match role {
            RegionRoles::PositionSearch { scalar, .. } => (*scalar)?,
            RegionRoles::SetIteration { set_value, .. } => *set_value,
            _ => continue,
        };
        let (width, signed) = match function.get_node(scalar).map(|n| &n.ty) {
            Some(Type::Integer { width, signed, .. }) => (width.bits() as usize, *signed),
            _ => continue,
        };
        if signed || width == 0 || width > 64 {
            continue;
        }
        return Some((scalar, width));
    }
    None
}

pub(crate) fn sequence_domain(id: VariableId, width: usize) -> FiniteDomain {
    FiniteDomain {
        variables: vec![VariableSpec {
            id,
            kind: VariableKind::LogicalSequence { length: width },
        }],
    }
}
