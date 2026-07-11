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
        let packed = crate::recipes::helpers::emit_pack(function, region, &mut builder)?;
        let pop = builder.popcount(packed, Span::unknown());

        // 3. Map old result → new popcount
        let result = region.result()?;
        Ok(builder.finish(vec![ReplacementValue {
            old: result,
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

        let patch = recipe.build_patch(&region, builder).unwrap();

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

        let result = recipe.build_patch(&region, builder);
        assert!(result.is_err());
        match result {
            Err(RewriteError::MissingRole { .. }) => {} // expected
            other => panic!("expected MissingRole, got {:?}", other),
        }
    }
}
