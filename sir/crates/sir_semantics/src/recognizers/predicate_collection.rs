use sir_analysis::facts::FactDatabase;
use sir_nodes::{Function, NodeKind};
use sir_types::{NodeId, Type};

use crate::concepts::SemanticConcept;
use crate::region::RecognitionExplanation;

/// Recognize predicate collections (dynamically constructed boolean arrays).
///
/// A predicate collection is formed by comparing elements of an array
/// against a scalar value (e.g. `array[i] > threshold`).
pub fn recognize_predicate_collection(
    func: &Function,
    _analysis: &FactDatabase,
) -> Vec<(SemanticConcept, RecognitionExplanation, Vec<NodeId>)> {
    let mut results = Vec::new();

    // Look for comparisons where one side is derived from an ArrayAccess
    for node in func.arena.iter() {
        if matches!(
            node.kind,
            NodeKind::Eq { .. }
                | NodeKind::Ne { .. }
                | NodeKind::Lt { .. }
                | NodeKind::Le { .. }
                | NodeKind::Gt { .. }
                | NodeKind::Ge { .. }
        ) {
            // Find the inputs
            let inputs = node.kind.input_nodes();
            if inputs.len() == 2 {
                let lhs = inputs[0];
                let rhs = inputs[1];

                let lhs_is_access = get_array_length(func, lhs).is_some();
                let rhs_is_access = get_array_length(func, rhs).is_some();

                if lhs_is_access || rhs_is_access {
                    results.push((
                        SemanticConcept::LogicalSequence,
                        RecognitionExplanation {
                            concept: SemanticConcept::LogicalSequence,
                            triggering_facts: vec![
                                "Comparison operator forms a dynamic boolean sequence from an array",
                            ],
                        },
                        vec![node.id, lhs, rhs],
                    ));
                }
            }
        }
    }

    results
}

pub fn recognize_dynamic_boolean_sequence(
    func: &Function,
    _analysis: &FactDatabase,
) -> Vec<(
    crate::region::RegionId,
    crate::structure::StructuralDescription,
)> {
    let mut results = Vec::new();

    for node in func.arena.iter() {
        if matches!(
            node.kind,
            NodeKind::Eq { .. }
                | NodeKind::Ne { .. }
                | NodeKind::Lt { .. }
                | NodeKind::Le { .. }
                | NodeKind::Gt { .. }
                | NodeKind::Ge { .. }
        ) {
            let inputs = node.kind.input_nodes();
            if inputs.len() == 2 {
                let lhs = inputs[0];
                let rhs = inputs[1];

                if let Some(len) =
                    get_array_length(func, lhs).or_else(|| get_array_length(func, rhs))
                {
                    let desc = crate::structure::StructuralDescription::new(
                        crate::region::RegionId::new(0),
                        sir_transform::structures::SourceStructure::DynamicBooleanSequence {
                            length: len,
                        },
                    )
                    .with_constraint(sir_transform::constraints::Constraint::FixedLength(len));

                    results.push((crate::region::RegionId::new(0), desc));
                }
            }
        }
    }

    results
}

fn get_array_length(func: &Function, id: NodeId) -> Option<usize> {
    if let Some(node) = func.get_node(id) {
        if let NodeKind::ArrayAccess { base, .. } = node.kind {
            if let Some(base_node) = func.get_node(base) {
                if let Type::Array { length, .. } = &base_node.ty {
                    return Some(*length);
                }
            }
        }
        if let NodeKind::Load { ptr } = node.kind {
            if let Some(ptr_node) = func.get_node(ptr) {
                if let NodeKind::ArrayAccess { base, .. } = ptr_node.kind {
                    if let Some(base_node) = func.get_node(base) {
                        if let Type::Array { length, .. } = &base_node.ty {
                            return Some(*length);
                        }
                    }
                }
            }
        }
    }
    None
}
