use sir_transform::ids::DefinitionId;

use crate::error::RewriteError;
use crate::patch::ReplacementPatch;
use crate::region::RewriteRegion;
use crate::subgraph_builder::SubgraphBuilder;

/// Canonical owner of graph construction for one transformation family.
///
/// Exactly analogous to `TransformationDefinition` in `sir_verification`.
/// One recipe per transformation family. Responsible only for constructing
/// the replacement subgraph in a detached arena.
///
/// Never clones graphs, never reconnects SSA, never computes diffs,
/// never validates IR.
pub trait RewriteRecipe {
    /// The `DefinitionId` this recipe corresponds to.
    /// Must match `Candidate.definition_id` and `Proof.definition_id`.
    fn definition(&self) -> DefinitionId;

    /// Human-readable name for debugging/reporting.
    fn name(&self) -> &'static str;

    /// Construct the replacement subgraph in a detached arena.
    ///
    /// Consumes the `SubgraphBuilder` by value — call `builder.finish()`
    /// to seal the patch. The recipe must not clone graphs, reconnect SSA,
    /// compute diffs, or validate IR.
    fn build_patch(
        &self,
        region: &RewriteRegion,
        builder: SubgraphBuilder,
    ) -> Result<ReplacementPatch, RewriteError>;
}

/// Registry of rewrite recipes, keyed by `DefinitionId`.
pub struct RecipeRegistry {
    recipes: Vec<Box<dyn RewriteRecipe>>,
}

impl RecipeRegistry {
    pub fn new() -> Self {
        Self { recipes: Vec::new() }
    }

    /// Register a rewrite recipe.
    pub fn register(&mut self, recipe: Box<dyn RewriteRecipe>) {
        self.recipes.push(recipe);
    }

    /// Look up a recipe by definition ID.
    pub fn lookup(&self, id: DefinitionId) -> Option<&dyn RewriteRecipe> {
        self.recipes.iter().find(|r| r.definition() == id).map(|r| r.as_ref())
    }

    /// Number of registered recipes.
    pub fn len(&self) -> usize {
        self.recipes.len()
    }

    /// Returns true if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.recipes.is_empty()
    }
}

impl Default for RecipeRegistry {
    fn default() -> Self {
        Self::new()
    }
}
