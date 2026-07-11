//! Constant propagation analysis.
//!
//! SCCP-style constant folding using a three-level lattice:
//!   Top (unknown) → Constant(ConstantData) → Bottom (overdefined)
//! Worklist-based iterative fixpoint computation.

use sir_nodes::{Function, NodeKind};
use sir_types::{ConstantData, NodeId};
use std::collections::{HashMap, VecDeque};

use crate::facts::{ConstantFact, ConstantLattice};
use crate::graph;

/// Run constant propagation on a function.
///
/// If `use_def_facts` is provided, reuses the precomputed user map
/// instead of building one from scratch (avoids redundant O(N) work).
///
/// Algorithm:
/// 1. Initialize: Constant nodes → Constant(v), Parameters → Top, others → Top
/// 2. Worklist: process nodes whose inputs are all known (not Top)
/// 3. Evaluate: if all inputs are Constant, compute result; if any input is Bottom, result is Bottom
/// 4. If result changes, add all users to worklist
/// 5. Iterate to fixpoint
pub fn run_constants(
    func: &Function,
    use_def_facts: Option<&HashMap<NodeId, crate::facts::UseDefFact>>,
) -> HashMap<NodeId, ConstantFact> {
    let mut lattice: HashMap<NodeId, ConstantLattice> = HashMap::new();
    let all_ids = graph::all_node_ids(func);

    // Reuse precomputed user map if available, otherwise build from scratch.
    let user_map: HashMap<NodeId, Vec<NodeId>> = if let Some(ud) = use_def_facts {
        ud.iter().map(|(&id, f)| (id, f.users.clone())).collect()
    } else {
        build_user_map(func)
    };

    // Initialize lattice.
    for &id in &all_ids {
        if let Some(node) = func.get_node(id) {
            let init = match &node.kind {
                NodeKind::Constant(data) => ConstantLattice::Constant(data.clone()),
                _ => ConstantLattice::Top,
            };
            lattice.insert(id, init);
        }
    }

    // Worklist: start with ALL nodes. The algorithm converges quickly
    // because nodes with Top inputs stay Top and don't re-trigger.
    let mut worklist: VecDeque<NodeId> = all_ids.iter().copied().collect();

    while let Some(id) = worklist.pop_front() {
        let current = lattice.get(&id).cloned().unwrap_or(ConstantLattice::Top);
        let new_value = evaluate_node(func, id, &lattice);

        if new_value != current {
            lattice.insert(id, new_value);
            // Add users to worklist.
            if let Some(users) = user_map.get(&id) {
                for &user in users {
                    worklist.push_back(user);
                }
            }
        }
    }

    // Build facts.
    lattice
        .into_iter()
        .map(|(id, value)| (id, ConstantFact { value }))
        .collect()
}

/// Build a map from node to its dataflow users.
fn build_user_map(func: &Function) -> HashMap<NodeId, Vec<NodeId>> {
    let mut map: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    for (id, node) in func.arena.nodes() {
        for input in graph::dataflow_inputs(&node.kind) {
            map.entry(input).or_default().push(*id);
        }
    }
    map
}

/// Check if a node kind represents a foldable operation.
fn is_foldable(kind: &NodeKind) -> bool {
    matches!(
        kind,
        NodeKind::Add { .. }
            | NodeKind::Sub { .. }
            | NodeKind::Mul { .. }
            | NodeKind::Div { .. }
            | NodeKind::Rem { .. }
            | NodeKind::Neg { .. }
            | NodeKind::And { .. }
            | NodeKind::Or { .. }
            | NodeKind::Xor { .. }
            | NodeKind::Shl { .. }
            | NodeKind::Shr { .. }
            | NodeKind::Not { .. }
            | NodeKind::Popcount { .. }
            | NodeKind::Eq { .. }
            | NodeKind::Ne { .. }
            | NodeKind::Lt { .. }
            | NodeKind::Le { .. }
            | NodeKind::Gt { .. }
            | NodeKind::Ge { .. }
            | NodeKind::BoolAnd { .. }
            | NodeKind::BoolOr { .. }
            | NodeKind::BoolNot { .. }
            | NodeKind::Select { .. }
    )
}

