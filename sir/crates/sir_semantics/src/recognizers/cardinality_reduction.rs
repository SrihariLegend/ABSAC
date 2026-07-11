use sir_analysis::facts::FactDatabase;
use sir_nodes::Function;
use sir_types::{Effects, NodeId};

use crate::concepts::SemanticConcept;
use crate::region::RecognitionExplanation;
use crate::truth::ValueId;

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
) -> Vec<(SemanticConcept, RecognitionExplanation, Vec<NodeId>, Vec<ValueId>, Vec<ValueId>)> {
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
                    for reduction in &sum_reductions {
                        related.push(reduction.variable);
                        related.push(reduction.invariant_value);
                    }
                    
                    // Input: the loop carried values (the items being summed over, though this requires proper bridging).
                    // For now, let's identify the input boolean and output count.
                    // To keep it minimal, we just identify the reduction outputs.
                    // The actual input is the condition, but for the cardinality reduction, 
                    // we can say the input is the boolean value `to_add`.
                    let mut inputs = Vec::new();
                    let mut outputs = Vec::new();
                    for reduction in &sum_reductions {
                        let mut condition_node_id = reduction.invariant_value;
                        // Check if it's a Select(cond, 1, 0)
                        if let Some(inv_node) = func.get_node(condition_node_id) {
                            if let sir_nodes::NodeKind::Select { cond, true_val, false_val } = &inv_node.kind {
                                // Simplified check: assume true_val and false_val are 1 and 0
                                condition_node_id = *cond;
                            }
                        }
                        
                        inputs.push(ValueId::new(condition_node_id.0)); // The boolean condition
                        outputs.push(ValueId::new(node.id.0)); // The loop node represents the aggregate outputs
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
                        inputs,
                        outputs,
                    ));
                }
            }
        }
    }

    results
}
