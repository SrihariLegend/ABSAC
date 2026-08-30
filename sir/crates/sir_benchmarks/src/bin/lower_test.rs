//! lower_test — tests the LLVM IR → SIR lowerer on real kernels.
//!
//! Reads a .ll file, lowers it, runs the pipeline, and reports what happened.

use sir_analysis::manager::AnalysisManager;
use sir_generation::candidate::Candidate;
use sir_generation::generator::CandidateGenerator;
use sir_inference::engine::InferenceEngine;
use sir_lower::lower;
use sir_optimizer::{Optimizer, OptimizerConfig};
use sir_printer::text::TextPrinter;
use sir_rewrite::registry::default_registry;
use sir_semantics::semantics::SemanticEngine;
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: lower_test <file.ll>");
        std::process::exit(1);
    }
    let ll_path = &args[1];
    let ll_text = match fs::read_to_string(ll_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error reading {}: {}", ll_path, e);
            std::process::exit(1);
        }
    };

    println!("=== Lowering {} ===", ll_path);

    let func = match lower(&ll_text) {
        Ok(f) => f,
        Err(e) => {
            println!("LOWER ERROR: {}", e);
            std::process::exit(1);
        }
    };

    let printer = TextPrinter::new(false);
    println!("\n=== Lowered SIR ===");
    println!("{}", printer.function_to_string(&func));

    // Verify
    let mut verifier = sir_verify::Verifier::new(&func);
    print!("Graph invariants: ");
    if verifier.verify() {
        println!("PASS");
    } else {
        println!("FAIL");
        for e in verifier.errors() {
            println!("  - {:?}", e);
        }
    }

    // Run pipeline
    println!("\n=== Pipeline ===");
    let mut analysis = AnalysisManager::new();
    analysis.run_all(&func);
    println!("facts: {}", analysis.database().total_facts());
    for (id, lf) in analysis.database().loops.iter() {
        println!("loop {:?}: reductions={:?}",
            id, lf.reductions.iter().map(|r| (&r.reduction_kind, r.variable)).collect::<Vec<_>>());
    }

    let mut semantics = SemanticEngine::new();
    semantics.derive(&func, analysis.database());
    let truths: Vec<_> = semantics.database().truths().cloned().collect();
    println!("regions: {}", semantics.database().region_count());
    println!("truths:  {}", truths.len());
    for (i, region) in semantics.database().regions() {
        println!("  region {:?}: {:?}", i, region.concepts());
    }

    let mut inference = InferenceEngine::new();
    inference.infer(semantics.database(), semantics.structural_database());
    let mut beliefs = 0;
    for (_, ctxs) in inference.context_database().contexts() {
        beliefs += ctxs.len();
        for ctx in ctxs {
            println!("  belief: {:?}", ctx.representation);
        }
    }
    println!("beliefs: {}", beliefs);

    let mut generator = CandidateGenerator::new();
    generator.generate(inference.context_database(), semantics.database());
    let candidates: Vec<Candidate> = generator.database().all_candidates().cloned().collect();
    println!("candidates: {}", candidates.len());
    for c in &candidates {
        println!("  candidate: {:?} {:?}", c.definition_id, c.strategy);
    }

    // Optimizer
    let config = OptimizerConfig::default();
    let registry = default_registry();
    let optimizer = Optimizer::new(config, registry);
    let result = optimizer.optimize(&func);
    println!("\nrewrites: {} ({} -> {})", result.rewrites_applied, result.initial_nodes, result.final_nodes);

    if result.rewrites_applied > 0 {
        println!("\n=== Rewritten SIR ===");
        println!("{}", printer.function_to_string(&result.function));
    }
}
