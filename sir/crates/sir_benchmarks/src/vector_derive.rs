//! Vector-plan derivation — the recognition half of the
//! `emit_vectorized` pipeline, extracted so the Gate 4B checker can run
//! the real pipeline and bind the emitted artifact to the SIR region it
//! was derived from.
//!
//! This is a verbatim move of the derivation that used to live in
//! `bin/emit_vectorized.rs`; the binary now calls this module.

use sir_nodes::{Function, NodeKind};
use sir_semantics::concepts::SemanticConcept;
use sir_semantics::truth::{Provenance, SemanticTruth};
use sir_types::NodeId;

use crate::vector_plan::{TailPolicy, VectorOp, VectorPlan, VectorPredicate};

pub fn param_name(func: &Function, nid: NodeId) -> String {
    let idx = nid.as_u64() as usize;
    if idx < func.params.len() {
        let p = &func.params[idx];
        if p.name.starts_with('%') {
            format!("p{}", idx)
        } else {
            p.name.clone()
        }
    } else {
        format!("v{}", nid.as_u64())
    }
}

/// Resolve a NodeId to a C expression (constant value or parameter name)
fn resolve_value(func: &Function, nid: NodeId) -> String {
    if let Some(node) = func.arena.get(nid) {
        match &node.kind {
            NodeKind::Constant(data) => {
                if let Some(v) = data.as_u64() {
                    return format!("{}", v);
                }
                "0".to_string()
            }
            NodeKind::Parameter { .. } => param_name(func, nid),
            _ => format!("v{}", nid.as_u64()),
        }
    } else {
        "0".to_string()
    }
}

pub fn derive_vector_plan(func: &Function, truths: &[SemanticTruth]) -> Option<VectorPlan> {
    let has_cardinality = truths.iter().any(|t| t.concept == SemanticConcept::CardinalityReduction);
    let has_conjunctive = truths.iter().any(|t| t.concept == SemanticConcept::ConjunctiveReduction);
    let has_predicate_map = truths.iter().any(|t| t.concept == SemanticConcept::PredicateMap);
    // A dedicated Sum concept was added after the first plan derivation;
    // the sum kernels are exactly those recognized as SumReduction.
    let has_sum = truths.iter().any(|t| t.concept == SemanticConcept::SumReduction);

    // Find the loop body nodes to check for comparisons INSIDE the loop
    let loop_body_has_comparison = find_loop_comparison(func);

    // Distinguish Cardinality (has comparison in loop) from Sum (no comparison in loop)
    if has_cardinality && loop_body_has_comparison {
        return derive_cardinality_plan(func, truths);
    }

    if has_sum {
        return derive_sum_plan(func);
    }

    if has_cardinality && !loop_body_has_comparison {
        // k43_sum_ascii: sum += buf[i], misidentified as CardinalityReduction
        return derive_sum_plan(func);
    }

    if has_conjunctive {
        return derive_all_plan(func, truths);
    }

    // Fallback: check for AND-reduction pattern (k18_all_equal)
    // The lowered SIR uses Select (v ? 1 : 0) for the AND pattern, not BoolAnd
    if has_predicate_map {
        // Check if the loop body has a Select that implements AND-reduction
        let has_and_pattern = check_and_reduction_pattern(func);
        if has_and_pattern {
            return derive_all_plan(func, truths);
        }
    }

    None
}

/// Resolve a value to a C expression only when it is genuinely a
/// parameter (by node kind, not by node-id position) or a constant.
fn resolve_traversal_value(func: &Function, nid: NodeId) -> Option<String> {
    let node = func.arena.get(nid)?;
    match &node.kind {
        NodeKind::Parameter { index } => {
            let p = func.params.get(*index)?;
            if p.name.starts_with('%') {
                Some(format!("p{}", index))
            } else {
                Some(p.name.clone())
            }
        }
        NodeKind::Constant(data) => data.as_u64().map(|v| format!("{}", v)),
        _ => None,
    }
}

