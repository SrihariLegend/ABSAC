//! Use-Definition analysis.
//!
//! Builds producer-consumer relationships in a single O(N) pass.
//! For each node, records which nodes define its inputs and which
//! nodes use its result. Also detects dead code.

use sir_nodes::Function;
use sir_types::NodeId;
use std::collections::HashMap;

use crate::facts::UseDefFact;
use crate::graph;

/// Run Use-Def analysis on a function.
///
/// Single pass over the arena. For each node:
/// 1. `definitions` = dataflow inputs of the node
/// 2. For each input, add this node as a user of that input
/// 3. After collecting all users, mark nodes with 0 users as dead
///    (unless they are the return node)
pub fn run_use_def(func: &Function) -> HashMap<NodeId, UseDefFact> {
    let mut facts: HashMap<NodeId, UseDefFact> = HashMap::new();
    let all_ids = graph::all_node_ids(func);

    // Initialize with empty facts.
    for &id in &all_ids {
        facts.insert(
            id,
            UseDefFact {
                definitions: vec![],
                users: vec![],
                is_dead: true,
                use_count: 0,
            },
        );
    }

    // First pass: record definitions and collect users.
    let mut user_map: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    for &id in &all_ids {
        if let Some(node) = func.get_node(id) {
            let defs = graph::dataflow_inputs(&node.kind);
            if let Some(fact) = facts.get_mut(&id) {
                fact.definitions = defs.clone();
            }
            // For each definition (input), this node is a user.
            for def in &defs {
                user_map.entry(*def).or_default().push(id);
            }
        }
    }

    // Second pass: assign users and detect dead code.
    let return_id = func.return_node;
    for &id in &all_ids {
        let users = user_map.remove(&id).unwrap_or_default();
        let use_count = users.len();
        let is_dead = use_count == 0 && Some(id) != return_id;

        if let Some(fact) = facts.get_mut(&id) {
            fact.users = users;
            fact.use_count = use_count;
            fact.is_dead = is_dead;
        }
    }

    facts
}

#[cfg(test)]
mod tests {
    use super::*;
    use sir_builder::Builder;
    use sir_types::{ConstantData, Span, Type};

    fn i32_type() -> Type {
        Type::i32()
    }
    fn unknown_span() -> Span {
        Span::unknown()
    }

    #[test]
    fn empty_function_has_no_use_def() {
        let func = Function::new("empty", Type::Unit);
        let facts = run_use_def(&func);
        assert!(facts.is_empty());
    }

    #[test]
    fn single_constant() {
        let mut b = Builder::new("c", &[], i32_type());
        let c = b.constant(ConstantData::i32(42), i32_type(), unknown_span());
        b.return_value(c, unknown_span()).unwrap();
        let func = b.build();
        let facts = run_use_def(&func);

        let c_fact = facts.get(&c).unwrap();
        assert!(c_fact.definitions.is_empty()); // constant has no inputs
        assert!(!c_fact.is_dead); // used by return
        assert_eq!(c_fact.use_count, 1);
    }

    #[test]
    fn chained_arithmetic() {
        let mut b = Builder::new("chain", &[("x", i32_type()), ("y", i32_type())], i32_type());
        let x = b.parameter_index(0).unwrap();
        let y = b.parameter_index(1).unwrap();
        let s1 = b.add(x, y, unknown_span()).unwrap();
        let s2 = b.mul(s1, x, unknown_span()).unwrap();
        b.return_value(s2, unknown_span()).unwrap();
        let func = b.build();
        let facts = run_use_def(&func);

        // x is used by s1 and s2: 2 users.
        let x_fact = facts.get(&x).unwrap();
        assert_eq!(x_fact.use_count, 2);
        assert!(x_fact.users.contains(&s1));
        assert!(x_fact.users.contains(&s2));

        // y is used by s1 only: 1 user.
        let y_fact = facts.get(&y).unwrap();
        assert_eq!(y_fact.use_count, 1);

        // s1 definitions are x and y.
        let s1_fact = facts.get(&s1).unwrap();
        assert_eq!(s1_fact.definitions.len(), 2);
        assert!(s1_fact.definitions.contains(&x));
        assert!(s1_fact.definitions.contains(&y));
        assert_eq!(s1_fact.use_count, 1); // used by s2

        // s2 definitions are s1 and x.
        let s2_fact = facts.get(&s2).unwrap();
        assert_eq!(s2_fact.definitions.len(), 2);
        assert!(!s2_fact.is_dead); // used by return
    }

