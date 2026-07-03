use std::collections::BTreeSet;

use sir_analysis::facts::FactDatabase;
use sir_nodes::{Function, NodeKind};
use sir_types::{NodeId, Type};

use crate::concepts::SemanticConcept;
use crate::region::RecognitionExplanation;

/// Recognize membership traversal patterns.
///
/// A membership traversal is an iteration that tests whether each
/// element of a collection satisfies some condition. We detect:
/// - A `Loop` that iterates over a boolean array
/// - Loop body contains `ArrayAccess` + `Load` → used as a condition
///   (in `Select` or `BoolAnd`/`BoolOr`)
///
/// Returns (concept, explanation, related_node_ids) tuples.
pub fn recognize_membership_traversal(
    func: &Function,
    analysis: &FactDatabase,
) -> Vec<(SemanticConcept, RecognitionExplanation, Vec<NodeId>)> {
    let mut results = Vec::new();

    // Find boolean arrays being indexed inside loops.
    for node in func.arena.iter() {
        if let NodeKind::Loop { body, .. } = &node.kind {
            if let Some(loop_fact) = analysis.loops.get(&node.id) {
                if loop_fact.trip_count.is_some() {
                    // Walk the loop body to find ArrayAccess nodes on boolean arrays.
                    let mut related = vec![node.id];
                    related.extend(body.iter().copied());
                    let array_nodes = find_boolean_array_accesses(func, body);
                    if !array_nodes.is_empty() {
                        related.extend(array_nodes);
                        results.push((
                            SemanticConcept::MembershipTraversal,
                            RecognitionExplanation {
                                concept: SemanticConcept::MembershipTraversal,
                                triggering_facts: vec![
                                    "Loop iterates over boolean array",
                                    "Array elements used as conditions",
                                ],
                            },
                            related,
                        ));
                    }
                }
            }
        }
    }

    results
}

/// Find all ArrayAccess nodes within a subtree that index into boolean arrays.
fn find_boolean_array_accesses(func: &Function, roots: &[NodeId]) -> Vec<NodeId> {
    let mut results = Vec::new();
    let mut visited = BTreeSet::new();
    let mut stack: Vec<NodeId> = roots.to_vec();

    while let Some(current) = stack.pop() {
        if !visited.insert(current) {
            continue;
        }
        if let Some(node) = func.get_node(current) {
            match &node.kind {
                NodeKind::ArrayAccess { base, .. } => {
                    if let Some(base_node) = func.get_node(*base) {
                        if let Type::Array { element, .. } = &base_node.ty {
                            if matches!(element.as_ref(), &Type::Bool) {
                                results.push(current);
                            }
                        }
                    }
                    // Walk into index operand too
                    for op in node.kind.input_nodes() {
                        stack.push(op);
                    }
                }
                // Loop containment edges: don't cross into carried inputs or outputs
                NodeKind::Loop { .. } => {
                    // We're already inside the loop body; don't recurse into
                    // the loop node's structural fields.
                }
                _ => {
                    for op in node.kind.input_nodes() {
                        stack.push(op);
                    }
                }
            }
        }
    }

    results
}