/// Resolve the loop region's traversal expressions from its own structure:
/// the buffer is the base of the array accesses in the loop body, and the
/// length is the bound of the normalized `Lt(carry, bound)` termination.
///
/// The historical derivation assumed parameter positions 0 and 1; that held
/// for the Gate 5A kernels but not for outlined multi-loop regions, whose
/// parameters keep the original kernel's positions (Gate 6B).
fn region_traversal(func: &Function) -> Option<(String, String)> {
    for node in func.arena.iter() {
        let NodeKind::Loop {
            body, termination, ..
        } = &node.kind
        else {
            continue;
        };
        let len_node = match &func.get_node(*termination)?.kind {
            NodeKind::Lt { rhs, .. } => *rhs,
            _ => continue,
        };
        let mut buffer: Option<NodeId> = None;
        for &bid in body {
            for nid in collect_subgraph(func, bid) {
                if let Some(n) = func.arena.get(nid) {
                    match &n.kind {
                        NodeKind::ArrayAccess { base, .. } => buffer = Some(*base),
                        NodeKind::Load { ptr } => {
                            if buffer.is_none() {
                                buffer = Some(*ptr);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        let buffer = resolve_traversal_value(func, buffer?)?;
        let length = resolve_traversal_value(func, len_node)?;
        return Some((buffer, length));
    }
    None
}

/// Region-scoped derivation for the Gate 6B fusion composition: the same
/// primitive plan as [`derive_vector_plan`], with the buffer and length
/// resolved from the loop region itself rather than from parameter
/// positions 0 and 1.
pub fn derive_vector_plan_for_region(
    func: &Function,
    truths: &[SemanticTruth],
) -> Option<VectorPlan> {
    let mut plan = derive_vector_plan(func, truths)?;
    if let Some((buffer, length)) = region_traversal(func) {
        plan.buffer_name = buffer;
        plan.length_name = length;
    }
    Some(plan)
}

/// Check if the loop body contains a comparison (Eq/Ne) that's NOT the loop termination check
fn find_loop_comparison(func: &Function) -> bool {
    // Find the Loop node
    for node in func.arena.iter() {
        if let NodeKind::Loop { body, termination, .. } = &node.kind {
            // Collect all body subgraph nodes
            let mut body_nodes = Vec::new();
            for &bid in body {
                body_nodes.extend(collect_subgraph(func, bid));
            }
            // Check for Eq/Ne in body (excluding the termination check)
            for &bid in &body_nodes {
                if let Some(bn) = func.arena.get(bid) {
                    if matches!(&bn.kind, NodeKind::Eq { .. } | NodeKind::Ne { .. }) {
                        if bid != *termination {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

/// Collect all nodes reachable from a starting node (subgraph)
fn collect_subgraph(func: &Function, start: NodeId) -> Vec<NodeId> {
    let mut visited = std::collections::HashSet::new();
    let mut stack = vec![start];
    while let Some(nid) = stack.pop() {
        if visited.contains(&nid) { continue; }
        visited.insert(nid);
        if let Some(node) = func.arena.get(nid) {
            for input in node.kind.input_nodes() {
                stack.push(input);
            }
        }
    }
    visited.into_iter().collect()
}

/// Check if the loop implements an AND-reduction pattern
/// (Select with true_val = carried accumulator, false_val = 0/false)
fn check_and_reduction_pattern(func: &Function) -> bool {
    for node in func.arena.iter() {
        if let NodeKind::Loop { body, .. } = &node.kind {
            let mut body_nodes = Vec::new();
            for &bid in body {
                body_nodes.extend(collect_subgraph(func, bid));
            }
            for &bid in &body_nodes {
                if let Some(bn) = func.arena.get(bid) {
                    if let NodeKind::Select { cond, true_val, false_val } = &bn.kind {
                        // Check if cond is an Eq and false_val is 0
                        if let Some(cn) = func.arena.get(*cond) {
                            if matches!(&cn.kind, NodeKind::Eq { .. }) {
                                if let Some(fn_) = func.arena.get(*false_val) {
                                    if let NodeKind::Constant(data) = &fn_.kind {
                                        if data.as_u64() == Some(0) {
                                            return true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

fn derive_cardinality_plan(func: &Function, truths: &[SemanticTruth]) -> Option<VectorPlan> {
    let pred_truth = truths.iter().find(|t| t.concept == SemanticConcept::PredicateMap)?;
    let buf_name = param_name(func, NodeId::new(0));
    let len_name = param_name(func, NodeId::new(1));
    let predicate = extract_predicate(func, &pred_truth.provenance);

    Some(VectorPlan {
        operation: VectorOp::Cardinality,
        buffer_name: buf_name,
        length_name: len_name,
        predicate,
        vector_width: 32,
        tail: TailPolicy::NarrowerVector,
    })
}

fn derive_sum_plan(func: &Function) -> Option<VectorPlan> {
    Some(VectorPlan {
        operation: VectorOp::Sum,
        buffer_name: param_name(func, NodeId::new(0)),
        length_name: param_name(func, NodeId::new(1)),
        predicate: Some(VectorPredicate::Identity),
        vector_width: 32,
        tail: TailPolicy::NarrowerVector,
    })
}

fn derive_all_plan(func: &Function, truths: &[SemanticTruth]) -> Option<VectorPlan> {
    let pred_truth = truths.iter().find(|t| t.concept == SemanticConcept::PredicateMap)?;
    let buf_name = param_name(func, NodeId::new(0));
    let len_name = param_name(func, NodeId::new(1));
    let predicate = extract_predicate(func, &pred_truth.provenance);

    Some(VectorPlan {
        operation: VectorOp::All,
        buffer_name: buf_name,
        length_name: len_name,
        predicate,
        vector_width: 32,
        tail: TailPolicy::NarrowerVector,
    })
}

fn extract_predicate(func: &Function, provenance: &Provenance) -> Option<VectorPredicate> {
    let nodes = match provenance {
        Provenance::Physical { nodes } => nodes,
        _ => return None,
    };

    for &nid in nodes {
        if let Some(node) = func.arena.get(nid) {
            match &node.kind {
                NodeKind::Eq { lhs, rhs } => {
                    // Check for masked equality: (x & mask) == target
                    if let Some(lhs_node) = func.arena.get(*lhs) {
                        if let NodeKind::And { lhs: and_lhs, rhs: and_rhs } = &lhs_node.kind {
                            let mask = resolve_value(func, *and_rhs);
                            let target = resolve_value(func, *rhs);
                            return Some(VectorPredicate::MaskedEqual { mask, target });
                        }
                    }
                    // Simple equality: x == val
                    let target = resolve_value(func, *rhs);
                    return Some(VectorPredicate::Equal(target));
                }
                _ => {}
            }
        }
    }
    None
}
