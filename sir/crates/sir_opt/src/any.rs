//! Fixed-extent `Any` reduction over an integer array: OR-reduce the
//! elements into a straight-line bitwise-OR chain.

use sir_nodes::{Function, Node, NodeKind};
use sir_types::{ConstantData, Effects, NodeId, Span, Type};

use crate::patch::{apply, Patch};
use crate::prove;

struct AnyPlan {
    target: NodeId,
    collection: NodeId,
    extent: usize,
    index_ty: Type,
    accumulator_ty: Type,
    expression: NodeKind,
    element_access: NodeId,
}

pub fn optimize_any(function: &Function) -> Option<Function> {
    crate::family::with_facts(function, |facts| apply_one(function, facts))
}

pub(crate) fn apply_one(
    function: &Function,
    facts: &sir_analysis::facts::FactDatabase,
) -> Option<Function> {
    let plan = recognize(function, facts)?;
    let width = type_bits(&plan.accumulator_ty)?;
    if !prove::or_laws(width) {
        return None;
    }
    let max = function.arena.ids().map(|id| id.0).max().unwrap_or(0);
    let mut next = max + 1;
    let mut nodes: Vec<(u64, Node)> = Vec::new();
    let mut chain: Option<u64> = None;
    for index in 0..plan.extent {
        let constant = push(
            &mut nodes,
            &mut next,
            NodeKind::Constant(ConstantData::u64(index as u64)),
            plan.index_ty.clone(),
        );
        let value = instantiate(
            &plan.expression,
            plan.element_access,
            plan.collection,
            constant,
            &plan.accumulator_ty,
            &mut nodes,
            &mut next,
        )?;
        chain = Some(match chain {
            None => value,
            Some(acc) => push(
                &mut nodes,
                &mut next,
                NodeKind::Or {
                    lhs: NodeId::new(acc),
                    rhs: NodeId::new(value),
                },
                plan.accumulator_ty.clone(),
            ),
        });
    }
    apply(
        function,
        &Patch {
            nodes,
            replacements: vec![(plan.target, chain?)],
        },
    )
}

fn recognize(
    function: &Function,
    facts: &sir_analysis::facts::FactDatabase,
) -> Option<AnyPlan> {
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
) -> Option<AnyPlan> {
    let loop_node = function.get_node(loop_id)?;
    if !(loop_node.effects - Effects::READ_MEMORY).is_empty() {
        return None;
    }
    let NodeKind::Loop {
        body,
        outputs,
        carried_inputs,
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
    let counter = fact.reductions.iter().find(|reduction| {
        reduction.reduction_kind == "sum" && is_one(function, reduction.invariant_value)
    })?;
    let reduction = fact
        .reductions
        .iter()
        .find(|reduction| reduction.reduction_kind == "bitwise_or")?;
    let expression = function.get_node(reduction.invariant_value)?.kind.clone();
    let access = match &expression {
        NodeKind::ArrayAccess { .. } => reduction.invariant_value,
        NodeKind::Or { lhs, rhs } => {
            if matches!(&function.get_node(*lhs)?.kind, NodeKind::ArrayAccess { .. }) {
                *lhs
            } else if matches!(&function.get_node(*rhs)?.kind, NodeKind::ArrayAccess { .. }) {
                *rhs
            } else {
                return None;
            }
        }
        _ => return None,
    };
    let NodeKind::ArrayAccess { base, index } = &function.get_node(access)?.kind else {
        return None;
    };
    if *index != counter.variable {
        return None;
    }
    let Type::Array { element, length } = &function.get_node(*base)?.ty else {
        return None;
    };
    let extent = *length;
    if extent == 0 || extent > 512 || fact.trip_count != Some(extent as u64) {
        return None;
    }
    let accumulator_ty = function.get_node(reduction.variable)?.ty.clone();
    if **element != accumulator_ty {
        return None;
    }
    let position = carried_inputs.iter().position(|id| *id == reduction.variable)?;
    let _internal = outputs.get(position)?;
    let target = function
        .arena
        .iter()
        .find(|node| {
            matches!(
                &node.kind,
                NodeKind::TupleExtract { tuple, index: slot }
                    if *tuple == loop_id && *slot == position
            )
        })?
        .id;
    Some(AnyPlan {
        target,
        collection: *base,
        extent,
        index_ty: function.get_node(*index)?.ty.clone(),
        accumulator_ty,
        expression,
        element_access: access,
    })
}

fn instantiate(
    expression: &NodeKind,
    element_access: NodeId,
    collection: NodeId,
    index: u64,
    accumulator_ty: &Type,
    nodes: &mut Vec<(u64, Node)>,
    next: &mut u64,
) -> Option<u64> {
    let kind = match expression {
        NodeKind::ArrayAccess { .. } => NodeKind::ArrayAccess {
            base: collection,
            index: NodeId::new(index),
        },
        NodeKind::Or { lhs, rhs } => {
            let (access_side, constant_side) = if *lhs == element_access {
                (lhs, rhs)
            } else if *rhs == element_access {
                (rhs, lhs)
            } else {
                return None;
            };
            let _ = access_side;
            NodeKind::Or {
                lhs: NodeId::new(index),
                rhs: *constant_side,
            }
        }
        _ => return None,
    };
    Some(push(nodes, next, kind, accumulator_ty.clone()))
}

fn push(
    nodes: &mut Vec<(u64, Node)>,
    next: &mut u64,
    kind: NodeKind,
    ty: Type,
) -> u64 {
    let id = *next;
    *next += 1;
    nodes.push((
        id,
        Node::new(
            NodeId::new(id),
            kind,
            ty,
            Effects::empty(),
            Span::unknown(),
        ),
    ));
    id
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
