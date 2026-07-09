//! Value numbering analysis.
//!
//! Hash-based global value numbering. Assigns each node a congruence
//! class based on its operation and the value numbers of its inputs.
//! Nodes in the same class compute the same value.

use std::collections::HashMap;
use sir_nodes::{Function, NodeKind};
use sir_types::NodeId;

use crate::facts::ValueNumberFact;
use crate::graph;

/// Run value numbering on a function.
///
/// Processes nodes in topological order, assigning each a hash based
/// on its NodeKind variant and the value numbers of its inputs.
/// Nodes with identical hashes are in the same congruence class.
pub fn run_value_numbering(func: &Function) -> HashMap<NodeId, ValueNumberFact> {
    let order = graph::topological_sort(func);
    let mut vn_map: HashMap<NodeId, u64> = HashMap::new();
    let mut class_map: HashMap<u64, Vec<NodeId>> = HashMap::new();

    for &id in &order {
        let node = match func.get_node(id) {
            Some(n) => n,
            None => continue,
        };

        let hash = compute_hash(&node.kind, &vn_map);
        vn_map.insert(id, hash);
        class_map.entry(hash).or_default().push(id);
    }

    // Build facts with canonical representatives.
    let mut facts = HashMap::new();
    for (&id, &hash) in &vn_map {
        let canonical = class_map
            .get(&hash)
            .and_then(|nodes| nodes.first())
            .copied()
            .unwrap_or(id);

        facts.insert(
            id,
            ValueNumberFact {
                congruence_class: hash,
                canonical,
            },
        );
    }

    facts
}

/// Compute a deterministic hash for a node kind.
///
/// Uses FNV-1a, which is deterministic across Rust versions and process runs.
/// Hashes the variant discriminant, value numbers of all dataflow inputs
/// (in order), and additional immediate data (field names, constant values).
fn compute_hash(
    kind: &NodeKind,
    vn_map: &HashMap<NodeId, u64>,
) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;

    // Mix a u64 value.
    fn mix(h: &mut u64, v: u64) {
        *h ^= v;
        *h = h.wrapping_mul(0x100000001b3);
    }

    // Mix bytes.
    fn mix_bytes(h: &mut u64, bytes: &[u8]) {
        for &b in bytes {
            mix(h, b as u64);
        }
    }

    // Hash the variant discriminant.
    mix(&mut h, kind_variant_tag(kind) as u64);

    // Hash the value numbers of dataflow inputs (in order).
    for input in graph::dataflow_inputs(kind) {
        mix(&mut h, vn_map.get(&input).copied().unwrap_or(0));
    }

    // Hash additional immediate data.
    match kind {
        NodeKind::Constant(data) => mix_bytes(&mut h, format!("{data}").as_bytes()),
        NodeKind::Parameter { index } => mix(&mut h, *index as u64),
        NodeKind::FieldAccess { field, .. } => mix_bytes(&mut h, field.as_bytes()),
        NodeKind::Intrinsic { name, .. } | NodeKind::ExternalCall { name, .. } => {
            mix_bytes(&mut h, name.as_bytes());
        }
        NodeKind::Allocate { ty, .. } => mix_bytes(&mut h, ty.type_name().as_bytes()),
        _ => {}
    }

    h
}

