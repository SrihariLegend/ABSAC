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
        _ => false,
    };
    if !proved {
        return None;
    }
    let amount = if plan.definition == MODULO_AND {
        (1u64 << plan.shift) - 1
    } else {
        u64::from(plan.shift)
    };
    let constant = Node::new(
        NodeId::new(0),
        NodeKind::Constant(ConstantData::u64(amount)),
        plan.ty.clone(),
        Effects::empty(),
        Span::unknown(),
    );
    let replacement = match plan.definition {
        DIVIDE_SHIFT => NodeKind::Shr {
            lhs: plan.dynamic,
            rhs: NodeId::new(0),
        },
        MODULO_AND => NodeKind::And {
            lhs: plan.dynamic,
            rhs: NodeId::new(0),
        },
        _ => NodeKind::Shl {
            lhs: plan.dynamic,
            rhs: NodeId::new(0),
        },
    };
    let replacement = Node::new(
        NodeId::new(1),
        replacement,
        plan.ty,
        Effects::empty(),
        Span::unknown(),
    );
    let patch = Patch {
        nodes: vec![(0, constant), (1, replacement)],
        replacements: vec![(plan.target, 1)],
    };
    apply(function, &patch).map(|function| (function, plan.definition))
}

fn recognize(function: &Function) -> Option<ArithmeticPlan> {
    for node in function.arena.iter() {
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
