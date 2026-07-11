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
struct IterationResult {
    function: Function,
    record: IterationRecord,
    converged: bool,
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
        let mut current = function.clone();
        let mut total_rewrites: usize = 0;
        let mut iterations_detail: Vec<IterationRecord> = Vec::new();

        for iteration in 1..=self.config.max_iterations {
            let result = self.optimize_iteration(&current, iteration);
            total_rewrites += result.record.rewrites_applied;
            iterations_detail.push(result.record);

            if result.converged {
                return OptimizationResult {
                    function: result.function,
                    iterations: iteration,
                    rewrites_applied: total_rewrites,
                    iterations_detail,
                    termination: TerminationReason::FixedPoint,
                };
            }

            if let Some(max_rewrites) = self.config.max_total_rewrites {
                if total_rewrites >= max_rewrites {
                    return OptimizationResult {
                        function: result.function,
                        iterations: iteration,
                        rewrites_applied: total_rewrites,
                        iterations_detail,
                        termination: TerminationReason::IterationLimitReached,
                    };
                }
            }

            current = result.function;
        }

        OptimizationResult {
            function: current,
            iterations: self.config.max_iterations,
            rewrites_applied: total_rewrites,
            iterations_detail,
            termination: TerminationReason::IterationLimitReached,
        }
    }

    /// Execute one full pipeline pass.
    ///
    /// 1. Analysis  → run_all()
    /// 2. Semantics → derive() (includes cost derivation)
    /// 3. Inference → infer()
    /// 4. Generation → generate()
    /// 5. Verification → build_obligations() + verify()
    /// 6. Selection → select_all()
    /// 7. Rewrite → exactly one per iteration (highest score)
    fn optimize_iteration(&self, function: &Function, iteration_number: usize) -> IterationResult {
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
            return IterationResult {
                function: function.clone(),
                record: IterationRecord {
                    iteration: iteration_number,
                    facts_discovered,
                    truths_discovered,
                    beliefs_inferred,
                    candidates_generated: 0,
                    concepts_discovered: concepts_discovered.clone(),
                    representations_inferred: representations_inferred.clone(),
                    outcome: IterationOutcome::NoCandidate,
                    ..Default::default()
                },
                converged: true,
            };
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
            return IterationResult {
                function: function.clone(),
                record: IterationRecord {
                    iteration: iteration_number,
                    facts_discovered,
                    truths_discovered,
                    beliefs_inferred,
                    candidates_generated: candidate_count,
                    proofs_attempted,
                    proofs_succeeded: 0,
                    concepts_discovered: concepts_discovered.clone(),
                    representations_inferred: representations_inferred.clone(),
                    outcome: IterationOutcome::NoProof,
                    ..Default::default()
                },
                converged: true,
            };
        }

        // ── 6. Selection ──────────────────────────────────────
        let selector = Selector::new(DefaultCostModel);
        let cost_db = semantics.cost_database();
        let selection = selector.select_all(&proven, cost_db);

        if selection.chosen.is_empty() {
            return IterationResult {
                function: function.clone(),
                record: IterationRecord {
                    iteration: iteration_number,
                    facts_discovered,
                    truths_discovered,
                    beliefs_inferred,
                    candidates_generated: candidate_count,
                    proofs_attempted,
                    proofs_succeeded,
                    candidates_selected: 0,
                    concepts_discovered: concepts_discovered.clone(),
                    representations_inferred: representations_inferred.clone(),
                    outcome: IterationOutcome::NoSelection,
                    ..Default::default()
                },
                converged: true,
            };
        }

        let candidates_selected = selection.chosen.len();

        // ── 7. Rewrite (exactly one per iteration) ────────────
        // Apply only the highest-scoring candidate. Multiple rewrites
        // are sequenced across fixed-point iterations — this eliminates
        // overlapping-rewrite concerns entirely.
        let best = &selection.chosen[0];

        // LOGGING HACK
        println!(
            "Iteration {}: Selected candidate {} with strategy {:?}",
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
                (function.clone(), 0usize)
            }
        };

        IterationResult {
            function: next_function,
            record: IterationRecord {
                iteration: iteration_number,
                facts_discovered,
                truths_discovered,
                beliefs_inferred,
                candidates_generated: candidate_count,
                proofs_attempted,
                proofs_succeeded,
                candidates_selected,
                rewrites_applied,
                concepts_discovered,
                representations_inferred,
                outcome: if rewrites_applied > 0 {
                    IterationOutcome::RewriteApplied
                } else {
                    IterationOutcome::NoSelection
                },
            },
            converged: rewrites_applied == 0,
        }
    }
}
