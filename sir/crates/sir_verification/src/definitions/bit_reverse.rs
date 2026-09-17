use sir_generation::candidate::Candidate;
use sir_semantics::structure::StructuralDescription;
use sir_transform::ids::{DefinitionId, VariableId};
use sir_transform::roles::{PermutationKind, RegionRoles};
use sir_types::ConstantData;

use crate::obligation::{FiniteDomain, ProofObligation, VariableKind, VariableSpec};
use crate::registry::{TransformationDefinition, VerificationStatus};
use crate::semantic::expression::SemanticExpression;
use crate::semantic::theorem::Theorem;

/// Full or partial bit reversal: the recognized swap-network reverses the
/// low `perm_width` of the operand's `type_width` bits, which is the
/// full-width reversal shifted down by `type_width - perm_width`.
pub struct BitReverseDefinition {
    id: DefinitionId,
}

impl BitReverseDefinition {
    pub fn new(id: DefinitionId) -> Self {
        Self { id }
    }
}

impl TransformationDefinition for BitReverseDefinition {
    fn verification_status(&self) -> VerificationStatus {
        VerificationStatus::ConcreteSolverChecked
    }

    fn id(&self) -> DefinitionId {
        self.id
    }

    fn name(&self) -> &'static str {
        "Reverse Bits"
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
        structural: &StructuralDescription,
    ) -> ProofObligation {
        self.bind(candidate, structural)
            .unwrap_or_else(|| self.unbound_obligation(candidate))
    }
}

impl BitReverseDefinition {
    fn bind(
        &self,
        candidate: &Candidate,
        structural: &StructuralDescription,
    ) -> Option<ProofObligation> {
        let (operand, perm_width, type_width) =
            structural.roles.iter().find_map(|role| match role {
                RegionRoles::BitPermutation {
                    operand,
                    kind:
                        PermutationKind::BitReverse {
                            perm_width,
                            type_width,
                        },
                    ..
                } => Some((*operand, *perm_width as usize, *type_width as usize)),
                _ => None,
            })?;
        if type_width == 0 || perm_width == 0 || perm_width > type_width || type_width > 64 {
            return None;
        }

        let x = VariableId::new(operand.as_u64());
        // The source pattern: reverse the low `perm_width` bits, high
        // bits dropped.
        let mut pattern: Option<SemanticExpression> = None;
        for i in 0..perm_width {
            let bit = SemanticExpression::BitwiseAnd(
                Box::new(SemanticExpression::ShiftRight(
                    Box::new(SemanticExpression::Variable(x)),
                    Box::new(SemanticExpression::Constant(ConstantData::u64(i as u64))),
                )),
                Box::new(SemanticExpression::Constant(ConstantData::u64(1))),
            );
            let placed = SemanticExpression::ShiftLeft(
                Box::new(bit),
                Box::new(SemanticExpression::Constant(ConstantData::u64(
                    (perm_width - 1 - i) as u64,
                ))),
            );
            pattern = Some(match pattern {
                None => placed,
                Some(acc) => SemanticExpression::BitwiseOr(Box::new(acc), Box::new(placed)),
            });
        }
        let reversed = SemanticExpression::BitReverse(Box::new(SemanticExpression::Variable(x)));
        let lhs = if type_width == perm_width {
            reversed
        } else {
            SemanticExpression::ShiftRight(
                Box::new(reversed),
                Box::new(SemanticExpression::Constant(ConstantData::u64(
                    (type_width - perm_width) as u64,
                ))),
            )
        };

        Some(ProofObligation {
            id: sir_transform::ids::ObligationId::new(0),
            region: candidate.region,
            candidate: candidate.id,
            definition: self.id,
            theorem: Theorem::new(lhs, pattern?),
            assumptions: vec![],
            domain: Some(FiniteDomain {
                variables: vec![VariableSpec {
                    id: x,
                    kind: VariableKind::BitVector { width: type_width },
                }],
            }),
        })
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
