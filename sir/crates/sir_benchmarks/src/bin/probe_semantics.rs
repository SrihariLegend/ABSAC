use sir_analysis::manager::AnalysisManager;
use sir_lower::lower_function;
use sir_semantics::semantics::SemanticEngine;

fn main() {
    let ll_text = std::fs::read_to_string("../corpus/kernels.ll").unwrap();
    for name in &["k18_all_equal", "k43_sum_ascii", "k50_count_masked"] {
        let func = lower_function(&ll_text, name).unwrap();
        let mut mgr = AnalysisManager::new();
        mgr.run_all(&func);
        let mut engine = SemanticEngine::new();
        engine.derive(&func, mgr.database());
        let db = engine.database();
        println!("=== {} ===", name);
        for truth in db.truths() {
            println!("  {:?}", truth);
        }
    }
}
