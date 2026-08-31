use sir_analysis::facts::FactDatabase;
use sir_analysis::graph;
use sir_nodes::Function;
use sir_types::{ConstantData, Effects, NodeId};

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
/// - The loop has no volatile memory accesses
/// - The loop counter increments by 1 (contiguous stride)
/// - Multiple accumulators are independent (no data dependency)
/// - The accumulated value is a boolean predicate, not a raw value (Sum)
///
/// Returns (concept, explanation, related_node_ids) tuples.
pub fn recognize_cardinality_reduction(
    func: &Function,
    analysis: &FactDatabase,
) -> Vec<(SemanticConcept, RecognitionExplanation, Vec<NodeId>, Vec<ValueId>, Vec<ValueId>)> {
    let mut results = Vec::new();

    for node in func.arena.iter() {
        if let sir_nodes::NodeKind::Loop { .. } = &node.kind {
            // ── Safety check 1: Reject loops with side effects ──
            // Allowed: READ_MEMORY (pure reads from a buffer)
            // Rejected: WRITE_MEMORY, ALLOCATE, IO, ATOMIC, VOLATILE
            let allowed_effects = Effects::READ_MEMORY;
            if !(node.effects - allowed_effects).is_empty() {
                continue;
            }

            // ── Safety check 2: Reject loops with volatile accesses ──
            // Check all nodes in the loop body for VOLATILE effects
            let body_nodes = collect_loop_body_nodes(&node.kind);
            let has_volatile = body_nodes.iter().any(|&nid| {
                func.get_node(nid)
                    .map(|n| n.effects.contains(Effects::VOLATILE))
                    .unwrap_or(false)
            });
            if has_volatile {
                continue;
            }

            if let Some(loop_fact) = analysis.loops.get(&node.id) {
                let sum_reductions: Vec<_> = loop_fact
                    .reductions
                    .iter()
                    .filter(|r| r.reduction_kind == "sum")
                    .collect();

                if sum_reductions.len() < 2 {
                    continue;
                }

                // ── Safety check 3: Verify contiguous stride (increment by 1) ──
                // The loop counter is the sum reduction with invariant_value = constant 1.
                // If no such reduction exists, the stride is non-unit → reject.
                let counter = sum_reductions.iter().find(|r| {
                    is_constant_one(func, r.invariant_value)
                });
                if counter.is_none() {
                    continue;
                }

                // ── Safety check 4: Distinguish Cardinality from Sum ──
                // For each non-counter sum reduction, check if the invariant_value
                // is a boolean predicate (Select(cond, 1, 0) or comparison node).
                // If it's a raw value (e.g., zext of a load), it's a Sum, not Cardinality.
                let non_counter_reductions: Vec<_> = sum_reductions
                    .iter()
                    .filter(|r| !is_constant_one(func, r.invariant_value))
                    .collect();

                let cardinality_reductions: Vec<_> = non_counter_reductions
                    .iter()
                    .filter(|r| is_boolean_predicate(func, r.invariant_value))
                    .collect();

                // Only fire if there's at least one genuine cardinality reduction
                if cardinality_reductions.is_empty() {
                    continue;
                }

                // ── Safety check 5: Check accumulator independence ──
                // If there are multiple non-counter accumulators, verify that
                // the invariant_value of one doesn't depend on the variable
                // of another. If they're dependent, the reduction is not
                // independently vectorizable.
                if non_counter_reductions.len() > 1 {
                    let all_independent = non_counter_reductions.iter().all(|r| {
                        let transitive = graph::transitive_inputs(r.invariant_value, &func.arena);
                        // Check that no other reduction's variable is in our transitive inputs
                        non_counter_reductions
                            .iter()
                            .all(|other| other.variable == r.variable || !transitive.contains(&other.variable))
                    });
                    if !all_independent {
                        continue;
                    }
                }

                // ── Passed all safety checks — emit recognition ──
                let mut related = vec![node.id];
                for reduction in &cardinality_reductions {
                    related.push(reduction.variable);
                    related.push(reduction.invariant_value);
                }

                let mut inputs = Vec::new();
                let mut outputs = Vec::new();
                for reduction in &cardinality_reductions {
                    let mut condition_node_id = reduction.invariant_value;
                    // Unwrap Select(cond, 1, 0) to get the condition
                    if let Some(inv_node) = func.get_node(condition_node_id) {
                        if let sir_nodes::NodeKind::Select { cond, .. } = &inv_node.kind {
                            condition_node_id = *cond;
                        }
                    }

                    inputs.push(ValueId::new(condition_node_id.0));
                    outputs.push(ValueId::new(node.id.0));
                }

                results.push((
                    SemanticConcept::CardinalityReduction,
                    RecognitionExplanation {
                        concept: SemanticConcept::CardinalityReduction,
                        triggering_facts: vec![
                            "Loop has additive reduction",
                            "Reduction variable accumulates boolean conditions",
                            "Loop has no volatile accesses",
                            "Loop has contiguous stride (increment by 1)",
                            "Accumulators are independent",
                        ],
                    },
                    related,
                    inputs,
                    outputs,
                ));
            }
        }
    }

    results
}

