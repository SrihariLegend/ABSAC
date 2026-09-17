//! Shared binding for constant-amount circular rotations.
//!
//! The role supplies the operand, direction and amount node; the binding
//! then VERIFIES the actual source pattern in the authorized region
//! (`Or(Shl(x,k), Shr(x, W-k))` for a left rotation and the mirror for a
//! right rotation) before the obligation is built. Variable amounts are
//! not bound: their definedness (k < W) is the separate
//! DefinednessCertificate gate.

use sir_nodes::{Function, NodeKind};
use sir_semantics::structure::StructuralDescription;
use sir_transform::roles::{PermutationKind, RegionRoles, ShiftDirection};
use sir_types::{NodeId, Type};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RotateDir {
    Left,
    Right,
}

pub struct BoundRotate {
    pub result: NodeId,
    pub operand: NodeId,
    pub amount: u64,
    pub width: usize,
}

pub fn bind_rotate(
    function: &Function,
    region_nodes: &[NodeId],
    structural: &StructuralDescription,
    direction: RotateDir,
) -> Option<BoundRotate> {
    let (operand, amount_node, role_direction) =
        structural.roles.iter().find_map(|role| match role {
            RegionRoles::BitPermutation {
                operand,
                kind: PermutationKind::Circular { direction, amount },
                ..
            } => Some((*operand, *amount, *direction)),
            _ => None,
        })?;
    let expected = match direction {
        RotateDir::Left => ShiftDirection::Left,
        RotateDir::Right => ShiftDirection::Right,
    };
    if role_direction != expected {
        return None;
    }
    let width = match function.get_node(operand).map(|n| &n.ty) {
        Some(Type::Integer { width, .. }) => width.bits() as usize,
        _ => return None,
    };
    if width == 0 || width > 64 {
        return None;
    }
    let amount = constant_value(function, amount_node)?;
    if amount == 0 || amount >= width as u64 {
        return None;
    }

    // Verify the source pattern is really this rotation before binding.
    for &id in region_nodes {
        if is_rotate_pattern(function, id, operand, amount, width, direction) {
            return Some(BoundRotate {
                result: id,
                operand,
                amount,
                width,
            });
        }
    }
    None
}

fn is_rotate_pattern(
    function: &Function,
    id: NodeId,
    operand: NodeId,
    k: u64,
    width: usize,
    direction: RotateDir,
) -> bool {
    let Some(NodeKind::Or { lhs, rhs }) = function.get_node(id).map(|n| &n.kind) else {
        return false;
    };
    let (first, second) = match direction {
        RotateDir::Left => (ShiftKind::Shl, ShiftKind::Shr),
        RotateDir::Right => (ShiftKind::Shr, ShiftKind::Shl),
    };
    (is_shift(function, *lhs, operand, k, first)
        && is_shift_complement(function, *rhs, operand, k, width, second))
        || (is_shift(function, *rhs, operand, k, first)
            && is_shift_complement(function, *lhs, operand, k, width, second))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ShiftKind {
    Shl,
    Shr,
}

/// `x <shift> k`
fn is_shift(
    function: &Function,
    id: NodeId,
    operand: NodeId,
    k: u64,
    kind: ShiftKind,
) -> bool {
    let Some(node) = function.get_node(id) else {
        return false;
    };
    match (&node.kind, kind) {
        (NodeKind::Shl { lhs, rhs }, ShiftKind::Shl)
        | (NodeKind::Shr { lhs, rhs }, ShiftKind::Shr) => {
            *lhs == operand && constant_value(function, *rhs) == Some(k)
        }
        _ => false,
    }
}

/// `x <shift> (width - k)` where the amount is a constant subtraction.
fn is_shift_complement(
    function: &Function,
    id: NodeId,
    operand: NodeId,
    k: u64,
    width: usize,
    kind: ShiftKind,
) -> bool {
    let Some(node) = function.get_node(id) else {
        return false;
    };
    let amount = match (&node.kind, kind) {
        (NodeKind::Shl { lhs, rhs }, ShiftKind::Shl)
        | (NodeKind::Shr { lhs, rhs }, ShiftKind::Shr) => {
            if *lhs != operand {
                return false;
            }
            *rhs
        }
        _ => return false,
    };
    let Some(NodeKind::Sub { lhs, rhs }) = function.get_node(amount).map(|n| &n.kind) else {
        return false;
    };
    constant_value(function, *lhs) == Some(width as u64)
        && constant_value(function, *rhs) == Some(k)
}

pub fn constant_value(function: &Function, id: NodeId) -> Option<u64> {
    match &function.get_node(id)?.kind {
        NodeKind::Constant(data) => data
            .as_u64()
            .or_else(|| data.as_i64().and_then(|v| u64::try_from(v).ok())),
        _ => None,
    }
}
