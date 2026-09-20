//! Runtime-bound sum: `for (i = 0; i < n; i++) acc += zext(a[i])`.
//!
//! The bound is not statically known, so the loop is replaced with a
//! four-accumulator helper (contract-backed, differential/sanitizer
//! checked) rather than an unrolled chain.

use sir_nodes::{Function, Node, NodeKind};
use sir_types::{ConstantData, Effects, NodeId, Span, Type};

use crate::patch::{apply, Patch};

const RUNTIME_SUM: u64 = 404;

struct RuntimeSumPlan {
    target: NodeId,
    collection: NodeId,
    element_ty: Type,
    accumulator_ty: Type,
    bound: NodeId,
}

pub fn optimize_runtime_sum(function: &Function) -> Option<Function> {
    crate::family::with_facts(function, |facts| {
        apply_one(function, facts).map(|(function, _)| function)
    })
}

pub(crate) fn apply_one(
    function: &Function,
    facts: &sir_analysis::facts::FactDatabase,
) -> Option<(Function, u64)> {
    let plan = recognize(function, facts)?;
    let element_width = type_bits(&plan.element_ty)?;
    let name = match element_width {
        8 => "sir_sum_u8",
        16 => "sir_sum_u16",
        32 => "sir_sum_u32",
        64 => "sir_sum_u64",
        _ => return None,
    };
    let max = function.arena.ids().map(|id| id.0).max().unwrap_or(0);
    let local = max + 1;
    let node = Node::new(
        NodeId::new(local),
        NodeKind::Intrinsic {
            name: name.to_string(),
            args: vec![plan.collection, plan.bound],
        },
        plan.accumulator_ty,
        Effects::empty(),
        Span::unknown(),
    );
    apply(
        function,
        &Patch {
            nodes: vec![(local, node)],
            replacements: vec![(plan.target, local)],
        },
    )
    .map(|function| (function, RUNTIME_SUM))
}

fn recognize(
    function: &Function,
    facts: &sir_analysis::facts::FactDatabase,
) -> Option<RuntimeSumPlan> {
    let loops: Vec<NodeId> = function
        .arena
        .iter()
        .filter(|node| matches!(node.kind, NodeKind::Loop { .. }))
        .map(|node| node.id)
        .collect();
    for loop_id in loops {
        if let Some(plan) = recognize_loop(function, facts, loop_id) {
            return Some(plan);
        }
    }
    None
}

fn recognize_loop(
    function: &Function,
    facts: &sir_analysis::facts::FactDatabase,
    loop_id: NodeId,
) -> Option<RuntimeSumPlan> {
    let loop_node = function.get_node(loop_id)?;
    if !(loop_node.effects - Effects::READ_MEMORY).is_empty() {
        return None;
    }
    let NodeKind::Loop {
        body,
        outputs,
        carried_inputs,
        termination,
        ..
    } = &loop_node.kind
    else {
        return None;
    };
    for id in body.iter().chain(outputs).chain(carried_inputs).copied() {
        if !(function.get_node(id)?.effects - Effects::READ_MEMORY).is_empty() {
            return None;
        }
    }
    let fact = facts.loops.get(&loop_id)?;
    if fact.trip_count.is_some() {
        return None;
    }
    let counter = fact.reductions.iter().find(|reduction| {
        reduction.reduction_kind == "sum" && is_one_constant(function, reduction.invariant_value)
    })?;
    let reduction = fact.reductions.iter().find(|reduction| {
        reduction.reduction_kind == "sum" && reduction.invariant_value != counter.invariant_value
    })?;
    if !is_zero_constant(function, reduction.variable) {
        return None;
    }
    let counter_pos = carried_inputs
        .iter()
        .position(|id| *id == counter.variable)?;
    let accumulator_pos = carried_inputs
        .iter()
        .position(|id| *id == reduction.variable)?;
    let accumulator = reduction.variable;
    // Runtime bound: `termination` is `counter < bound` (possibly under
    // a find/guard conjuncture), and the counter is stepped by one.
    let bound = find_lt_bound(function, *termination, counter.variable)?;
    if is_constant(function, bound) {
        return None;
    }
    let step = function.get_node(outputs[counter_pos])?;
    if !matches!(&step.kind, NodeKind::Add { lhs, rhs }
        if (*lhs == carried_inputs[counter_pos] && is_one_constant(function, *rhs))
            || (*rhs == carried_inputs[counter_pos] && is_one_constant(function, *lhs)))
    {
        return None;
    }
    let accumulator_next = function.get_node(outputs[accumulator_pos])?;
    let NodeKind::Add { lhs, rhs } = &accumulator_next.kind else {
        return None;
    };
    let value = if *lhs == accumulator {
        *rhs
    } else if *rhs == accumulator {
        *lhs
    } else {
        return None;
    };
    let access = match &function.get_node(value)?.kind {
        NodeKind::ArrayAccess { .. } => value,
        NodeKind::Convert {
            operand,
            kind: sir_nodes::ConvertKind::ZeroExtend,
            ..
        } if matches!(
            &function.get_node(*operand)?.kind,
            NodeKind::ArrayAccess { .. }
        ) =>
        {
            *operand
        }
        _ => return None,
    };
    let NodeKind::ArrayAccess { base, index } = &function.get_node(access)?.kind else {
        return None;
    };
    if *index != carried_inputs[counter_pos] {
        return None;
    }
    let element_ty = function.get_node(access)?.ty.clone();
    type_bits(&element_ty)?;
    let accumulator_ty = function.get_node(accumulator)?.ty.clone();
    type_bits(&accumulator_ty)?;
    let target = function
        .arena
        .iter()
        .find(|node| {
            matches!(
                &node.kind,
                NodeKind::TupleExtract { tuple, index } if *tuple == loop_id && *index == accumulator_pos
            )
        })?
        .id;
    Some(RuntimeSumPlan {
        target,
        collection: *base,
        element_ty,
        accumulator_ty,
        bound,
    })
}

fn find_lt_bound(function: &Function, termination: NodeId, counter: NodeId) -> Option<NodeId> {
    match &function.get_node(termination)?.kind {
        NodeKind::Lt { lhs, rhs } if *lhs == counter => Some(*rhs),
        NodeKind::BoolAnd { lhs, rhs } => find_lt_bound(function, *lhs, counter)
            .or_else(|| find_lt_bound(function, *rhs, counter)),
        _ => None,
    }
}

fn is_zero_constant(function: &Function, id: NodeId) -> bool {
    matches!(
        function.get_node(id).map(|node| &node.kind),
        Some(NodeKind::Constant(ConstantData::Integer { value, .. }))
            if value.trim_start_matches(['+', '-']).trim_start_matches('0').is_empty()
    )
}

fn is_one_constant(function: &Function, id: NodeId) -> bool {
    matches!(
        function.get_node(id).map(|node| &node.kind),
        Some(NodeKind::Constant(ConstantData::Integer { value, .. })) if value == "1"
    )
}

fn is_constant(function: &Function, id: NodeId) -> bool {
    matches!(
        function.get_node(id).map(|node| &node.kind),
        Some(NodeKind::Constant(_))
    )
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
