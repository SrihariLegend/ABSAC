//! Kernighan popcount: `while (x) { x &= x - 1; count++ }`.
//!
//! The loop clears one set bit per iteration, so the accumulator is
//! `popcount(x)`. The obligation is discharged by bounded symbolic
//! unrolling for widths ≤ 16; wider widths await the kernel induction
//! proof (H3).

use sir_nodes::{Function, Node, NodeKind};
use sir_types::{ConstantData, Effects, NodeId, Span, Type};

use crate::patch::{apply, Patch};
use crate::prove;

const KERNIGHAN_POPCOUNT: u64 = 7;

struct PopcountPlan {
    target: NodeId,
    value: NodeId,
    value_ty: Type,
    count_ty: Type,
    width: u32,
}

pub fn optimize_popcount(function: &Function) -> Option<Function> {
    crate::family::with_facts(function, |facts| {
        apply_one(function, facts).map(|(function, _)| function)
    })
}

pub(crate) fn apply_one(
    function: &Function,
    _facts: &sir_analysis::facts::FactDatabase,
) -> Option<(Function, u64)> {
    let plan = recognize(function)?;
    if !prove::kernighan_laws(plan.width) {
        return None;
    }
    let max = function.arena.ids().map(|id| id.0).max().unwrap_or(0);
    let popcount_local = max + 1;
    let popcount = Node::new(
        NodeId::new(popcount_local),
        NodeKind::Popcount {
            operand: plan.value,
        },
        plan.value_ty.clone(),
        Effects::empty(),
        Span::unknown(),
    );
    let value_width = type_bits(&plan.value_ty)?;
    let count_width = type_bits(&plan.count_ty)?;
    if value_width == count_width {
        return apply(
            function,
            &Patch {
                nodes: vec![(popcount_local, popcount)],
                replacements: vec![(plan.target, popcount_local)],
            },
        )
        .map(|function| (function, KERNIGHAN_POPCOUNT));
    }
    let convert_local = max + 2;
    let kind = if count_width > value_width {
        sir_nodes::ConvertKind::ZeroExtend
    } else {
        sir_nodes::ConvertKind::Truncate
    };
    let convert = Node::new(
        NodeId::new(convert_local),
        NodeKind::Convert {
            operand: NodeId::new(popcount_local),
            target_ty: plan.count_ty.clone(),
            kind,
        },
        plan.count_ty,
        Effects::empty(),
        Span::unknown(),
    );
    apply(
        function,
        &Patch {
            nodes: vec![(popcount_local, popcount), (convert_local, convert)],
            replacements: vec![(plan.target, convert_local)],
        },
    )
    .map(|function| (function, KERNIGHAN_POPCOUNT))
}

fn recognize(function: &Function) -> Option<PopcountPlan> {
    let loops: Vec<NodeId> = function
        .arena
        .iter()
        .filter(|node| matches!(node.kind, NodeKind::Loop { .. }))
        .map(|node| node.id)
        .collect();
    for loop_id in loops {
        if let Some(plan) = recognize_loop(function, loop_id) {
            return Some(plan);
        }
    }
    None
}

fn recognize_loop(function: &Function, loop_id: NodeId) -> Option<PopcountPlan> {
    let loop_node = function.get_node(loop_id)?;
    let NodeKind::Loop {
        outputs,
        carried_inputs,
        termination,
        ..
    } = &loop_node.kind
    else {
        return None;
    };
    if carried_inputs.len() != 2 || outputs.len() != 2 {
        return None;
    }
    let count_pos = carried_inputs
        .iter()
        .position(|id| is_zero(function, *id))?;
    let value_pos = if count_pos == 0 { 1 } else { 0 };
    let count = carried_inputs[count_pos];
    let value = carried_inputs[value_pos];
    let value_ty = function.get_node(value)?.ty.clone();
    let count_ty = function.get_node(count)?.ty.clone();
    let width = type_bits(&value_ty)?;
    if width > 16 {
        return None;
    }
    // `while (x != 0)` termination.
    let NodeKind::Ne { lhs, rhs } = &function.get_node(*termination)?.kind else {
        return None;
    };
    if !((*lhs == value && is_zero(function, *rhs))
        || (*rhs == value && is_zero(function, *lhs)))
    {
        return None;
    }
    // `x_next = x & (x - 1)`.
    let value_next = &function.get_node(outputs[value_pos])?.kind;
    let NodeKind::And { lhs, rhs } = value_next else {
        return None;
    };
    let other = if *lhs == value {
        *rhs
    } else if *rhs == value {
        *lhs
    } else {
        return None;
    };
    if !is_decrement(function, other, value) {
        return None;
    }
    // `count_next = count + 1`.
    let NodeKind::Add { lhs, rhs } = &function.get_node(outputs[count_pos])?.kind else {
        return None;
    };
    let count_step = if *lhs == count {
        *rhs
    } else if *rhs == count {
        *lhs
    } else {
        return None;
    };
    if !is_one(function, count_step) {
        return None;
    }
    let target = function
        .arena
        .iter()
        .find(|node| {
            matches!(
                &node.kind,
                NodeKind::TupleExtract { tuple, index } if *tuple == loop_id && *index == count_pos
            )
        })?
        .id;
    Some(PopcountPlan {
        target,
        value,
        value_ty,
        count_ty,
        width,
    })
}

fn is_decrement(function: &Function, candidate: NodeId, value: NodeId) -> bool {
    match function.get_node(candidate).map(|node| &node.kind) {
        Some(NodeKind::Sub { lhs, rhs }) => *lhs == value && is_one(function, *rhs),
        Some(NodeKind::Add { lhs, rhs }) => {
            (*lhs == value && is_minus_one(function, *rhs))
                || (*rhs == value && is_minus_one(function, *lhs))
        }
        _ => false,
    }
}

fn is_zero(function: &Function, id: NodeId) -> bool {
    matches!(
        function.get_node(id).map(|node| &node.kind),
        Some(NodeKind::Constant(ConstantData::Integer { value, .. }))
            if value.trim_start_matches(['+', '-']).trim_start_matches('0').is_empty()
    )
}

fn is_one(function: &Function, id: NodeId) -> bool {
    matches!(
        function.get_node(id).map(|node| &node.kind),
        Some(NodeKind::Constant(ConstantData::Integer { value, .. })) if value == "1"
    )
}

fn is_minus_one(function: &Function, id: NodeId) -> bool {
    let Some(NodeKind::Constant(ConstantData::Integer { value, width, signed })) =
        function.get_node(id).map(|node| &node.kind)
    else {
        return false;
    };
    let bits = match width {
        sir_types::IntegerWidth::I8 => 8u32,
        sir_types::IntegerWidth::I16 => 16,
        sir_types::IntegerWidth::I32 => 32,
        sir_types::IntegerWidth::I64 => 64,
        sir_types::IntegerWidth::I128 => return value == "-1",
    };
    if *signed {
        return value == "-1";
    }
    value
        .parse::<u128>()
        .map(|parsed| parsed == (1u128 << bits) - 1)
        .unwrap_or(false)
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
}