/// Collect all NodeIds referenced within a Loop's body, outputs, and carried_inputs.
fn collect_loop_body_nodes(kind: &sir_nodes::NodeKind) -> Vec<NodeId> {
    if let sir_nodes::NodeKind::Loop {
        body,
        termination,
        outputs,
        carried_inputs,
    } = kind
    {
        let mut nodes = vec![*termination];
        nodes.extend(body.iter().copied());
        nodes.extend(outputs.iter().copied());
        nodes.extend(carried_inputs.iter().copied());
        nodes
    } else {
        Vec::new()
    }
}

/// Check if a node is a constant integer 1.
fn is_constant_one(func: &Function, id: NodeId) -> bool {
    if let Some(node) = func.get_node(id) {
        if let sir_nodes::NodeKind::Constant(data) = &node.kind {
            if let ConstantData::Integer { value, .. } = data {
                return value == "1";
            }
        }
    }
    false
}

/// Check if a node represents a boolean predicate (0 or 1), not a raw value.
///
/// A boolean predicate is:
/// - A Select(cond, 1, 0) — explicit boolean selection
/// - A comparison node (Eq, Ne, Lt, Le, Gt, Ge) — produces Bool
/// - A Convert from Bool to integer (zext of a comparison result)
fn is_boolean_predicate(func: &Function, id: NodeId) -> bool {
    let node = match func.get_node(id) {
        Some(n) => n,
        None => return false,
    };

    match &node.kind {
        sir_nodes::NodeKind::Select { true_val, false_val, .. } => {
            // Check if true_val and false_val are constants 1 and 0 (or 0 and 1)
            is_constant_one(func, *true_val) && is_constant_zero(func, *false_val)
                || is_constant_zero(func, *true_val) && is_constant_one(func, *false_val)
        }
        sir_nodes::NodeKind::Eq { .. }
        | sir_nodes::NodeKind::Ne { .. }
        | sir_nodes::NodeKind::Lt { .. }
        | sir_nodes::NodeKind::Le { .. }
        | sir_nodes::NodeKind::Gt { .. }
        | sir_nodes::NodeKind::Ge { .. } => true,
        sir_nodes::NodeKind::Convert { .. } => {
            // Check if the source is a comparison (Bool → integer conversion)
            // For Convert, the operand is the first input
            let inputs = sir_analysis::graph::dataflow_inputs(&node.kind);
            if let Some(&src) = inputs.first() {
                if let Some(src_node) = func.get_node(src) {
                    matches!(
                        src_node.kind,
                        sir_nodes::NodeKind::Eq { .. }
                            | sir_nodes::NodeKind::Ne { .. }
                            | sir_nodes::NodeKind::Lt { .. }
                            | sir_nodes::NodeKind::Le { .. }
                            | sir_nodes::NodeKind::Gt { .. }
                            | sir_nodes::NodeKind::Ge { .. }
                    )
                } else {
                    false
                }
            } else {
                false
            }
        }
        _ => false,
    }
}

/// Check if a node is a constant integer 0.
fn is_constant_zero(func: &Function, id: NodeId) -> bool {
    if let Some(node) = func.get_node(id) {
        if let sir_nodes::NodeKind::Constant(data) = &node.kind {
            if let ConstantData::Integer { value, .. } = data {
                return value == "0";
            }
        }
    }
    false
}
