use sir_analysis::manager::AnalysisManager;
use sir_lower::lower_function;
use sir_semantics::semantics::SemanticEngine;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let ll_path = args.get(1).cloned().unwrap_or_else(|| "../gate6a/heldout_corpus.ll".to_string());
    let ll_text = std::fs::read_to_string(&ll_path).unwrap();
    let names: Vec<String> = if args.len() > 2 {
        args[2..].to_vec()
    } else {
        sir_lower::list_functions(&ll_text)
    };
    for name in &names {
        let func = match lower_function(&ll_text, name) {
            Ok(f) => f,
            Err(e) => {
                println!("=== {} ===", name);
                println!("  ABSTAINED: {}", e);
                continue;
            }
        };
        let mut mgr = AnalysisManager::new();
        mgr.run_all(&func);
        let mut engine = SemanticEngine::new();
        engine.derive(&func, mgr.database());
        let db = engine.database();
        println!("=== {} ===", name);
        for t in db.truths() {
            println!("  {:?}", t);
        }
    }
}
