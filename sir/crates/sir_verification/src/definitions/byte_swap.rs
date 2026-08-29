use sir_generation::candidate::Candidate;
use sir_transform::ids::{DefinitionId, VariableId};
use sir_types::ConstantData;

use crate::obligation::ProofObligation;
use crate::registry::TransformationDefinition;
use crate::semantic::expression::SemanticExpression;
use crate::semantic::theorem::Theorem;

/// The ByteSwap transformation: swapping two adjacent bytes is the masked
/// shift pair `((x & 0xFF) << 8) | ((x >> 8) & 0xFF)`.
///
/// The semantic concept is the byte permutation; the recipe selects the
/// `bswap` instruction only after this identity is proven.
pub struct ByteSwapDefinition {
    id: DefinitionId,
}

impl ByteSwapDefinition {
    pub fn new(id: DefinitionId) -> Self {
        Self { id }
    }
}

impl TransformationDefinition for ByteSwapDefinition {
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
        let x = SemanticExpression::Variable(VariableId::new(0));
        let mask = SemanticExpression::Constant(ConstantData::u64(0xFF));
        let eight = SemanticExpression::Constant(ConstantData::u64(8));

        let candidate_expr = SemanticExpression::BitwiseOr(
            Box::new(SemanticExpression::ShiftLeft(
                Box::new(SemanticExpression::BitwiseAnd(Box::new(x.clone()), Box::new(mask.clone()))),
                Box::new(eight.clone()),
            )),
            Box::new(SemanticExpression::BitwiseAnd(
                Box::new(SemanticExpression::ShiftRight(Box::new(x.clone()), Box::new(eight))),
                Box::new(mask),
            )),
        );

        ProofObligation {
            id: sir_transform::ids::ObligationId::new(0),
            region: candidate.region,
            definition: self.id,
            candidate: candidate.id,
            theorem: Theorem::new(SemanticExpression::ByteSwap(Box::new(x)), candidate_expr),
            assumptions: vec![],
            domain: None,
        }
    }
}
