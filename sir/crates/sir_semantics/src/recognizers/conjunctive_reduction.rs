use sir_analysis::facts::FactDatabase;
use sir_analysis::graph;
use sir_nodes::{Function, NodeKind};
use sir_types::{ConstantData, Effects, NodeId};

use crate::concepts::SemanticConcept;
use crate::region::RecognitionExplanation;
use crate::truth::ValueId;

/// Recognize conjunctive reduction (All) patterns.
///
/// A conjunctive reduction checks if all elements satisfy a condition:
/// `result = 1 iff pred(elem_i) holds for every element`. Two accumulator
/// forms are recognized:
///
/// 1. **bitwise_and** — `acc = acc & pred` (builder-constructed loops,
///    e.g. BR003 boolean collections).
///
/// 2. **select_reset** — `acc = pred ? acc : 0` with carried init 1.
///    This is the form real frontends emit after branch removal:
///    `ok = ok & (a[i] == want)` lowers to `select(cmp, ok, 0)`.
///    (Gate 6A-v1 finding: V03/H06 use this form, which no recognizer
///    handled — the All-reduction recall gap.)
///
/// Safety checks for the select_reset form (mandatory — see
/// certificate.rs for recognition-vs-authorization separation):
/// - No side effects beyond READ_MEMORY
/// - No volatile accesses
/// - Closed-world memory footprint: at most one resolvable base
/// - Contiguous stride (unit-increment counter exists)
/// - Carried init is 1, reset arm is 0 (the All identity pair)
/// - The select's condition is a boolean predicate over data
///
/// Returns (concept, explanation, related_node_ids, inputs, outputs) tuples.
pub fn recognize_conjunctive_reduction(
    func: &Function,
    analysis: &FactDatabase,
) -> Vec<(SemanticConcept, RecognitionExplanation, Vec<NodeId>, Vec<ValueId>, Vec<ValueId>)> {
    let mut results = Vec::new();

    for node in func.arena.iter() {
        let NodeKind::Loop {
            body: _,
            termination: _,
            outputs,
            carried_inputs,
        } = &node.kind
        else {
            continue;
        };
        let outputs: &Vec<NodeId> = outputs;
        let carried_inputs: &Vec<NodeId> = carried_inputs;

        // ── Safety check 1: Reject loops with side effects ──
        let allowed_effects = Effects::READ_MEMORY;
        if !(node.effects - allowed_effects).is_empty() {
            continue;
        }

        let body_nodes = collect_loop_body_nodes(&node.kind);

        // ── Safety check 2: Reject loops with volatile accesses ──
        let has_volatile = body_nodes.iter().any(|&nid| {
            func.get_node(nid)
                .map(|n| n.effects.contains(Effects::VOLATILE))
                .unwrap_or(false)
        });
        if has_volatile {
            continue;
        }

        // ── Closed-world check (ReductionCertificate completeness) ──
        // Same invariant as Cardinality/Sum: the certificate must bind
        // every memory base the region touches. A multi-base or
        // unresolvable footprint forces abstention.
        match crate::certificate::memory_footprint(func, &body_nodes) {
            crate::certificate::FootprintCheck::Complete(bases) if bases.len() <= 1 => {}
            _ => continue,
        }

        let Some(loop_fact) = analysis.loops.get(&node.id) else {
            continue;
        };

        // Form 1: bitwise_and reductions (legacy path — unchanged).
        let and_reductions: Vec<_> = loop_fact
            .reductions
            .iter()
            .filter(|r| r.reduction_kind == "bitwise_and")
            .collect();

        if !and_reductions.is_empty() {
            let mut related = vec![node.id];
            for reduction in &and_reductions {
                related.push(reduction.variable);
                related.push(reduction.invariant_value);
            }
            results.push((
                SemanticConcept::ConjunctiveReduction,
                RecognitionExplanation {
                    concept: SemanticConcept::ConjunctiveReduction,
                    triggering_facts: vec![
                        "Loop has bitwise AND reduction",
                        "Reduction variable accumulates boolean conditions",
                    ],
                },
                related,
                vec![], // legacy form: no concrete input binding
                vec![ValueId::new(node.id.0)],
            ));
            continue;
        }

        // Form 2: select_reset — All via sticky-zero accumulator.
        let select_reductions: Vec<_> = loop_fact
            .reductions
            .iter()
            .filter(|r| r.reduction_kind == "select_reset")
            .collect();

        if select_reductions.len() != 1 {
            continue;
        }
        let all_acc = select_reductions[0];
        let carry = all_acc.variable;

        // ── Safety check: contiguous stride ──
        // A unit-increment counter must exist alongside the accumulator.
        let has_unit_counter = loop_fact
            .reductions
            .iter()
            .any(|r| r.reduction_kind == "sum" && is_constant_one(func, r.invariant_value));
        if !has_unit_counter {
            continue;
        }

        // ── Safety check: the accumulator is a sticky-reset select ──
        // Locate the select node: the output paired with this carried input.
        let Some(slot) = carried_inputs.iter().position(|&c| c == carry) else {
            continue;
        };
        if slot >= outputs.len() {
            continue;
        }
        let select_id = outputs[slot];
        let Some(select_node) = func.get_node(select_id) else {
            continue;
        };
        let NodeKind::Select {
            cond,
            true_val,
            false_val,
        } = &select_node.kind
        else {
            continue;
        };

        // Carried init must be constant 1 (the AND identity).
        let init_id = carried_inputs[slot];
        if !is_constant_one(func, init_id) {
            continue;
        }

        // The non-carry arm (the reset value) must be constant 0.
        let reset_arm = if *true_val == carry { *false_val } else { *true_val };
        if !is_constant_zero(func, reset_arm) {
            continue;
        }

        // ── Safety check: the condition is a boolean predicate ──
        if !is_boolean_predicate(func, *cond) {
            continue;
        }

        // ── Safety check: accumulator independence ──
        // The select's condition must not depend on any other carried
        // accumulator variable (mirrors the Cardinality/Sum check).
        // The unit counter is not an accumulator — exclude it.
        let transitive = graph::transitive_inputs(*cond, &func.arena);
        let dependent_on_other = loop_fact.reductions.iter().any(|r| {
            r.variable != carry
                && !(r.reduction_kind == "sum" && is_constant_one(func, r.invariant_value))
                && transitive.contains(&r.variable)
        });
        if dependent_on_other {
            continue;
        }

        // ── Passed all checks — emit recognition ──
        let predicate = *cond;

        results.push((
            SemanticConcept::ConjunctiveReduction,
            RecognitionExplanation {
                concept: SemanticConcept::ConjunctiveReduction,
                triggering_facts: vec![
                    "Loop has sticky-reset select accumulator (select(cond, acc, 0))",
                    "Carried init is 1, reset arm is 0 (All identity pair)",
                    "Loop has contiguous stride (increment by 1)",
                    "Loop has no volatile accesses",
                    "Memory footprint is a single declared base",
                ],
            },
            vec![node.id, select_id],
            vec![ValueId::new(predicate.0)],
            vec![ValueId::new(node.id.0)],
        ));
    }

    results
}

