//! Dominance analysis.
//!
//! Computes the dominator tree using iterative dataflow:
//! DOM(n) = {n} ∪ ⋂ DOM(p) for all dataflow predecessors p.
//! Also computes immediate dominators and dominator tree children.

use std::collections::{BTreeSet, HashMap, HashSet};
use sir_nodes::Function;
use sir_types::NodeId;

use crate::facts::DominanceFact;
use crate::graph;

/// Run dominance analysis on a function.
///
/// Uses the classic iterative algorithm. In a functional DAG,
/// roots are Parameter and Constant nodes (they have no dataflow
/// inputs).
pub fn run_dominance(func: &Function) -> HashMap<NodeId, DominanceFact> {
    let all_ids: HashSet<NodeId> = func.arena.nodes().keys().copied().collect();
    if all_ids.is_empty() {
        return HashMap::new();
    }

    let all_set: BTreeSet<NodeId> = all_ids.iter().copied().collect();
    let preds = graph::predecessor_map(func);

    // Roots: nodes with no dataflow predecessors.
    let roots: BTreeSet<NodeId> = preds
        .iter()
        .filter(|(_, p)| p.is_empty())
        .map(|(&id, _)| id)
        .collect();

    let actual_roots: BTreeSet<NodeId> = if roots.is_empty() {
        // If no roots found (unusual), use all nodes.
        all_set.clone()
    } else {
        roots
    };

    // Initialize DOM sets.
    let mut dom: HashMap<NodeId, BTreeSet<NodeId>> = HashMap::new();
    for &id in &all_ids {
        if actual_roots.contains(&id) {
            // Root dominates only itself.
            let mut s = BTreeSet::new();
            s.insert(id);
            dom.insert(id, s);
        } else {
            // Non-root: initially dominated by all nodes.
            dom.insert(id, all_set.clone());
        }
    }

    // Iterate to fixpoint.
    let mut changed = true;
    while changed {
        changed = false;
        for &id in &all_ids {
            if actual_roots.contains(&id) {
                continue;
            }
            // DOM(n) = {n} ∪ ⋂ DOM(p) for p in preds(n)
            let mut new_dom: BTreeSet<NodeId> = all_set.clone();
            let ps = preds.get(&id).map(|v| v.as_slice()).unwrap_or(&[]);
            if ps.is_empty() {
                let mut s = BTreeSet::new();
                s.insert(id);
                new_dom = s;
            } else {
                for p in ps {
                    if let Some(pdom) = dom.get(p) {
                        new_dom = new_dom.intersection(pdom).copied().collect();
                    }
                }
                new_dom.insert(id);
            }

            if dom.get(&id) != Some(&new_dom) {
                dom.insert(id, new_dom);
                changed = true;
            }
        }
    }

    // Compute immediate dominators.
    // idom(n) = the node in DOM(n) \ {n} that is dominated by all others in DOM(n) \ {n}.
    let idom = compute_idom(&dom, &all_ids, &actual_roots);

    // Build dominator tree children.
    let mut children: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    for (&id, &idom_id) in &idom {
        children.entry(idom_id).or_default().push(id);
    }

    // Build result facts.
    let mut facts = HashMap::new();
    for &id in &all_ids {
        facts.insert(
            id,
            DominanceFact {
                idom: idom.get(&id).copied(),
                dominates: dom_tree_descendants(id, &children),
                dominators: dom.get(&id).cloned().unwrap_or_default(),
                dom_tree_children: children.get(&id).cloned().unwrap_or_default(),
            },
        );
    }

    facts
}

/// Compute immediate dominators from the DOM sets.
fn compute_idom(
    dom: &HashMap<NodeId, BTreeSet<NodeId>>,
    all_ids: &HashSet<NodeId>,
    roots: &BTreeSet<NodeId>,
) -> HashMap<NodeId, NodeId> {
    let mut idom = HashMap::new();
    for &id in all_ids {
        if roots.contains(&id) {
            continue; // no idom for roots
        }
        let dset = dom.get(&id).unwrap();
        // Strict dominators: DOM(n) \ {n}
        let strict: BTreeSet<NodeId> = dset.iter().copied().filter(|&d| d != id).collect();
        // idom is the strict dominator that dominates all other strict dominators.
        // In a DAG, this is the strict dominator with the largest DOM set.
        let mut best: Option<NodeId> = None;
        let mut best_size: usize = 0;
        for &sd in &strict {
            let sd_dom_size = dom.get(&sd).map(|s| s.len()).unwrap_or(0);
            if sd_dom_size > best_size {
                best = Some(sd);
                best_size = sd_dom_size;
            }
        }
        if let Some(b) = best {
            idom.insert(id, b);
        }
    }
    idom
}

