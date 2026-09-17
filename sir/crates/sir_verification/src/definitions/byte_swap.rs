use sir_generation::candidate::Candidate;
use sir_semantics::structure::StructuralDescription;
use sir_transform::ids::{DefinitionId, VariableId};
use sir_transform::roles::{PermutationKind, RegionRoles};
use sir_types::ConstantData;

use crate::obligation::{FiniteDomain, ProofObligation, VariableKind, VariableSpec};
use crate::registry::{TransformationDefinition, VerificationStatus};
use crate::semantic::expression::SemanticExpression;
use crate::semantic::theorem::Theorem;

/// Byte-order reversal, possibly covering only the low `perm_width` of
/// the operand's `type_width` bits.
///
/// The recognized source pattern (e.g. `((x & 0xFF) << 8) | ((x >> 8) & 0xFF)`
/// over a 32-bit word) is exactly the full byte reversal shifted down by
/// `type_width - perm_width`; the obligation binds the actual operand,
/// widths and permutation span from the authorized structural role and
/// the concrete solver discharges the identity.
pub struct ByteSwapDefinition {
    id: DefinitionId,
}

impl ByteSwapDefinition {
    pub fn new(id: DefinitionId) -> Self {
        Self { id }
    }
}

impl TransformationDefinition for ByteSwapDefinition {
    fn verification_status(&self) -> VerificationStatus {
        VerificationStatus::ConcreteSolverChecked
    }

    fn id(&self) -> DefinitionId {
        self.id
    }

    fn name(&self) -> &'static str {
        "Byte Swap"
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
        // Without the recognized role there is no authorized permutation
        // span; fail closed.
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

impl ByteSwapDefinition {
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
                        PermutationKind::ByteSwap {
                            perm_width,
                            type_width,
                        },
                    ..
                } => Some((*operand, *perm_width as usize, *type_width as usize)),
                _ => None,
            })?;
        if type_width == 0
            || perm_width == 0
            || perm_width % 8 != 0
            || perm_width > type_width
            || type_width > 64
        {
            return None;
        }

        let x = VariableId::new(operand.as_u64());
        let bytes = perm_width / 8;
        // The source pattern: swap the bytes of the low `perm_width`
        // bits, high bits dropped.
        let mut pattern: Option<SemanticExpression> = None;
        for i in 0..bytes {
            let byte = SemanticExpression::BitwiseAnd(
                Box::new(SemanticExpression::ShiftRight(
                    Box::new(SemanticExpression::Variable(x)),
                    Box::new(SemanticExpression::Constant(ConstantData::u64(
                        (8 * i) as u64,
                    ))),
                )),
                Box::new(SemanticExpression::Constant(ConstantData::u64(0xFF))),
            );
            let placed = SemanticExpression::ShiftLeft(
                Box::new(byte),
                Box::new(SemanticExpression::Constant(ConstantData::u64(
                    (8 * (bytes - 1 - i)) as u64,
                ))),
            );
            pattern = Some(match pattern {
                None => placed,
                Some(acc) => SemanticExpression::BitwiseOr(Box::new(acc), Box::new(placed)),
            });
        }
        let swapped = SemanticExpression::ByteSwap(Box::new(SemanticExpression::Variable(x)));
        let lhs = if type_width == perm_width {
            swapped
        } else {
            SemanticExpression::ShiftRight(
                Box::new(swapped),
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
