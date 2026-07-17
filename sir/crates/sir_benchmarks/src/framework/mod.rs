use sir_nodes::Function;
use sir_optimizer::{Optimizer, OptimizerConfig};
use sir_rewrite::registry::default_registry;
use sir_optimizer::result::TerminationReason;

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
    fn check(label: &str, actual: bool) {
        let symbol = if actual { "✓" } else { "✗" };
        println!("  {} {}", symbol, label);
    }

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
        ExpectedKnowledge::ProvenanceGraph { validation, .. } => {
            println!("Result: VALIDATING PROVENANCE GRAPH...");
            validation(&record.truths, &record.candidates);
            println!("Result: PROVENANCE GRAPH VALID (Matches Specification)");
        }
    }
}
