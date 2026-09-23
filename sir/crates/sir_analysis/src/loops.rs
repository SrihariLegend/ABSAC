//! Loop analysis.
//!
//! Analyzes Loop nodes: trip count, finiteness, nesting,
//! carried variables, and reduction pattern detection.

use sir_nodes::{Function, NodeKind};
use sir_types::{ConstantData, NodeId};
use std::collections::HashMap;

use crate::facts::{LoopFact, ReductionVar};

/// Run loop analysis on a function.
pub fn run_loops(func: &Function) -> HashMap<NodeId, LoopFact> {
    let mut facts: HashMap<NodeId, LoopFact> = HashMap::new();

    // Collect all Loop nodes.
    let loop_nodes: Vec<NodeId> = func
        .arena
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::Loop { .. }))
        .map(|n| n.id)
        .collect();

    for &loop_id in &loop_nodes {
        let node = func.get_node(loop_id).unwrap();
        if let NodeKind::Loop {
            body,
            termination,
            outputs,
            carried_inputs,
        } = &node.kind
        {
            let trip_count = estimate_trip_count(func, *termination, carried_inputs, outputs);
            let is_nested = is_nested_loop(func, body);
            let carried = carried_inputs.clone();
            let reductions = detect_reductions(func, carried_inputs, outputs);

            facts.insert(
                loop_id,
                LoopFact {
                    is_finite: trip_count.is_some(),
                    trip_count,
                    is_nested,
                    carried,
                    reductions,
                },
            );
        }
    }

    facts
}

/// Estimate trip count for a counted loop.
///
/// Looks for the pattern: a carried input that is incremented/decremented
/// by a constant amount in each iteration, compared against a bound.
fn estimate_trip_count(
    func: &Function,
    termination: NodeId,
    carried_inputs: &[NodeId],
    outputs: &[NodeId],
) -> Option<u64> {
    let term_node = func.get_node(termination)?;

    // Guarded terminations (`i < limit && flag`): the trip-count
    // comparison may be a conjunct of a boolean AND. The bound alone only
    // bounds the loop if the other conjunct cannot make it exit early, so
    // that conjunct must be provably constant-true; otherwise the count is
    // refused rather than guessed.
    if let NodeKind::BoolAnd { lhs, rhs } = &term_node.kind {
        return match (
            estimate_trip_count(func, *lhs, carried_inputs, outputs),
            estimate_trip_count(func, *rhs, carried_inputs, outputs),
        ) {
            (Some(trip), _) if is_true_constant(func, *rhs) => Some(trip),
            (_, Some(trip)) if is_true_constant(func, *lhs) => Some(trip),
            _ => None,
        };
    }

    // Look for a comparison that controls the loop.
    if let NodeKind::Lt { lhs, rhs }
    | NodeKind::Le { lhs, rhs }
    | NodeKind::Gt { lhs, rhs }
    | NodeKind::Ge { lhs, rhs } = &term_node.kind
    {
        // Check if one side is a carried variable being incremented.
        for (index, &carry) in carried_inputs.iter().enumerate() {
            let next = *outputs.get(index)?;
            if let Some(trip) = try_count_loop(func, carry, next, *lhs, *rhs) {
                return Some(trip);
            }
        }
    } else if let NodeKind::Eq { lhs, rhs } | NodeKind::Ne { lhs, rhs } = &term_node.kind {
        for (index, &carry) in carried_inputs.iter().enumerate() {
            let next = *outputs.get(index)?;
            if let Some(trip) = try_count_loop(func, carry, next, *lhs, *rhs) {
                return Some(trip);
            }
        }
    }

    None
}

