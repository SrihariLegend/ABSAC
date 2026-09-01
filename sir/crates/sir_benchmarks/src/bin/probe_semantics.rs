use sir_analysis::manager::AnalysisManager;
use sir_lower::lower_function;
use sir_semantics::semantics::SemanticEngine;

fn main() {
    let ll_text = std::fs::read_to_string("../gate6a/v1_corpus.ll").unwrap();
    for name in &["v03_all_same_i64", "v04_count_above_u32", "v05_sum_signed", "v06_count_not_equal",
                    "v07_count_then_sum", "v08_sum_bounded", "n09_atomic_load", "n11_reverse_sum",
                    "n13_dynamic_stride", "n15_count_equal_pairs"] {
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
