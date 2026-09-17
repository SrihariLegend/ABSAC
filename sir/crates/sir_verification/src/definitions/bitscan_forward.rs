use sir_generation::candidate::Candidate;
use sir_nodes::NodeKind;
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
        let collection = collection_id(structural)?;
        let length = position_collection_length(function, structural)?;
        // The theorem's no-hit result is the sequence length; the loop's
        // position select must carry exactly that sentinel, or the rewrite
        // would change the no-match result.
        if !sentinel_matches_length(function, structural, length) {
            return None;
        }
        let v = VariableId::new(collection.as_u64());
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

/// The forward search's position select (`hit ? index : sentinel`) must
/// carry the length sentinel the theorem uses; the successor-as-result
/// form (the loop exports `i + 1` and exits on `successor < BOUND`)
/// uses the bound as its sentinel, so the bound must be the length.
fn sentinel_matches_length(
    function: &sir_nodes::Function,
    structural: &StructuralDescription,
    length: usize,
) -> bool {
    let Some(loop_node) = structural.roles.iter().find_map(|role| match role {
        RegionRoles::PositionSearch { result, .. } => Some(*result),
        _ => None,
    }) else {
        return false;
    };
    let Some(NodeKind::Loop {
        body,
        termination,
        outputs,
        ..
    }) = function.get_node(loop_node).map(|n| &n.kind)
    else {
        return false;
    };
    let select_form = body.iter().any(|id| {
        matches!(
            function.get_node(*id).map(|n| &n.kind),
            Some(NodeKind::Select { false_val, .. })
                if folded_constant_u64(function, *false_val) == Some(length as u64)
        )
    });
    select_form
        || termination_bound_matches(function, *termination, outputs, length)
}

/// Search a termination conjunction for a successor/bound comparison
/// whose no-hit sentinel equals `length`.
fn termination_bound_matches(
    function: &sir_nodes::Function,
    term: sir_types::NodeId,
    outputs: &[sir_types::NodeId],
    length: usize,
) -> bool {
    let side_match = |lhs: sir_types::NodeId,
                      rhs: sir_types::NodeId,
                      inclusive: bool|
     -> bool {
        let side = |succ: sir_types::NodeId, bound: sir_types::NodeId| {
            outputs.contains(&succ)
                && folded_constant_u64(function, bound).is_some_and(|b| {
                    let sentinel = if inclusive { b.saturating_add(1) } else { b };
                    sentinel == length as u64
                })
        };
        side(lhs, rhs) || side(rhs, lhs)
    };
    match function.get_node(term).map(|n| &n.kind) {
        Some(NodeKind::Lt { lhs, rhs }) | Some(NodeKind::Ne { lhs, rhs }) => {
            side_match(*lhs, *rhs, false)
        }
        Some(NodeKind::Le { lhs, rhs }) => side_match(*lhs, *rhs, true),
        Some(NodeKind::BoolAnd { lhs, rhs }) => {
            termination_bound_matches(function, *lhs, outputs, length)
                || termination_bound_matches(function, *rhs, outputs, length)
        }
        Some(NodeKind::BoolNot { operand }) => {
            termination_bound_matches(function, *operand, outputs, length)
        }
        _ => false,
    }
}

/// Fold a constant tree (literals plus Add/Sub of constants).
fn folded_constant_u64(function: &sir_nodes::Function, id: sir_types::NodeId) -> Option<u64> {
    match &function.get_node(id)?.kind {
        NodeKind::Constant(data) => data
            .as_u64()
            .or_else(|| data.as_i64().and_then(|v| u64::try_from(v).ok())),
        NodeKind::Add { lhs, rhs } => Some(
            folded_constant_u64(function, *lhs)?.wrapping_add(folded_constant_u64(function, *rhs)?),
        ),
        NodeKind::Sub { lhs, rhs } => Some(
            folded_constant_u64(function, *lhs)?.wrapping_sub(folded_constant_u64(function, *rhs)?),
        ),
        _ => None,
    }
}

/// The recognized PositionSearch collection and its declared extent
/// (length ≤ 512, the emitter's bitvector capacity). Predicate
/// collections are integer arrays; the obligation's sequence is the
/// predicate results, so only the extent is needed. Extents beyond the
/// 64-bit concrete solver are discharged by the symbolic scan identity
/// and honestly issued as SchemaChecked rather than solver-checked.
pub(crate) fn position_collection_length(
    function: &sir_nodes::Function,
    structural: &StructuralDescription,
) -> Option<usize> {
    let collection = collection_id(structural)?;
    match function.get_node(collection).map(|n| &n.ty) {
        Some(Type::Array { length, .. }) if *length > 0 && *length <= 512 => Some(*length),
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