/// Collect all descendants of a node in the dominator tree.
fn dom_tree_descendants(
    root: NodeId,
    children: &HashMap<NodeId, Vec<NodeId>>,
) -> BTreeSet<NodeId> {
    let mut result = BTreeSet::new();
    let mut stack = vec![root];
    while let Some(current) = stack.pop() {
        if result.insert(current) {
            if let Some(kids) = children.get(&current) {
                stack.extend(kids);
            }
        }
    }
    result.remove(&root); // don't include self
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use sir_builder::Builder;
    use sir_types::{Span, Type};

    fn i32_type() -> Type { Type::i32() }
    fn unknown_span() -> Span { Span::unknown() }

    #[test]
    fn single_node_dominates_itself() {
        let mut b = Builder::new("f", &[("x", i32_type())], i32_type());
        let x = b.parameter_index(0).unwrap();
        b.return_value(x, unknown_span()).unwrap();
        let func = b.build();
        let facts = run_dominance(&func);

        let x_fact = facts.get(&x).unwrap();
        assert!(x_fact.dominators.contains(&x));
        // Parameter is a root — no idom.
        assert!(x_fact.idom.is_none());
    }

    #[test]
    fn linear_chain_dominance() {
        let mut b = Builder::new("chain", &[("x", i32_type()), ("y", i32_type())], i32_type());
        let x = b.parameter_index(0).unwrap();
        let y = b.parameter_index(1).unwrap();
        let s1 = b.add(x, y, unknown_span()).unwrap();
        let s2 = b.neg(s1, unknown_span()).unwrap();
        b.return_value(s2, unknown_span()).unwrap();
        let func = b.build();
        let facts = run_dominance(&func);

        // s1 dominates s2 (all paths from roots to s2 go through s1).
        let s2_dom = &facts.get(&s2).unwrap().dominators;
        assert!(s2_dom.contains(&s1));
        assert!(s2_dom.contains(&s2));

        // s1 is dominated only by itself (has two independent inputs x,y).
        let s1_dom = &facts.get(&s1).unwrap().dominators;
        assert!(s1_dom.contains(&s1));
        assert_eq!(s1_dom.len(), 1);
    }

    #[test]
    fn diamond_dominance() {
        // x -> s1 = x+x, s2 = x+x, s3 = s1+s2
        let mut b = Builder::new("diamond", &[("x", i32_type())], i32_type());
        let x = b.parameter_index(0).unwrap();
        let s1 = b.add(x, x, unknown_span()).unwrap();
        let s2 = b.add(x, x, unknown_span()).unwrap();
        let s3 = b.add(s1, s2, unknown_span()).unwrap();
        b.return_value(s3, unknown_span()).unwrap();
        let func = b.build();
        let facts = run_dominance(&func);

        // s3 must be dominated by x (all paths from roots to s3 pass through x).
        let s3_dom = &facts.get(&s3).unwrap().dominators;
        assert!(s3_dom.contains(&x));
        assert!(s3_dom.contains(&s3));
    }

    #[test]
    fn parameters_dominate_themselves() {
        let mut b = Builder::new("f", &[("a", i32_type()), ("b", i32_type())], i32_type());
        let a = b.parameter_index(0).unwrap();
        let b_param = b.parameter_index(1).unwrap();
        let s = b.add(a, b_param, unknown_span()).unwrap();
        b.return_value(s, unknown_span()).unwrap();
        let func = b.build();
        let facts = run_dominance(&func);

        // Parameters dominate themselves.
        assert!(facts.get(&a).unwrap().dominators.contains(&a));
        assert!(facts.get(&b_param).unwrap().dominators.contains(&b_param));
        // s dominates itself.
        assert!(facts.get(&s).unwrap().dominators.contains(&s));
        // In a dataflow DAG, s has two predecessors (a and b).
        // Neither a nor b dominates s because there are alternate paths.
    }
}
