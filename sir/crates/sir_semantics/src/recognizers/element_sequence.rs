use sir_analysis::facts::FactDatabase;
use sir_nodes::{Function, NodeKind};
use sir_types::{NodeId, Type};

use crate::concepts::SemanticConcept;
use crate::region::RecognitionExplanation;
use crate::truth::ValueId;

/// Recognize a sequence of elements being iterated over.
///
/// We look for a Collection (e.g. an Array) that is accessed inside a loop.
pub fn recognize_element_sequence(
    func: &Function,
    _analysis: &FactDatabase,
) -> Vec<(SemanticConcept, RecognitionExplanation, Vec<NodeId>, Vec<ValueId>, Vec<ValueId>)> {
    let mut results = Vec::new();

    // Find arrays with known lengths.
    let arrays_with_length: Vec<_> = func
        .arena
        .iter()
        .filter_map(|node| {
            if let Type::Array { .. } = &node.ty {
                Some(node.id)
            } else {
                None
            }
        })
        .collect();

    for &array_id in &arrays_with_length {
        // Find loop nodes that access this array.
        for node in func.arena.iter() {
            if let NodeKind::Loop { body, .. } = &node.kind {
                // Check if the loop accesses the array
                for &body_node_id in body {
                    if let Some(body_node) = func.get_node(body_node_id) {
                        if let NodeKind::ArrayAccess { base, .. } = &body_node.kind {
                            if *base == array_id {
                                let explanation = RecognitionExplanation {
                                    concept: SemanticConcept::ElementSequence,
                                    triggering_facts: vec![
                                        "Collection is accessed element-wise in a sequence",
                                    ],
                                };

                                // The inputs are the Collection, the output is the yielded Element (ArrayAccess).
                                results.push((
                                    SemanticConcept::ElementSequence,
                                    explanation,
                                    vec![node.id, body_node_id],
                                    vec![ValueId::new(array_id.0)],
                                    vec![ValueId::new(body_node_id.0)],
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    results
}
