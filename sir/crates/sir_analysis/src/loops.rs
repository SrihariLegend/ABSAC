//! Loop analysis.
//!
//! Analyzes Loop nodes: trip count, finiteness, nesting,
//! carried variables, and reduction pattern detection.

use sir_nodes::{Function, NodeKind};
use sir_types::NodeId;
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
            let trip_count = estimate_trip_count(func, *termination, carried_inputs);
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
) -> Option<u64> {
    let term_node = func.get_node(termination)?;

    // Look for a comparison that controls the loop.
    if let NodeKind::Lt { lhs, rhs }
    | NodeKind::Le { lhs, rhs }
    | NodeKind::Gt { lhs, rhs }
    | NodeKind::Ge { lhs, rhs } = &term_node.kind
    {
        // Check if one side is a carried variable being incremented.
        for &carry in carried_inputs {
            if let Some(trip) = try_count_loop(func, carry, *lhs, *rhs) {
                return Some(trip);
            }
        }
    } else if let NodeKind::Eq { lhs, rhs } | NodeKind::Ne { lhs, rhs } = &term_node.kind {
        for &carry in carried_inputs {
            if let Some(trip) = try_count_loop(func, carry, *lhs, *rhs) {
                return Some(trip);
            }
        }
    }

    None
}

/// Try to compute a trip count when a carried variable is compared
/// against a constant bound.
fn try_count_loop(func: &Function, carried: NodeId, lhs: NodeId, rhs: NodeId) -> Option<u64> {
    // Check if one operand is the carried variable and the other is a constant.
    let bound_side = if lhs == carried {
        rhs
    } else if rhs == carried {
        lhs
    } else {
        return None;
    };

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

#[cfg(test)]
mod tests {
    use super::*;
    use sir_builder::Builder;
    use sir_types::{ConstantData, Span, Type};

    fn i32_type() -> Type {
        Type::i32()
    }
    fn u64_type() -> Type {
        Type::u64()
    }
    fn unknown_span() -> Span {
        Span::unknown()
    }

    #[test]
    fn simple_counted_loop_has_trip_count() {
        let mut b = Builder::new("counted", &[("start", u64_type())], u64_type());
        let start = b.parameter_index(0).unwrap();
        let one = b.constant(ConstantData::u64(1), u64_type(), unknown_span());
        let bound = b.constant(ConstantData::u64(64), u64_type(), unknown_span());

        // Loop body: start + 1 (increment).
        let next = b.add(start, one, unknown_span()).unwrap();
        // Termination: start < 64 (carried variable compared to constant bound).
        let cond = b.lt(start, bound, unknown_span()).unwrap();

        let loop_node = b
            .r#loop(
                &[next, cond],
                cond,
                &[next],
                &[start],
                u64_type(),
                unknown_span(),
            )
            .unwrap();
        b.return_value(loop_node, unknown_span()).unwrap();
        let func = b.build();
        let facts = run_loops(&func);

        let loop_fact = facts.get(&loop_node).unwrap();
        assert!(loop_fact.is_finite);
        assert!(loop_fact.trip_count.is_some());
    }

    #[test]
    fn empty_function_has_no_loops() {
        let mut b = Builder::new("f", &[("x", i32_type())], i32_type());
        let x = b.parameter_index(0).unwrap();
        b.return_value(x, unknown_span()).unwrap();
        let func = b.build();
        let facts = run_loops(&func);
        assert!(facts.is_empty());
    }

    #[test]
    fn reduction_detection() {
        let mut b = Builder::new("reduce", &[("init", u64_type())], u64_type());
        let init = b.parameter_index(0).unwrap();
        let one = b.constant(ConstantData::u64(1), u64_type(), unknown_span());

        // Loop body: init + 1 (sum reduction).
        let next = b.add(init, one, unknown_span()).unwrap();
        let cond = b.constant(ConstantData::Bool(true), Type::Bool, unknown_span());

        let loop_node = b
            .r#loop(&[next], cond, &[next], &[init], u64_type(), unknown_span())
            .unwrap();
        b.return_value(loop_node, unknown_span()).unwrap();
        let func = b.build();
        let facts = run_loops(&func);

        let loop_fact = facts.get(&loop_node).unwrap();
        assert!(!loop_fact.reductions.is_empty());
        let red = &loop_fact.reductions[0];
        assert_eq!(red.reduction_kind, "sum");
    }

    #[test]
    fn nested_loop_detection() {
        let mut b = Builder::new("nested", &[("x", u64_type())], u64_type());
        let x = b.parameter_index(0).unwrap();
        let one = b.constant(ConstantData::u64(1), u64_type(), unknown_span());
        let t = b.constant(ConstantData::Bool(true), Type::Bool, unknown_span());
        let next_x = b.add(x, one, unknown_span()).unwrap();

        // Inner loop.
        let inner = b
            .r#loop(&[next_x], t, &[next_x], &[x], u64_type(), unknown_span())
            .unwrap();

        // Outer loop containing inner.
        let outer = b
            .r#loop(
                &[inner, next_x],
                t,
                &[next_x],
                &[x],
                u64_type(),
                unknown_span(),
            )
            .unwrap();
        b.return_value(outer, unknown_span()).unwrap();
        let func = b.build();
        let facts = run_loops(&func);

        let outer_fact = facts.get(&outer).unwrap();
        assert!(outer_fact.is_nested);
    }
}
