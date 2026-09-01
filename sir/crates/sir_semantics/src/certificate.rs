//! Reduction eligibility certificates (Gate 6A-v1, Priority 0B).
//!
//! ## Recognition is not authorization
//!
//! Gate 6A-v1 finding (N15): the recognizer fired `CardinalityReduction`
//! on `count_i(a[i] == b[i])`. The recognition was semantically TRUE —
//! that IS a cardinality reduction of a predicate. But the emitted truth
//! declared only ONE input, omitting the second memory region `b`. A
//! target plan derived from that partial truth would lower the region
//! incorrectly or unsafely.
//!
//! Lesson: a concept can be true while a transformation authorization
//! derived from it is unsafe. Recognition and transformation eligibility
//! must be separate:
//!
//! ```text
//! Recognition:   a semantic concept is present in this region.
//! Binding:       all concrete operands and boundaries are known.
//! Eligibility:   the transformation's preconditions are satisfied.
//! Authorization: a specific candidate may be generated.
//! Verification:  the concrete candidate is equivalent.
//! ```
//!
//! This module implements the first closed-world completeness checks
//! that separate Recognition from Binding. A recognizer may only claim
//! a reduction truth when the certificate is complete:
//!
//! ```text
//! Closed-world checks:
//!   1. Every memory base reachable from the region
//!      must be declared as a certificate input.
//!   2. Every observable effect must be declared.
//!   3. Every loop-carried dependency must be either the recognized
//!      accumulator/induction or explicitly declared and supported.
//!   4. Every output leaving the region must be bound.
//! ```
//!
//! If completeness cannot be proven, recognition abstains. This is more
//! robust than special-casing individual failures (e.g., "two arrays"):
//! any future pattern whose footprint the certificate cannot describe
//! is refused by the same invariant.
//!
//! This is a partial instance of the full ReductionCertificate design
//! (see docs/GATE6A_RESULTS.md Part 3). Remaining fields (iteration
//! domain, overflow semantics, trap conditions, per-field provenance)
//! are added as their failure modes are observed.

use sir_analysis::graph;
use sir_nodes::{Function, NodeKind};
use sir_types::NodeId;

/// A memory base declared by a certificate: the identity of one memory
/// region the recognized computation reads from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryBase {
    /// NodeId of the base pointer (currently: a Parameter node).
    pub base: NodeId,
}

/// The result of a closed-world memory-footprint check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FootprintCheck {
    /// All memory bases in the region were identified and are listed.
    Complete(Vec<MemoryBase>),
    /// The region's memory footprint could not be fully characterized.
    /// Recognition must abstain — an incomplete footprint can authorize
    /// an unsafe transformation even when the high-level concept is true.
    Incomplete(String),
}

/// Enumerate every distinct memory base read or written by the given
/// region nodes, resolving through ArrayAccess / Load chains down to
/// the root pointer (a Parameter or Constant).
///
/// Returns `FootprintCheck::Incomplete` when a base cannot be resolved
/// to a root pointer — an unresolvable footprint must never authorize
/// a transformation.
pub fn memory_footprint(func: &Function, region_nodes: &[NodeId]) -> FootprintCheck {
    let mut bases: Vec<MemoryBase> = Vec::new();

    // Walk the full transitive input set of the region. Memory bases
    // are the root pointers feeding Load/ArrayAccess nodes.
    let roots: Vec<NodeId> = region_nodes.to_vec();
    let all_inputs = collect_transitive_with_roots(func, &roots);

    for node_id in all_inputs {
        let Some(node) = func.get_node(node_id) else {
            return FootprintCheck::Incomplete(format!(
                "region references unknown node %{}",
                node_id.0
            ));
        };
        match &node.kind {
            NodeKind::Load { .. } => {
                // The load's address operand is its first dataflow input.
                let inputs = graph::dataflow_inputs(&node.kind);
                match inputs.first() {
                    Some(&addr) => match resolve_base(func, addr) {
                        Some(base) => push_unique(&mut bases, base),
                        _ => {
                            return FootprintCheck::Incomplete(format!(
                                "load %{} has an unresolvable base pointer",
                                node_id.0
                            ))
                        }
                    },
                    None => {
                        return FootprintCheck::Incomplete(format!(
                            "load %{} has no resolvable address operand",
                            node_id.0
                        ))
                    }
                }
            }
            // Bare ArrayAccess used as a value (e.g., passed to Eq) — its base
            // is a memory input.
            NodeKind::ArrayAccess { base, .. } => match resolve_base(func, *base) {
                Some(base) => push_unique(&mut bases, base),
                None => {
                    return FootprintCheck::Incomplete(format!(
                        "array access %{} has an unresolvable base",
                        node_id.0
                    ))
                }
            },
            _ => {}
        }
    }

    FootprintCheck::Complete(bases)
}

/// Resolve a pointer expression down to its root base (Parameter or
/// constant). Returns None if the chain cannot be resolved.
fn resolve_base(func: &Function, id: NodeId) -> Option<NodeId> {
    let node = func.get_node(id)?;
    match &node.kind {
        NodeKind::Parameter { .. } => Some(id),
        NodeKind::ArrayAccess { base, .. } => resolve_base(func, *base),
        NodeKind::Convert { operand, .. } => resolve_base(func, *operand),
        _ => None,
    }
}

/// Collect the transitive inputs of a set of root nodes, including the
/// roots themselves, deduplicated.
fn collect_transitive_with_roots(func: &Function, roots: &[NodeId]) -> Vec<NodeId> {
    let mut seen = std::collections::BTreeSet::new();
    let mut order = Vec::new();
    let mut stack: Vec<NodeId> = roots.to_vec();
    while let Some(id) = stack.pop() {
        if !seen.insert(id) {
            continue;
        }
        if let Some(node) = func.get_node(id) {
            order.push(id);
            for input in graph::dataflow_inputs(&node.kind) {
                if !seen.contains(&input) {
                    stack.push(input);
                }
            }
        }
    }
    order
}

fn push_unique(bases: &mut Vec<MemoryBase>, base: NodeId) {
    if !bases.iter().any(|b| b.base == base) {
        bases.push(MemoryBase { base });
    }
}
