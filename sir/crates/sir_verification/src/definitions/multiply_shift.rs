use sir_generation::candidate::Candidate;
use sir_nodes::NodeKind;
use sir_transform::ids::{DefinitionId, VariableId};
use sir_types::{ConstantData, Type};

use crate::obligation::{FiniteDomain, ProofObligation, VariableKind, VariableSpec};
use crate::registry::{TransformationDefinition, VerificationStatus};
use crate::semantic::expression::SemanticExpression;
use crate::semantic::theorem::Theorem;

pub struct MultiplyShiftDefinition {
    id: DefinitionId,
}

impl MultiplyShiftDefinition {
    pub fn new(id: DefinitionId) -> Self {
        Self { id }
    }
}

impl TransformationDefinition for MultiplyShiftDefinition {
    fn verification_status(&self) -> VerificationStatus {
        // UPGRADED (advisor P0 item 3): the obligation is built from the
        // candidate's authorized region (the actual multiply node and its
        // power-of-two constant) and discharged by the concrete
        // bit-blasting solver at the operand's real width. Mutating the
        // constant, width, or claimed shift produces a counterexample.
        // This is a CAP: the verifier issues ConcreteSolverChecked only
        // when the solver actually proves the bound obligation.
        VerificationStatus::ConcreteSolverChecked
    }

    fn id(&self) -> DefinitionId {
        self.id
    }

    fn name(&self) -> &'static str {
        "Multiply Power of Two to Bitwise Shift Left"
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

impl MultiplyShiftDefinition {
    /// Bind the actual source pair: a `Mul` node in the candidate's
    /// authorized region whose one operand is a power-of-two constant.
    /// Returns the theorem `x * C == x << log2(C)` at the real width.
    fn bind(
        &self,
        candidate: &Candidate,
        function: &sir_nodes::Function,
    ) -> Option<ProofObligation> {
        for &id in &candidate.authorization.region_nodes {
            let Some(node) = function.get_node(id) else {
                continue;
            };
            let NodeKind::Mul { lhs, rhs } = &node.kind else {
                continue;
            };
            // Exactly one side must be a power-of-two constant.
            let (dynamic, constant_node) = {
                let lhs_const = constant_value(function, *lhs);
                let rhs_const = constant_value(function, *rhs);
                match (lhs_const, rhs_const) {
                    (Some(_), None) => (*rhs, *lhs),
                    (None, Some(_)) => (*lhs, *rhs),
                    _ => continue,
                }
            };
            let constant = constant_value(function, constant_node)?;
            if constant == 0 || !constant.is_power_of_two() {
                continue;
            }
            let width = match function.get_node(dynamic).map(|n| &n.ty) {
                Some(Type::Integer { width, .. }) => width.bits() as usize,
                _ => continue,
            };
            let shift = constant.trailing_zeros() as u64;
            if shift as usize >= width {
                continue;
            }

            // The dynamic operand is opaque to the theorem: the identity
            // holds for every W-bit value, so a fresh variable is a sound
            // (and stronger) abstraction of it.
            let v = VariableId::new(dynamic.as_u64());
            let lhs_expr = SemanticExpression::Multiply(
                Box::new(SemanticExpression::Variable(v)),
                Box::new(SemanticExpression::Constant(ConstantData::u64(constant))),
            );
            let rhs_expr = SemanticExpression::ShiftLeft(
                Box::new(SemanticExpression::Variable(v)),
                Box::new(SemanticExpression::Constant(ConstantData::u64(shift))),
            );
            return Some(self.obligation_with(
                candidate,
                Theorem::new(lhs_expr, rhs_expr),
                Some(FiniteDomain {
                    variables: vec![VariableSpec {
                        id: v,
                        kind: VariableKind::BitVector { width },
                    }],
                }),
            ));
        }
        None
    }

    /// Fail-closed template for candidates whose actual pair could not
    /// be bound: the theorem is not an identity and carries no domain,
    /// so no backend can discharge it.
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

fn constant_value(function: &sir_nodes::Function, id: sir_types::NodeId) -> Option<u64> {
    match &function.get_node(id)?.kind {
        // Signed constants (i32 2, …) carry the same bit pattern as
        // their unsigned counterpart; accept non-negative signed values.
        NodeKind::Constant(data) => data
            .as_u64()
            .or_else(|| data.as_i64().and_then(|v| u64::try_from(v).ok())),
        _ => None,
    }
}