/// Collect all NodeIds referenced within a Loop's body, outputs, and carried_inputs.
fn collect_loop_body_nodes(kind: &NodeKind) -> Vec<NodeId> {
    if let NodeKind::Loop {
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
    is_constant_value(func, id, "1")
}

/// Check if a node is a constant integer 0.
fn is_constant_zero(func: &Function, id: NodeId) -> bool {
    is_constant_value(func, id, "0")
}

fn is_constant_value(func: &Function, id: NodeId, value: &str) -> bool {
    if let Some(node) = func.get_node(id) {
        if let NodeKind::Constant(data) = &node.kind {
            if let ConstantData::Integer { value: v, .. } = data {
                return v == value;
            }
        }
    }
    false
}

/// Check if a node represents a boolean predicate (0 or 1), not a raw value.
///
/// A boolean predicate is:
/// - A comparison node (Eq, Ne, Lt, Le, Gt, Ge) — produces Bool
/// - A Convert from a comparison (Bool → integer conversion)
/// - A Select(cond, 1, 0) — explicit boolean selection
fn is_boolean_predicate(func: &Function, id: NodeId) -> bool {
    let node = match func.get_node(id) {
        Some(n) => n,
        None => return false,
    };

    match &node.kind {
        NodeKind::Select { true_val, false_val, .. } => {
            (is_constant_one(func, *true_val) && is_constant_zero(func, *false_val))
                || (is_constant_zero(func, *true_val) && is_constant_one(func, *false_val))
        }
        NodeKind::Eq { .. }
        | NodeKind::Ne { .. }
        | NodeKind::Lt { .. }
        | NodeKind::Le { .. }
        | NodeKind::Gt { .. }
        | NodeKind::Ge { .. } => true,
        NodeKind::Convert { .. } => {
            let inputs = graph::dataflow_inputs(&node.kind);
            match inputs.first() {
                Some(&src) => match func.get_node(src) {
                    Some(src_node) => matches!(
                        src_node.kind,
                        NodeKind::Eq { .. }
                            | NodeKind::Ne { .. }
                            | NodeKind::Lt { .. }
                            | NodeKind::Le { .. }
                            | NodeKind::Gt { .. }
                            | NodeKind::Ge { .. }
                    ),
                    None => false,
                },
                None => false,
            }
        }
        _ => false,
    }
}
