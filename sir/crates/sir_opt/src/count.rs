//! Fixed-extent cardinality: count predicate hits with a straight-line
//! add chain of zero-extended comparison bits.

use sir_nodes::{Function, Node, NodeKind};
use sir_types::{ConstantData, Effects, NodeId, Span, Type};

use crate::patch::{apply, Patch};
use crate::prove;

struct CountPlan {
    target: NodeId,
    collection: NodeId,
    extent: usize,
    index_ty: Type,
    accumulator_ty: Type,
    invariant: NodeId,
    element_access: NodeId,
}

pub fn optimize_count(function: &Function) -> Option<Function> {
    crate::family::with_facts(function, |facts| apply_one(function, facts))
}

pub(crate) fn apply_one(
    function: &Function,
    facts: &sir_analysis::facts::FactDatabase,
) -> Option<Function> {
    let plan = recognize(function, facts)?;
    let width = type_bits(&plan.accumulator_ty)?;
    if !prove::sum_laws(width) {
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
            Effects::empty(),
        );
        let value = instantiate(function, &plan, plan.invariant, constant, &mut nodes, &mut next, 0)?;
        chain = Some(match chain {
            None => value,
            Some(acc) => push(
                &mut nodes,
                &mut next,
                NodeKind::Add {
                    lhs: NodeId::new(acc),
                    rhs: NodeId::new(value),
                },
                plan.accumulator_ty.clone(),
                Effects::empty(),
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
) -> Option<CountPlan> {
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
) -> Option<CountPlan> {
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
    let reduction = fact.reductions.iter().find(|reduction| {
        reduction.reduction_kind == "sum" && reduction.invariant_value != counter.invariant_value
    })?;
    let invariant = reduction.invariant_value;
    let NodeKind::Convert {
        operand,
        target_ty,
        kind: sir_nodes::ConvertKind::ZeroExtend,
    } = &function.get_node(invariant)?.kind
    else {
        return None;
    };
    if *target_ty != function.get_node(reduction.variable)?.ty {
        return None;
    }
    let predicate = *operand;
    if function.get_node(predicate)?.ty != Type::Bool {
        return None;
    }
    let (element_access, base, index) =
        find_element_access(function, predicate, counter.variable)?;
    let Type::Array { element, length } = &function.get_node(base)?.ty else {
        return None;
    };
    let extent = *length;
    if extent == 0 || extent > 512 || fact.trip_count != Some(extent as u64) {
        return None;
    }
    let _ = element;
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
    Some(CountPlan {
        target,
        collection: base,
        extent,
        index_ty: function.get_node(index)?.ty.clone(),
        accumulator_ty: function.get_node(reduction.variable)?.ty.clone(),
        invariant,
        element_access,
    })
}

fn instantiate(
    function: &Function,
    plan: &CountPlan,
    node_id: NodeId,
    index_local: u64,
    nodes: &mut Vec<(u64, Node)>,
    next: &mut u64,
    depth: usize,
) -> Option<u64> {
    if depth > 32 {
        return None;
    }
    if node_id == plan.element_access {
        return Some(push(
            nodes,
            next,
            NodeKind::ArrayAccess {
                base: plan.collection,
                index: NodeId::new(index_local),
            },
            function.get_node(node_id)?.ty.clone(),
            Effects::READ_MEMORY,
        ));
    }
    let node = function.get_node(node_id)?;
    let mut rec = |id: NodeId| {
        instantiate(function, plan, id, index_local, nodes, next, depth + 1)
    };
    let kind = match &node.kind {
        NodeKind::Constant(data) => NodeKind::Constant(data.clone()),
        // Keep external parameters by their original NodeId: the
        // emitter resolves parameter names by the node id, and the patch
        // applier treats ids outside the local range as external.
        NodeKind::Parameter { .. } => return Some(node_id.as_u64()),
        NodeKind::Convert {
            operand,
            target_ty,
            kind,
        } => NodeKind::Convert {
            operand: NodeId::new(rec(*operand)?),
            target_ty: target_ty.clone(),
            kind: *kind,
        },
        NodeKind::Lt { lhs, rhs } => NodeKind::Lt {
            lhs: NodeId::new(rec(*lhs)?),
            rhs: NodeId::new(rec(*rhs)?),
        },
        NodeKind::Le { lhs, rhs } => NodeKind::Le {
            lhs: NodeId::new(rec(*lhs)?),
            rhs: NodeId::new(rec(*rhs)?),
        },
        NodeKind::Gt { lhs, rhs } => NodeKind::Gt {
            lhs: NodeId::new(rec(*lhs)?),
            rhs: NodeId::new(rec(*rhs)?),
        },
        NodeKind::Ge { lhs, rhs } => NodeKind::Ge {
            lhs: NodeId::new(rec(*lhs)?),
            rhs: NodeId::new(rec(*rhs)?),
        },
        NodeKind::Eq { lhs, rhs } => NodeKind::Eq {
            lhs: NodeId::new(rec(*lhs)?),
            rhs: NodeId::new(rec(*rhs)?),
        },
        NodeKind::Ne { lhs, rhs } => NodeKind::Ne {
            lhs: NodeId::new(rec(*lhs)?),
            rhs: NodeId::new(rec(*rhs)?),
        },
        NodeKind::BoolAnd { lhs, rhs } => NodeKind::BoolAnd {
            lhs: NodeId::new(rec(*lhs)?),
            rhs: NodeId::new(rec(*rhs)?),
        },
        NodeKind::BoolOr { lhs, rhs } => NodeKind::BoolOr {
            lhs: NodeId::new(rec(*lhs)?),
            rhs: NodeId::new(rec(*rhs)?),
        },
        NodeKind::BoolNot { operand } => NodeKind::BoolNot {
            operand: NodeId::new(rec(*operand)?),
        },
        NodeKind::Select {
            cond,
            true_val,
            false_val,
        } => NodeKind::Select {
            cond: NodeId::new(rec(*cond)?),
            true_val: NodeId::new(rec(*true_val)?),
            false_val: NodeId::new(rec(*false_val)?),
        },
        _ => return None,
    };
    Some(push(nodes, next, kind, node.ty.clone(), Effects::empty()))
}

fn push(
    nodes: &mut Vec<(u64, Node)>,
    next: &mut u64,
    kind: NodeKind,
    ty: Type,
    effects: Effects,
) -> u64 {
    let id = *next;
    *next += 1;
    nodes.push((
        id,
        Node::new(NodeId::new(id), kind, ty, effects, Span::unknown()),
    ));
    id
}

fn find_element_access(
    function: &Function,
    root: NodeId,
    induction: NodeId,
) -> Option<(NodeId, NodeId, NodeId)> {
    let mut found = None;
    let mut worklist = vec![root];
    let mut seen = std::collections::HashSet::new();
    while let Some(id) = worklist.pop() {
        if !seen.insert(id) {
            continue;
        }
        let node = function.get_node(id)?;
        if let NodeKind::ArrayAccess { base, index } = &node.kind {
            if *index != induction {
                continue;
            }
            if found.is_some() {
                return None;
            }
            found = Some((id, *base, *index));
        }
        worklist.extend(node.kind.input_nodes());
    }
    found
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
