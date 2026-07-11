use sir_analysis::facts::FactDatabase;
use sir_nodes::Function;
use sir_types::{Effects, NodeId};

use crate::concepts::SemanticConcept;
use crate::region::RecognitionExplanation;

/// Recognize cardinality reduction patterns.
///
/// A cardinality reduction counts how many elements of a collection
/// satisfy a condition. We detect:
/// - A loop with a reduction variable of kind "sum"
/// - The reduction combines a boolean condition (0 or 1) into a counter
/// - The loop has no observable side effects (IO, WRITE_MEMORY, etc.)
///
/// Returns (concept, explanation, related_node_ids) tuples.
pub fn recognize_cardinality_reduction(
    func: &Function,
    analysis: &FactDatabase,
) -> Vec<(SemanticConcept, RecognitionExplanation, Vec<NodeId>)> {
    let mut results = Vec::new();

    for node in func.arena.iter() {
        if let sir_nodes::NodeKind::Loop { .. } = &node.kind {
            // Reject loops with side effects (IO, memory writes, allocations)
            let allowed_effects = Effects::READ_MEMORY;
            if !(node.effects - allowed_effects).is_empty() {
                continue;
            }

            if let Some(loop_fact) = analysis.loops.get(&node.id) {
                // Sum reductions include both the loop counter (i = i + 1, always present)
                // and the actual counting reduction (count = count + inc).
                // Require at least 2 sum reductions — one is the loop counter, the
                // other is the cardinality reduction.
                let sum_reductions: Vec<_> = loop_fact
                    .reductions
                    .iter()
                    .filter(|r| r.reduction_kind == "sum")
                    .collect();
                if sum_reductions.len() >= 2 {
                    let mut related = vec![node.id];
                    for reduction in sum_reductions {
                        related.push(reduction.variable);
                        related.push(reduction.invariant_value);
                    }
                    results.push((
                        SemanticConcept::CardinalityReduction,
                        RecognitionExplanation {
                            concept: SemanticConcept::CardinalityReduction,
                            triggering_facts: vec![
                                "Loop has additive reduction",
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
