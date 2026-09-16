//! emit_vectorized — lower LLVM IR, recognize semantic reduction, emit vectorized C.
//! Usage: emit_vectorized <file.ll> <function_name>

use sir_analysis::manager::AnalysisManager;
use sir_lower::lower_function;
use sir_semantics::semantics::SemanticEngine;
use sir_semantics::truth::SemanticTruth;
use sir_benchmarks::vector_emit::emit_vectorized;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: emit_vectorized <file.ll> <function_name>");
        std::process::exit(1);
    }
    let ll_path = &args[1];
    let func_name = &args[2];

    let ll_text = std::fs::read_to_string(ll_path)
        .unwrap_or_else(|e| { eprintln!("error reading {}: {}", ll_path, e); std::process::exit(1); });

    let func = lower_function(&ll_text, func_name)
        .unwrap_or_else(|e| { eprintln!("lower error: {:?}", e); std::process::exit(1); });

    let mut mgr = AnalysisManager::new();
    mgr.run_all(&func);
    let mut engine = SemanticEngine::new();
    engine.derive(&func, mgr.database());
    let db = engine.database();
    let truths: Vec<SemanticTruth> = db.truths().cloned().collect();

    let plan = derive_vector_plan(&func, &truths);

    match plan {
        Some(p) => {
            eprintln!("Recognized: {:?} with predicate {:?}", p.operation, p.predicate);
            println!("#include <stdint.h>");
            println!("#include <stdbool.h>");
            println!("#include <immintrin.h>");
            println!();
            println!("{}", emit_vectorized(&func, &p));
        }
        None => {
            eprintln!("No vectorizable semantic reduction found. Falling back to scalar.");
            println!("{}", sir_benchmarks::emit::emit_c(&func));
        }
    }
}

use sir_benchmarks::vector_derive::derive_vector_plan;
