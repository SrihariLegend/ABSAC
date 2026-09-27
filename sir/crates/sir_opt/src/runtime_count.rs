//! Runtime-bound predicate count: unknown-bound loops that count
//! `zext(icmp(a[i], k))` hits.

use sir_nodes::{Function, Node, NodeKind};
use sir_types::{ConstantData, Effects, NodeId, Span, Type};

use crate::patch::{apply, Patch};

const RUNTIME_COUNT: u64 = 405;

struct RuntimeCountPlan {
    target: NodeId,
    collection: NodeId,
    element_ty: Type,
    accumulator_ty: Type,
    bound: NodeId,
    key: NodeId,
    op: &'static str,
}

pub fn optimize_runtime_count(function: &Function) -> Option<Function> {
    crate::family::with_facts(function, |facts| {
        apply_one(function, facts).map(|(function, _)| function)
    })
}

pub(crate) fn apply_one(
    function: &Function,
    facts: &sir_analysis::facts::FactDatabase,
) -> Option<(Function, u64)> {
    let plan = recognize(function, facts)?;
    let width = type_bits(&plan.element_ty)?;
    let suffix = match width {
        8 => "u8",
        16 => "u16",
        32 => "u32",
        64 => "u64",
        _ => return None,
    };
    let max = function.arena.ids().map(|id| id.0).max().unwrap_or(0);
    let local = max + 1;
    let node = Node::new(
        NodeId::new(local),
        NodeKind::Intrinsic {
            name: format!("sir_count_{}_{suffix}", plan.op),
            args: vec![plan.collection, plan.bound, plan.key],
        },
        plan.accumulator_ty,
        Effects::READ_MEMORY,
        Span::unknown(),
    );
    apply(
        function,
        &Patch {
            nodes: vec![(local, node)],
            replacements: vec![(plan.target, local)],
        },
    )
    .map(|function| (function, RUNTIME_COUNT))
}

fn recognize(
    function: &Function,
    facts: &sir_analysis::facts::FactDatabase,
) -> Option<RuntimeCountPlan> {
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
) -> Option<RuntimeCountPlan> {
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
    if fact.constant_bound.is_some() {
        return None;
    }
    let counter = fact.reductions.iter().find(|reduction| {
        reduction.reduction_kind == "sum" && is_one(function, reduction.invariant_value)
    })?;
    let reduction = fact.reductions.iter().find(|reduction| {
        reduction.reduction_kind == "sum" && reduction.invariant_value != counter.invariant_value
    })?;
    if !is_zero(function, reduction.variable) {
        return None;
    }
    let contribution = reduction.invariant_value;
    let NodeKind::Convert {
        operand: predicate,
        kind: sir_nodes::ConvertKind::ZeroExtend,
        ..
    } = &function.get_node(contribution)?.kind
    else {
        return None;
    };
    let (lhs, rhs, op) = match &function.get_node(*predicate)?.kind {
        NodeKind::Eq { lhs, rhs } => (*lhs, *rhs, "eq"),
        NodeKind::Ne { lhs, rhs } => (*lhs, *rhs, "ne"),
        NodeKind::Lt { lhs, rhs } => (*lhs, *rhs, "lt"),
        NodeKind::Le { lhs, rhs } => (*lhs, *rhs, "le"),
        NodeKind::Gt { lhs, rhs } => (*lhs, *rhs, "gt"),
        NodeKind::Ge { lhs, rhs } => (*lhs, *rhs, "ge"),
        _ => return None,
    };
    let (access, key) = if matches!(
        &function.get_node(lhs)?.kind,
        NodeKind::ArrayAccess { index, .. } if *index == counter.variable
    ) {
        let NodeKind::ArrayAccess { base, index } = &function.get_node(lhs)?.kind else {
            return None;
        };
        if *index != counter.variable {
            return None;
        }
        (*base, rhs)
    } else if matches!(
        &function.get_node(rhs)?.kind,
        NodeKind::ArrayAccess { index, .. } if *index == counter.variable
    ) {
        let NodeKind::ArrayAccess { base, index } = &function.get_node(rhs)?.kind else {
            return None;
        };
        if *index != counter.variable {
            return None;
        }
        (*base, lhs)
    } else {
        return None;
    };
    let element_ty = function
        .get_node(
            if matches!(&function.get_node(lhs)?.kind, NodeKind::ArrayAccess { .. }) {
                lhs
            } else {
                rhs
            },
        )?
        .ty
        .clone();
    if function.get_node(key)?.ty != element_ty {
        return None;
    }
    let bound = find_lt_bound(function, *termination, counter.variable)?;
    if matches!(
        function.get_node(bound).map(|node| &node.kind),
        Some(NodeKind::Constant(_))
    ) {
        return None;
    }
    let position = carried_inputs
        .iter()
        .position(|id| *id == reduction.variable)?;
    let target = function
        .arena
        .iter()
        .find(|node| {
            matches!(
                &node.kind,
                NodeKind::TupleExtract { tuple, index } if *tuple == loop_id && *index == position
            )
        })?
        .id;
    Some(RuntimeCountPlan {
        target,
        collection: access,
        element_ty,
        accumulator_ty: function.get_node(reduction.variable)?.ty.clone(),
        bound,
        key,
        op,
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
