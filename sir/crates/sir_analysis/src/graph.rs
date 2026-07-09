//! Graph query primitives for traversing SIR functions.
//!
//! Every analysis builds on these functions for traversing the dataflow
//! graph. Key distinction: `dataflow_inputs` returns only true dataflow
//! edges (operands), filtering out Loop containment edges (body, outputs,
//! carried_inputs) that `NodeKind::input_nodes()` includes.

use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use sir_nodes::{Function, NodeArena, NodeKind};
use sir_types::NodeId;

/// Return only the dataflow input NodeIds for a node kind.
///
/// This filters out Loop containment edges: `body`, `outputs`, and
/// `carried_inputs` from `Loop` nodes are NOT dataflow edges.
/// Everything else — operands, conditions, pointers, indices, call
/// arguments — IS a dataflow edge.
pub fn dataflow_inputs(kind: &NodeKind) -> Vec<NodeId> {
    match kind {
        NodeKind::Constant(_) | NodeKind::Parameter { .. } => vec![],

        // Binary: two operands.
        NodeKind::Add { lhs, rhs }
        | NodeKind::Sub { lhs, rhs }
        | NodeKind::Mul { lhs, rhs }
        | NodeKind::Div { lhs, rhs }
        | NodeKind::Rem { lhs, rhs }
        | NodeKind::And { lhs, rhs }
        | NodeKind::Or { lhs, rhs }
        | NodeKind::Xor { lhs, rhs }
        | NodeKind::Shl { lhs, rhs }
        | NodeKind::Shr { lhs, rhs }
        | NodeKind::Rol { lhs, rhs }
        | NodeKind::Ror { lhs, rhs }
        | NodeKind::Eq { lhs, rhs }
        | NodeKind::Ne { lhs, rhs }
        | NodeKind::Lt { lhs, rhs }
        | NodeKind::Le { lhs, rhs }
        | NodeKind::Gt { lhs, rhs }
        | NodeKind::Ge { lhs, rhs }
        | NodeKind::BoolAnd { lhs, rhs }
        | NodeKind::BoolOr { lhs, rhs } => vec![*lhs, *rhs],

        // Unary: one operand.
        NodeKind::Neg { operand }
        | NodeKind::Not { operand }
        | NodeKind::Popcount { operand }
        | NodeKind::LeadingZeros { operand }
        | NodeKind::TrailingZeros { operand }
        | NodeKind::BoolNot { operand } => vec![*operand],

        // Pack: wraps an array value.
        NodeKind::Pack { array } => vec![*array],

        // Select: condition + two value branches.
        NodeKind::Select { cond, true_val, false_val } => vec![*cond, *true_val, *false_val],

        // Memory: pointer + value.
        NodeKind::Load { ptr } => vec![*ptr],
        NodeKind::Store { ptr, value } => vec![*ptr, *value],
        NodeKind::Allocate { count, .. } => vec![*count],
        NodeKind::Deallocate { ptr } => vec![*ptr],

        // Field/array access.
        NodeKind::FieldAccess { base, .. } => vec![*base],
        NodeKind::ArrayAccess { base, index } => vec![*base, *index],

        // Calls.
        NodeKind::Call { callee, args } => {
            let mut v = vec![*callee];
            v.extend(args);
            v
        }
        NodeKind::Intrinsic { args, .. } | NodeKind::ExternalCall { args, .. } => args.clone(),

        // Loop: only the termination condition is a dataflow edge.
        // body, outputs, carried_inputs are containment, not dataflow.
        NodeKind::Loop { termination, carried_inputs, .. } => {
            let mut v = vec![*termination];
            v.extend(carried_inputs);
            v
        }

        // Iterator: collection is the operand.
        NodeKind::Iterator { collection } => vec![*collection],

        // Return: value is the dataflow edge.
        NodeKind::ArrayCmpMask { array, scalar, op: _ } => vec![*array, *scalar],
        NodeKind::Return { value } => vec![*value],
    }
}

/// Return all NodeIds in a function.
pub fn all_node_ids(func: &Function) -> Vec<NodeId> {
    func.arena.nodes().keys().copied().collect()
}

/// Find all users of a node's result by scanning the entire arena.
///
/// O(N) in the number of nodes. Returns nodes that use `node_id` as a
/// **dataflow** input (not containment).
pub fn users(node_id: NodeId, arena: &NodeArena) -> Vec<NodeId> {
    let mut result = Vec::new();
    for (id, node) in arena.nodes() {
        if dataflow_inputs(&node.kind).contains(&node_id) {
            result.push(*id);
        }
    }
    result
}

