use sir_analysis::facts::FactDatabase;
use sir_generation::candidate::Candidate;
use sir_nodes::{Function, NodeKind};
use sir_semantics::binding::LiveOutSlot;
use sir_semantics::binding::{derive_proposal_binding, ProposalBinding};
use sir_semantics::structure::StructuralDatabase;
use sir_types::NodeId;
use sir_verification::application_artifact::{CheckedApplication, EndToEndVerificationArtifact};
use sir_verification::registry::VerificationStatus;
use sir_verification::Proof;

use crate::builder::RewriteBuilder;
use crate::error::RewriteError;
use crate::patch::ReplacementPatch;
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
    /// 3. Assemble RewriteRegion
    /// 4. Look up and invoke recipe → ReplacementPatch
    /// 5. Assemble RewritePlan
    /// 6. RewriteBuilder::apply() → rewritten Function
    /// 7. Run sir_verify on rewritten function
    /// 8. If verification fails: discard, return error
    /// 9. Compute provenance, diff, return RewriteResult
    pub fn rewrite(
        &self,
        function: &Function,
        candidate: &Candidate,
        proof: &Proof,
        structural_db: &StructuralDatabase,
    ) -> Result<RewriteResult, RewriteError> {
        self.rewrite_checked(
            function,
            candidate,
            proof,
            structural_db,
            &sir_semantics::authorization::AuthorizationDatabase::new(),
            &FactDatabase::new(),
        )
    }

    /// Full rewrite entry point: as `rewrite`, but also enforces the
    /// exact authorization comparison against the immutable database
    /// (advisor directive: the optimizer AND the rewrite layer both
    /// retrieve the original authorization; the candidate's carried
    /// copies are advisory). Unit-test ids skip the lookup.
    pub fn rewrite_checked(
        &self,
        function: &Function,
        candidate: &Candidate,
        proof: &Proof,
        structural_db: &StructuralDatabase,
        authorization_db: &sir_semantics::authorization::AuthorizationDatabase,
        facts: &FactDatabase,
    ) -> Result<RewriteResult, RewriteError> {
        // 1. Verify ID alignment
        self.verify_ids(candidate, proof)?;

        // 1.5 Revalidate authorization + binding (advisor P0A item 2:
        // the rewrite layer revalidates AuthorizationRef + binding
        // digest before trusting the candidate), then the exact
        // database comparison (advisor directive 1: the FNV digest is
        // a diagnostic; the immutable database is the authority).
        if !candidate.authorization.matches_function(function)
            || !candidate.binding_digest_valid()
            || !sir_generation::candidate::exact_binding_matches(candidate, authorization_db)
        {
            return Err(RewriteError::RecipeFailed(
                "candidate authorization is stale, forged, or binding digest invalid".to_string(),
            ));
        }

        // 2. Fetch StructuralDescription
        let structural = structural_db
            .region(candidate.region)
            .ok_or_else(|| {
                RewriteError::RecipeFailed(format!(
                    "no structural description for region {:?}",
                    candidate.region
                ))
            })?
            .clone();

        // 3. Derive the application binding (canonical binder — the
        //    ONE legitimate role scan) and assemble RewriteRegion.
        //    Reduction recipes refuse to rewrite without a binding;
        //    scalar recipes proceed without one.
        let rewrite_region =
            match binding_for_region(function, facts, &structural, authorization_db) {
                Ok(Some(binding)) => RewriteRegion::new(structural).with_binding(binding),
                Ok(None) => RewriteRegion::new(structural),
                Err(binding_error) => {
                    return Err(RewriteError::RecipeFailed(format!(
                        "application binding refused: {binding_error}"
                    )));
                }
            };

        // 4. Look up recipe
        let recipe = self
            .recipe_registry
            .lookup(candidate.definition_id)
            .ok_or_else(|| {
                RewriteError::RecipeFailed(format!(
                    "no recipe for definition {}",
                    candidate.definition_id
                ))
            })?;

        // 5. Invoke recipe → ReplacementPatch
        let builder = crate::subgraph_builder::SubgraphBuilder::with_function(function);
        let patch = recipe.build_patch(function, &rewrite_region, builder)?;

        // 5.5 Concrete base binding (advisor P0A item 2): when the
        // authorization binds concrete memory bases, any collection the
        // recipe binds must be one of them. A recipe reaching a
        // different array (the "authorize A, rewrite B" confusion) is
        // denied here. Regions without bound bases (pure scalar) skip.
        if let Ok(collection) = rewrite_region.collection() {
            let bases = &candidate.authorization.concrete.memory_bases;
            if !bases.is_empty() && !bases.contains(&collection) {
                return Err(RewriteError::RecipeFailed(format!(
                    "recipe binds collection %{} outside the authorized memory bases {:?}",
                    collection.0, bases
                )));
            }
        }

        // 5.75 Candidate frame + application assurance (advisor
        //      directive: source frame ↔ candidate frame; the
        //      matched end-to-end artifact is the ONLY route to
        //      mutation). The candidate must not introduce writes,
        //      calls, allocations, loops, traps, or memory reads
        //      beyond the authorized collection.
        let application = match &rewrite_region.binding {
            Some(binding) => {
                check_candidate_frame(function, &patch, binding, candidate)?;
                let live_out_digest = live_out_digest(&binding.live_outs);
                let app = CheckedApplication::new(
                    candidate.authorization.authorization_id.0,
                    sir_semantics::authorization::function_fingerprint(function),
                    candidate.region.0,
                    candidate.id.0,
                    binding.map.digest(),
                    live_out_digest,
                    binding.frame.supported_conservative(),
                    true,
                    0,
                    proof.assurance.min(VerificationStatus::SchemaChecked),
                    proof.obligation_digest,
                );
                match EndToEndVerificationArtifact::new(proof.clone(), app) {
                    Ok(e2e) => Some(e2e),
                    Err(mismatch) => {
                        return Err(RewriteError::RecipeFailed(format!(
                            "end-to-end artifact construction failed: {mismatch:?}"
                        )));
                    }
                }
            }
            None => None,
        };

        // 6. Assemble RewritePlan
        let plan = RewritePlan {
            region: rewrite_region,
            patch,
            proof: proof.clone(),
        };

        // 7. RewriteBuilder::apply()
        let rewritten = RewriteBuilder::apply(function, plan)?;

        // 8. Run structural verification
        let mut verifier = sir_verify::Verifier::new(&rewritten);
        if !verifier.verify() {
            return Err(RewriteError::StructuralVerificationFailed(
                verifier.errors().to_vec(),
            ));
        }

        // 9. Compute provenance and diff
        let provenance = Self::compute_provenance(candidate);
        let diff = Self::compute_diff(function, &rewritten);

        Ok(RewriteResult {
            rewritten,
            provenance,
            diff,
            proof: proof.clone(),
            end_to_end: application,
        })
    }

    /// Verify Candidate.definition_id == Recipe.definition()
    fn verify_ids(&self, candidate: &Candidate, _proof: &Proof) -> Result<(), RewriteError> {
        let recipe_id = self
            .recipe_registry
            .lookup(candidate.definition_id)
            .map(|r| r.definition())
            .ok_or_else(|| {
                RewriteError::RecipeFailed(format!(
                    "no recipe for definition {}",
                    candidate.definition_id
                ))
            })?;

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

        let original_ids: BTreeSet<sir_types::NodeId> =
            original.arena.nodes().keys().copied().collect();

        let rewritten_ids: BTreeSet<sir_types::NodeId> =
            rewritten.arena.nodes().keys().copied().collect();

        let removed_nodes: BTreeSet<_> = original_ids.difference(&rewritten_ids).copied().collect();

        let added_nodes: BTreeSet<_> = rewritten_ids.difference(&original_ids).copied().collect();

        crate::result::GraphDiff {
            removed_nodes,
            added_nodes,
            modified_edges: Vec::new(), // v0.1: edge changes computed in future refinement
        }
    }
}

