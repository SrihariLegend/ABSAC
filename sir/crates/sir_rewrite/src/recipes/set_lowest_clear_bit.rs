use sir_transform::ids::DefinitionId;
use sir_types::Span;

use crate::error::RewriteError;
use crate::patch::{ReplacementPatch, ReplacementValue};
use crate::recipe::RewriteRecipe;
use crate::region::RewriteRegion;
use crate::subgraph_builder::SubgraphBuilder;

/// Rewrite recipe for `SetLowestClearBit` (`x | (x + 1)`).
///
/// The lowest clear bit of `x` is the lowest set bit of `~x`, so on x86 the
/// expression lowers to `x | blsmsk(~x)` (blsmsk isolates the lowest set bit
/// of its argument and everything below). Instruction selection happens here,
/// in the recipe; recognition and the proof work purely on the semantic
/// operation `SetLowestClearBit(x)`.
pub struct SetLowestClearBitRecipe {
    id: DefinitionId,
}

impl SetLowestClearBitRecipe {
    pub fn new(id: DefinitionId) -> Self {
        Self { id }
    }
}

impl RewriteRecipe for SetLowestClearBitRecipe {
    fn definition(&self) -> DefinitionId {
        self.id
    }

    fn name(&self) -> &'static str {
        "SetLowestClearBit"
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

        // x | blsmsk(~x): blsmsk(~x) is the mask of the lowest set bit of ~x
        // (the lowest clear bit of x) and everything below; x already has all
        // bits below that set, so the OR sets exactly the lowest clear bit.
        let not_x = builder.bit_not(local_operand, Span::unknown());
        let blsmsk = builder.intrinsic("blsmsk".to_string(), vec![not_x], original_ty.clone(), Span::unknown());
        let or = builder.bitwise_or(local_operand, blsmsk, Span::unknown());

        Ok(builder.finish(vec![ReplacementValue {
            old: result_node,
            new: or,
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
    fn set_lowest_clear_bit_recipe_has_correct_definition_id() {
        let recipe = SetLowestClearBitRecipe::new(DefinitionId::new(303));
        assert_eq!(recipe.definition(), DefinitionId::new(303));
    }

    #[test]
    fn set_lowest_clear_bit_recipe_has_correct_name() {
        let recipe = SetLowestClearBitRecipe::new(DefinitionId::new(303));
        assert_eq!(recipe.name(), "SetLowestClearBit");
    }

    #[test]
    fn set_lowest_clear_bit_recipe_produces_or_blsmsk_of_not_patch() {
        let recipe = SetLowestClearBitRecipe::new(DefinitionId::new(303));
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

        // The patch contains exactly three new nodes: Not(x), blsmsk(Not(x)), Or(x, blsmsk).
        assert_eq!(patch.arena.len(), 3);

        let mut not_node = None;
        let mut blsmsk_node = None;
        let mut or_node = None;
        for (_, n) in patch.arena.iter() {
            match &n.kind {
                NodeKind::Not { operand } => {
                    assert_eq!(operand, &NodeId::new(1));
                    not_node = Some(n.id);
                }
                NodeKind::Intrinsic { name, args } => {
                    assert_eq!(name, "blsmsk");
                    blsmsk_node = Some((n.id, args.clone()));
                }
                NodeKind::Or { lhs, rhs } => {
                    or_node = Some((n.id, *lhs, *rhs));
                }
                other => panic!("unexpected node {:?}", other),
            }
        }

        let not_id = not_node.expect("expected a Not(x) node");
        let (blsmsk_id, args) = blsmsk_node.expect("expected a blsmsk intrinsic");
        assert_eq!(args, vec![not_id], "blsmsk must take Not(x)");

        // Or(x, blsmsk): the base operand is x (NodeId 1), the mask side is blsmsk.
        let (or_id, or_lhs, or_rhs) = or_node.expect("expected an Or node");
        assert_eq!(or_lhs, NodeId::new(1), "Or lhs must be the base operand x");
        assert_eq!(or_rhs, blsmsk_id, "Or rhs must be the blsmsk result");
        assert_ne!(or_id, NodeId::new(3));

        // One replacement: result → or.
        assert_eq!(patch.replacements.len(), 1);
        assert_eq!(patch.replacements[0].old, NodeId::new(3));
    }
}