/// Compute a topological order of nodes in the function.
///
/// Parameters and constants come first, then dataflow order,
/// return node comes last.
pub fn topological_sort(func: &Function) -> Vec<NodeId> {
    let mut order = Vec::new();
    let all_ids: BTreeSet<NodeId> = func.arena.nodes().keys().copied().collect();

    // Start with leaves (no dataflow inputs).
    let leaves: Vec<NodeId> = all_ids
        .iter()
        .filter(|&&id| {
            func.get_node(id)
                .map(|n| dataflow_inputs(&n.kind).is_empty())
                .unwrap_or(false)
        })
        .copied()
        .collect();

    let mut visited: HashSet<NodeId> = HashSet::new();
    let mut return_id: Option<NodeId> = None;

    // DFS from each leaf.
    for leaf in &leaves {
        if visited.contains(leaf) {
            continue;
        }
        let mut stack: Vec<NodeId> = vec![*leaf];
        let mut path: Vec<NodeId> = Vec::new();

        while let Some(current) = stack.pop() {
            if visited.contains(&current) {
                continue;
            }

            if let Some(node) = func.get_node(current) {
                if matches!(node.kind, NodeKind::Return { .. }) {
                    return_id = Some(current);
                    visited.insert(current);
                    continue;
                }
            }

            visited.insert(current);
            path.push(current);

            // Visit dataflow inputs first.
            if let Some(node) = func.get_node(current) {
                for input in dataflow_inputs(&node.kind) {
                    if !visited.contains(&input) {
                        stack.push(input);
                    }
                }
            }
        }
        order.extend(path.into_iter().rev());
    }

    // Add unvisited nodes (may happen with disconnected subgraphs).
    for &id in &all_ids {
        if !visited.contains(&id) {
            visited.insert(id);
            order.push(id);
        }
    }

    // Append return node last.
    if let Some(rid) = return_id {
        if !order.contains(&rid) {
            order.push(rid);
        }
    }

    order
}

/// Check if `from` is reachable from `to` via dataflow edges.
///
/// Uses BFS from `from`, following dataflow inputs backwards.
pub fn reachable(from: NodeId, to: NodeId, arena: &NodeArena) -> bool {
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(from);
    visited.insert(from);

    while let Some(current) = queue.pop_front() {
        if current == to {
            return true;
        }
        if let Some(node) = arena.get(current) {
            for input in dataflow_inputs(&node.kind) {
                if !visited.contains(&input) {
                    visited.insert(input);
                    queue.push_back(input);
                }
            }
        }
    }
    false
}

/// Check whether a node is a leaf (has no dataflow inputs).
pub fn is_leaf(kind: &NodeKind) -> bool {
    dataflow_inputs(kind).is_empty()
}

/// Check whether a node is a Return node.
pub fn is_return(kind: &NodeKind) -> bool {
    matches!(kind, NodeKind::Return { .. })
}

/// Return all leaf nodes (Constant + Parameter) in the function.
pub fn leaf_nodes(func: &Function) -> Vec<NodeId> {
    func.arena
        .iter()
        .filter(|n| is_leaf(&n.kind))
        .map(|n| n.id)
        .collect()
}

/// Return all nodes whose dataflow reaches the given node (transitive inputs).
pub fn transitive_inputs(root: NodeId, arena: &NodeArena) -> HashSet<NodeId> {
    let mut result = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(root);

    while let Some(current) = queue.pop_front() {
        if let Some(node) = arena.get(current) {
            for input in dataflow_inputs(&node.kind) {
                if result.insert(input) {
                    queue.push_back(input);
                }
            }
        }
    }
    result
}

