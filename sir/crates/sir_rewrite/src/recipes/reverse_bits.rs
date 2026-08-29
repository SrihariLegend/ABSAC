use sir_transform::ids::DefinitionId;
use sir_transform::roles::PermutationKind;
use sir_types::{ConstantData, Span};

use crate::error::RewriteError;
use crate::local_id::LocalNodeId;
use crate::patch::{ReplacementPatch, ReplacementValue};
use crate::recipe::RewriteRecipe;
use crate::region::RewriteRegion;
use crate::subgraph_builder::SubgraphBuilder;

/// Recipe for the BitReverse transformation.
///
/// Replaces the entire three-stage swap pipeline with a reverse-bits
/// intrinsic followed by a shift that aligns the permuted bits: the
/// permutation covers `perm_width` of the operand's `type_width` bits, so the
/// intrinsic result is shifted right by `type_width - perm_width` (HD005
/// reverses the low 8 bits of a 32-bit word: `rbit(x) >> 24`).
pub struct ReverseBitsRecipe {
    id: DefinitionId,
}

impl ReverseBitsRecipe {
    pub fn new(id: DefinitionId) -> Self {
        Self { id }
    }
}

impl RewriteRecipe for ReverseBitsRecipe {
    fn definition(&self) -> DefinitionId {
        self.id
    }

    fn name(&self) -> &'static str {
        "Reverse Bits"
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
            PermutationKind::BitReverse {
                perm_width,
                type_width,
            } => (*perm_width, *type_width),
            other => {
                return Err(RewriteError::MissingRole {
                    role: format!("BitReverse permutation kind, found {:?}", other),
                })
            }
        };

        let operand_local = LocalNodeId::new(operand.as_u64());
        let operand_ty = builder
            .get_type(operand_local)
            .unwrap_or(sir_types::Type::u32());

        // Reverse all bits of the full word.
        let reversed = builder.intrinsic(
            "rbit".to_string(),
            vec![operand_local],
            operand_ty.clone(),
            Span::unknown(),
        );

        // Align the permuted bits: shift out the untouched high bits.
        let shift_amount = type_width.saturating_sub(perm_width);
        let mut new_value = reversed;
        if shift_amount > 0 {
            let data = match &operand_ty {
                sir_types::Type::Integer { signed: false, .. } => ConstantData::u32(shift_amount),
                _ => ConstantData::u64(shift_amount as u64),
            };
            let amount = builder.constant(data, operand_ty.clone(), Span::unknown());
            new_value = builder.shr(reversed, amount, Span::unknown());
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

    fn make_bit_reverse_region() -> RewriteRegion {
        let structural = StructuralDescription::new(
            RegionId::new(0),
            SourceStructure::BitPermutation { width: 32 },
        )
        .with_roles(RegionRoles::BitPermutation {
            operand: NodeId::new(0),
            result: NodeId::new(5),
            kind: PermutationKind::BitReverse {
                perm_width: 8,
                type_width: 32,
            },
        });

        RewriteRegion::new(structural)
    }

    fn make_function() -> Function {
        let mut func = Function::new("reverse_bits", sir_types::Type::u32());
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
    fn reverse_bits_recipe_emits_rbit_and_shift() {
        let recipe = ReverseBitsRecipe::new(DefinitionId::new(313));
        let region = make_bit_reverse_region();
        let builder = SubgraphBuilder::new();
        let func = make_function();

        let patch = recipe.build_patch(&func, &region, builder).unwrap();
        assert_eq!(patch.replacements.len(), 1);
        assert_eq!(patch.replacements[0].old, NodeId::new(5));

        let mut rbit_seen = false;
        let mut shr_seen = false;
        for (_, n) in patch.arena.iter() {
            match &n.kind {
                NodeKind::Intrinsic { name, .. } if name == "rbit" => rbit_seen = true,
                NodeKind::Shr { .. } => shr_seen = true,
                _ => {}
            }
        }
        assert!(rbit_seen, "expected an rbit intrinsic");
        assert!(shr_seen, "expected a Shr for width alignment");
    }
}
