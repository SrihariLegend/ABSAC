//! Graph query primitives for traversing SIR functions.
//!
//! Every analysis builds on these functions for traversing the dataflow
//! graph. Key distinction: `dataflow_inputs` returns only true dataflow
//! edges (operands), filtering out Loop containment edges (body, outputs,
//! carried_inputs) that `NodeKind::input_nodes()` includes.

use sir_nodes::{NodeArena, NodeKind};
use sir_types::NodeId;
use std::collections::{HashSet, VecDeque};

/// Return only the dataflow input NodeIds for a node kind.
///
/// This filters out Loop containment edges: `body`, `outputs`, and
/// `carried_inputs` from `Loop` nodes are NOT dataflow edges.
/// Everything else — operands, conditions, pointers, indices, call
/// arguments — IS a dataflow edge.
pub fn dataflow_inputs(kind: &NodeKind) -> Vec<NodeId> {
    match kind {
        NodeKind::Constant(_) | NodeKind::Parameter { .. } | NodeKind::Global { .. } => vec![],

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
        | NodeKind::BitScanReverse { operand }
        | NodeKind::BoolNot { operand } => vec![*operand],

        // Pack: wraps an array value.
        NodeKind::Pack { array } => vec![*array],

        // Convert: wraps a single operand.
        NodeKind::Convert { operand, .. } => vec![*operand],

        // Select: condition + two value branches.
        NodeKind::Select {
            cond,
            true_val,
            false_val,
        } => vec![*cond, *true_val, *false_val],

        // Memory: pointer + value.
        NodeKind::Load { ptr } => vec![*ptr],
        NodeKind::Store { ptr, value } => vec![*ptr, *value],
        NodeKind::Allocate { count, .. } => vec![*count],
        NodeKind::Deallocate { ptr } => vec![*ptr],

        // Field/array access.
        NodeKind::FieldAccess { base, .. } => vec![*base],
        NodeKind::ArrayAccess { base, index } => vec![*base, *index],
        NodeKind::PointerOffset { base, index } => vec![*base, *index],
        NodeKind::TupleExtract { tuple, .. } => vec![*tuple],
        NodeKind::Tuple { elements } => elements.clone(),

        // Calls.
        NodeKind::Call { callee, args } => {
            let mut v = vec![*callee];
            v.extend(args);
            v
        }
        NodeKind::Intrinsic { args, .. } | NodeKind::ExternalCall { args, .. } => args.clone(),

        // Loop: only the termination condition is a dataflow edge.
        // body, outputs, carried_inputs are containment, not dataflow.
        NodeKind::Loop {
            termination,
            carried_inputs,
            ..
        } => {
            let mut v = vec![*termination];
            v.extend(carried_inputs);
            v
        }

        // Iterator: collection is the operand.
        NodeKind::Iterator { collection } => vec![*collection],

        // Return: value is the dataflow edge.
        NodeKind::ArrayCmpMask {
            array,
            scalar,
            op: _,
        } => vec![*array, *scalar],
        NodeKind::Return { value } => vec![*value],
    }
}
/// Find all users of a node's result by scanning the entire arena.
///
/// O(N) in the number of nodes. Returns nodes that use `node_id` as a
/// **dataflow** input (not containment).
pub fn users(node_id: NodeId, arena: &NodeArena) -> Vec<NodeId> {
    let mut result = Vec::new();
    for node in arena.iter() {
        if dataflow_inputs(&node.kind).contains(&node_id) {
            result.push(node.id);
        }
    }
    result
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
