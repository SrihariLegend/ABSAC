use sir_nodes::Function;
use sir_optimizer::{Optimizer, OptimizerConfig};
use sir_rewrite::registry::default_registry;
use sir_optimizer::result::TerminationReason;
use sir_analysis::manager::AnalysisManager;
use sir_semantics::semantics::SemanticEngine;
use sir_inference::engine::InferenceEngine;
use sir_generation::generator::CandidateGenerator;
use sir_generation::candidate::Candidate;
use sir_semantics::truth::SemanticTruth;

#[derive(Clone)]
pub enum ExpectedKnowledge {
    Optimizes {
        semantic_domain: &'static str,
        concepts: Vec<&'static str>,
        representation: &'static str,
        candidate: &'static str,
        proof: &'static str,
        rewrite: &'static str,
    },
    MissingKnowledge {
        concepts: Vec<&'static str>,
        closure: Vec<&'static str>,
        representations: Vec<&'static str>,
        rewrites: Vec<&'static str>,
    },
    NonOptimizable {
        reason: &'static str,
    },
    ProvenanceGraph {
        expected_truths: Vec<&'static str>,
        validation: fn(&[sir_semantics::truth::SemanticTruth], &[sir_generation::candidate::Candidate]),
    },
}

#[derive(Clone)]
pub struct BenchmarkSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub input_desc: &'static str,
    pub expected: ExpectedKnowledge,
}

#[derive(Clone)]
pub struct BenchmarkDef {
    pub spec: BenchmarkSpec,
    pub func: fn() -> Function,
}

/// Knowledge gathered from one pipeline pass (analysis → semantics →
/// inference → generation), independent of whether a rewrite applied.
struct KnowledgePass {
    truths: Vec<SemanticTruth>,
    candidates: Vec<Candidate>,
    concepts_discovered: Vec<String>,
    representations_inferred: Vec<String>,
    facts_discovered: usize,
    truths_discovered: usize,
    beliefs_inferred: usize,
    candidates_generated: usize,
}

/// Run the knowledge pipeline directly, without requiring a rewrite.
///
/// Provenance-graph benchmarks validate the knowledge model (truths and
/// candidates) rather than rewrite execution, so they consume this pass
/// instead of the optimizer's (rewrite-only) iteration records.
fn knowledge_pass(func: &Function) -> KnowledgePass {
    let mut analysis = AnalysisManager::new();
    analysis.run_all(func);
    let facts_discovered = analysis.database().total_facts();

    let mut semantics = SemanticEngine::new();
    semantics.derive(func, analysis.database());

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
    let truths: Vec<SemanticTruth> = semantics.database().truths().cloned().collect();

    let mut inference = InferenceEngine::new();
    inference.infer(semantics.database(), semantics.structural_database());

    let mut representations_inferred = Vec::new();
    let mut beliefs_inferred = 0;
    for (_, ctxs) in inference.context_database().contexts() {
        beliefs_inferred += ctxs.len();
        for ctx in ctxs {
            representations_inferred.push(format!("{:?}", ctx.representation));
        }
    }

    let authorizations = sir_semantics::authorization::derive_authorizations(
        func,
        analysis.database(),
        semantics.database(),
    );
    let mut generator = CandidateGenerator::new();
    generator.generate(inference.context_database(), semantics.database(), &authorizations);
    let candidates: Vec<Candidate> = generator.database().all_candidates().cloned().collect();
    let candidates_generated = candidates.len();

    KnowledgePass {
        truths,
        candidates,
        concepts_discovered,
        representations_inferred,
        facts_discovered,
        truths_discovered,
        beliefs_inferred,
        candidates_generated,
    }
}

fn check(label: &str, actual: bool) {
    let symbol = if actual { "✓" } else { "✗" };
    println!("  {} {}", symbol, label);
}

