use sir_transform::ids::DefinitionId;
use sir_types::Span;

use crate::error::RewriteError;
use crate::patch::{ReplacementPatch, ReplacementValue};
use crate::recipe::RewriteRecipe;
use crate::region::RewriteRegion;
use crate::subgraph_builder::SubgraphBuilder;

/// Recipe for the Popcount transformation.
///
/// Replaces a boolean-array counting loop with:
///   pack(board) → popcount(packed)
///
/// The replacement subgraph:
///   Pack(array) → Popcount(packed) → (replaces result)
pub struct PopcountRecipe {
    id: DefinitionId,
}

impl PopcountRecipe {
    pub fn new(id: DefinitionId) -> Self {
        Self { id }
    }
}

impl RewriteRecipe for PopcountRecipe {
    fn definition(&self) -> DefinitionId {
        self.id
    }

    fn name(&self) -> &'static str {
        "Popcount"
    }

    fn build_patch(
        &self,
        function: &sir_nodes::Function,
        region: &RewriteRegion,
        mut builder: SubgraphBuilder,
    ) -> Result<ReplacementPatch, RewriteError> {
        let mut old_result = region.result()?;
        
        if region.structural.roles.iter().any(|r| matches!(r, sir_transform::roles::RegionRoles::SetIteration { .. })) {
            // Find the TupleExtract that uses the loop node
            for node in function.arena.iter() {
                if let sir_nodes::NodeKind::TupleExtract { tuple, .. } = &node.kind {
                    if *tuple == old_result {
                        old_result = node.id;
                        break;
                    }
                }
            }
        }
        
        let original_ty = function.get_node(old_result).unwrap().ty.clone();

        let packed = if let Some(set_val) = region.structural.roles.iter().find_map(|r| {
            if let sir_transform::roles::RegionRoles::SetIteration { set_value, .. } = r {
                Some(*set_value)
            } else {
                None
            }
        }) {
            use crate::local_id::LocalNodeId;
            LocalNodeId::new(set_val.as_u64())
        } else {
            crate::recipes::helpers::emit_pack(function, region, &mut builder)?
        };
        let pop = builder.popcount(packed, original_ty, Span::unknown());

        Ok(builder.finish(vec![ReplacementValue {
            old: old_result,
            new: pop,
        }]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sir_semantics::structure::StructuralDescription;
    use sir_transform::roles::RegionRoles;
    use sir_transform::structures::SourceStructure;
    use sir_types::RegionId;

    fn make_test_region() -> RewriteRegion {
        let structural = StructuralDescription::new(
            RegionId::new(0),
            SourceStructure::LogicalSequence { length: 64 },
        )
        .with_roles(RegionRoles::BooleanCollectionReduction {
            collection: sir_types::NodeId::new(10),
            accumulator: None,
            result: sir_types::NodeId::new(20),
        });

        RewriteRegion::new(structural)
    }

    #[test]
    fn popcount_recipe_has_correct_definition_id() {
        let recipe = PopcountRecipe::new(DefinitionId::new(42));
        assert_eq!(recipe.definition(), DefinitionId::new(42));
    }

    #[test]
    fn popcount_recipe_has_correct_name() {
        let recipe = PopcountRecipe::new(DefinitionId::new(0));
        assert_eq!(recipe.name(), "Popcount");
    }

    #[test]
    fn popcount_recipe_produces_patch_with_correct_structure() {
        let recipe = PopcountRecipe::new(DefinitionId::new(0));
        let region = make_test_region();
        let builder = SubgraphBuilder::new();
        let func = sir_nodes::Function::new("test", sir_types::Type::Unit);

        let patch = recipe.build_patch(&func, &region, builder).unwrap();

        // The patch contains 2 nodes: Pack + Popcount
        assert_eq!(patch.arena.len(), 2);

        // One replacement: result → popcount
        assert_eq!(patch.replacements.len(), 1);
        assert_eq!(patch.replacements[0].old, sir_types::NodeId::new(20));
    }

    #[test]
    fn popcount_recipe_fails_without_collection_role() {
        let recipe = PopcountRecipe::new(DefinitionId::new(0));
        // Create a region without roles
        let structural = StructuralDescription::new(
            RegionId::new(0),
            SourceStructure::LogicalSequence { length: 64 },
        );
        let region = RewriteRegion::new(structural);
        let builder = SubgraphBuilder::new();
        let func = sir_nodes::Function::new("test", sir_types::Type::Unit);

        let result = recipe.build_patch(&func, &region, builder);
        assert!(result.is_err());
        match result {
            Err(RewriteError::MissingRole { .. }) => {} // expected
            other => panic!("expected MissingRole, got {:?}", other),
        }
    }
}
