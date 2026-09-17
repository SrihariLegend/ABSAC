use sir_analysis::facts::FactDatabase;
use sir_generation::candidate::Candidate;
use sir_nodes::{Function, NodeKind};
use sir_semantics::binding::{
    derive_proposal_binding, LiveOutBinding, LiveOutKind, LiveOutSlot, ProposalBinding,
};
use sir_semantics::structure::StructuralDatabase;
use sir_types::NodeId;
use sir_verification::application_artifact::{ApplicationChecker, EndToEndVerificationArtifact};
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

    /// Whether the configured registry enables a definition for execution.
    /// Candidate generation remains exploratory, but the optimizer uses this
    /// gate before theorem verification/selection so a freeze registry cannot
    /// silently execute another family.
    pub fn supports_definition(&self, definition: sir_transform::ids::DefinitionId) -> bool {
        self.recipe_registry.lookup(definition).is_some()
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
        // Derive the analysis facts this function's region needs for
        // proposal binding (loop reductions, trip counts, live-outs).
        // The production path passes its own facts via rewrite_checked;
        // the legacy convenience entry derives them here.
        let mut analysis = sir_analysis::manager::AnalysisManager::new();
        analysis.run_all(function);
        self.rewrite_checked(
            function,
            candidate,
            proof,
            structural_db,
            &sir_semantics::authorization::AuthorizationDatabase::new(),
            analysis.database(),
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
        // Unit-test AuthorizationRefs have no issuer (documented
        // escape in `for_unit_test`/`exact_binding_matches`); only
        // production candidates (real AuthorizationIds) are gated.
        let is_unit_test = candidate.authorization.authorization_id
            == sir_semantics::authorization::AuthorizationId::UNIT_TEST;
        if !is_unit_test
            && (!candidate.authorization.matches_function(function)
                || !candidate.binding_digest_valid()
                || !sir_generation::candidate::exact_binding_matches(candidate, authorization_db))
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
        let rewrite_region = match binding_for_region(
            function,
            facts,
            &structural,
            authorization_db,
            candidate.authorization.authorization_id,
        ) {
            Ok(Some(binding)) => RewriteRegion::new(structural).with_binding(binding),
            Ok(None) => RewriteRegion::new(structural),
            Err(binding_error) => {
                // PositionSearch candidates are authorized by their own
                // X06 certificate; the reduction binding is not their
                // application artifact (their live-out is the position
                // slot, which the reduction classifier refuses by
                // design). Scalar/position recipes proceed without a
                // binding.
                if candidate
                    .authorization
                    .domains
                    .contains(&sir_semantics::authorization::DomainKind::PositionSearch)
                {
                    RewriteRegion::new(structural)
                } else {
                    return Err(RewriteError::RecipeFailed(format!(
                        "application binding refused: {binding_error}"
                    )));
                }
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
        if let Some(binding) = &rewrite_region.binding {
            let collection = binding.map.collection;
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
                check_candidate_frame(function, &patch, binding, candidate.strategy)?;
                let live_out_digest = live_out_digest(&binding.live_outs);
                let authorization_id = candidate.authorization.authorization_id.0;
                let source_fingerprint =
                    sir_semantics::authorization::function_fingerprint(function);
                let source_region = candidate.region.0;
                let candidate_id = candidate.id.0;
                let definition_id = candidate.definition_id.0;
                let role_map_digest = binding.map.digest();
                let source_frame_digest = binding.frame.digest();
                let candidate_frame_digest = candidate_frame_digest(&patch);
                let assumptions_digest = assumptions_digest(candidate);
                let theorem_issuer = sir_verification::Verifier::new();
                let checked_theorem = theorem_issuer.bind_checked_theorem(
                    proof.clone(),
                    authorization_id,
                    source_fingerprint,
                    source_region,
                    candidate_id,
                    definition_id,
                    role_map_digest,
                    live_out_digest,
                    source_frame_digest,
                    candidate_frame_digest,
                    assumptions_digest,
                );
                let app = ApplicationChecker::issue(
                    authorization_id,
                    source_fingerprint,
                    source_region,
                    candidate_id,
                    definition_id,
                    role_map_digest,
                    live_out_digest,
                    source_frame_digest,
                    candidate_frame_digest,
                    binding.frame.supported_conservative(),
                    true,
                    assumptions_digest,
                    proof.assurance.min(VerificationStatus::SchemaChecked),
                    checked_theorem.theorem_digest,
                    proof.obligation_digest,
                );
                match EndToEndVerificationArtifact::new(checked_theorem, app) {
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
    authorization_id: sir_semantics::authorization::AuthorizationId,
) -> Result<Option<ProposalBinding>, sir_semantics::binding::BindingError> {
    if !has_reduction_roles(structural) {
        return Ok(None);
    }
    let concrete = if authorization_id == sir_semantics::authorization::AuthorizationId::UNIT_TEST {
        // Legacy/unit-test entry points have no issuing database. Keep
        // their existing default-deny behavior without weakening the
        // production exact-id path below.
        authorization_db
            .for_region(structural.region)
            .first()
            .map(|auth| auth.concrete.clone())
            .unwrap_or_default()
    } else {
        let authorization = authorization_db.authorization(authorization_id).ok_or(
            sir_semantics::binding::BindingError::AuthorizationMismatch(
                "authorization id was not issued by this database",
            ),
        )?;
        if authorization.region != structural.region {
            return Err(sir_semantics::binding::BindingError::AuthorizationMismatch(
                "authorization region differs from structural region",
            ));
        }
        authorization.concrete.clone()
    };
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
/// Check the abstract candidate frame against the source binding.
///
/// This is intentionally public so adversarial tests and later
/// application-checking stages can exercise the same gate as the
/// rewrite engine. It checks the abstract SIR candidate only; a later
/// LLVM/vector lowering artifact must separately prove concrete load
/// bounds and tail behavior.
pub fn check_candidate_frame(
    function: &Function,
    patch: &ReplacementPatch,
    binding: &ProposalBinding,
    strategy: sir_generation::candidate::ImplementationStrategy,
) -> Result<(), RewriteError> {
    use std::collections::BTreeSet;

    let original_ids: BTreeSet<NodeId> = function.arena.nodes().keys().copied().collect();
    let local_ids: BTreeSet<NodeId> = patch.arena.nodes().map(|node| node.id).collect();
    let is_any = strategy == sir_generation::candidate::ImplementationStrategy::Any;
    let allowed_candidate_inputs: BTreeSet<NodeId> = if is_any {
        let mut inputs = BTreeSet::from([binding.map.collection]);
        if let Some(scalar) = binding.map.predicate_scalar {
            inputs.insert(scalar);
        }
        inputs
    } else {
        binding.map.live_ins.iter().copied().collect()
    };
    let collection_width = if is_any {
        match function
            .get_node(binding.map.collection)
            .map(|node| &node.ty)
        {
            Some(sir_types::Type::Array { length, .. }) => Some(*length),
            _ => {
                return Err(RewriteError::RecipeFailed(
                    "bound collection has no fixed array extent".to_string(),
                ));
            }
        }
    } else {
        None
    };

    if is_any {
        let element_ty_for_shape = collection_element_type(function, binding);
        let implicit_for_shape = element_ty_for_shape.as_ref().is_some_and(|ty| {
            crate::recipes::helpers::implicit_element_nonzero(&binding.map, ty)
        });
        check_any_patch_shape(
            patch,
            binding,
            collection_width.unwrap(),
            element_ty_for_shape.as_ref(),
            implicit_for_shape,
        )?
    }

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
                | NodeKind::Parameter { .. }
                | NodeKind::Return { .. }
                | NodeKind::Load { .. }
                | NodeKind::ArrayAccess { .. }
                | NodeKind::Iterator { .. }
                | NodeKind::Div { .. }
                | NodeKind::Rem { .. }
        );
        if forbidden {
            return Err(RewriteError::RecipeFailed(format!(
                "candidate introduces a node the conservative frame forbids: {:?}",
                node.kind
            )));
        }
        if !node.effects.is_pure() {
            return Err(RewriteError::RecipeFailed(
                "candidate introduces a non-pure effect outside the abstract frame".to_string(),
            ));
        }
        if is_any {
            match &node.kind {
                NodeKind::Pack { array } => {
                    if binding.map.predicate_op.is_some() {
                        return Err(RewriteError::RecipeFailed(
                            "predicate binding requires ArrayCmpMask, not Pack".to_string(),
                        ));
                    }
                    if *array != binding.map.collection {
                        return Err(RewriteError::RecipeFailed(
                            "candidate reads a collection other than the bound collection"
                                .to_string(),
                        ));
                    }
                    if node.ty
                        != (sir_types::Type::BitVector {
                            width: collection_width.unwrap(),
                        })
                    {
                        return Err(RewriteError::RecipeFailed(
                            "candidate pack extent differs from the bound collection".to_string(),
                        ));
                    }
                }
                NodeKind::ArrayCmpMask { array, scalar, op } => {
                    if *array != binding.map.collection {
                        return Err(RewriteError::RecipeFailed(
                            "candidate reads a collection other than the bound collection"
                                .to_string(),
                        ));
                    }
                    if binding.map.predicate_op.is_some() {
                        if binding.map.predicate_op != Some(*op)
                            || binding.map.predicate_scalar != Some(*scalar)
                        {
                            return Err(RewriteError::RecipeFailed(
                                "candidate predicate does not match the bound operator and scalar"
                                    .to_string(),
                            ));
                        }
                    } else {
                        // D4: implicit element-truthiness form — the
                        // certified raw-element OR reduction lowers to
                        // `mask(x != 0)`, whose scalar is the canonical
                        // zero of the element type, synthesized as a
                        // patch-local constant.
                        let element_ty = collection_element_type(function, binding);
                        let scalar_is_element_zero = match (
                            &element_ty,
                            patch
                                .arena
                                .get(crate::local_id::LocalNodeId::new(scalar.as_u64())),
                        ) {
                            (Some(ty), Some(node)) => {
                                node.ty == *ty
                                    && matches!(
                                        &node.kind,
                                        NodeKind::Constant(data)
                                            if data.as_u64() == Some(0)
                                                || data.as_i64() == Some(0)
                                    )
                            }
                            _ => false,
                        };
                        let certified = element_ty
                            .as_ref()
                            .is_some_and(|ty| {
                                crate::recipes::helpers::implicit_element_nonzero(&binding.map, ty)
                            });
                        if *op != sir_nodes::CmpOperator::Ne
                            || !scalar_is_element_zero
                            || !certified
                        {
                            return Err(RewriteError::RecipeFailed(
                                "implicit element predicate is not the certified `x[i] != 0` form"
                                    .to_string(),
                            ));
                        }
                    }
                    if node.ty
                        != (sir_types::Type::BitVector {
                            width: collection_width.unwrap(),
                        })
                    {
                        return Err(RewriteError::RecipeFailed(
                            "candidate mask extent differs from the bound collection".to_string(),
                        ));
                    }
                }
                _ => {}
            }
        }
        // Every external input of every candidate node must be a
        // certified live-in (collection, constants, scalar — all
        // bound by the role map). An uncertified input is an
        // over-read/over-reach and refuses the rewrite.
        for input in node.kind.input_nodes() {
            if original_ids.contains(&input) {
                if !allowed_candidate_inputs.contains(&input) {
                    return Err(RewriteError::RecipeFailed(format!(
                        "candidate references uncertified input %{}",
                        input.0
                    )));
                }
            } else if !local_ids.contains(&input) {
                return Err(RewriteError::RecipeFailed(format!(
                    "candidate contains a dangling input %{}",
                    input.0
                )));
            }
        }
    }
    Ok(())
}

/// Check the complete Any patch grammar, not merely its effects and
/// external operands. The theorem definition describes `pack/mask != 0`;
/// accepting an arbitrary pure detached graph would let a mutated recipe
/// replace the source with a constant while still passing the frame gate.
fn check_any_patch_shape(
    patch: &ReplacementPatch,
    binding: &ProposalBinding,
    width: usize,
    element_ty: Option<&sir_types::Type>,
    implicit_element_nonzero: bool,
) -> Result<(), RewriteError> {
    // The explicit/Pack forms have exactly three nodes (packed, zero,
    // nonzero); the implicit element form has a fourth — the canonical
    // element zero compared by the mask.
    let expected_node_count = if implicit_element_nonzero { 4 } else { 3 };
    if patch.arena.len() != expected_node_count
        || patch.replacements.len() != 1
        || patch.roots.len() != expected_node_count
    {
        return Err(RewriteError::RecipeFailed(
            "Any candidate graph is not exactly pack/mask, zero, and nonzero".to_string(),
        ));
    }

    let target = binding
        .live_outs
        .iter()
        .filter_map(|observable| match &observable.binding {
            LiveOutBinding::Reconstructed { .. } => {
                let site = observable.use_site?;
                Some(match observable.kind {
                    LiveOutKind::WholeValue => binding.map.loop_node,
                    LiveOutKind::Slot(_) => site,
                })
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let [target] = target.as_slice() else {
        return Err(RewriteError::RecipeFailed(
            "Any binding does not expose exactly one reconstructed target".to_string(),
        ));
    };
    let replacement = &patch.replacements[0];
    if replacement.old != *target || !patch.roots.contains(&replacement.new) {
        return Err(RewriteError::RecipeFailed(
            "Any patch target is not the bound live-out site".to_string(),
        ));
    }

    let root = patch.arena.get(replacement.new).ok_or_else(|| {
        RewriteError::RecipeFailed("Any patch root is missing from its detached arena".to_string())
    })?;
    let NodeKind::Ne { lhs, rhs } = &root.kind else {
        return Err(RewriteError::RecipeFailed(
            "Any patch root is not a nonzero comparison".to_string(),
        ));
    };
    if root.ty != sir_types::Type::Bool {
        return Err(RewriteError::RecipeFailed(
            "Any patch root is not Bool".to_string(),
        ));
    }

    let packed_id = crate::local_id::LocalNodeId::new(lhs.as_u64());
    let zero_id = crate::local_id::LocalNodeId::new(rhs.as_u64());
    let packed = patch.arena.get(packed_id).ok_or_else(|| {
        RewriteError::RecipeFailed("Any nonzero comparison has no packed operand".to_string())
    })?;
    let zero = patch.arena.get(zero_id).ok_or_else(|| {
        RewriteError::RecipeFailed("Any nonzero comparison has no zero operand".to_string())
    })?;
    let expected_nodes: std::collections::BTreeSet<_> = patch.roots.iter().copied().collect();
    let actual_nodes: std::collections::BTreeSet<_> = patch.arena.inner().keys().copied().collect();
    if actual_nodes != expected_nodes
        || !expected_nodes.contains(&packed_id)
        || !expected_nodes.contains(&zero_id)
    {
        return Err(RewriteError::RecipeFailed(
            "Any patch contains an unreachable or duplicate-shape node".to_string(),
        ));
    }
    if zero.ty != (sir_types::Type::BitVector { width })
        || !matches!(&zero.kind, NodeKind::Constant(data) if data.as_u64() == Some(0))
    {
        return Err(RewriteError::RecipeFailed(
            "Any comparison operand is not a zero bit-vector of the bound extent".to_string(),
        ));
    }
    if packed.ty != (sir_types::Type::BitVector { width }) {
        return Err(RewriteError::RecipeFailed(
            "Any packed operand has the wrong extent".to_string(),
        ));
    }
    match (
        &packed.kind,
        binding.map.predicate_op,
        binding.map.predicate_scalar,
    ) {
        (NodeKind::Pack { array }, None, None) if *array == binding.map.collection => {}
        (
            NodeKind::ArrayCmpMask { array, scalar, op },
            Some(expected_op),
            Some(expected_scalar),
        ) if *array == binding.map.collection
            && *scalar == expected_scalar
            && *op == expected_op => {}
        (NodeKind::ArrayCmpMask { array, scalar, op }, None, None)
            if implicit_element_nonzero
                && *array == binding.map.collection
                && *op == sir_nodes::CmpOperator::Ne =>
        {
            // The implicit form's scalar must be the canonical zero of
            // the element type, synthesized inside the patch.
            let element_ty = element_ty.ok_or_else(|| {
                RewriteError::RecipeFailed(
                    "implicit element predicate without a collection element type".to_string(),
                )
            })?;
            let scalar_node = patch
                .arena
                .get(crate::local_id::LocalNodeId::new(scalar.as_u64()))
                .ok_or_else(|| {
                    RewriteError::RecipeFailed(
                        "implicit element predicate scalar is not a patch node".to_string(),
                    )
                })?;
            let is_element_zero = scalar_node.ty == *element_ty
                && matches!(
                    &scalar_node.kind,
                    NodeKind::Constant(data)
                        if data.as_u64() == Some(0) || data.as_i64() == Some(0)
                );
            if !is_element_zero {
                return Err(RewriteError::RecipeFailed(
                    "implicit element predicate scalar is not the element type's zero"
                        .to_string(),
                ));
            }
        }
        _ => {
            return Err(RewriteError::RecipeFailed(
                "Any packed operand does not match the bound collection predicate".to_string(),
            ));
        }
    }
    Ok(())
}

/// The element type of the binding's collection (None when the
/// collection is not a fixed-length array — the frame check already
/// refuses that case before this is used).
fn collection_element_type(
    function: &Function,
    binding: &ProposalBinding,
) -> Option<sir_types::Type> {
    match function.get_node(binding.map.collection).map(|n| &n.ty) {
        Some(sir_types::Type::Array { element, .. }) => Some((**element).clone()),
        _ => None,
    }
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
                "kind={:?}|binding={:?}|closure={:?}|site={:?}",
                slot.kind, slot.binding, slot.closure, slot.use_site
            )
        })
        .collect();
    parts.sort();
    sir_verification::artifact::fnv1a64(parts.join(";").as_bytes())
}

/// Digest the exact detached candidate graph after the candidate-frame
/// checker has discharged it. This binds application artifacts to the
/// graph that was actually checked, not merely to a candidate ID.
fn candidate_frame_digest(patch: &ReplacementPatch) -> u64 {
    sir_verification::artifact::fnv1a64(format!("{:?}", patch).as_bytes())
}

/// Assumptions are set-valued; sort their debug forms before digesting
/// so artifact identity is deterministic across HashSet iteration order.
fn assumptions_digest(candidate: &Candidate) -> u64 {
    let mut assumptions: Vec<String> = candidate
        .assumptions
        .iter()
        .map(|assumption| format!("{:?}", assumption))
        .collect();
    assumptions.sort();
    sir_verification::artifact::fnv1a64(assumptions.join("|").as_bytes())
}
