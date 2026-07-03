use sir_analysis::facts::FactDatabase;
use sir_nodes::Function;
use sir_types::NodeId;

use crate::concepts::SemanticConcept;
use crate::region::RecognitionExplanation;

/// Recognize conjunctive reduction patterns.
///
/// A conjunctive reduction checks if all elements satisfy a condition
/// (e.g. `&&` over a boolean collection). We detect:
/// - A loop with a reduction variable of kind "bitwise_and"
///
/// Returns (concept, explanation, related_node_ids) tuples.
pub fn recognize_conjunctive_reduction(
    func: &Function,
    analysis: &FactDatabase,
) -> Vec<(SemanticConcept, RecognitionExplanation, Vec<NodeId>)> {
    let mut results = Vec::new();

    for node in func.arena.iter() {
        if let sir_nodes::NodeKind::Loop { .. } = &node.kind {
            if let Some(loop_fact) = analysis.loops.get(&node.id) {
                let and_reductions: Vec<_> = loop_fact
                    .reductions
                    .iter()
                    .filter(|r| r.reduction_kind == "bitwise_and")
                    .collect();

                if !and_reductions.is_empty() {
                    let mut related = vec![node.id];
                    for reduction in and_reductions {
                        related.push(reduction.variable);
                        related.push(reduction.invariant_value);
                    }
                    results.push((
                        SemanticConcept::ConjunctiveReduction,
                        RecognitionExplanation {
                            concept: SemanticConcept::ConjunctiveReduction,
                            triggering_facts: vec![
                                "Loop has bitwise AND reduction",
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