/// Build a map from each node to its immediate dataflow predecessors.
pub fn predecessor_map(func: &Function) -> HashMap<NodeId, Vec<NodeId>> {
    let mut map: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    for (id, node) in func.arena.nodes() {
        map.insert(*id, dataflow_inputs(&node.kind));
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;
    use sir_builder::Builder;
    use sir_types::{ConstantData, Span, Type};

    fn i32_type() -> Type { Type::i32() }
    fn unknown_span() -> Span { Span::unknown() }

    #[test]
    fn dataflow_inputs_constant_is_empty() {
        let c = NodeKind::Constant(ConstantData::i32(42));
        assert!(dataflow_inputs(&c).is_empty());
    }

    #[test]
    fn dataflow_inputs_add_returns_two() {
        let add = NodeKind::Add { lhs: NodeId::new(1), rhs: NodeId::new(2) };
        assert_eq!(dataflow_inputs(&add), vec![NodeId::new(1), NodeId::new(2)]);
    }

    #[test]
    fn dataflow_inputs_loop_filters_body() {
        let loop_node = NodeKind::Loop {
            body: vec![NodeId::new(10), NodeId::new(11)],
            termination: NodeId::new(5),
            outputs: vec![NodeId::new(12)],
            carried_inputs: vec![NodeId::new(3)],
        };
        let inputs = dataflow_inputs(&loop_node);
        // Only termination and carried_inputs are dataflow; body and outputs are not.
        assert!(inputs.contains(&NodeId::new(5)));
        assert!(inputs.contains(&NodeId::new(3)));
        assert!(!inputs.contains(&NodeId::new(10)));
        assert!(!inputs.contains(&NodeId::new(12)));
    }

    #[test]
    fn users_finds_all_consumers() {
        let mut b = Builder::new("test", &[("x", i32_type()), ("y", i32_type())], i32_type());
        let x = b.parameter_index(0).unwrap();
        let y = b.parameter_index(1).unwrap();
        let s1 = b.add(x, y, unknown_span()).unwrap();
        let s2 = b.add(x, s1, unknown_span()).unwrap();
        b.return_value(s2, unknown_span()).unwrap();
        let func = b.build();

        let x_users = users(x, &func.arena);
        // x is used by both add instructions.
        assert_eq!(x_users.len(), 2);
        assert!(x_users.contains(&s1));
        assert!(x_users.contains(&s2));

        let y_users = users(y, &func.arena);
        assert_eq!(y_users.len(), 1);
        assert!(y_users.contains(&s1));
    }

    #[test]
    fn topological_sort_parameters_first_return_last() {
        let mut b = Builder::new("add", &[("a", i32_type()), ("b", i32_type())], i32_type());
        let a = b.parameter_index(0).unwrap();
        let b_param = b.parameter_index(1).unwrap();
        let sum = b.add(a, b_param, unknown_span()).unwrap();
        b.return_value(sum, unknown_span()).unwrap();
        let func = b.build();

        let order = topological_sort(&func);
        // Parameters should be first in the order.
        let a_pos = order.iter().position(|&id| id == a).unwrap();
        let b_pos = order.iter().position(|&id| id == b_param).unwrap();
        // Both params must precede the add.
        let sum_pos = order.iter().position(|&id| id == sum).unwrap();
        assert!(a_pos < sum_pos);
        assert!(b_pos < sum_pos);
    }

    #[test]
    fn is_leaf_for_constant_and_parameter() {
        assert!(is_leaf(&NodeKind::Constant(ConstantData::Unit)));
        assert!(is_leaf(&NodeKind::Parameter { index: 0 }));
        assert!(!is_leaf(&NodeKind::Add { lhs: NodeId::new(0), rhs: NodeId::new(1) }));
    }

    #[test]
    fn is_return_detection() {
        assert!(is_return(&NodeKind::Return { value: NodeId::new(0) }));
        assert!(!is_return(&NodeKind::Add { lhs: NodeId::new(0), rhs: NodeId::new(1) }));
    }

    #[test]
    fn leaf_nodes_finds_constants_and_parameters() {
        let mut b = Builder::new("mixed", &[("p", i32_type())], i32_type());
        let p = b.parameter_index(0).unwrap();
        let c = b.constant(ConstantData::i32(10), i32_type(), unknown_span());
        let sum = b.add(p, c, unknown_span()).unwrap();
        b.return_value(sum, unknown_span()).unwrap();
        let func = b.build();

        let leaves = leaf_nodes(&func);
        assert!(leaves.contains(&p));
        assert!(leaves.contains(&c));
        assert!(!leaves.contains(&sum));
    }

    #[test]
    fn reachable_true_for_direct_edge() {
        let mut b = Builder::new("f", &[("x", i32_type())], i32_type());
        let x = b.parameter_index(0).unwrap();
        let neg = b.neg(x, unknown_span()).unwrap();
        b.return_value(neg, unknown_span()).unwrap();
        let func = b.build();

        assert!(reachable(neg, x, &func.arena));
        assert!(!reachable(x, neg, &func.arena)); // reverse edge
    }

    #[test]
    fn transitive_inputs_collects_subtree() {
        let mut b = Builder::new("f", &[("x", i32_type()), ("y", i32_type())], i32_type());
        let x = b.parameter_index(0).unwrap();
        let y = b.parameter_index(1).unwrap();
        let s1 = b.add(x, y, unknown_span()).unwrap();
        let s2 = b.mul(s1, x, unknown_span()).unwrap();
        b.return_value(s2, unknown_span()).unwrap();
        let func = b.build();

        let inputs = transitive_inputs(s2, &func.arena);
        assert!(inputs.contains(&s1));
        assert!(inputs.contains(&x));
        assert!(inputs.contains(&y));
        assert!(!inputs.contains(&s2)); // not self
    }
}
