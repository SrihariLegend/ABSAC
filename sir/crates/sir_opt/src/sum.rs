//! Fixed-extent sum reduction: recognition, proof, straight-line patch.

use sir_nodes::{Function, Node, NodeKind};
use sir_types::{ConstantData, Effects, NodeId, Span, Type};

use crate::patch::{apply, Patch};
use crate::prove;

const MAX_UNROLL: usize = 512;

struct SumPlan {
    target: NodeId,
    collection: NodeId,
    extent: usize,
    index_ty: Type,
    element_ty: Type,
    accumulator_ty: Type,
    map_expression: NodeId,
    element_access: NodeId,
    definition: u64,
    carry_first: bool,
}

/// Optimize a fixed-extent raw-element sum loop into a straight-line
/// add chain. Returns `None` unless every recognition, proof, and patch
/// step succeeds (fail closed).
pub fn optimize_sum(function: &Function) -> Option<Function> {
    optimize_sum_with_definition(function).map(|(function, _)| function)
}

/// Optimize a sum and report whether definition 400 (raw) or 401
/// (mapped) produced the patch.
pub fn optimize_sum_with_definition(function: &Function) -> Option<(Function, u64)> {
    crate::family::with_facts(function, |facts| apply_one(function, facts))
}

pub(crate) fn apply_one(
    function: &Function,
    facts: &sir_analysis::facts::FactDatabase,
) -> Option<(Function, u64)> {
    let plan = recognize(function, facts)?;
    let width = type_bits(&plan.accumulator_ty)?;
    if !prove::sum_laws(width) {
        return None;
    }
    let patch = build_patch(function, &plan)?;
    apply(function, &patch).map(|function| (function, plan.definition))
}

fn recognize(function: &Function, facts: &sir_analysis::facts::FactDatabase) -> Option<SumPlan> {
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
) -> Option<SumPlan> {
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
        let node = function.get_node(id)?;
        if !(node.effects - Effects::READ_MEMORY).is_empty() {
            return None;
        }
    }

    let fact = facts.loops.get(&loop_id)?;
    let counter = fact.reductions.iter().find(|reduction| {
        reduction.reduction_kind == "sum" && is_one(function, reduction.invariant_value)
    })?;
    let element = fact.reductions.iter().find(|reduction| {
        (reduction.reduction_kind == "sum" || reduction.reduction_kind == "sum_offset")
            && reduction.invariant_value != counter.invariant_value
    })?;
    if !is_zero(function, element.variable) {
        return None;
    }
    let position = carried_inputs
        .iter()
        .position(|id| *id == element.variable)?;
    let output = *outputs.get(position)?;
    if element.reduction_kind == "sum_offset"
        && !folded_offset_is_zero(function, output, element.variable)
    {
        return None;
    }
    // Match the loop's operand order exactly; the identity laws cover
    // either orientation, so no commutativity proof is needed.
    let carry_first = !matches!(
        &function.get_node(output)?.kind,
        NodeKind::Add { lhs, .. } if *lhs == element.invariant_value
    );
    let (element_access, base, index) =
        find_element_access(function, element.invariant_value, counter.variable)?;
    let map_expression = element.invariant_value;
    let base_node = function.get_node(base)?;
    let Type::Array {
        element: array_element,
        length,
    } = &base_node.ty
    else {
        return None;
    };
    let extent = *length;
    if extent == 0 || extent > MAX_UNROLL || fact.trip_count != Some(extent as u64) {
        return None;
    }
    let _internal_result = *outputs.get(position)?;
    // The live target is the tuple projection that exports this output
    // slot; the internal output id is replaced by the projection's
    // replacement during the sweep.
    let target = function
        .arena
        .iter()
        .find(|node| {
            matches!(
                &node.kind,
                NodeKind::TupleExtract { tuple, index }
                    if *tuple == loop_id && *index == position
            )
        })?
        .id;
    let accumulator_ty = function.get_node(element.variable)?.ty.clone();
    let index_ty = function.get_node(index)?.ty.clone();
    Some(SumPlan {
        target,
        collection: base,
        extent,
        index_ty,
        element_ty: array_element.as_ref().clone(),
        accumulator_ty,
        map_expression,
        element_access,
        carry_first,
        definition: if is_raw_map(function, map_expression, element_access) {
            400
        } else {
            401
        },
    })
}

fn is_raw_map(function: &Function, root: NodeId, access: NodeId) -> bool {
    if root == access {
        return true;
    }
    matches!(
        function.get_node(root).map(|node| &node.kind),
        Some(NodeKind::Convert { operand, .. }) if *operand == access
    )
}