/// Derive the canonical ProposalBinding for a region that carries a
/// reduction role set; `Ok(None)` for regions without one (scalar
/// recipes proceed under their own authorization). The concrete facts
/// come from the candidate's authorization (retrieved from the
/// database — the authority — not from the candidate's carried copy).
fn binding_for_region(
    function: &Function,
    facts: &FactDatabase,
    structural: &sir_semantics::structure::StructuralDescription,
    authorization_db: &sir_semantics::authorization::AuthorizationDatabase,
) -> Result<Option<ProposalBinding>, sir_semantics::binding::BindingError> {
    if !has_reduction_roles(structural) {
        return Ok(None);
    }
    let concrete = authorization_db
        .for_region(structural.region)
        .first()
        .map(|auth| auth.concrete.clone())
        .unwrap_or_default();
    match derive_proposal_binding(function, facts, structural, &concrete) {
        Ok(binding) => Ok(Some(binding)),
        Err(e) => Err(e),
    }
}

fn has_reduction_roles(structural: &sir_semantics::structure::StructuralDescription) -> bool {
    structural.roles.iter().any(|role| {
        matches!(
            role,
            sir_transform::roles::RegionRoles::BooleanCollectionReduction { .. }
                | sir_transform::roles::RegionRoles::PredicateCollectionReduction { .. }
        )
    })
}

