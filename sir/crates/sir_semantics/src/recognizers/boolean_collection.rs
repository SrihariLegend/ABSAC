use sir_analysis::facts::FactDatabase;
use sir_nodes::{Function, NodeKind};
use sir_types::{NodeId, Type};

use crate::concepts::SemanticConcept;
use crate::region::RecognitionExplanation;

/// Recognize boolean collection patterns in the function.
///
/// A boolean collection is an array whose element type is `Bool`.
/// We look for:
/// - `Allocate` nodes that allocate `Array { element: Bool, .. }` types
/// - Parameter nodes with `Array { element: Bool, .. }` types
/// - Any `ArrayAccess` into such arrays
///
/// Returns (concept, explanation, related_node_ids) tuples.
pub fn recognize_boolean_collection(
    func: &Function,
    _analysis: &FactDatabase,
) -> Vec<(SemanticConcept, RecognitionExplanation, Vec<NodeId>)> {
    let mut results = Vec::new();

    for node in func.arena.iter() {
        // Check if this node's type is Array<bool>
        if let Type::Array { element, length: _ } = &node.ty {
            if matches!(element.as_ref(), &Type::Bool) {
                let related = collect_array_related_nodes(func, node.id);
                results.push((
                    SemanticConcept::LogicalSequence,
                    RecognitionExplanation {
                        concept: SemanticConcept::LogicalSequence,
                        triggering_facts: vec!["Array element type is Bool"],
                    },
                    related,
                ));
            }
        }

        // Also recognize dynamically generated boolean collections
        if let NodeKind::ArrayCmpMask {
            array,
            scalar,
            op: _,
        } = &node.kind
        {
            results.push((
                SemanticConcept::LogicalSequence,
                RecognitionExplanation {
                    concept: SemanticConcept::LogicalSequence,
                    triggering_facts: vec![
                        "Dynamic comparison over array constructs boolean collection",
                    ],
                },
                vec![node.id, *array, *scalar],
            ));
        }
    }

    results
}

/// Collect nodes related to an array: its allocation site, all accesses, all loads/stores.
fn collect_array_related_nodes(func: &Function, array_node: NodeId) -> Vec<NodeId> {
    let mut related = vec![array_node];

    for node in func.arena.iter() {
        match &node.kind {
            NodeKind::ArrayAccess { base, .. } if *base == array_node => {
                related.push(node.id);
            }
            NodeKind::Load { ptr } => {
                // If loading through an ArrayAccess that targets this array
                if let Some(access_node) = func.get_node(*ptr) {
                    if let NodeKind::ArrayAccess { base, .. } = &access_node.kind {
                        if *base == array_node {
                            related.push(node.id);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    related
}
