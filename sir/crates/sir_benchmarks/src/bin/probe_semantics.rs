use sir_analysis::manager::AnalysisManager;
use sir_lower::lower_function;
use sir_semantics::semantics::SemanticEngine;

fn main() {
    let ll_text = std::fs::read_to_string("../gate6a/heldout_corpus.ll").unwrap();
    for name in &["h03_count_nonzero", "h05_sum_int32", "h07_count_zero", "h08_count_above",
                    "n02_volatile_read", "n04_stride2", "n06_dependent"] {
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
