use sir_transform::ids::DefinitionId;
use sir_types::Span;

use crate::error::RewriteError;
use crate::patch::{ReplacementPatch, ReplacementValue};
use crate::recipe::RewriteRecipe;
use crate::region::RewriteRegion;
use crate::subgraph_builder::SubgraphBuilder;

/// Rewrite recipe for `LowestSetBit` (`x & -x`).
/// Maps to the hardware intrinsic `blsi` (isolate lowest set bit).
pub struct IsolateLowestSetBitRecipe {
    id: DefinitionId,
}

impl IsolateLowestSetBitRecipe {
    pub fn new(id: DefinitionId) -> Self {
        Self { id }
    }
}

impl RewriteRecipe for IsolateLowestSetBitRecipe {
    fn definition(&self) -> DefinitionId {
        self.id
    }

    fn name(&self) -> &'static str {
        "IsolateLowestSetBit"
    }

    fn build_patch(
        &self,
        function: &sir_nodes::Function,
        region: &RewriteRegion,
        mut builder: SubgraphBuilder,
    ) -> Result<ReplacementPatch, RewriteError> {
        let (operand, result_node) = region.mask_operation()?;

        let original_ty = function.get_node(result_node).unwrap().ty.clone();

        use crate::local_id::LocalNodeId;
        let local_operand = LocalNodeId::new(operand.as_u64());
        let blsi = builder.intrinsic(
            "blsi".to_string(),
            vec![local_operand],
            original_ty,
            Span::unknown(),
        );

        Ok(builder.finish(vec![ReplacementValue {
            old: result_node,
            new: blsi,
        }]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sir_nodes::{Function, Node, NodeKind};
    use sir_semantics::structure::StructuralDescription;
    use sir_transform::roles::RegionRoles;
    use sir_transform::structures::SourceStructure;
    use sir_types::{NodeId, RegionId};

    fn make_test_region(operand: NodeId, result: NodeId) -> RewriteRegion {
        let structural = StructuralDescription::new(
            RegionId::new(0),
            SourceStructure::MaskAlgebraExpression,
        )
        .with_roles(RegionRoles::MaskOperation { operand, result });

        RewriteRegion::new(structural)
    }

    #[test]
    fn isolate_lowest_set_bit_recipe_has_correct_definition_id() {
        let recipe = IsolateLowestSetBitRecipe::new(DefinitionId::new(301));
        assert_eq!(recipe.definition(), DefinitionId::new(301));
    }

    #[test]
    fn isolate_lowest_set_bit_recipe_has_correct_name() {
        let recipe = IsolateLowestSetBitRecipe::new(DefinitionId::new(301));
        assert_eq!(recipe.name(), "IsolateLowestSetBit");
    }

    #[test]
    fn isolate_lowest_set_bit_recipe_produces_blsi_patch() {
        let recipe = IsolateLowestSetBitRecipe::new(DefinitionId::new(301));
        let region = make_test_region(NodeId::new(1), NodeId::new(3));
        let builder = SubgraphBuilder::new();

        // Region result (node 3) must exist in the function with the mask type.
        let mut func = Function::new("test", sir_types::Type::u64());
        func.arena.insert(Node::new(
            NodeId::new(3),
            NodeKind::Constant(sir_types::ConstantData::u64(0)),
            sir_types::Type::u64(),
            sir_types::Effects::empty(),
            sir_types::Span::unknown(),
        ));

        let patch = recipe.build_patch(&func, &region, builder).unwrap();

        // The patch contains exactly one new node: the blsi intrinsic.
        assert_eq!(patch.arena.len(), 1);
        let (_, intrinsic) = patch.arena.iter().next().unwrap();
        match &intrinsic.kind {
            NodeKind::Intrinsic { name, args } => {
                assert_eq!(name, "blsi");
                assert_eq!(args, &vec![NodeId::new(1)]);
            }
            other => panic!("expected Intrinsic node, got {:?}", other),
        }

        // One replacement: result → blsi.
        assert_eq!(patch.replacements.len(), 1);
        assert_eq!(patch.replacements[0].old, NodeId::new(3));
    }
}
