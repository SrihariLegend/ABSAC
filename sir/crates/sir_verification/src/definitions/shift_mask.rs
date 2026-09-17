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

/// `(x << k) >> k` extracts the low `W - k` bits: `x & ((1 << (W-k)) - 1)`
/// for UNSIGNED `x` and a constant `0 < k < W` (the recognizer already
/// refuses signed operands; the binding re-checks). `k = 0` is the
/// identity with an all-ones mask.
pub struct ShiftMaskDefinition {
    id: DefinitionId,
}

impl ShiftMaskDefinition {
    pub fn new(id: DefinitionId) -> Self {
        Self { id }
    }
}

impl TransformationDefinition for ShiftMaskDefinition {
    fn verification_status(&self) -> VerificationStatus {
        VerificationStatus::ConcreteSolverChecked
    }

    fn id(&self) -> DefinitionId {
        self.id
    }

    fn name(&self) -> &'static str {
        "Shift Sequence to Mask Extract"
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

impl ShiftMaskDefinition {
    fn bind(
        &self,
        candidate: &Candidate,
        function: &sir_nodes::Function,
        structural: &StructuralDescription,
    ) -> Option<ProofObligation> {
        for role in &structural.roles {
            let RegionRoles::ArithmeticOperation {
                operator_node,
                lhs,
                rhs,
                ..
            } = role
            else {
                continue;
            };
            let (inner, left_amount) = match function.get_node(*lhs).map(|n| &n.kind) {
                Some(NodeKind::Shl { lhs, rhs }) => (*lhs, *rhs),
                _ => continue,
            };
            // The role's operator must be the Shr of this Shl, and both
            // shift amounts must be the same constant.
            let shl_node = *lhs;
            let is_expected_shr = matches!(
                function.get_node(*operator_node).map(|n| &n.kind),
                Some(NodeKind::Shr { lhs, .. }) if *lhs == shl_node
            );
            if !is_expected_shr {
                continue;
            }
            let k_right = constant_value(function, *rhs)?;
            let k_left = constant_value(function, left_amount)?;
            if k_right != k_left {
                continue;
            }
            let (width, signed) = match function.get_node(inner).map(|n| &n.ty) {
                Some(Type::Integer { width, signed, .. }) => (width.bits() as usize, *signed),
                _ => continue,
            };
            if signed || width == 0 || width > 64 || k_left >= width as u64 {
                continue;
            }
            let mask = if k_left == 0 {
                if width == 64 {
                    u64::MAX
                } else {
                    (1u64 << width) - 1
                }
            } else {
                (1u64 << (width - k_left as usize)) - 1
            };

            let x = VariableId::new(inner.as_u64());
            let k = ConstantData::u64(k_left);
            let lhs_expr = SemanticExpression::ShiftRight(
                Box::new(SemanticExpression::ShiftLeft(
                    Box::new(SemanticExpression::Variable(x)),
                    Box::new(SemanticExpression::Constant(k.clone())),
                )),
                Box::new(SemanticExpression::Constant(k)),
            );
            let rhs_expr = SemanticExpression::BitwiseAnd(
                Box::new(SemanticExpression::Variable(x)),
                Box::new(SemanticExpression::Constant(ConstantData::u64(mask))),
            );
            return Some(ProofObligation {
                id: sir_transform::ids::ObligationId::new(0),
                region: candidate.region,
                candidate: candidate.id,
                definition: self.id,
                theorem: Theorem::new(lhs_expr, rhs_expr),
                assumptions: vec![],
                domain: Some(FiniteDomain {
                    variables: vec![VariableSpec {
                        id: x,
                        kind: VariableKind::BitVector { width },
                    }],
                }),
            });
        }
        None
    }

    fn unbound_obligation(&self, candidate: &Candidate) -> ProofObligation {
        let v = VariableId::new(u64::MAX);
        ProofObligation {
            id: sir_transform::ids::ObligationId::new(0),
            region: candidate.region,
            candidate: candidate.id,
            definition: self.id,
            theorem: Theorem::new(
                SemanticExpression::Variable(v),
                SemanticExpression::Constant(ConstantData::u64(0)),
            ),
            assumptions: vec![],
            domain: None,
        }
    }
}

fn constant_value(function: &sir_nodes::Function, id: sir_types::NodeId) -> Option<u64> {
    match &function.get_node(id)?.kind {
        NodeKind::Constant(data) => data
            .as_u64()
            .or_else(|| data.as_i64().and_then(|v| u64::try_from(v).ok())),
        _ => None,
    }
}