pub fn run_benchmark(func: Function, spec: &BenchmarkSpec) {
    println!("\nBenchmark {} - {}", spec.id, spec.name);
    println!("Category:\n  {}", spec.category);
    println!("Input:\n  {}\n", spec.input_desc);
    
    match &spec.expected {
        ExpectedKnowledge::Optimizes { semantic_domain, concepts, representation, candidate, proof, rewrite } => {
            println!("Specification:");
            println!("  Expected: Optimizes");
            println!("  SemanticDomain: {}", semantic_domain);
            println!("  Concepts: {:?}", concepts);
            println!("  Representation: {}", representation);
            println!("  Candidate: {}", candidate);
            println!("  Proof: {}", proof);
            println!("  Rewrite: {}\n", rewrite);
        },
        ExpectedKnowledge::MissingKnowledge { concepts, closure, representations, rewrites } => {
            println!("Specification:");
            println!("  Expected: MissingKnowledge");
            println!("  Missing Concepts:       {:?}", concepts);
            println!("  Missing Closure Rules:  {:?}", closure);
            println!("  Missing Reps:           {:?}", representations);
            println!("  Missing Rewrites:       {:?}\n", rewrites);
        },
        ExpectedKnowledge::NonOptimizable { reason } => {
            println!("Specification:");
            println!("  Expected: NonOptimizable");
            println!("  Reason: {}\n", reason);
        },
        ExpectedKnowledge::ProvenanceGraph { expected_truths, .. } => {
            println!("Specification:");
            println!("  Expected: ProvenanceGraph test");
            println!("  Expected Truths: {:?}\n", expected_truths);
        }
    }

    // Provenance-graph benchmarks validate the knowledge model — the truths
    // and candidates produced by the pipeline — not rewrite execution. The
    // optimizer only records iterations when a rewrite applies, so run the
    // knowledge pipeline directly for this benchmark kind.
    if let ExpectedKnowledge::ProvenanceGraph { validation, .. } = &spec.expected {
        let knowledge = knowledge_pass(&func);

        println!("Discovered Concepts:");
        for c in &knowledge.concepts_discovered {
            println!("  - {}", c);
        }
        println!("Inferred Representations:");
        for r in &knowledge.representations_inferred {
            println!("  - {}", r);
        }
        println!();
        println!("Execution:");
        check("Facts", knowledge.facts_discovered > 0);
        check("Semantic concepts", knowledge.truths_discovered > 0);
        check("Representation", knowledge.beliefs_inferred > 0);
        check("Candidate generation", knowledge.candidates_generated > 0);
        println!();

        println!("Result: VALIDATING PROVENANCE GRAPH...");
        validation(&knowledge.truths, &knowledge.candidates);
        println!("Result: PROVENANCE GRAPH VALID (Matches Specification)");
        return;
    }

    let config = OptimizerConfig::default();
    let registry = default_registry();
    let optimizer = Optimizer::new(config, registry);
    
    let result = optimizer.optimize(&func);
    
    let record = result.iterations_detail.first().cloned().unwrap_or_default();
    
    println!("Discovered Concepts:");
    for c in &record.concepts_discovered {
        println!("  - {}", c);
    }
    println!("Inferred Representations:");
    for r in &record.representations_inferred {
        println!("  - {}", r);
    }
    
    let has_facts = record.facts_discovered > 0;
    let has_semantics = record.truths_discovered > 0;
    let has_representation = record.beliefs_inferred > 0;
    let has_candidates = record.candidates_generated > 0;
    let has_proof = record.proofs_succeeded > 0;
    let has_rewrite = result.rewrites_applied > 0;
    let is_fixed_point = result.termination == TerminationReason::FixedPoint;

    println!("Semantic Compression:");
    println!("  Initial IR nodes: {}", result.initial_nodes);
    println!("  Semantic truths:  {}", result.max_truths);
    println!("  Final IR nodes:   {}", result.final_nodes);
    if result.initial_nodes > 0 {
        let ratio = result.final_nodes as f64 / result.initial_nodes as f64;
        println!("  Compression:      {:.2}x", 1.0 / ratio);
    }
    println!();

    println!("Execution:");

    check("Facts", has_facts);
    check("Semantic concepts", has_semantics);
    check("Representation", has_representation);
    check("Candidate generation", has_candidates);
    check("Proof", has_proof);
    check("Rewrite", has_rewrite);
    check("Fixed point", is_fixed_point);
    println!();

    match &spec.expected {
        ExpectedKnowledge::Optimizes { concepts, representation, candidate, rewrite, .. } => {
            assert!(has_semantics, "Expected to find semantics");
            assert!(has_representation, "Expected to infer representation");
            assert!(has_candidates, "Expected to generate candidates");
            assert!(has_proof, "Expected to prove candidates");
            assert!(has_rewrite, "Expected to rewrite");
            
            println!("Chain of Discovery:");
            println!("  Concepts       -> {:?}", concepts);
            println!("  Representation -> {}", representation);
            println!("  Candidate      -> {}", candidate);
            println!("  Rewrite        -> {}", rewrite);
            println!("\nResult: SUCCESS (Matches Specification)");
        },
        ExpectedKnowledge::MissingKnowledge { concepts, closure, representations, rewrites } => {
            let found_concepts = concepts.iter().any(|&c| record.concepts_discovered.iter().any(|rc| rc.contains(c)));
            let found_reps = representations.iter().any(|&r| record.representations_inferred.iter().any(|rr| rr.contains(r)));
            
            if !concepts.is_empty() {
                assert!(!found_concepts, "Found concepts that were supposed to be missing!");
            }
            if !representations.is_empty() {
                assert!(!found_reps, "Found representations that were supposed to be missing!");
            }
            if !rewrites.is_empty() {
                assert!(!has_rewrite, "Rewrote the graph but we expected it to fail due to missing rewrites: {:?}", rewrites);
            }
            
            println!("Result: KNOWLEDGE GAP IDENTIFIED (Fails gracefully due to missing knowledge)");
        },
        ExpectedKnowledge::NonOptimizable { .. } => {
            assert!(!has_rewrite, "Should not have rewritten a non-optimizable benchmark");
            println!("Result: DECLINED OPTIMIZATION (Matches Specification)");
        },
        ExpectedKnowledge::ProvenanceGraph { .. } => {
            // Handled above via the direct knowledge pass.
            unreachable!("ProvenanceGraph handled before the optimizer path");
        }
    }
}
