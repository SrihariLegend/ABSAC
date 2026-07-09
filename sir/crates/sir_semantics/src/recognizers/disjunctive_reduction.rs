use sir_analysis::facts::FactDatabase;
use sir_nodes::Function;
use sir_types::NodeId;

use crate::concepts::SemanticConcept;
use crate::region::RecognitionExplanation;

/// Recognize disjunctive reduction patterns.
///
/// A disjunctive reduction checks if at least one element satisfies a condition
/// (e.g. `||` over a boolean collection). We detect:
/// - A loop with a reduction variable of kind "bitwise_or"
///
/// Returns (concept, explanation, related_node_ids) tuples.
pub fn recognize_disjunctive_reduction(
    func: &Function,
    analysis: &FactDatabase,
) -> Vec<(SemanticConcept, RecognitionExplanation, Vec<NodeId>)> {
    let mut results = Vec::new();

    for node in func.arena.iter() {
        if let sir_nodes::NodeKind::Loop { .. } = &node.kind {
            let allowed_effects = sir_types::Effects::READ_MEMORY;
            if !(node.effects - allowed_effects).is_empty() {
                continue;
            }
            if let Some(loop_fact) = analysis.loops.get(&node.id) {
                let or_reductions: Vec<_> = loop_fact
                    .reductions
                    .iter()
                    .filter(|r| r.reduction_kind == "bitwise_or")
                    .collect();

                if !or_reductions.is_empty() {
                    let mut related = vec![node.id];
                    for reduction in or_reductions {
                        related.push(reduction.variable);
                        related.push(reduction.invariant_value);
                    }
                    results.push((
                        SemanticConcept::DisjunctiveReduction,
                        RecognitionExplanation {
                            concept: SemanticConcept::DisjunctiveReduction,
                            triggering_facts: vec![
                                "Loop has bitwise OR reduction",
                                "Reduction variable accumulates boolean conditions",
                            ],
                        },
                        related,
                    ));
                }
            }
        }
    }

    results
}
