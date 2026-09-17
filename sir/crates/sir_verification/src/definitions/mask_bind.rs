//! Shared binding for the mask-algebra identities
//! (`x & (x-1)`, `x & -x`, `~x & (x+1)`, `x | (x+1)`).
//!
//! The definition reads the candidate's ACTUAL pattern from its
//! authorized region nodes: the base operand `x` and its integer width.
//! A region that does not contain the exact structural pattern cannot be
//! bound (no domain ⇒ no proof).

use sir_nodes::{Function, NodeKind};
use sir_types::{NodeId, Type};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MaskPattern {
    /// `x & (x - 1)`
    ClearLowestSetBit,
    /// `x & -x` (Neg) or `x & (0 - x)`
    LowestSetBit,
    /// `~x & (x + 1)`
    LowestClearBitMask,
    /// `x | (x + 1)`
    SetLowestClearBit,
}

pub struct BoundMask {
    /// The pattern root (And/Or).
    pub root: NodeId,
    /// The base operand `x`.
    pub operand: NodeId,
    /// Operand width in bits.
    pub width: usize,
}

pub fn bind_mask(
    function: &Function,
    region_nodes: &[NodeId],
    pattern: MaskPattern,
) -> Option<BoundMask> {
    for &id in region_nodes {
        let Some(node) = function.get_node(id) else {
            continue;
        };
        let operand = match (&node.kind, pattern) {
            (NodeKind::And { lhs, rhs }, MaskPattern::ClearLowestSetBit) => {
                and_pattern(function, *lhs, *rhs, |f, x, other| {
                    sub_one(f, other, x)
                })
            }
            (NodeKind::And { lhs, rhs }, MaskPattern::LowestSetBit) => {
                and_pattern(function, *lhs, *rhs, |f, x, other| {
                    neg_of(f, other, x) || sub_zero(f, other, x)
                })
            }
            (NodeKind::And { lhs, rhs }, MaskPattern::LowestClearBitMask) => {
                and_pattern(function, *lhs, *rhs, |f, x, other| {
                    // `x` here is the `~x0` side; `other` must be `x0 + 1`.
                    match not_operand(f, x) {
                        Some(x0) => add_one(f, x0, other),
                        None => false,
                    }
                })
            }
            (NodeKind::Or { lhs, rhs }, MaskPattern::SetLowestClearBit) => {
                or_pattern(function, *lhs, *rhs, |f, x, other| add_one(f, x, other))
            }
            _ => None,
        };
        let Some(operand) = operand else {
            continue;
        };
        let width = match function.get_node(operand).map(|n| &n.ty) {
            Some(Type::Integer { width, .. }) => width.bits() as usize,
            _ => continue,
        };
        return Some(BoundMask {
            root: id,
            operand,
            width,
        });
    }
    None
}

/// `And` where one side is `x` and the predicate holds for the other.
fn and_pattern(
    function: &Function,
    lhs: NodeId,
    rhs: NodeId,
    predicate: impl Fn(&Function, NodeId, NodeId) -> bool,
) -> Option<NodeId> {
    if predicate(function, lhs, rhs) {
        Some(lhs)
    } else if predicate(function, rhs, lhs) {
        Some(rhs)
    } else {
        None
    }
}

/// `Or` where one side is `x` and the predicate holds for the other.
fn or_pattern(
    function: &Function,
    lhs: NodeId,
    rhs: NodeId,
    predicate: impl Fn(&Function, NodeId, NodeId) -> bool,
) -> Option<NodeId> {
    if predicate(function, lhs, rhs) {
        Some(lhs)
    } else if predicate(function, rhs, lhs) {
        Some(rhs)
    } else {
        None
    }
}

/// Is `candidate` the expression `x - 1`?
fn sub_one(function: &Function, candidate: NodeId, x: NodeId) -> bool {
    match function.get_node(candidate).map(|n| &n.kind) {
        Some(NodeKind::Sub { lhs, rhs }) => *lhs == x && is_const(function, *rhs, 1),
        _ => false,
    }
}

/// Is `candidate` the expression `0 - x`?
fn sub_zero(function: &Function, candidate: NodeId, x: NodeId) -> bool {
    match function.get_node(candidate).map(|n| &n.kind) {
        Some(NodeKind::Sub { lhs, rhs }) => *rhs == x && is_const(function, *lhs, 0),
        _ => false,
    }
}

/// Is `candidate` the expression `-x`?
fn neg_of(function: &Function, candidate: NodeId, x: NodeId) -> bool {
    match function.get_node(candidate).map(|n| &n.kind) {
        Some(NodeKind::Neg { operand }) => *operand == x,
        _ => false,
    }
}

/// The operand of a `~x` node, if the candidate is one.
fn not_operand(function: &Function, candidate: NodeId) -> Option<NodeId> {
    match &function.get_node(candidate)?.kind {
        NodeKind::Not { operand } => Some(*operand),
        _ => None,
    }
}

/// Is `candidate` the expression `x + 1`?
fn add_one(function: &Function, x: NodeId, candidate: NodeId) -> bool {
    match function.get_node(candidate).map(|n| &n.kind) {
        Some(NodeKind::Add { lhs, rhs }) => *lhs == x && is_const(function, *rhs, 1),
        _ => false,
    }
}

fn is_const(function: &Function, id: NodeId, value: u64) -> bool {
    match &function.get_node(id).map(|n| &n.kind) {
        Some(NodeKind::Constant(data)) => data
            .as_u64()
            .or_else(|| data.as_i64().and_then(|v| u64::try_from(v).ok()))
            == Some(value),
        _ => false,
    }
}
