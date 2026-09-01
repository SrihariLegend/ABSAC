//! batch_run — batch-process all kernels from a .ll file.
//!
//! For each function: lower to SIR, run pipeline, report recognition results.
//! This is the scale tool — processes all 50 kernels in one run.
//!
//! Usage: cargo run -p sir_benchmarks --bin batch_run -- <file.ll>

use sir_analysis::manager::AnalysisManager;
use sir_generation::candidate::Candidate;
use sir_generation::generator::CandidateGenerator;
use sir_inference::engine::InferenceEngine;
use sir_lower::{list_functions, lower_function};
use sir_optimizer::{Optimizer, OptimizerConfig};
use sir_printer::text::TextPrinter;
use sir_rewrite::registry::default_registry;
use sir_semantics::semantics::SemanticEngine;
use std::env;
use std::fs;

struct KernelResult {
    name: String,
    lowered: bool,
    verified: bool,
    facts: usize,
    truths: usize,
    concepts: Vec<String>,
    beliefs: usize,
    candidates: usize,
    rewrites: usize,
    initial_nodes: usize,
    final_nodes: usize,
    error: Option<String>,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: batch_run <file.ll>");
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

    let functions = list_functions(&ll_text);
    println!("ABSAC Batch Run: {} functions found in {}\n", functions.len(), ll_path);
    println!("{:<30} {:<8} {:<8} {:<6} {:<6} {:<6} {:<6} {:<6} {:<8} {:<8} {:<8}",
        "Kernel", "Lowered", "Verify", "Facts", "Truths", "Belief", "Cands", "Rewrt", "iNodes", "fNodes", "Error");
    println!("{}", "-".repeat(120));

    let mut results: Vec<KernelResult> = Vec::new();

    for func_name in &functions {
        let mut result = KernelResult {
            name: func_name.clone(),
            lowered: false,
            verified: false,
            facts: 0,
            truths: 0,
            concepts: Vec::new(),
            beliefs: 0,
            candidates: 0,
            rewrites: 0,
            initial_nodes: 0,
            final_nodes: 0,
            error: None,
        };

        // Lower
        let func = match lower_function(&ll_text, func_name) {
            Ok(f) => {
                result.lowered = true;
                f
            }
            Err(e) => {
                result.error = Some(format!("lower: {}", e));
                print_result(&result);
                results.push(result);
                continue;
            }
        };

        // Verify
        // ══════════════════════════════════════════════════════════
        // MANDATORY GATE: only verified SIR may enter analysis/semantics.
        // No code path may invoke semantic recognition on unverified SIR.
        // (Gate 6A-v1 finding: V07 produced invalid SIR and recognition
        // still ran on it — this gate makes that structurally impossible.)
        let mut verifier = sir_verify::Verifier::new(&func);
        result.verified = verifier.verify();
        if !result.verified {
            result.error = Some("verify: SIR failed structural verification — analysis/semantics gated".to_string());
            print_result(&result);
            results.push(result);
            continue;
        }

        // Analysis (only reachable when SIR verified)
        let mut analysis = AnalysisManager::new();
        analysis.run_all(&func);
        result.facts = analysis.database().total_facts();

        // Semantics
        let mut semantics = SemanticEngine::new();
        semantics.derive(&func, analysis.database());
        result.truths = semantics.database().truths().count();
        for (_, region) in semantics.database().regions() {
            for c in region.concepts() {
                result.concepts.push(format!("{:?}", c));
            }
        }

        // Inference
        let mut inference = InferenceEngine::new();
        inference.infer(semantics.database(), semantics.structural_database());
        for (_, ctxs) in inference.context_database().contexts() {
            result.beliefs += ctxs.len();
        }

        // Authorization + Generation (X06 structural fix: candidates
        // require complete certificates; uncertified regions yield none)
        let authorizations = sir_semantics::authorization::derive_authorizations(
            &func,
            analysis.database(),
            semantics.database(),
        );
        let mut generator = CandidateGenerator::new();
        generator.generate(inference.context_database(), semantics.database(), &authorizations);
        let candidates: Vec<Candidate> = generator.database().all_candidates().cloned().collect();
        result.candidates = candidates.len();

        // Optimizer
        let config = OptimizerConfig::default();
        let registry = default_registry();
        let optimizer = Optimizer::new(config, registry);
        let opt_result = optimizer.optimize(&func);
        result.rewrites = opt_result.rewrites_applied;
        result.initial_nodes = opt_result.initial_nodes;
        result.final_nodes = opt_result.final_nodes;

        print_result(&result);
        results.push(result);
    }

    // Summary
    println!("\n{}", "=".repeat(120));
    let lowered = results.iter().filter(|r| r.lowered).count();
    let verified = results.iter().filter(|r| r.verified).count();
    let recognized = results.iter().filter(|r| r.truths > 0).count();
    let rewrote = results.iter().filter(|r| r.rewrites > 0).count();
    let total = results.len();
    println!("Summary: {}/{} lowered, {}/{} verified, {}/{} recognized, {}/{} rewrote",
        lowered, total, verified, total, recognized, total, rewrote, total);

    // List all concepts discovered
    let mut all_concepts: Vec<String> = results.iter()
        .flat_map(|r| r.concepts.clone())
        .collect();
    all_concepts.sort();
    all_concepts.dedup();
    println!("\nConcepts discovered across all kernels:");
    for c in &all_concepts {
        let count = results.iter().filter(|r| r.concepts.contains(c)).count();
        println!("  {:<30} (in {} kernels)", c, count);
    }

    // List kernels that failed to lower
    let failed: Vec<_> = results.iter().filter(|r| !r.lowered).collect();
    if !failed.is_empty() {
        println!("\nKernels that failed to lower (gap list):");
        for r in failed {
            println!("  {:<30} {}", r.name, r.error.as_deref().unwrap_or("?"));
        }
    }

    // List kernels where rewrites applied
    let rewrote_list: Vec<_> = results.iter().filter(|r| r.rewrites > 0).collect();
    if !rewrote_list.is_empty() {
        println!("\nKernels with rewrites applied:");
        for r in rewrote_list {
            println!("  {:<30} {} rewrite(s), {} -> {} nodes",
                r.name, r.rewrites, r.initial_nodes, r.final_nodes);
        }
    }
}

fn print_result(r: &KernelResult) {
    let error = r.error.as_deref().unwrap_or("");
    let error_short = if error.is_empty() { "" } else { &error[..error.len().min(30)] };
    println!("{:<30} {:<8} {:<8} {:<6} {:<6} {:<6} {:<6} {:<6} {:<8} {:<8} {:<8}",
        r.name,
        if r.lowered { "YES" } else { "NO" },
        if r.verified { "PASS" } else if r.lowered { "FAIL" } else { "-" },
        r.facts,
        r.truths,
        r.beliefs,
        r.candidates,
        r.rewrites,
        r.initial_nodes,
        r.final_nodes,
        error_short,
    );
}
