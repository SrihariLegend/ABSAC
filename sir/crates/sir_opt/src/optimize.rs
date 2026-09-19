//! Fixed-point driver over the family registry.

use sir_analysis::manager::AnalysisManager;
use sir_nodes::Function;

use crate::family::registry;

/// Run every allowed family to a fixed point (bounded defensively).
/// Returns the optimized function, the number of rewrites applied, and
/// the definitions applied in order.
pub fn optimize(
    function: &Function,
    allow: &dyn Fn(u64) -> bool,
) -> Option<(Function, usize, Vec<u64>)> {
    let families = registry();
    let mut current = function.clone();
    let mut rewrites = 0usize;
    let mut definitions = Vec::new();

    for _ in 0..32 {
        let mut analysis = AnalysisManager::new();
        analysis.run_all(&current);
        let facts = analysis.database();
        let mut changed = false;

        for family in &families {
            if !family
                .definitions()
                .iter()
                .any(|definition| allow(*definition))
            {
                continue;
            }
            if let Some(applied) = family.apply_one(&current, facts) {
                if allow(applied.definition) {
                    current = applied.function;
                    rewrites += 1;
                    definitions.push(applied.definition);
                    changed = true;
                    break;
                }
            }
        }

        if !changed {
            break;
        }
    }

    if rewrites == 0 {
        None
    } else {
        Some((current, rewrites, definitions))
    }
}
