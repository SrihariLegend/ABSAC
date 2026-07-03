use sir_analysis::facts::FactDatabase;
use sir_nodes::Function;
use sir_types::NodeId;

use crate::concepts::SemanticConcept;
use crate::region::RecognitionExplanation;

/// Recognize exclusive reduction patterns.
///
/// An exclusive reduction checks parity/XOR of boolean conditions
/// (e.g. `!=` or `^` over a boolean collection). We detect:
/// - A loop with a reduction variable of kind "bitwise_xor"
///
/// Returns (concept, explanation, related_node_ids) tuples.
pub fn recognize_exclusive_reduction(
    func: &Function,
    analysis: &FactDatabase,
) -> Vec<(SemanticConcept, RecognitionExplanation, Vec<NodeId>)> {
    let mut results = Vec::new();

    for node in func.arena.iter() {
        if let sir_nodes::NodeKind::Loop { .. } = &node.kind {
            if let Some(loop_fact) = analysis.loops.get(&node.id) {
                let xor_reductions: Vec<_> = loop_fact
                    .reductions
                    .iter()
                    .filter(|r| r.reduction_kind == "bitwise_xor")
                    .collect();

                if !xor_reductions.is_empty() {
                    let mut related = vec![node.id];
                    for reduction in xor_reductions {
                        related.push(reduction.variable);
                        related.push(reduction.invariant_value);
                    }
                    results.push((
                        SemanticConcept::ExclusiveReduction,
                        RecognitionExplanation {
                            concept: SemanticConcept::ExclusiveReduction,
                            triggering_facts: vec![
                                "Loop has bitwise XOR reduction",
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
