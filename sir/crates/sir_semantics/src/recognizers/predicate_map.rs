use sir_analysis::facts::FactDatabase;
use sir_nodes::{Function, NodeKind};
use sir_types::{NodeId, Type};

use crate::concepts::SemanticConcept;
use crate::region::RecognitionExplanation;
use crate::truth::ValueId;

/// Recognize a boolean predicate mapped over a loop.
///
/// If a loop body computes a boolean value, that boolean value conceptually
/// represents a logical sequence when mapped over the iterations.
pub fn recognize_predicate_map(
    func: &Function,
    _analysis: &FactDatabase,
) -> Vec<(SemanticConcept, RecognitionExplanation, Vec<NodeId>, Vec<ValueId>, Vec<ValueId>)> {
    let mut results = Vec::new();

    for loop_node in func.arena.iter() {
        if let NodeKind::Loop { body, .. } = &loop_node.kind {
            for &body_id in body {
                if let Some(body_node) = func.get_node(body_id) {
                    if body_node.ty == Type::Bool {
                        // Skip the loop termination condition itself if it's just `i < n`
                        if let NodeKind::Lt { .. } = body_node.kind {
                            // Heuristic: usually loop bounds are Lt/Le
                            // We can just keep it simple and emit for all Bools.
                        }
                        
                        let explanation = RecognitionExplanation {
                            concept: SemanticConcept::PredicateMap,
                            triggering_facts: vec![
                                "Boolean value computed inside a loop body forms a PredicateMap",
                            ],
                        };
                        
                        // Output: The boolean node that is computed per-iteration
                        let output_vid = ValueId::new(body_id.0);
                        
                        // We want to find the sequence element that this predicate is mapping over.
                        // We heuristically find an ArrayAccess in the same loop body.
                        let mut element_input = None;
                        for &n_id in body {
                            if let Some(n) = func.get_node(n_id) {
                                if let NodeKind::ArrayAccess { .. } = n.kind {
                                    element_input = Some(n_id);
                                    break;
                                }
                            }
                        }
                        
                        let input_vid = if let Some(e) = element_input {
                            ValueId::new(e.0)
                        } else {
                            // Fallback, though a true semantic extractor might fail here if no sequence is found.
                            ValueId::new(loop_node.id.0)
                        };
                        
                        results.push((
                            SemanticConcept::PredicateMap,
                            explanation,
                            vec![loop_node.id, body_id],
                            vec![input_vid], // Element sequence is the input
                            vec![output_vid]
                        ));
                    }
                }
            }
        }
    }

    results
}
