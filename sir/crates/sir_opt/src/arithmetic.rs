//! Power-of-two arithmetic family: unsigned division, remainder, and
//! multiplication by a constant are shifts and masks.
//!
//! Mined from `HEAD` definitions 100–102. The obligation is discharged
//! by `sir_mech` at the operand's real width, and signed operands are
//! never considered (the lowerer refuses them).

use sir_nodes::{Function, Node, NodeKind};
use sir_types::{ConstantData, Effects, NodeId, Span, Type};

use crate::patch::{apply, Patch};
use crate::prove;

const DIVIDE_SHIFT: u64 = 100;
const MODULO_AND: u64 = 101;
const MULTIPLY_SHIFT: u64 = 102;
const SHIFT_MASK: u64 = 103;

struct ArithmeticPlan {
    target: NodeId,
    definition: u64,
    dynamic: NodeId,
    shift: u32,
    width: u32,
    ty: Type,
}

pub fn optimize_arithmetic(function: &Function) -> Option<Function> {
    crate::family::with_facts(function, |facts| {
        apply_one(function, facts).map(|(function, _)| function)
    })
}

pub(crate) fn apply_one(
    function: &Function,
    _facts: &sir_analysis::facts::FactDatabase,
) -> Option<(Function, u64)> {
    let plan = recognize(function)?;
    let proved = match plan.definition {
        DIVIDE_SHIFT => prove::divide_pow2_laws(plan.width, plan.shift),
        MODULO_AND => prove::modulo_pow2_laws(plan.width, plan.shift),
        MULTIPLY_SHIFT => prove::multiply_pow2_laws(plan.width, plan.shift),
        SHIFT_MASK => prove::shift_mask_laws(plan.width, plan.shift),
        _ => false,
    };
    if !proved {
        return None;
    }
    let amount = match plan.definition {
        MODULO_AND => (1u64 << plan.shift) - 1,
        SHIFT_MASK => (1u64 << (plan.width - plan.shift)) - 1,
        _ => u64::from(plan.shift),
    };
    let max = function.arena.ids().map(|id| id.0).max().unwrap_or(0);
    let constant_local = max + 1;
    let replacement_local = max + 2;
    let constant = Node::new(
        NodeId::new(constant_local),
        NodeKind::Constant(ConstantData::u64(amount)),
        plan.ty.clone(),
        Effects::empty(),
        Span::unknown(),
    );
    let replacement = match plan.definition {
        DIVIDE_SHIFT => NodeKind::Shr {
            lhs: plan.dynamic,
            rhs: NodeId::new(constant_local),
        },
        MODULO_AND => NodeKind::And {
            lhs: plan.dynamic,
            rhs: NodeId::new(constant_local),
        },
        SHIFT_MASK => NodeKind::And {
            lhs: plan.dynamic,
            rhs: NodeId::new(constant_local),
        },
        _ => NodeKind::Shl {
            lhs: plan.dynamic,
            rhs: NodeId::new(constant_local),
        },
    };
    let replacement = Node::new(
        NodeId::new(replacement_local),
        replacement,
        plan.ty,
        Effects::empty(),
        Span::unknown(),
    );
    let patch = Patch {
        nodes: vec![(constant_local, constant), (replacement_local, replacement)],
        replacements: vec![(plan.target, replacement_local)],
    };
    apply(function, &patch).map(|function| (function, plan.definition))
}

fn recognize(function: &Function) -> Option<ArithmeticPlan> {
    for node in function.arena.iter() {
        if let NodeKind::Shr { lhs, rhs } = &node.kind {
            if let Some(plan) = recognize_shift_mask(function, node.id, *lhs, *rhs, &node.ty) {
                return Some(plan);
            }
            continue;
        }
        let (lhs, rhs, definition) = match &node.kind {
            NodeKind::Div { lhs, rhs } => (*lhs, *rhs, DIVIDE_SHIFT),
            NodeKind::Rem { lhs, rhs } => (*lhs, *rhs, MODULO_AND),
            NodeKind::Mul { lhs, rhs } => (*lhs, *rhs, MULTIPLY_SHIFT),
            _ => continue,
        };
        let (dynamic, constant) = match (constant_u64(function, lhs), constant_u64(function, rhs)) {
            (Some(_), None) => (rhs, lhs),
            (None, Some(_)) => (lhs, rhs),
            _ => continue,
        };
        let constant = constant_u64(function, constant)?;
        if constant <= 1 || !constant.is_power_of_two() {
            continue;
        }
        let width = type_bits(&function.get_node(dynamic)?.ty)?;
        let shift = constant.trailing_zeros();
        if shift == 0 || shift >= width {
            continue;
        }
        return Some(ArithmeticPlan {
            target: node.id,
            definition,
            dynamic,
            shift,
            width,
            ty: node.ty.clone(),
        });
    }
    None
}

fn recognize_shift_mask(
    function: &Function,
    target: NodeId,
    lhs: NodeId,
    rhs: NodeId,
    ty: &Type,
) -> Option<ArithmeticPlan> {
    let inner = function.get_node(lhs)?;
    let NodeKind::Shl {
        lhs: x,
        rhs: inner_amount,
    } = &inner.kind
    else {
        return None;
    };
    let outer = constant_u64(function, rhs)?;
    let inner = constant_u64(function, *inner_amount)?;
    if outer != inner || outer == 0 {
        return None;
    }
    let width = type_bits(&function.get_node(*x)?.ty)?;
    if outer >= u64::from(width) {
        return None;
    }
    Some(ArithmeticPlan {
        target,
        definition: SHIFT_MASK,
        dynamic: *x,
        shift: outer as u32,
        width,
        ty: ty.clone(),
    })
}

fn constant_u64(function: &Function, id: NodeId) -> Option<u64> {
    match function.get_node(id).map(|node| &node.kind) {
        Some(NodeKind::Constant(ConstantData::Integer { value, .. })) => value.parse().ok(),
        _ => None,
    }
}

fn type_bits(ty: &Type) -> Option<u32> {
    match ty {
        Type::Integer { width, .. } => Some(match width {
            sir_types::IntegerWidth::I8 => 8,
            sir_types::IntegerWidth::I16 => 16,
            sir_types::IntegerWidth::I32 => 32,
            sir_types::IntegerWidth::I64 => 64,
            sir_types::IntegerWidth::I128 => 128,
        }),
        Type::BitVector { width } => Some(*width as u32),
        _ => None,
    }
    .filter(|width| *width <= 64)
}
