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
    ExpectedFailure {
        stage: &'static str,
        missing_knowledge: &'static str,
        needed_concept: &'static str,
    },
    NonOptimizable {
        reason: &'static str,
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
        ExpectedKnowledge::ExpectedFailure { stage, missing_knowledge, needed_concept } => {
            println!("Specification:");
            println!("  Expected: ExpectedFailure");
            println!("  Stage: {}", stage);
            println!("  MissingKnowledge: {}", missing_knowledge);
            println!("  NeededConcept: {}\n", needed_concept);
        },
        ExpectedKnowledge::NonOptimizable { reason } => {
            println!("Specification:");
            println!("  Expected: NonOptimizable");
            println!("  Reason: {}\n", reason);
        }
    }

    let config = OptimizerConfig::default();
    let registry = default_registry();
    let optimizer = Optimizer::new(config, registry);
    
    let result = optimizer.optimize(&func);
    
    let record = result.iterations_detail.first().cloned().unwrap_or_default();
    
    let has_facts = record.facts_discovered > 0;
    let has_semantics = record.truths_discovered > 0;
    let has_representation = record.beliefs_inferred > 0;
    let has_candidates = record.candidates_generated > 0;
    let has_proof = record.proofs_succeeded > 0;
    let has_rewrite = result.rewrites_applied > 0;
    let is_fixed_point = result.termination == TerminationReason::FixedPoint;

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
        ExpectedKnowledge::ExpectedFailure { stage, .. } => {
            let stage_matches = match *stage {
                "Semantics" => !has_semantics,
                "Representation" => has_semantics && !has_representation,
                "Candidate" => has_representation && !has_candidates,
                "Proof" => has_candidates && !has_proof,
                "Rewrite" => has_proof && !has_rewrite,
                _ => panic!("Unknown stage {}", stage),
            };
            assert!(stage_matches, "Did not fail at the expected stage: {}", stage);
            println!("Result: ARTIFACT VALID (Fails exactly at expected stage: {})", stage);
        },
        ExpectedKnowledge::NonOptimizable { .. } => {
            assert!(!has_rewrite, "Should not have rewritten a non-optimizable benchmark");
            println!("Result: DECLINED OPTIMIZATION (Matches Specification)");
        }
    }
}
