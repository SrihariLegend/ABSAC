use sir_generation::candidate::Candidate;
use sir_nodes::Function;
use sir_semantics::structure::StructuralDatabase;
use sir_verification::Proof;

use crate::builder::RewriteBuilder;
use crate::error::RewriteError;
use crate::plan::RewritePlan;
use crate::recipe::RecipeRegistry;
use crate::region::RewriteRegion;
use crate::result::RewriteResult;

/// Orchestrates verified rewriting.
///
/// Never builds nodes, never manipulates SSA. Responsibilities:
/// verify IDs, fetch region, invoke recipe, invoke builder, run sir_verify,
/// produce result. Pure orchestration — all knowledge lives elsewhere.
pub struct RewriteEngine {
    recipe_registry: RecipeRegistry,
}

impl RewriteEngine {
    /// Create a new engine with the given recipe registry.
    pub fn new(recipe_registry: RecipeRegistry) -> Self {
        Self { recipe_registry }
    }

    /// Execute a verified rewrite.
    ///
    /// Pipeline:
    /// 1. Verify IDs align
    /// 2. Fetch StructuralDescription for the candidate's region
    /// 3. Compute external users
    /// 4. Assemble RewriteRegion
    /// 5. Look up and invoke recipe → ReplacementPatch
    /// 6. Assemble RewritePlan
    /// 7. RewriteBuilder::apply() → rewritten Function
    /// 8. Run sir_verify on rewritten function
    /// 9. If verification fails: discard, return error
    /// 10. Compute provenance, diff, return RewriteResult
    pub fn rewrite(
        &self,
        function: &Function,
        candidate: &Candidate,
        proof: &Proof,
        structural_db: &StructuralDatabase,
    ) -> Result<RewriteResult, RewriteError> {
        // 1. Verify ID alignment
        self.verify_ids(candidate, proof)?;

        // 2. Fetch StructuralDescription
        let structural = structural_db
            .region(candidate.region)
            .ok_or_else(|| RewriteError::RecipeFailed(format!(
                "no structural description for region {:?}", candidate.region
            )))?.clone();

        // 3. Compute external users of the region's outputs
        let external_users = Self::compute_external_users(function, &structural);

        // 4. Assemble RewriteRegion
        let rewrite_region = RewriteRegion::new(structural, external_users);

        // 5. Look up recipe
        let recipe = self
            .recipe_registry
            .lookup(candidate.definition_id)
            .ok_or_else(|| RewriteError::RecipeFailed(format!(
                "no recipe for definition {}", candidate.definition_id
            )))?;

        // 6. Invoke recipe → ReplacementPatch
        let builder = crate::subgraph_builder::SubgraphBuilder::new();
        let patch = recipe.build_patch(&rewrite_region, builder)?;

        // 7. Assemble RewritePlan
        let plan = RewritePlan {
            region: rewrite_region,
            patch,
            proof: proof.clone(),
        };

        // 8. RewriteBuilder::apply()
        let rewritten = RewriteBuilder::apply(function, plan)?;

        // 9. Run structural verification
        let mut verifier = sir_verify::Verifier::new(&rewritten);
        if !verifier.verify() {
            return Err(RewriteError::StructuralVerificationFailed(
                verifier.errors().to_vec(),
            ));
        }

        // 10. Compute provenance and diff
        let provenance = Self::compute_provenance(candidate);
        let diff = Self::compute_diff(function, &rewritten);

        Ok(RewriteResult {
            rewritten,
            provenance,
            diff,
            proof: proof.clone(),
        })
    }

    /// Verify Candidate.definition_id == Recipe.definition()
    fn verify_ids(
        &self,
        candidate: &Candidate,
        _proof: &Proof,
    ) -> Result<(), RewriteError> {
        let recipe_id = self
            .recipe_registry
            .lookup(candidate.definition_id)
            .map(|r| r.definition())
            .ok_or_else(|| RewriteError::RecipeFailed(format!(
                "no recipe for definition {}", candidate.definition_id
            )))?;

        // We verify that the recipe matches the candidate's definition.
        // Proof does not carry DefinitionId in v0.1; add a third field when it does.
        if candidate.definition_id != recipe_id {
            return Err(RewriteError::DefinitionMismatch {
                candidate: candidate.definition_id,
                recipe: recipe_id,
            });
        }

        Ok(())
    }

    /// Compute the set of nodes outside the region that reference region outputs.
    fn compute_external_users(
        function: &Function,
        structural: &sir_semantics::structure::StructuralDescription,
    ) -> std::collections::BTreeSet<sir_types::NodeId> {
        let mut external = std::collections::BTreeSet::new();

        // Collect region output nodes from roles
        let output_nodes = match &structural.roles {
            Some(sir_transform::roles::RegionRoles::BooleanCollectionReduction { result, .. }) => {
                vec![*result]
            }
            None => vec![],
        };

        // Find all nodes outside the region that reference these outputs
        for output_node in output_nodes {
            for user in sir_analysis::graph::users(output_node, &function.arena) {
                external.insert(user);
            }
        }

        external
    }

    /// Compute provenance for the rewrite (v0.1: simple mapping).
    fn compute_provenance(_candidate: &Candidate) -> Vec<crate::result::NodeProvenance> {
        // v0.1: provenance is computed from the patch's ReplacementValues.
        // Full implementation is deferred — the BS001 integration test
        // will validate correctness.
        Vec::new()
    }

    /// Compute a GraphDiff between original and rewritten functions.
    fn compute_diff(original: &Function, rewritten: &Function) -> crate::result::GraphDiff {
        use std::collections::BTreeSet;

        let original_ids: BTreeSet<sir_types::NodeId> = original
            .arena
            .nodes()
            .keys()
            .copied()
            .collect();

        let rewritten_ids: BTreeSet<sir_types::NodeId> = rewritten
            .arena
            .nodes()
            .keys()
            .copied()
            .collect();

        let removed_nodes: BTreeSet<_> = original_ids
            .difference(&rewritten_ids)
            .copied()
            .collect();

        let added_nodes: BTreeSet<_> = rewritten_ids
            .difference(&original_ids)
            .copied()
            .collect();

        crate::result::GraphDiff {
            removed_nodes,
            added_nodes,
            modified_edges: Vec::new(), // v0.1: edge changes computed in future refinement
        }
    }
}