fn build_patch(function: &Function, plan: &SumPlan) -> Option<Patch> {
    let max = function.arena.ids().map(|id| id.0).max().unwrap_or(0);
    let mut next = max + 1;
    let mut nodes: Vec<(u64, Node)> = Vec::new();

    let mut chain: Option<u64> = None;
    for index in 0..plan.extent {
        let constant = push_node(
            &mut nodes,
            &mut next,
            NodeKind::Constant(ConstantData::u64(index as u64)),
            plan.index_ty.clone(),
            Effects::empty(),
        );
        let value = clone_with_index(
            function,
            plan,
            plan.map_expression,
            constant,
            &mut nodes,
            &mut next,
            0,
        )?;
        chain = Some(match chain {
            None => value,
            Some(acc) => push_node(
                &mut nodes,
                &mut next,
                NodeKind::Add {
                    lhs: NodeId::new(if plan.carry_first { acc } else { value }),
                    rhs: NodeId::new(if plan.carry_first { value } else { acc }),
                },
                plan.accumulator_ty.clone(),
                Effects::empty(),
            ),
        });
    }
    Some(Patch {
        nodes,
        replacements: vec![(plan.target, chain?)],
    })
}

/// Re-instantiate the certified map expression with the element access
/// at a constant index. Only total, pure operators are cloned.
fn clone_with_index(
    function: &Function,
    plan: &SumPlan,
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
        return Some(push_node(
            nodes,
            next,
            NodeKind::ArrayAccess {
                base: plan.collection,
                index: NodeId::new(index_local),
            },
            plan.element_ty.clone(),
            Effects::READ_MEMORY,
        ));
    }
    let node = function.get_node(node_id)?;
    let mut rec =
        |id: NodeId| clone_with_index(function, plan, id, index_local, nodes, next, depth + 1);
    let kind = match &node.kind {
        NodeKind::Constant(data) => NodeKind::Constant(data.clone()),
        NodeKind::Convert {
            operand,
            target_ty,
            kind,
        } => NodeKind::Convert {
            operand: NodeId::new(rec(*operand)?),
            target_ty: target_ty.clone(),
            kind: *kind,
        },
        NodeKind::Xor { lhs, rhs } => NodeKind::Xor {
            lhs: NodeId::new(rec(*lhs)?),
            rhs: NodeId::new(rec(*rhs)?),
        },
        NodeKind::Add { lhs, rhs } => NodeKind::Add {
            lhs: NodeId::new(rec(*lhs)?),
            rhs: NodeId::new(rec(*rhs)?),
        },
        NodeKind::Sub { lhs, rhs } => NodeKind::Sub {
            lhs: NodeId::new(rec(*lhs)?),
            rhs: NodeId::new(rec(*rhs)?),
        },
        NodeKind::Mul { lhs, rhs } => NodeKind::Mul {
            lhs: NodeId::new(rec(*lhs)?),
            rhs: NodeId::new(rec(*rhs)?),
        },
        NodeKind::And { lhs, rhs } => NodeKind::And {
            lhs: NodeId::new(rec(*lhs)?),
            rhs: NodeId::new(rec(*rhs)?),
        },
        NodeKind::Or { lhs, rhs } => NodeKind::Or {
            lhs: NodeId::new(rec(*lhs)?),
            rhs: NodeId::new(rec(*rhs)?),
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
    Some(push_node(
        nodes,
        next,
        kind,
        node.ty.clone(),
        Effects::empty(),
    ))
}

fn push_node(
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

fn is_zero(function: &Function, id: NodeId) -> bool {
    matches!(
        function.get_node(id).map(|node| &node.kind),
        Some(NodeKind::Constant(ConstantData::Integer { value, .. }))
            if value.trim_start_matches(['+', '-']).trim_start_matches('0').is_empty()
    )
}

fn folded_offset_is_zero(function: &Function, output: NodeId, carry: NodeId) -> bool {
    let Some(node) = function.get_node(output) else {
        return false;
    };
    let NodeKind::Add { lhs, rhs } = &node.kind else {
        return false;
    };
    for inner in [*lhs, *rhs] {
        let Some(inner) = function.get_node(inner) else {
            continue;
        };
        let NodeKind::Add { lhs, rhs } = &inner.kind else {
            continue;
        };
        let offset = if *lhs == carry {
            Some(*rhs)
        } else if *rhs == carry {
            Some(*lhs)
        } else {
            None
        };
        if let Some(offset) = offset {
            return is_zero(function, offset);
        }
    }
    false
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
