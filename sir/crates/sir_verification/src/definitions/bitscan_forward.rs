use sir_generation::candidate::Candidate;
use sir_semantics::structure::StructuralDescription;
use sir_transform::ids::{DefinitionId, VariableId};
use sir_transform::roles::RegionRoles;
use sir_types::{ConstantData, Type};

use crate::obligation::{FiniteDomain, ProofObligation, VariableKind, VariableSpec};
use crate::registry::{TransformationDefinition, VerificationStatus};
use crate::semantic::expression::SemanticExpression;
use crate::semantic::theorem::Theorem;

/// A forward position search returns the index of the first true element
/// (or the length sentinel): `FirstTrue(seq) == ctz(Pack(seq))` under the
/// tzcnt convention. The obligation binds the recognized PositionSearch
/// collection's declared extent.
pub struct BitScanForwardDefinition {
    id: DefinitionId,
}

impl BitScanForwardDefinition {
    pub fn new(id: DefinitionId) -> Self {
        Self { id }
    }
}

impl TransformationDefinition for BitScanForwardDefinition {
    fn verification_status(&self) -> VerificationStatus {
        VerificationStatus::ConcreteSolverChecked
    }

    fn id(&self) -> DefinitionId {
        self.id
    }

    fn name(&self) -> &'static str {
        "FirstTrue to TrailingZeroCount"
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

impl BitScanForwardDefinition {
    fn bind(
        &self,
        candidate: &Candidate,
        function: &sir_nodes::Function,
        structural: &StructuralDescription,
    ) -> Option<ProofObligation> {
        let length = position_collection_length(function, structural)?;
        let v = VariableId::new(collection_id(structural)?.as_u64());
        let seq = SemanticExpression::LogicalSequence { variable: v };
        let lhs = SemanticExpression::FirstTrue(Box::new(seq.clone()));
        let rhs = SemanticExpression::TrailingZeros(Box::new(SemanticExpression::Pack(
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

/// The recognized PositionSearch collection and its declared extent
/// (length ≤ 64 for the solver). Predicate collections are integer
/// arrays; the obligation's sequence is the predicate results, so only
/// the extent is needed.
pub(crate) fn position_collection_length(
    function: &sir_nodes::Function,
    structural: &StructuralDescription,
) -> Option<usize> {
    let collection = collection_id(structural)?;
    match function.get_node(collection).map(|n| &n.ty) {
        Some(Type::Array { length, .. }) if *length > 0 && *length <= 64 => Some(*length),
        _ => None,
    }
}

pub(crate) fn collection_id(
    structural: &StructuralDescription,
) -> Option<sir_types::NodeId> {
    structural.roles.iter().find_map(|role| match role {
        RegionRoles::PositionSearch { collection, .. } => *collection,
        _ => None,
    })
}

pub(crate) fn sequence_domain(id: VariableId, length: usize) -> FiniteDomain {
    FiniteDomain {
        variables: vec![VariableSpec {
            id,
            kind: VariableKind::LogicalSequence { length },
        }],
    }
}
