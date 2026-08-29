use sir_transform::ids::DefinitionId;
use sir_transform::roles::PermutationKind;
use sir_types::{ConstantData, Span};

use crate::error::RewriteError;
use crate::local_id::LocalNodeId;
use crate::patch::{ReplacementPatch, ReplacementValue};
use crate::recipe::RewriteRecipe;
use crate::region::RewriteRegion;
use crate::subgraph_builder::SubgraphBuilder;

/// Recipe for the ByteSwap transformation.
///
/// Replaces the entire masked-swap expression with a byte-swap intrinsic
/// followed by a shift that aligns the permuted bits: the permutation covers
/// `perm_width` of the operand's `type_width` bits, so the intrinsic result
/// is shifted right by `type_width - perm_width` (HD004 swaps the low two
/// bytes of a 32-bit word: `bswap(x) >> 16`).
pub struct ByteSwapRecipe {
    id: DefinitionId,
}

impl ByteSwapRecipe {
    pub fn new(id: DefinitionId) -> Self {
        Self { id }
    }
}

impl RewriteRecipe for ByteSwapRecipe {
    fn definition(&self) -> DefinitionId {
        self.id
    }

    fn name(&self) -> &'static str {
        "Byte Swap"
    }

    fn build_patch(
        &self,
        _function: &sir_nodes::Function,
        region: &RewriteRegion,
        mut builder: SubgraphBuilder,
    ) -> Result<ReplacementPatch, RewriteError> {
        let result = region.result()?;
        let (operand, kind) = region.permutation()?;
        let (perm_width, type_width) = match &kind {
            PermutationKind::ByteSwap {
                perm_width,
                type_width,
            } => (*perm_width, *type_width),
            other => {
                return Err(RewriteError::MissingRole {
                    role: format!("ByteSwap permutation kind, found {:?}", other),
                })
            }
        };

        let operand_local = LocalNodeId::new(operand.as_u64());
        let operand_ty = builder
            .get_type(operand_local)
            .unwrap_or(sir_types::Type::u32());

        // bswap intrinsic over the full operand word.
        let swapped = builder.intrinsic(
            "bswap".to_string(),
            vec![operand_local],
            operand_ty.clone(),
            Span::unknown(),
        );

        // Align the permuted bits: shift out the untouched high bits.
        let shift_amount = type_width.saturating_sub(perm_width);
        let mut new_value = swapped;
        if shift_amount > 0 {
            let data = match &operand_ty {
                sir_types::Type::Integer { signed: false, .. } => ConstantData::u32(shift_amount),
                _ => ConstantData::u64(shift_amount as u64),
            };
            let amount = builder.constant(data, operand_ty.clone(), Span::unknown());
            new_value = builder.shr(swapped, amount, Span::unknown());
        }

        Ok(builder.finish(vec![ReplacementValue {
            old: result,
            new: new_value,
        }]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sir_nodes::{Function, Node, NodeKind};
    use sir_semantics::structure::StructuralDescription;
    use sir_transform::roles::{PermutationKind, RegionRoles};
    use sir_transform::structures::SourceStructure;
    use sir_types::{NodeId, RegionId};

    fn make_byte_swap_region() -> RewriteRegion {
        let structural = StructuralDescription::new(
            RegionId::new(0),
            SourceStructure::BitPermutation { width: 32 },
        )
        .with_roles(RegionRoles::BitPermutation {
            operand: NodeId::new(0),
            result: NodeId::new(5),
            kind: PermutationKind::ByteSwap {
                perm_width: 16,
                type_width: 32,
            },
        });

        RewriteRegion::new(structural)
    }

    fn make_function() -> Function {
        let mut func = Function::new("byte_swap", sir_types::Type::u32());
        for id in [0u64, 5] {
            func.arena.insert(Node::new(
                NodeId::new(id),
                NodeKind::Constant(sir_types::ConstantData::u32(0)),
                sir_types::Type::u32(),
                sir_types::Effects::empty(),
                sir_types::Span::unknown(),
            ));
        }
        func
    }

    #[test]
    fn byte_swap_recipe_emits_bswap_and_shift() {
        let recipe = ByteSwapRecipe::new(DefinitionId::new(312));
        let region = make_byte_swap_region();
        let builder = SubgraphBuilder::new();
        let func = make_function();

        let patch = recipe.build_patch(&func, &region, builder).unwrap();
        assert_eq!(patch.replacements.len(), 1);
        assert_eq!(patch.replacements[0].old, NodeId::new(5));

        let mut bswap_seen = false;
        let mut shr_seen = false;
        for (_, n) in patch.arena.iter() {
            match &n.kind {
                NodeKind::Intrinsic { name, .. } if name == "bswap" => bswap_seen = true,
                NodeKind::Shr { .. } => shr_seen = true,
                _ => {}
            }
        }
        assert!(bswap_seen, "expected a bswap intrinsic");
        assert!(shr_seen, "expected a Shr for width alignment");
    }
}
