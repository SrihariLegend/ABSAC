//! Purity analysis.
//!
//! Bottom-up propagation: a node's subgraph is pure if the node itself
//! has no effects AND all its dataflow inputs are pure.

use std::collections::HashMap;
use sir_nodes::Function;
use sir_types::NodeId;

use crate::facts::{PurityFact, PurityLevel};
use crate::graph;

/// Run purity analysis on a function.
///
/// Bottom-up: compute `subgraph_is_pure` for each node. A node is
/// "subgraph pure" iff it has no effects AND all dataflow inputs
/// are also subgraph pure.
///
/// Uses topological order to process leaves first.
pub fn run_purity(func: &Function) -> HashMap<NodeId, PurityFact> {
    let order = graph::topological_sort(func);
    let mut facts: HashMap<NodeId, PurityFact> = HashMap::new();

    for &id in &order {
        let node = match func.get_node(id) {
            Some(n) => n,
            None => continue,
        };

        let node_pure = node.effects.is_pure();

        // Determine purity level from effects.
        let purity = if node_pure {
            PurityLevel::Pure
        } else if node.effects.contains(sir_types::Effects::IO) {
            PurityLevel::IO
        } else if node.effects.contains(sir_types::Effects::ATOMIC) {
            PurityLevel::Atomic
        } else if node.effects.contains(sir_types::Effects::ALLOCATE) {
            PurityLevel::Allocates
        } else if node.effects.contains(sir_types::Effects::WRITE_MEMORY) {
            PurityLevel::WritesMemory
        } else if node.effects.contains(sir_types::Effects::READ_MEMORY) {
            PurityLevel::ReadsMemory
        } else {
            PurityLevel::Unknown
        };

        // Subgraph purity: this node + all inputs must be pure.
        let inputs = graph::dataflow_inputs(&node.kind);
        let all_inputs_pure = inputs.iter().all(|iid| {
            facts
                .get(iid)
                .map(|f| f.subgraph_is_pure)
                .unwrap_or(false)
        });
        let subgraph_is_pure = node_pure && all_inputs_pure;

        facts.insert(
            id,
            PurityFact {
                purity,
                subgraph_is_pure,
            },
        );
    }

    facts
}

#[cfg(test)]
mod tests {
    use super::*;
    use sir_builder::Builder;
    use sir_types::{ConstantData, Span, Type};

    fn i32_type() -> Type { Type::i32() }
    fn u64_type() -> Type { Type::u64() }
    fn unknown_span() -> Span { Span::unknown() }

    #[test]
    fn pure_arithmetic_is_pure() {
        let mut b = Builder::new("f", &[("a", i32_type()), ("b", i32_type())], i32_type());
        let a = b.parameter_index(0).unwrap();
        let b_param = b.parameter_index(1).unwrap();
        let sum = b.add(a, b_param, unknown_span()).unwrap();
        b.return_value(sum, unknown_span()).unwrap();
        let func = b.build();
        let facts = run_purity(&func);

        let sum_fact = facts.get(&sum).unwrap();
        assert_eq!(sum_fact.purity, PurityLevel::Pure);
        assert!(sum_fact.subgraph_is_pure);
    }

    #[test]
    fn load_is_not_pure() {
        let mut b = Builder::new("mem", &[], i32_type());
        let count = b.constant(ConstantData::u64(1), u64_type(), unknown_span());
        let ptr = b.allocate(i32_type(), count, unknown_span()).unwrap();
        let loaded = b.load(ptr, i32_type(), unknown_span()).unwrap();
        b.return_value(loaded, unknown_span()).unwrap();
        let func = b.build();
        let facts = run_purity(&func);

        let load_fact = facts.get(&loaded).unwrap();
        assert_eq!(load_fact.purity, PurityLevel::ReadsMemory);
        assert!(!load_fact.subgraph_is_pure);
    }

    #[test]
    fn transitive_impurity() {
        let mut b = Builder::new("trans", &[], i32_type());
        let count = b.constant(ConstantData::u64(1), u64_type(), unknown_span());
        let ptr = b.allocate(i32_type(), count, unknown_span()).unwrap();
        let loaded = b.load(ptr, i32_type(), unknown_span()).unwrap();
        let one = b.constant(ConstantData::i32(1), i32_type(), unknown_span());
        // add(pure_const, load_result) — add itself is pure but depends on impure load.
        let sum = b.add(one, loaded, unknown_span()).unwrap();
        b.return_value(sum, unknown_span()).unwrap();
        let func = b.build();
        let facts = run_purity(&func);

        let sum_fact = facts.get(&sum).unwrap();
        assert_eq!(sum_fact.purity, PurityLevel::Pure); // the add node itself is pure
        assert!(!sum_fact.subgraph_is_pure); // but its subgraph is impure (depends on load)
    }

    #[test]
    fn store_is_not_pure() {
        let mut b = Builder::new("store", &[], Type::Unit);
        let count = b.constant(ConstantData::u64(1), u64_type(), unknown_span());
        let ptr = b.allocate(i32_type(), count, unknown_span()).unwrap();
        let val = b.constant(ConstantData::i32(10), i32_type(), unknown_span());
        let stored = b.store(ptr, val, unknown_span()).unwrap();
        b.return_value(stored, unknown_span()).unwrap();
        let func = b.build();
        let facts = run_purity(&func);

        let s_fact = facts.get(&stored).unwrap();
        assert_eq!(s_fact.purity, PurityLevel::WritesMemory);
        assert!(!s_fact.subgraph_is_pure);
    }

    #[test]
    fn pure_function_all_pure() {
        let mut b = Builder::new("pure_func", &[("x", i32_type())], i32_type());
        let x = b.parameter_index(0).unwrap();
        let c = b.constant(ConstantData::i32(2), i32_type(), unknown_span());
        let result = b.mul(x, c, unknown_span()).unwrap();
        b.return_value(result, unknown_span()).unwrap();
        let func = b.build();
        let facts = run_purity(&func);

        // Parameters are pure (no effects), constants are pure.
        for (_, fact) in &facts {
            assert!(fact.subgraph_is_pure);
        }
    }

    #[test]
    fn select_with_impure_branch() {
        let mut b = Builder::new("sel_impure", &[("cond", Type::Bool), ("x", i32_type())], i32_type());
        let cond = b.parameter_index(0).unwrap();
        let x = b.parameter_index(1).unwrap();

        // Build an impure branch: allocate + load.
        let count = b.constant(ConstantData::u64(1), u64_type(), unknown_span());
        let ptr = b.allocate(i32_type(), count, unknown_span()).unwrap();
        let loaded = b.load(ptr, i32_type(), unknown_span()).unwrap();

        let sel = b.select(cond, x, loaded, unknown_span()).unwrap();
        b.return_value(sel, unknown_span()).unwrap();
        let func = b.build();
        let facts = run_purity(&func);

        // The select itself is pure, but its subgraph is impure.
        let sel_fact = facts.get(&sel).unwrap();
        assert_eq!(sel_fact.purity, PurityLevel::Pure);
        assert!(!sel_fact.subgraph_is_pure);
    }
}