/// Return a numeric tag for the NodeKind variant (for hashing).
pub(crate) fn kind_variant_tag(kind: &NodeKind) -> u32 {
    match kind {
        NodeKind::Constant(_) => 1,
        NodeKind::Parameter { .. } => 2,
        NodeKind::Add { .. } => 3,
        NodeKind::Sub { .. } => 4,
        NodeKind::Mul { .. } => 5,
        NodeKind::Div { .. } => 6,
        NodeKind::Rem { .. } => 7,
        NodeKind::Neg { .. } => 8,
        NodeKind::And { .. } => 9,
        NodeKind::Or { .. } => 10,
        NodeKind::Xor { .. } => 11,
        NodeKind::Shl { .. } => 12,
        NodeKind::Shr { .. } => 13,
        NodeKind::Rol { .. } => 14,
        NodeKind::Ror { .. } => 15,
        NodeKind::Not { .. } => 16,
        NodeKind::Popcount { .. } => 17,
        NodeKind::LeadingZeros { .. } => 18,
        NodeKind::TrailingZeros { .. } => 19,
        NodeKind::Eq { .. } => 20,
        NodeKind::Ne { .. } => 21,
        NodeKind::Lt { .. } => 22,
        NodeKind::Le { .. } => 23,
        NodeKind::Gt { .. } => 24,
        NodeKind::Ge { .. } => 25,
        NodeKind::BoolAnd { .. } => 26,
        NodeKind::BoolOr { .. } => 27,
        NodeKind::BoolNot { .. } => 28,
        NodeKind::Select { .. } => 29,
        NodeKind::Load { .. } => 30,
        NodeKind::Store { .. } => 31,
        NodeKind::Allocate { .. } => 32,
        NodeKind::Deallocate { .. } => 33,
        NodeKind::FieldAccess { .. } => 34,
        NodeKind::ArrayAccess { .. } => 35,
        NodeKind::Call { .. } => 36,
        NodeKind::Intrinsic { .. } => 37,
        NodeKind::ExternalCall { .. } => 38,
        NodeKind::Loop { .. } => 39,
        NodeKind::Iterator { .. } => 40,
        NodeKind::Pack { .. } => 42,
        NodeKind::ArrayCmpMask { .. } => 43,
        NodeKind::Return { .. } => 41,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sir_builder::Builder;
    use sir_types::{ConstantData, Span, Type};

    fn i32_type() -> Type { Type::i32() }
    fn unknown_span() -> Span { Span::unknown() }

    #[test]
    fn identical_add_same_class() {
        let mut b = Builder::new("f", &[("x", i32_type()), ("y", i32_type())], i32_type());
        let x = b.parameter_index(0).unwrap();
        let y = b.parameter_index(1).unwrap();
        let s1 = b.add(x, y, unknown_span()).unwrap();
        let s2 = b.add(x, y, unknown_span()).unwrap();
        b.return_value(s2, unknown_span()).unwrap();
        let func = b.build();
        let facts = run_value_numbering(&func);

        // s1 and s2 should be in the same congruence class.
        let s1_fact = facts.get(&s1).unwrap();
        let s2_fact = facts.get(&s2).unwrap();
        assert_eq!(s1_fact.congruence_class, s2_fact.congruence_class);
    }

    #[test]
    fn different_ops_different_class() {
        let mut b = Builder::new("f", &[("x", i32_type()), ("y", i32_type())], i32_type());
        let x = b.parameter_index(0).unwrap();
        let y = b.parameter_index(1).unwrap();
        let s = b.add(x, y, unknown_span()).unwrap();
        let p = b.mul(x, y, unknown_span()).unwrap();
        // Use select to combine them into a single return value.
        let cond = b.constant(ConstantData::Bool(true), Type::Bool, unknown_span());
        let sel = b.select(cond, s, p, unknown_span()).unwrap();
        b.return_value(sel, unknown_span()).unwrap();
        let func = b.build();
        let facts = run_value_numbering(&func);

        // Add and Mul should be in different classes.
        assert_ne!(
            facts.get(&s).unwrap().congruence_class,
            facts.get(&p).unwrap().congruence_class
        );
    }

    #[test]
    fn same_constant_same_class() {
        let mut b = Builder::new("f", &[], i32_type());
        let c1 = b.constant(ConstantData::i32(42), i32_type(), unknown_span());
        let c2 = b.constant(ConstantData::i32(42), i32_type(), unknown_span());
        let s = b.add(c1, c2, unknown_span()).unwrap();
        b.return_value(s, unknown_span()).unwrap();
        let func = b.build();
        let facts = run_value_numbering(&func);

        // Same constant value → same class.
        assert_eq!(
            facts.get(&c1).unwrap().congruence_class,
            facts.get(&c2).unwrap().congruence_class
        );
    }

    #[test]
    fn different_constant_different_class() {
        let mut b = Builder::new("f", &[], i32_type());
        let c1 = b.constant(ConstantData::i32(42), i32_type(), unknown_span());
        let c2 = b.constant(ConstantData::i32(17), i32_type(), unknown_span());
        b.return_value(c1, unknown_span()).unwrap();
        let func = b.build();
        let facts = run_value_numbering(&func);

        // Different constant values → different classes.
        assert_ne!(
            facts.get(&c1).unwrap().congruence_class,
            facts.get(&c2).unwrap().congruence_class
        );
    }

    #[test]
    fn canonical_is_smallest_node_id() {
        let mut b = Builder::new("f", &[], i32_type());
        let c1 = b.constant(ConstantData::i32(42), i32_type(), unknown_span());
        let c2 = b.constant(ConstantData::i32(42), i32_type(), unknown_span());
        b.return_value(c1, unknown_span()).unwrap();
        let func = b.build();
        let facts = run_value_numbering(&func);

        // Both constants in same class — canonical should be the smaller NodeId.
        let c1_fact = facts.get(&c1).unwrap();
        let c2_fact = facts.get(&c2).unwrap();
        assert_eq!(c1_fact.canonical, c2_fact.canonical);
        assert_eq!(c1_fact.canonical, c1.min(c2));
    }
}
