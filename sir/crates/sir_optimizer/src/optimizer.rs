use sir_analysis::manager::AnalysisManager;
use sir_generation::generator::CandidateGenerator;
use sir_inference::engine::InferenceEngine;
use sir_nodes::Function;
use sir_rewrite::engine::RewriteEngine;
use sir_rewrite::recipe::RecipeRegistry;
use sir_selection::selector::{Selector, VerifiedCandidate};
use sir_selection::DefaultCostModel;
use sir_semantics::semantics::SemanticEngine;
use sir_verification::{VerificationResult, Verifier};

use crate::config::OptimizerConfig;
use crate::result::{IterationOutcome, IterationRecord, OptimizationResult, TerminationReason};

/// Fixed-point optimization driver.
///
/// Owns configuration and the rewrite engine (which is stateless —
/// a pure function of its inputs). All other pipeline stages are
/// constructed fresh each iteration. The optimizer carries no mutable
/// state, no caches, and never walks SIR.
pub struct Optimizer {
    config: OptimizerConfig,
    rewrite_engine: RewriteEngine,
}

/// Internal result from a single iteration.
/// Global search state for the optimizer.
#[derive(Clone)]
struct SearchState {
    function: Function,
    total_rewrites: usize,
    iterations_detail: Vec<IterationRecord>,
}

impl Optimizer {
    /// Create a new optimizer.
    ///
    /// `recipe_registry` maps DefinitionId → graph rewrite recipe.
    /// Cost model is `DefaultCostModel` (v0.1 — single model).
    pub fn new(config: OptimizerConfig, recipe_registry: RecipeRegistry) -> Self {
        let rewrite_engine = RewriteEngine::new(recipe_registry);
        Self {
            config,
            rewrite_engine,
        }
    }

    /// Run optimization to fixed point.
    ///
    /// Idempotent: if optimize(f) = g, then optimize(g) = g.
    /// Accepts `&Function` — the optimizer does not consume its input.
    /// Every iteration constructs fresh pipeline stages from scratch.
    pub fn optimize(&self, function: &Function) -> OptimizationResult {
        let initial_state = SearchState {
            function: function.clone(),
            total_rewrites: 0,
            iterations_detail: Vec::new(),
        };

        let mut current_beam = vec![initial_state];
        let mut fixed_points = Vec::new();
        let beam_width = self.config.beam_width.unwrap_or(3); // Add beam_width to config or hardcode to 3

        for iteration in 1..=self.config.max_iterations {
            let mut next_beam = Vec::new();
            let mut any_advanced = false;

            for state in std::mem::take(&mut current_beam) {
                let branches = self.expand_state(&state.function, iteration);
                
                if branches.is_empty() {
                    // This state has reached a fixed point
                    fixed_points.push(state);
                } else {
                    any_advanced = true;
                    for (next_function, record) in branches {
                        let mut next_state = state.clone();
                        next_state.function = next_function;
                        next_state.total_rewrites += record.rewrites_applied;
                        next_state.iterations_detail.push(record);
                        next_beam.push(next_state);
                    }
                }
            }

            if !any_advanced {
                break; // All paths reached fixed points
            }

            // Prune beam: pick the top N based on the number of reachable nodes from the return node
            next_beam.sort_by_key(|s| {
                let ret_node = s.function.return_node.unwrap();
                sir_analysis::graph::transitive_inputs(ret_node, &s.function.arena).len()
            });
            if next_beam.len() > beam_width {
                next_beam.truncate(beam_width);
            }
            current_beam = next_beam;
        }

        // Add any states that hit the iteration limit to fixed_points for final evaluation
        fixed_points.extend(current_beam);

        // Pick the best terminal state based on reachable nodes
        fixed_points.sort_by_key(|s| {
            let ret_node = s.function.return_node.unwrap();
            sir_analysis::graph::transitive_inputs(ret_node, &s.function.arena).len()
        });
        let best_state = fixed_points.into_iter().next().unwrap();

        let termination = if best_state.iterations_detail.len() < self.config.max_iterations {
            TerminationReason::FixedPoint
        } else {
            TerminationReason::IterationLimitReached
        };

        let initial_nodes = function.arena.len();
        let max_truths = best_state.iterations_detail.iter().map(|r| r.truths_discovered).max().unwrap_or(0);
        let final_nodes = best_state.function.arena.len();

        OptimizationResult {
            function: best_state.function,
            iterations: best_state.iterations_detail.len(),
            rewrites_applied: best_state.total_rewrites,
            iterations_detail: best_state.iterations_detail,
            termination,
            initial_nodes,
            max_truths,
            final_nodes,
        }
    }