/// Try to compute a trip count when a carried variable is compared
/// against a constant bound.
fn try_count_loop(
    func: &Function,
    carried: NodeId,
    next: NodeId,
    lhs: NodeId,
    rhs: NodeId,
) -> Option<u64> {
    // Check if one operand is the carried variable and the other is a constant.
    let bound_side = if lhs == carried {
        rhs
    } else if rhs == carried {
        lhs
    } else {
        return None;
    };

    // The carried value must actually be stepped by a nonzero constant;
    // otherwise a runtime bound sentinel that merely appears in the
    // comparison would be mistaken for a counter.
    let step = match func.get_node(next).map(|node| &node.kind) {
        Some(NodeKind::Add { lhs, rhs }) if *lhs == carried => get_constant_u64(func, *rhs),
        Some(NodeKind::Add { lhs, rhs }) if *rhs == carried => get_constant_u64(func, *lhs),
        Some(NodeKind::Sub { lhs, rhs }) if *lhs == carried => get_constant_u64(func, *rhs),
        _ => None,
    }?;
    if step == 0 {
        return None;
    }

    // Get the constant bound.
    let bound_val = get_constant_u64(func, bound_side)?;

    // Very rough: if there's a constant bound against a carried variable,
    // the trip count is at most that bound.
    Some(bound_val)
}

/// Get a u64 constant value from a node.
fn get_constant_u64(func: &Function, id: NodeId) -> Option<u64> {
    let node = func.get_node(id)?;
    if let NodeKind::Constant(data) = &node.kind {
        data.as_u64()
    } else {
        None
    }
}

/// Check if a node is the boolean constant `true`.
fn is_true_constant(func: &Function, id: NodeId) -> bool {
    matches!(
        func.get_node(id).map(|node| &node.kind),
        Some(NodeKind::Constant(ConstantData::Bool(true)))
    )
}

/// Check if a node is an integer constant 0.
fn is_zero_constant(func: &Function, id: NodeId) -> bool {
    get_constant_u64(func, id) == Some(0)
}

/// Check if a Loop body contains another Loop (nesting).
fn is_nested_loop(func: &Function, body: &[NodeId]) -> bool {
    for &body_id in body {
        if let Some(node) = func.get_node(body_id) {
            if matches!(node.kind, NodeKind::Loop { .. }) {
                return true;
            }
        }
    }
    false
}

