use sir_analysis::facts::FactDatabase;
use sir_nodes::{Function, NodeKind};
use sir_types::{NodeId, Type};

use crate::concepts::SemanticConcept;
use crate::region::RecognitionExplanation;

/// Recognize finite collection patterns.
///
/// A collection is "finite" when it has a statically known size.
/// We look for:
/// - `Array` types with known length (not Slice, not dynamic)
/// - Loop nodes that have a known trip count equal to the array length
///
/// Returns (concept, explanation, related_node_ids) tuples.
pub fn recognize_finite_collection(
    func: &Function,
    analysis: &FactDatabase,
) -> Vec<(SemanticConcept, RecognitionExplanation, Vec<NodeId>)> {
    let mut results = Vec::new();

    // Find arrays with known lengths.
    let arrays_with_length: Vec<_> = func
        .arena
        .iter()
        .filter_map(|node| {
            if let Type::Array { element: _, length } = &node.ty {
                Some((node.id, *length))
            } else {
                None
            }
        })
        .collect();

    for (array_id, array_len) in &arrays_with_length {
        // Check if any loop iterates exactly array_len times
        // and accesses this array.
        for node in func.arena.iter() {
            if let NodeKind::Loop { .. } = &node.kind {
                if let Some(loop_fact) = analysis.loops.get(&node.id) {
                    if let Some(trip_count) = loop_fact.trip_count {
                        if trip_count == *array_len as u64 {
                            let mut related = vec![*array_id, node.id];
                            // Also include loop body nodes
                            if let NodeKind::Loop { body, .. } = &node.kind {
                                related.extend(body.iter().copied());
                            }
                            results.push((
                                SemanticConcept::FiniteCollection,
                                RecognitionExplanation {
                                    concept: SemanticConcept::FiniteCollection,
                                    triggering_facts: vec![
                                        "Array has static length",
                                        "Loop trip count equals array length",
                                    ],
                                },
                                related,
                            ));
                        }
                    }
                }
            }
        }
    }

    results
}