/// Evaluate a node given the current lattice values of its inputs.
fn evaluate_node(
    func: &Function,
    id: NodeId,
    lattice: &HashMap<NodeId, ConstantLattice>,
) -> ConstantLattice {
    let node = match func.get_node(id) {
        Some(n) => n,
        None => return ConstantLattice::Top,
    };

    // If it's already a constant, stay constant.
    if let NodeKind::Constant(data) = &node.kind {
        return ConstantLattice::Constant(data.clone());
    }

    // Parameters and other non-foldable nodes: keep current lattice value.
    if !is_foldable(&node.kind) {
        return lattice.get(&id).cloned().unwrap_or(ConstantLattice::Top);
    }

    // Get lattice values of dataflow inputs.
    let inputs = graph::dataflow_inputs(&node.kind);
    let input_vals: Vec<&ConstantLattice> = inputs
        .iter()
        .map(|iid| lattice.get(iid).unwrap_or(&ConstantLattice::Top))
        .collect();

    // If any input is Top, result is Top (not ready).
    if input_vals.iter().any(|v| v.is_top()) {
        return ConstantLattice::Top;
    }

    // If any input is Bottom, result is Bottom (overdefined).
    if input_vals.iter().any(|v| v.is_bottom()) {
        return ConstantLattice::Bottom;
    }

    // All inputs are Constant — try to fold.
    let consts: Vec<&ConstantData> = input_vals
        .iter()
        .filter_map(|v| match v {
            ConstantLattice::Constant(d) => Some(d),
            _ => None,
        })
        .collect();

    if consts.len() != inputs.len() {
        return ConstantLattice::Top; // shouldn't happen
    }

    fold_operation(&node.kind, &consts)
}

/// Fold a node kind with all-constant inputs.
fn fold_operation(kind: &NodeKind, inputs: &[&ConstantData]) -> ConstantLattice {
    match kind {
        // ── Arithmetic ──
        NodeKind::Add { .. } => fold_binary_arith(inputs, |a, b| a + b, |a, b| a + b),
        NodeKind::Sub { .. } => fold_binary_arith(inputs, |a, b| a - b, |a, b| a - b),
        NodeKind::Mul { .. } => fold_binary_arith(inputs, |a, b| a * b, |a, b| a * b),
        NodeKind::Div { .. } => {
            if inputs.len() == 2 {
                let rhs = inputs[1];
                if rhs.as_i64() == Some(0) || rhs.as_u64() == Some(0) {
                    return ConstantLattice::Bottom; // division by zero
                }
            }
            fold_binary_arith(inputs, |a, b| a / b, |a, b| a / b)
        }
        NodeKind::Rem { .. } => {
            if inputs.len() == 2 {
                let rhs = inputs[1];
                if rhs.as_i64() == Some(0) || rhs.as_u64() == Some(0) {
                    return ConstantLattice::Bottom;
                }
            }
            fold_binary_arith(inputs, |a, b| a % b, |a, b| a % b)
        }
        NodeKind::Neg { .. } => fold_unary_int(inputs, |a| -a),

        // ── Bitwise (integers only) ──
        NodeKind::And { .. } => fold_binary_int(inputs, |a, b| a & b),
        NodeKind::Or { .. } => fold_binary_int(inputs, |a, b| a | b),
        NodeKind::Xor { .. } => fold_binary_int(inputs, |a, b| a ^ b),
        NodeKind::Shl { .. } => fold_binary_int(inputs, |a, b| a << b),
        NodeKind::Shr { .. } => fold_binary_int(inputs, |a, b| a >> b),
        NodeKind::Not { .. } => fold_unary_int(inputs, |a| !a),

        // ── Unary bitwise that changes width ──
        NodeKind::Popcount { .. } => {
            if let Some(v) = inputs.first().and_then(|c| c.as_u64()) {
                let ct = v.count_ones() as u64;
                return ConstantLattice::Constant(ConstantData::u64(ct));
            }
            ConstantLattice::Bottom
        }

        // ── Comparisons ──
        NodeKind::Eq { .. } => fold_cmp(inputs, |a, b| a == b),
        NodeKind::Ne { .. } => fold_cmp(inputs, |a, b| a != b),
        NodeKind::Lt { .. } => fold_cmp(inputs, |a, b| a < b),
        NodeKind::Le { .. } => fold_cmp(inputs, |a, b| a <= b),
        NodeKind::Gt { .. } => fold_cmp(inputs, |a, b| a > b),
        NodeKind::Ge { .. } => fold_cmp(inputs, |a, b| a >= b),

        // ── Boolean ──
        NodeKind::BoolAnd { .. } => fold_bool(inputs, |a, b| a && b),
        NodeKind::BoolOr { .. } => fold_bool(inputs, |a, b| a || b),
        NodeKind::BoolNot { .. } => {
            if let Some(ConstantData::Bool(v)) = inputs.first() {
                return ConstantLattice::Constant(ConstantData::Bool(!v));
            }
            ConstantLattice::Bottom
        }

        // ── Select ──
        NodeKind::Select { .. } => {
            if inputs.len() == 3 {
                if let ConstantData::Bool(cond) = inputs[0] {
                    return ConstantLattice::Constant(if *cond {
                        inputs[1].clone()
                    } else {
                        inputs[2].clone()
                    });
                }
            }
            ConstantLattice::Bottom
        }

        // ── Not foldable ──
        _ => ConstantLattice::Bottom,
    }
}