/// Candidate-frame check (advisor: source frame ↔ candidate frame).
/// The candidate must not introduce writes, calls, allocations, nested
/// loops, standalone loads, or division traps, and every external
/// input it references must be a certified live-in of the binding
/// (reads never go beyond the authorized collection).
fn check_candidate_frame(
    function: &Function,
    patch: &ReplacementPatch,
    binding: &ProposalBinding,
    _candidate: &Candidate,
) -> Result<(), RewriteError> {
    use std::collections::BTreeSet;

    let original_ids: BTreeSet<NodeId> = function.arena.nodes().keys().copied().collect();
    let allowed_inputs: BTreeSet<NodeId> = binding.map.live_ins.iter().copied().collect();

    for node in patch.arena.nodes() {
        // Forbidden kinds: the conservative frame admits NO writes,
        // calls, allocations, nested loops, standalone loads, or
        // division traps in the candidate. Vectorized reads happen
        // ONLY through Pack/ArrayCmpMask, whose array operand is the
        // authorized collection (checked below).
        let forbidden = matches!(
            node.kind,
            NodeKind::Store { .. }
                | NodeKind::Call { .. }
                | NodeKind::Intrinsic { .. }
                | NodeKind::ExternalCall { .. }
                | NodeKind::Allocate { .. }
                | NodeKind::Deallocate { .. }
                | NodeKind::Loop { .. }
                | NodeKind::Load { .. }
                | NodeKind::Div { .. }
                | NodeKind::Rem { .. }
        );
        if forbidden {
            return Err(RewriteError::RecipeFailed(format!(
                "candidate introduces a node the conservative frame forbids: {:?}",
                node.kind
            )));
        }
        // Every external input of every candidate node must be a
        // certified live-in (collection, constants, scalar — all
        // bound by the role map). An uncertified input is an
        // over-read/over-reach and refuses the rewrite.
        for input in node.kind.input_nodes() {
            if original_ids.contains(&input) && !allowed_inputs.contains(&input) {
                return Err(RewriteError::RecipeFailed(format!(
                    "candidate references uncertified input %{}",
                    input.0
                )));
            }
        }
    }
    Ok(())
}

/// Digest of the complete live-out classification: every slot's kind,
/// binding state (with use-closure evidence), and replacement site.
/// Two functions with the same digest expose the same observable
/// interface to this rewrite.
fn live_out_digest(live_outs: &[LiveOutSlot]) -> u64 {
    let mut parts: Vec<String> = live_outs
        .iter()
        .map(|slot| {
            format!(
                "kind={:?}|binding={:?}|site={:?}",
                slot.kind, slot.binding, slot.use_site
            )
        })
        .collect();
    parts.sort();
    sir_verification::artifact::fnv1a64(parts.join(";").as_bytes())
}
