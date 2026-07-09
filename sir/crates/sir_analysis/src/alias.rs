//! Alias analysis.
//!
//! Allocation-site-based pointer analysis. Tracks which Allocate nodes
//! each pointer may reference, propagating through field access, array
//! indexing, and conditional select.

use std::collections::{BTreeSet, HashMap};
use sir_nodes::{Function, NodeKind};
use sir_types::NodeId;

use crate::facts::{AliasFact, AliasKind};
use crate::graph;

/// Run alias analysis on a function.
///
/// Simple flow-insensitive, field-insensitive allocation-site analysis.
/// Each node gets an `AliasFact` tracking which `Allocate` nodes it
/// may point to.
pub fn run_alias(func: &Function) -> HashMap<NodeId, AliasFact> {
    let order = graph::topological_sort(func);
    let mut facts: HashMap<NodeId, AliasFact> = HashMap::new();

    for &id in &order {
        let node = match func.get_node(id) {
            Some(n) => n,
            None => continue,
        };

        let alias = compute_alias(node, &facts);
        facts.insert(id, alias);
    }

    facts
}

/// Compute alias fact for a single node.
fn compute_alias(
    node: &sir_nodes::Node,
    facts: &HashMap<NodeId, AliasFact>,
) -> AliasFact {
    match &node.kind {
        // Allocate creates a fresh, unique allocation site.
        NodeKind::Allocate { .. } => {
            let mut set = BTreeSet::new();
            set.insert(node.id);
            AliasFact {
                kind: AliasKind::MustAlias,
                allocation_site: Some(node.id),
            }
        }

        // Load: pointee targets are whatever the pointer may alias.
        NodeKind::Load { ptr } => facts.get(ptr).cloned().unwrap_or_else(|| unknown_alias()),

        // FieldAccess propagates the base pointer's aliases.
        NodeKind::FieldAccess { base, .. } => {
            facts.get(base).cloned().unwrap_or_else(|| unknown_alias())
        }

        // ArrayAccess propagates the base's aliases.
        NodeKind::ArrayAccess { base, .. } => {
            facts.get(base).cloned().unwrap_or_else(|| unknown_alias())
        }

        // Select: union of both branches.
        NodeKind::Select {
            true_val, false_val, ..
        } => {
            let t = facts.get(true_val);
            let f = facts.get(false_val);
            union_aliases(t, f)
        }

        // Constant pointers (null, etc.) alias nothing.
        NodeKind::Constant(_) => AliasFact {
            kind: AliasKind::NoAlias,
            allocation_site: None,
        },

        // Parameters and other values: unknown.
        _ => unknown_alias(),
    }
}

/// Create an unknown alias fact.
fn unknown_alias() -> AliasFact {
    AliasFact {
        kind: AliasKind::Unknown,
        allocation_site: None,
    }
}

/// Union two optional alias facts.
fn union_aliases(a: Option<&AliasFact>, b: Option<&AliasFact>) -> AliasFact {
    match (a, b) {
        (Some(av), Some(bv)) => {
            let a_site = av.allocation_site;
            let b_site = bv.allocation_site;
            if a_site == b_site && a_site.is_some() {
                AliasFact {
                    kind: AliasKind::MustAlias,
                    allocation_site: a_site,
                }
            } else {
                let mut set = BTreeSet::new();
                if let Some(s) = a_site {
                    set.insert(s);
                }
                if let Some(s) = b_site {
                    set.insert(s);
                }
                if set.is_empty() {
                    AliasFact {
                        kind: AliasKind::NoAlias,
                        allocation_site: None,
                    }
                } else {
                    AliasFact {
                        kind: AliasKind::MayAlias(set),
                        allocation_site: None,
                    }
                }
            }
        }
        (Some(av), None) => av.clone(),
        (None, Some(bv)) => bv.clone(),
        (None, None) => unknown_alias(),
    }
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
    fn allocate_aliases_itself() {
        let mut b = Builder::new("f", &[], i32_type());
        let count = b.constant(ConstantData::u64(1), u64_type(), unknown_span());
        let ptr = b.allocate(i32_type(), count, unknown_span()).unwrap();
        let loaded = b.load(ptr, i32_type(), unknown_span()).unwrap();
        b.return_value(loaded, unknown_span()).unwrap();
        let func = b.build();
        let facts = run_alias(&func);

        let ptr_fact = facts.get(&ptr).unwrap();
        assert_eq!(ptr_fact.allocation_site, Some(ptr));
        assert_eq!(ptr_fact.kind, AliasKind::MustAlias);
    }

    #[test]
    fn two_allocations_no_alias() {
        let mut b = Builder::new("f", &[], Type::Unit);
        let count = b.constant(ConstantData::u64(1), u64_type(), unknown_span());
        let ptr1 = b.allocate(i32_type(), count, unknown_span()).unwrap();
        let ptr2 = b.allocate(i32_type(), count, unknown_span()).unwrap();
        // Store to ptr1, load from ptr2.
        let val = b.constant(ConstantData::i32(5), i32_type(), unknown_span());
        b.store(ptr1, val, unknown_span()).unwrap();
        let loaded = b.load(ptr2, i32_type(), unknown_span()).unwrap();
        b.return_value(loaded, unknown_span()).unwrap();
        let func = b.build();
        let facts = run_alias(&func);

        // ptr1 and ptr2 are different allocation sites.
        let f1 = facts.get(&ptr1).unwrap();
        let f2 = facts.get(&ptr2).unwrap();
        assert_ne!(f1.allocation_site, f2.allocation_site);
    }

    #[test]
    fn select_union_aliases() {
        let mut b = Builder::new("f", &[("cond", Type::Bool)], i32_type());
        let cond = b.parameter_index(0).unwrap();
        let count = b.constant(ConstantData::u64(1), u64_type(), unknown_span());
        let ptr1 = b.allocate(i32_type(), count, unknown_span()).unwrap();
        let ptr2 = b.allocate(i32_type(), count, unknown_span()).unwrap();
        let sel = b.select(cond, ptr1, ptr2, unknown_span()).unwrap();
        b.return_value(sel, unknown_span()).unwrap();
        let func = b.build();
        let facts = run_alias(&func);

        let sel_fact = facts.get(&sel).unwrap();
        // Select of two different allocation sites → MayAlias.
        assert!(matches!(sel_fact.kind, AliasKind::MayAlias(_)));
    }

    #[test]
    fn field_access_propagates_alias() {
        let mut b = Builder::new("field", &[], i32_type());
        let count = b.constant(ConstantData::u64(1), u64_type(), unknown_span());
        let ptr = b.allocate(i32_type(), count, unknown_span()).unwrap();
        let field = b.field_access(ptr, "x", i32_type(), unknown_span()).unwrap();
        b.return_value(field, unknown_span()).unwrap();
        let func = b.build();
        let facts = run_alias(&func);

        let field_fact = facts.get(&field).unwrap();
        // Field access propagates the base pointer's allocation site.
        assert_eq!(field_fact.allocation_site, Some(ptr));
    }
}