// ── Folding helpers ────────────────────────────────────────

fn fold_binary_arith(
    inputs: &[&ConstantData],
    f_signed: fn(i64, i64) -> i64,
    f_unsigned: fn(u64, u64) -> u64,
) -> ConstantLattice {
    if inputs.len() != 2 {
        return ConstantLattice::Bottom;
    }
    // Try signed.
    if let (Some(a), Some(b)) = (inputs[0].as_i64(), inputs[1].as_i64()) {
        return ConstantLattice::Constant(ConstantData::i64(f_signed(a, b)));
    }
    // Try unsigned.
    if let (Some(a), Some(b)) = (inputs[0].as_u64(), inputs[1].as_u64()) {
        return ConstantLattice::Constant(ConstantData::u64(f_unsigned(a, b)));
    }
    ConstantLattice::Bottom
}

fn fold_binary_int(inputs: &[&ConstantData], f: fn(u64, u64) -> u64) -> ConstantLattice {
    if inputs.len() != 2 {
        return ConstantLattice::Bottom;
    }
    if let (Some(a), Some(b)) = (inputs[0].as_u64(), inputs[1].as_u64()) {
        return ConstantLattice::Constant(ConstantData::u64(f(a, b)));
    }
    ConstantLattice::Bottom
}

fn fold_unary_int(inputs: &[&ConstantData], f: fn(i64) -> i64) -> ConstantLattice {
    if let Some(v) = inputs.first().and_then(|c| c.as_i64()) {
        return ConstantLattice::Constant(ConstantData::i64(f(v)));
    }
    ConstantLattice::Bottom
}

fn fold_cmp(inputs: &[&ConstantData], f: fn(i64, i64) -> bool) -> ConstantLattice {
    if inputs.len() != 2 {
        return ConstantLattice::Bottom;
    }
    if let (Some(a), Some(b)) = (inputs[0].as_i64(), inputs[1].as_i64()) {
        return ConstantLattice::Constant(ConstantData::Bool(f(a, b)));
    }
    ConstantLattice::Bottom
}

fn fold_bool(inputs: &[&ConstantData], f: fn(bool, bool) -> bool) -> ConstantLattice {
    if inputs.len() != 2 {
        return ConstantLattice::Bottom;
    }
    if let (ConstantData::Bool(a), ConstantData::Bool(b)) = (inputs[0], inputs[1]) {
        return ConstantLattice::Constant(ConstantData::Bool(f(*a, *b)));
    }
    ConstantLattice::Bottom
}

#[cfg(test)]
mod tests {
    use super::*;
    use sir_builder::Builder;
    use sir_types::{Span, Type};

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
    fn constant_node_stays_constant() {
        let mut b = Builder::new("f", &[], i32_type());
        let c = b.constant(ConstantData::i32(42), i32_type(), unknown_span());
        b.return_value(c, unknown_span()).unwrap();
        let func = b.build();
        let facts = run_constants(&func, None);

        let c_fact = facts.get(&c).unwrap();
        assert_eq!(
            c_fact.value,
            ConstantLattice::Constant(ConstantData::i32(42))
        );
    }