    /// Execute one full pipeline pass and return all valid next states (branches).
    fn expand_state(&self, function: &Function, iteration_number: usize) -> Vec<(Function, IterationRecord)> {
        // ── 1. Analysis ───────────────────────────────────────
        let mut analysis = AnalysisManager::new();
        analysis.run_all(function);
        let facts_discovered = analysis.database().total_facts();

        // ── 2. Semantics (recognizers + structure + cost) ──────
        let mut semantics = SemanticEngine::new();
        semantics.derive(function, analysis.database());
        let mut concepts_discovered = Vec::new();
        for (_, region) in semantics.database().regions() {
            for concept in region.concepts() {
                concepts_discovered.push(format!("{:?}", concept));
            }
        }
        for truth in semantics.database().truths() {
            concepts_discovered.push(format!("{:?}", truth.concept));
        }
        let truths_discovered = semantics.database().region_count() + semantics.database().truths().count();

        // ── 3. Inference ──────────────────────────────────────
        let mut inference = InferenceEngine::new();
        inference.infer(semantics.database(), semantics.structural_database());
        let beliefs_inferred = inference
            .context_database()
            .contexts()
            .map(|(_, ctxs)| ctxs.len())
            .sum();

        let mut representations_inferred = Vec::new();
        for (_, ctxs) in inference.context_database().contexts() {
            for ctx in ctxs {
                representations_inferred.push(format!("{:?}", ctx.representation));
            }
        }

        // ── 4. Generation ─────────────────────────────────────
        let mut generator = CandidateGenerator::new();
        generator.generate(inference.context_database(), semantics.database());

        let candidate_count = generator.database().all_candidates().count();
        if candidate_count == 0 {
            return vec![];
        }

        // ── 5. Verification ───────────────────────────────────
        let verifier = Verifier::new();
        let obligations_db =
            verifier.build_obligations(generator.database(), inference.context_database());
        let proofs_attempted = obligations_db.len();
        let mut proven: Vec<VerifiedCandidate> = Vec::new();

        for obligation in obligations_db.all() {
            let contexts = inference.context_database().for_region(obligation.region);
            if let Some(context) = contexts.first() {
                let verification_result = verifier.verify(obligation, context);
                if let VerificationResult::Proven(proof) = &verification_result {
                    // Find the matching candidate from the generator's database
                    for candidate in generator.database().all_candidates() {
                        if candidate.id == obligation.candidate {
                            proven.push(VerifiedCandidate {
                                candidate: candidate.clone(),
                                proof: proof.clone(),
                            });
                            break;
                        }
                    }
                } else {
                    println!(
                        "Failed proof for {:?}: {:?}",
                        obligation.definition, verification_result
                    );
                }
            }
        }

        let proofs_succeeded = proven.len();
        if proven.is_empty() {
            return vec![];
        }

        // ── 6. Selection ──────────────────────────────────────
        let selector = Selector::new(DefaultCostModel);
        let cost_db = semantics.cost_database();
        let selection = selector.select_all(&proven, cost_db);

        if selection.chosen.is_empty() {
            return vec![];
        }

        let candidates_selected = selection.chosen.len();
        let mut branches = Vec::new();
        let beam_width = self.config.beam_width.unwrap_or(3);

        for best in selection.chosen.iter().take(beam_width) {
            println!(
                "Iteration {}: Branching on candidate {} with strategy {:?}",
                iteration_number, best.candidate.id, best.candidate.strategy
            );

            let (next_function, rewrites_applied) = match self.rewrite_engine.rewrite(
                function,
                best.candidate,
                best.proof,
                semantics.structural_database(),
            ) {
                Ok(rewrite_result) => (rewrite_result.rewritten, 1usize),
                Err(e) => {
                    println!("Rewrite failed: {:?}", e);
                    continue;
                }
            };

            let record = IterationRecord {
                iteration: iteration_number,
                facts_discovered,
                truths_discovered,
                beliefs_inferred,
                candidates_generated: candidate_count,
                proofs_attempted,
                proofs_succeeded,
                candidates_selected,
                rewrites_applied,
                concepts_discovered: concepts_discovered.clone(),
                representations_inferred: representations_inferred.clone(),
                truths: semantics.database().truths().cloned().collect(),
                candidates: generator.database().all_candidates().cloned().collect(),
                outcome: IterationOutcome::RewriteApplied,
            };

            branches.push((next_function, record));
        }

        branches
    }
}
