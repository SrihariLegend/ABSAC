use sir_transform::ids::DefinitionId;
use sir_types::Span;

use crate::error::RewriteError;
use crate::patch::{ReplacementPatch, ReplacementValue};
use crate::recipe::RewriteRecipe;
use crate::region::RewriteRegion;
use crate::subgraph_builder::SubgraphBuilder;

/// Rewrite recipe for `LowestClearBitMask` (`~x & (x + 1)`).
///
/// The lowest clear bit of `x` is the lowest set bit of `~x`, so on x86 it
/// lowers to `blsi(~x)`. Instruction selection happens here, in the recipe;
/// recognition and the proof work purely on the semantic operation.
pub struct IsolateLowestClearBitRecipe {
    id: DefinitionId,
}

impl IsolateLowestClearBitRecipe {
    pub fn new(id: DefinitionId) -> Self {
        Self { id }
    }
}

impl RewriteRecipe for IsolateLowestClearBitRecipe {
    fn definition(&self) -> DefinitionId {
        self.id
    }

    fn name(&self) -> &'static str {
        "IsolateLowestClearBit"
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

        // blsi(~x) isolates the lowest clear bit of x.
        let not_x = builder.bit_not(local_operand, Span::unknown());
        let blsi = builder.intrinsic("blsi".to_string(), vec![not_x], original_ty, Span::unknown());

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
    fn isolate_lowest_clear_bit_recipe_has_correct_definition_id() {
        let recipe = IsolateLowestClearBitRecipe::new(DefinitionId::new(302));
        assert_eq!(recipe.definition(), DefinitionId::new(302));
    }

    #[test]
    fn isolate_lowest_clear_bit_recipe_has_correct_name() {
        let recipe = IsolateLowestClearBitRecipe::new(DefinitionId::new(302));
        assert_eq!(recipe.name(), "IsolateLowestClearBit");
    }

    #[test]
    fn isolate_lowest_clear_bit_recipe_produces_blsi_of_not_patch() {
        let recipe = IsolateLowestClearBitRecipe::new(DefinitionId::new(302));
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

        // The patch contains exactly two new nodes: Not(x) and blsi(Not(x)).
        assert_eq!(patch.arena.len(), 2);

        let mut not_node = None;
        let mut blsi_node = None;
        for (_, n) in patch.arena.iter() {
            match &n.kind {
                NodeKind::Not { operand } => {
                    assert_eq!(operand, &NodeId::new(1));
                    not_node = Some(n.id);
                }
                NodeKind::Intrinsic { name, args } => {
                    assert_eq!(name, "blsi");
                    blsi_node = Some((n.id, args.clone()));
                }
                other => panic!("unexpected node {:?}", other),
            }
        }
        assert!(not_node.is_some(), "expected a Not(x) node");
        let (blsi_id, args) = blsi_node.expect("expected a blsi intrinsic");
        assert_eq!(args, vec![not_node.unwrap()]);
        assert_ne!(blsi_id, NodeId::new(3));

        // One replacement: result → blsi.
        assert_eq!(patch.replacements.len(), 1);
        assert_eq!(patch.replacements[0].old, NodeId::new(3));
    }
}