    #[test]
    fn dead_node_detection() {
        let mut b = Builder::new("dead", &[("x", i32_type())], i32_type());
        let x = b.parameter_index(0).unwrap();
        let _dead_val = b.add(x, x, unknown_span()).unwrap(); // computed but never used
        let alive = b.neg(x, unknown_span()).unwrap();
        b.return_value(alive, unknown_span()).unwrap();
        let func = b.build();
        let facts = run_use_def(&func);

        let dead_fact = facts.get(&_dead_val).unwrap();
        assert!(dead_fact.is_dead);
        assert_eq!(dead_fact.use_count, 0);

        let alive_fact = facts.get(&alive).unwrap();
        assert!(!alive_fact.is_dead);
    }

    #[test]
    fn return_node_not_dead() {
        let mut b = Builder::new("f", &[("x", i32_type())], i32_type());
        let x = b.parameter_index(0).unwrap();
        b.return_value(x, unknown_span()).unwrap();
        let func = b.build();
        let facts = run_use_def(&func);

        let ret_id = func.return_node.unwrap();
        let ret_fact = facts.get(&ret_id).unwrap();
        assert!(!ret_fact.is_dead); // return is never dead
    }

    #[test]
    fn diamond_pattern() {
        // x -> s1 = x+x, s2 = x+x, s3 = s1+s2
        let mut b = Builder::new("diamond", &[("x", i32_type())], i32_type());
        let x = b.parameter_index(0).unwrap();
        let s1 = b.add(x, x, unknown_span()).unwrap();
        let s2 = b.add(x, x, unknown_span()).unwrap();
        let s3 = b.add(s1, s2, unknown_span()).unwrap();
        b.return_value(s3, unknown_span()).unwrap();
        let func = b.build();
        let facts = run_use_def(&func);

        let x_fact = facts.get(&x).unwrap();
        // x appears twice in each of s1 and s2 (lhs+rhs), so 4 total references
        assert_eq!(x_fact.use_count, 4);
    }

    #[test]
    fn select_use_counts() {
        let mut b = Builder::new(
            "sel",
            &[("cond", Type::Bool), ("t", i32_type()), ("f", i32_type())],
            i32_type(),
        );
        let cond = b.parameter_index(0).unwrap();
        let t_val = b.parameter_index(1).unwrap();
        let f_val = b.parameter_index(2).unwrap();
        let sel = b.select(cond, t_val, f_val, unknown_span()).unwrap();
        b.return_value(sel, unknown_span()).unwrap();
        let func = b.build();
        let facts = run_use_def(&func);

        // Each parameter should have 1 user (the select).
        assert_eq!(facts.get(&cond).unwrap().use_count, 1);
        assert_eq!(facts.get(&t_val).unwrap().use_count, 1);
        assert_eq!(facts.get(&f_val).unwrap().use_count, 1);
    }

    #[test]
    fn memory_operations_use_def() {
        let mut b = Builder::new("mem", &[], i32_type());
        let count = b.constant(ConstantData::u64(1), Type::u64(), unknown_span());
        let ptr = b.allocate(i32_type(), count, unknown_span()).unwrap();
        let val = b.constant(ConstantData::i32(7), i32_type(), unknown_span());
        b.store(ptr, val, unknown_span()).unwrap();
        let loaded = b.load(ptr, i32_type(), unknown_span()).unwrap();
        b.return_value(loaded, unknown_span()).unwrap();
        let func = b.build();
        let facts = run_use_def(&func);

        // ptr is used by store and load.
        let ptr_fact = facts.get(&ptr).unwrap();
        assert_eq!(ptr_fact.use_count, 2);
    }
}
