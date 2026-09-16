use sir_analysis::facts::FactDatabase;
use sir_analysis::graph;
use sir_nodes::Function;
use sir_types::{ConstantData, Effects, NodeId};

use crate::concepts::SemanticConcept;
use crate::region::RecognitionExplanation;
use crate::truth::ValueId;

/// Recognize sum reduction patterns.
///
/// A sum reduction adds the values of elements in a collection. We detect:
/// - A loop with a reduction variable of kind "sum"
/// - The accumulated value is the raw element value (not a boolean predicate)
/// - The loop has no observable side effects (IO, WRITE_MEMORY, etc.)
/// - The loop has no volatile memory accesses
/// - The loop counter increments by 1 (contiguous stride)
/// - Multiple accumulators are independent (no data dependency)
///
/// This is the complement of CardinalityReduction, which counts how many
/// elements satisfy a boolean condition. SumReduction sums the raw values.
pub fn recognize_sum_reduction(
    func: &Function,
    analysis: &FactDatabase,
) -> Vec<(SemanticConcept, RecognitionExplanation, Vec<NodeId>, Vec<ValueId>, Vec<ValueId>)> {
    let mut results = Vec::new();

    for node in func.arena.iter() {
        if let sir_nodes::NodeKind::Loop { .. } = &node.kind {
            // ── Safety check 1: Reject loops with side effects ──
            let allowed_effects = Effects::READ_MEMORY;
            if !(node.effects - allowed_effects).is_empty() {
                continue;
            }

            // ── Safety check 2: Reject loops with volatile accesses ──
            let body_nodes = collect_loop_body_nodes(&node.kind);
            let has_volatile = body_nodes.iter().any(|&nid| {
                func.get_node(nid)
                    .map(|n| n.effects.contains(Effects::VOLATILE))
                    .unwrap_or(false)
            });
            if has_volatile {
                continue;
            }

            // ── Closed-world check (ReductionCertificate completeness) ──
            // See certificate.rs and cardinality_reduction.rs. A sum truth
            // may only fire when the loop's memory footprint is a single,
            // fully-resolvable base.
            {
                let body_nodes = collect_loop_body_nodes(&node.kind);
                match crate::certificate::memory_footprint(func, &body_nodes) {
                    crate::certificate::FootprintCheck::Complete(bases) if bases.len() <= 1 => {}
                    _ => continue,
                }
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

                // ── Safety check 3: Verify contiguous stride ──
                let counter = sum_reductions.iter().find(|r| {
                    is_constant_one(func, r.invariant_value)
                });
                if counter.is_none() {
                    continue;
                }

                // ── Distinguish Sum from Cardinality ──
                // For Sum: the invariant_value is a raw value (e.g., zext of a load),
                // NOT a boolean predicate (Select 1/0, comparison, Convert from Bool).
                let non_counter_reductions: Vec<_> = sum_reductions
                    .iter()
                    .filter(|r| !is_constant_one(func, r.invariant_value))
                    .collect();

                let sum_reductions_only: Vec<_> = non_counter_reductions
                    .iter()
                    .filter(|r| !is_boolean_predicate(func, r.invariant_value))
                    .filter(|r| is_raw_element_value(func, r.invariant_value))
                    .collect();

                if sum_reductions_only.is_empty() {
                    continue;
                }

                // ── Safety check 4: Check accumulator independence ──
                if non_counter_reductions.len() > 1 {
                    let all_independent = non_counter_reductions.iter().all(|r| {
                        let transitive = graph::transitive_inputs(r.invariant_value, &func.arena);
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
                for reduction in &sum_reductions_only {
                    related.push(reduction.variable);
                    related.push(reduction.invariant_value);
                }

                let mut inputs = Vec::new();
                let mut outputs = Vec::new();
                for reduction in &sum_reductions_only {
                    inputs.push(ValueId::new(reduction.invariant_value.0));
                    outputs.push(ValueId::new(node.id.0));
                }

                results.push((
                    SemanticConcept::SumReduction,
                    RecognitionExplanation {
                        concept: SemanticConcept::SumReduction,
                        triggering_facts: vec![
                            "Loop has additive reduction",
                            "Reduction variable accumulates raw element values",
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

/// Gate 6A-v3 finding: a conditional accumulation
/// (`if (buf[i] & 1) s += buf[i];` lowering to
/// `s += select(cond, zext(buf[i]), 0)`) was accepted as a raw-element sum.
/// The masked value is not the element; treating it as one is a false
/// positive that a naive vector plan would mis-vectorize. The invariant
/// must be the element itself, possibly through transparent conversion
/// (widen/narrow) nodes.
fn is_raw_element_value(func: &Function, id: NodeId) -> bool {
    let mut current = id;
    for _ in 0..8 {
        let Some(node) = func.get_node(current) else {
            return false;
        };
        match &node.kind {
            sir_nodes::NodeKind::Convert { operand, .. } => current = *operand,
            sir_nodes::NodeKind::ArrayAccess { .. } | sir_nodes::NodeKind::Load { .. } => {
                return true;
            }
            _ => return false,
        }
    }
    false
}

/// Check if a node represents a boolean predicate (0 or 1).
/// This is the SAME check as in cardinality_reduction.rs — used to
/// EXCLUDE boolean predicates from SumReduction.
fn is_boolean_predicate(func: &Function, id: NodeId) -> bool {
    let node = match func.get_node(id) {
        Some(n) => n,
        None => return false,
    };

    match &node.kind {
        sir_nodes::NodeKind::Select { true_val, false_val, .. } => {
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
            let inputs = graph::dataflow_inputs(&node.kind);
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
