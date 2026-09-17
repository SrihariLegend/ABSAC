//! Shared binding for the power-of-two division identities
//! (`x % 2^n`, `x / 2^n`, unsigned).

use sir_nodes::{Function, NodeKind};
use sir_semantics::structure::StructuralDescription;
use sir_transform::roles::RegionRoles;
use sir_types::{NodeId, Type};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Pow2Op {
    Rem,
    Div,
}

pub struct BoundPow2 {
    pub dynamic: NodeId,
    pub constant: u64,
    pub width: usize,
}

pub fn bind_pow2(
    function: &Function,
    structural: &StructuralDescription,
    op: Pow2Op,
) -> Option<BoundPow2> {
    for role in &structural.roles {
        let RegionRoles::ArithmeticOperation {
            operator_node,
            lhs,
            rhs,
            ..
        } = role
        else {
            continue;
        };
        let (node_lhs, node_rhs) = match function.get_node(*operator_node).map(|n| &n.kind) {
            Some(NodeKind::Rem { lhs, rhs }) if op == Pow2Op::Rem => (*lhs, *rhs),
            Some(NodeKind::Div { lhs, rhs }) if op == Pow2Op::Div => (*lhs, *rhs),
            _ => continue,
        };
        // The role must agree with the actual node operands.
        if node_lhs != *lhs || node_rhs != *rhs {
            continue;
        }
        let constant = constant_value(function, *rhs)?;
        if constant == 0 || !constant.is_power_of_two() {
            continue;
        }
        let (width, signed) = match function.get_node(*lhs).map(|n| &n.ty) {
            Some(Type::Integer { width, signed, .. }) => (width.bits() as usize, *signed),
            _ => continue,
        };
        if signed || width == 0 || width > 64 {
            continue;
        }
        if width < 64 && constant >= (1u64 << width) {
            continue;
        }
        return Some(BoundPow2 {
            dynamic: *lhs,
            constant,
            width,
        });
    }
    None
}

pub fn constant_value(function: &Function, id: NodeId) -> Option<u64> {
    match &function.get_node(id)?.kind {
        NodeKind::Constant(data) => data
            .as_u64()
            .or_else(|| data.as_i64().and_then(|v| u64::try_from(v).ok())),
        _ => None,
    }
}
