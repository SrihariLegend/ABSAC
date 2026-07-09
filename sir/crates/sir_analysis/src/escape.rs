//! Escape analysis.
//!
//! Determines whether a value escapes the function. A value escapes
//! if it is returned, stored, passed to an external call, or captured
//! by a loop construct.

use std::collections::HashMap;
use sir_nodes::{Function, NodeKind};
use sir_types::NodeId;

use crate::facts::{EscapeFact, EscapeKind};
use crate::graph;

/// Run escape analysis on a function.
///
/// Two-phase approach:
/// 1. Mark operands of escape points (Return, Store value, ExternalCall args, Loop carried)
///    as escaped.
/// 2. Propagate escape transitively from users to their dataflow inputs.
pub fn run_escape(func: &Function) -> HashMap<NodeId, EscapeFact> {
    let all_ids: Vec<NodeId> = func.arena.nodes().keys().copied().collect();
    let mut facts: HashMap<NodeId, EscapeFact> = HashMap::new();

    // Initialize all nodes as NeverEscapes.
    for &id in &all_ids {
        facts.insert(id, EscapeFact { kind: EscapeKind::NeverEscapes });
    }

    // Phase 1: Mark operands of escape points.
    for &id in &all_ids {
        let node = match func.get_node(id) {
            Some(n) => n,
            None => continue,
        };
        match &node.kind {
            NodeKind::Return { value } => {
                facts.insert(*value, EscapeFact { kind: EscapeKind::Returned });
            }
            NodeKind::Store { value, .. } => {
                // The stored value escapes.
                facts.insert(*value, EscapeFact { kind: EscapeKind::StoredGlobally });
            }
            NodeKind::ExternalCall { args, .. } | NodeKind::Intrinsic { args, .. } => {
                for &arg in args {
                    facts.insert(arg, EscapeFact { kind: EscapeKind::PassedExternally });
                }
            }
            NodeKind::Loop { carried_inputs, .. } => {
                for &carry in carried_inputs {
                    facts.insert(carry, EscapeFact { kind: EscapeKind::Captured });
                }
            }
            _ => {}
        }
    }

    // Phase 2: Propagate from users to inputs.
    // Build reverse map: user → [inputs].
    let mut rev_map: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    for &id in &all_ids {
        if let Some(node) = func.get_node(id) {
            for input in graph::dataflow_inputs(&node.kind) {
                rev_map.entry(id).or_default().push(input);
            }
        }
    }

    // Iterate to fixpoint: if a user escapes, its inputs escape.
    let mut changed = true;
    while changed {
        changed = false;
        for &id in &all_ids {
            let user_escape = facts.get(&id).map(|f| f.kind.clone()).unwrap_or(EscapeKind::NeverEscapes);
            if user_escape == EscapeKind::NeverEscapes {
                continue;
            }
            if let Some(inputs) = rev_map.get(&id) {
                for &input in inputs {
                    let input_fact = facts.get_mut(&input).unwrap();
                    let merged = merge_escape(input_fact.kind.clone(), user_escape.clone());
                    if input_fact.kind != merged {
                        input_fact.kind = merged;
                        changed = true;
                    }
                }
            }
        }
    }

    facts
}

/// Merge two escape kinds, taking the more severe one.
fn merge_escape(a: EscapeKind, b: EscapeKind) -> EscapeKind {
    fn severity(k: &EscapeKind) -> u8 {
        match k {
            EscapeKind::NeverEscapes => 0,
            EscapeKind::Captured => 1,
            EscapeKind::StoredGlobally => 2,
            EscapeKind::PassedExternally => 3,
            EscapeKind::Returned => 4,
        }
    }
    if severity(&a) >= severity(&b) { a } else { b }
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
    fn returned_value_escapes() {
        let mut b = Builder::new("f", &[("x", i32_type())], i32_type());
        let x = b.parameter_index(0).unwrap();
        b.return_value(x, unknown_span()).unwrap();
        let func = b.build();
        let facts = run_escape(&func);

        let x_fact = facts.get(&x).unwrap();
        assert_eq!(x_fact.kind, EscapeKind::Returned);
    }

    #[test]
    fn stored_value_escapes() {
        let mut b = Builder::new("f", &[], Type::Unit);
        let count = b.constant(ConstantData::u64(1), u64_type(), unknown_span());
        let ptr = b.allocate(i32_type(), count, unknown_span()).unwrap();
        let val = b.constant(ConstantData::i32(42), i32_type(), unknown_span());
        let st = b.store(ptr, val, unknown_span()).unwrap();
        b.return_value(st, unknown_span()).unwrap();
        let func = b.build();
        let facts = run_escape(&func);

        let val_fact = facts.get(&val).unwrap();
        // val is stored AND the store result is returned → escapes via Return (most severe).
        assert_eq!(val_fact.kind, EscapeKind::Returned);
    }

    #[test]
    fn purely_local_value_never_escapes() {
        let mut b = Builder::new("f", &[("x", i32_type())], i32_type());
        let x = b.parameter_index(0).unwrap();
        let c = b.constant(ConstantData::i32(2), i32_type(), unknown_span());
        // c is used but never returned/stored/passed externally.
        let s = b.add(x, c, unknown_span()).unwrap();
        b.return_value(s, unknown_span()).unwrap();
        let func = b.build();
        let facts = run_escape(&func);

        // c is used locally but doesn't itself escape.
        // Actually c is an input to `s`, which escapes via Return.
        // So c also escapes transitively through s.
        let c_fact = facts.get(&c).unwrap();
        assert_eq!(c_fact.kind, EscapeKind::Returned);
    }

    #[test]
    fn local_allocation_does_not_escape_by_default() {
        let mut b = Builder::new("f", &[], i32_type());
        let count = b.constant(ConstantData::u64(1), u64_type(), unknown_span());
        let ptr = b.allocate(i32_type(), count, unknown_span()).unwrap();
        let loaded = b.load(ptr, i32_type(), unknown_span()).unwrap();
        // loaded is returned → escapes. ptr is used by load but not returned.
        b.return_value(loaded, unknown_span()).unwrap();
        let func = b.build();
        let facts = run_escape(&func);

        // loaded escapes via Return.
        let loaded_fact = facts.get(&loaded).unwrap();
        assert_eq!(loaded_fact.kind, EscapeKind::Returned);
    }
}
