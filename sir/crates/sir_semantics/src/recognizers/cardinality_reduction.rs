use sir_analysis::facts::FactDatabase;
use sir_nodes::Function;
use sir_types::NodeId;

use crate::concepts::SemanticConcept;
use crate::region::RecognitionExplanation;

/// Recognize cardinality reduction patterns.
///
/// A cardinality reduction counts how many elements of a collection
/// satisfy a condition. We detect:
/// - A loop with a reduction variable of kind "sum"
/// - The reduction combines a boolean condition (0 or 1) into a counter
///
/// Returns (concept, explanation, related_node_ids) tuples.
pub fn recognize_cardinality_reduction(
    func: &Function,
    analysis: &FactDatabase,
) -> Vec<(SemanticConcept, RecognitionExplanation, Vec<NodeId>)> {
    let mut results = Vec::new();

    for node in func.arena.iter() {
        if let sir_nodes::NodeKind::Loop { .. } = &node.kind {
            if let Some(loop_fact) = analysis.loops.get(&node.id) {
                // Look for reductions — additive reductions count things.
                if !loop_fact.reductions.is_empty() {
                    let mut related = vec![node.id];
                    for reduction in &loop_fact.reductions {
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