    #[test]
    fn parameter_is_top() {
        let mut b = Builder::new("f", &[("x", i32_type())], i32_type());
        let x = b.parameter_index(0).unwrap();
        b.return_value(x, unknown_span()).unwrap();
        let func = b.build();
        let facts = run_constants(&func, None);

        assert!(facts.get(&x).unwrap().value.is_top());
    }

    #[test]
    fn constant_folding_add() {
        let mut b = Builder::new("fold", &[], i32_type());
        let a = b.constant(ConstantData::i32(1), i32_type(), unknown_span());
        let b_c = b.constant(ConstantData::i32(2), i32_type(), unknown_span());
        let sum = b.add(a, b_c, unknown_span()).unwrap();
        b.return_value(sum, unknown_span()).unwrap();
        let func = b.build();
        let facts = run_constants(&func, None);

        let sum_fact = facts.get(&sum).unwrap();
        assert_eq!(
            sum_fact.value,
            ConstantLattice::Constant(ConstantData::i64(3))
        );
    }

    #[test]
    fn constant_folding_select() {
        let mut b = Builder::new("sel_fold", &[], i32_type());
        let cond = b.constant(ConstantData::Bool(true), Type::Bool, unknown_span());
        let t_val = b.constant(ConstantData::i32(10), i32_type(), unknown_span());
        let f_val = b.constant(ConstantData::i32(20), i32_type(), unknown_span());
        let sel = b.select(cond, t_val, f_val, unknown_span()).unwrap();
        b.return_value(sel, unknown_span()).unwrap();
        let func = b.build();
        let facts = run_constants(&func, None);

        let sel_fact = facts.get(&sel).unwrap();
        // Select with true condition returns true_val.
        assert_eq!(
            sel_fact.value,
            ConstantLattice::Constant(ConstantData::i32(10))
        );
    }

    #[test]
    fn mixed_constant_and_param_is_top() {
        let mut b = Builder::new("mix", &[("x", i32_type())], i32_type());
        let x = b.parameter_index(0).unwrap();
        let c = b.constant(ConstantData::i32(5), i32_type(), unknown_span());
        let sum = b.add(x, c, unknown_span()).unwrap();
        b.return_value(sum, unknown_span()).unwrap();
        let func = b.build();
        let facts = run_constants(&func, None);

        // x + 5: x is Top, so result is Top (not ready).
        assert!(facts.get(&sum).unwrap().value.is_top());
    }

    #[test]
    fn chained_constant_folding() {
        // (1 + 2) * 3 = 9
        let mut b = Builder::new("chain", &[], i32_type());
        let one = b.constant(ConstantData::i32(1), i32_type(), unknown_span());
        let two = b.constant(ConstantData::i32(2), i32_type(), unknown_span());
        let three = b.constant(ConstantData::i32(3), i32_type(), unknown_span());
        let s = b.add(one, two, unknown_span()).unwrap();
        let p = b.mul(s, three, unknown_span()).unwrap();
        b.return_value(p, unknown_span()).unwrap();
        let func = b.build();
        let facts = run_constants(&func, None);

        assert!(facts.get(&s).unwrap().value.is_constant());
        assert!(facts.get(&p).unwrap().value.is_constant());
    }

    #[test]
    fn bitwise_folding() {
        // 0xFF & 0x0F = 0x0F
        let mut b = Builder::new("bitfold", &[], u64_type());
        let a = b.constant(ConstantData::u64(0xFF), u64_type(), unknown_span());
        let b_c = b.constant(ConstantData::u64(0x0F), u64_type(), unknown_span());
        let and = b.bit_and(a, b_c, unknown_span()).unwrap();
        b.return_value(and, unknown_span()).unwrap();
        let func = b.build();
        let facts = run_constants(&func, None);

        let and_fact = facts.get(&and).unwrap();
        assert!(and_fact.value.is_constant());
        assert_eq!(
            and_fact.value,
            ConstantLattice::Constant(ConstantData::u64(0x0F))
        );
    }
}