/// Detect reduction variables in a loop.
///
/// A reduction is a carried variable used in an associative operation
/// (Add, Mul, And, Or, Xor) with a loop-invariant value.
fn detect_reductions(
    func: &Function,
    carried_inputs: &[NodeId],
    outputs: &[NodeId],
) -> Vec<ReductionVar> {
    let mut reductions = Vec::new();

    for (&carry, &output) in carried_inputs.iter().zip(outputs.iter()) {
        if let Some(output_node) = func.get_node(output) {
            // Folded additive offset (clang reassociation of
            // `s += (e + c)`): `output = (carry + c) + e`. The per-
            // iteration contribution is the ELEMENT; the constant offset
            // is carried in the chain. Distinct kind so raw-sum
            // consumers never accept it.
            if let Some(element) = folded_add_offset(func, &output_node.kind, carry) {
                reductions.push(ReductionVar {
                    variable: carry,
                    reduction_kind: "sum_offset".to_string(),
                    invariant_value: element,
                });
                continue;
            }

            // Check if output node is an associative operation
            // where one operand is the carried input.
            let reduction_kind = match &output_node.kind {
                NodeKind::Add { lhs, rhs } => {
                    if *lhs == carry || *rhs == carry {
                        Some("sum".to_string())
                    } else {
                        None
                    }
                }
                NodeKind::Mul { lhs, rhs } => {
                    if *lhs == carry || *rhs == carry {
                        Some("product".to_string())
                    } else {
                        None
                    }
                }
                NodeKind::Sub { lhs, .. } => {
                    // Decrementing induction (reverse scan, PS002-style):
                    // i_next = i - step. Only a unit decrement counts as a
                    // contiguous-traversal counter.
                    if *lhs == carry {
                        Some("sub".to_string())
                    } else {
                        None
                    }
                }
                NodeKind::And { lhs, rhs } | NodeKind::BoolAnd { lhs, rhs } => {
                    if *lhs == carry || *rhs == carry {
                        Some("bitwise_and".to_string())
                    } else {
                        None
                    }
                }
                NodeKind::Or { lhs, rhs } | NodeKind::BoolOr { lhs, rhs } => {
                    if *lhs == carry || *rhs == carry {
                        Some("bitwise_or".to_string())
                    } else {
                        None
                    }
                }
                NodeKind::Xor { lhs, rhs } => {
                    if *lhs == carry || *rhs == carry {
                        Some("bitwise_xor".to_string())
                    } else {
                        None
                    }
                }
                NodeKind::Ne { lhs, rhs } => {
                    // a != b is XOR for boolean operands
                    if *lhs == carry || *rhs == carry {
                        Some("bitwise_xor".to_string())
                    } else {
                        None
                    }
                }
                NodeKind::Select {
                    true_val,
                    false_val,
                    ..
                } => {
                    // Sticky-reset reduction: keep the carried value while a
                    // condition holds, reset to a constant identity otherwise.
                    // Form 1: select(c, carry, reset) — All(c) with init 1.
                    // Form 2: select(c, reset, carry) — All(¬c) with init 1.
                    // (Gate 6A-v1: V03/k18 all-same accumulators lower to this
                    // form; the All-reduction recognizer needs the fact.)
                    if *true_val == carry && is_zero_constant(func, *false_val) {
                        Some("select_reset".to_string())
                    } else if *false_val == carry && is_zero_constant(func, *true_val) {
                        Some("select_reset".to_string())
                    } else {
                        None
                    }
                }
                _ => None,
            };

            if let Some(kind) = reduction_kind {
                // Find the other operand (the invariant value).
                let other = match &output_node.kind {
                    NodeKind::Add { lhs, rhs }
                    | NodeKind::Mul { lhs, rhs }
                    | NodeKind::And { lhs, rhs }
                    | NodeKind::Or { lhs, rhs }
                    | NodeKind::Xor { lhs, rhs }
                    | NodeKind::BoolAnd { lhs, rhs }
                    | NodeKind::BoolOr { lhs, rhs }
                    | NodeKind::Ne { lhs, rhs } => {
                        if *lhs == carry {
                            *rhs
                        } else {
                            *lhs
                        }
                    }
                    NodeKind::Sub { lhs, rhs } => {
                        // For a decrementing counter the step is the subtrahend.
                        if *lhs == carry {
                            *rhs
                        } else {
                            continue; // rhs - carry is not a carried reduction
                        }
                    }
                    NodeKind::Select {
                        true_val,
                        false_val,
                        ..
                    } => {
                        // For a carry-preserving select, the "invariant value"
                        // is the reset arm (the constant the accumulator
                        // collapses to when the condition fails).
                        if *true_val == carry {
                            *false_val
                        } else {
                            *true_val
                        }
                    }
                    _ => continue,
                };
                reductions.push(ReductionVar {
                    variable: carry,
                    reduction_kind: kind,
                    invariant_value: other,
                });
            }
        }
    }

    reductions
}

/// `(carry + c) + element` (or the commuted forms): returns the element.
/// `c` must be a constant literal.
fn folded_add_offset(func: &Function, kind: &NodeKind, carry: NodeId) -> Option<NodeId> {
    let NodeKind::Add { lhs, rhs } = kind else {
        return None;
    };
    for (inner_side, element_side) in [(*lhs, *rhs), (*rhs, *lhs)] {
        let Some(inner) = func.get_node(inner_side) else {
            continue;
        };
        let NodeKind::Add { lhs: a, rhs: b } = &inner.kind else {
            continue;
        };
        let offset_is_constant = if *a == carry {
            get_constant_u64(func, *b).is_some()
        } else if *b == carry {
            get_constant_u64(func, *a).is_some()
        } else {
            false
        };
        if offset_is_constant {
            return Some(element_side);
        }
    }
    None
}
