//! Fixed-extent `All` reduction over a comparison predicate: AND-chain
//! the zero/one indicator produced for each element.

use std::collections::HashSet;

use sir_nodes::{Function, Node, NodeKind};
use sir_types::{ConstantData, Effects, NodeId, Span, Type};

use crate::patch::{apply, carried_names, Patch};
use crate::prove;

struct AllPlan {
    target: NodeId,
    collection: NodeId,
    extent: usize,
    index_ty: Type,
    accumulator_ty: Type,
    invariant: NodeId,
    element_access: NodeId,
}

pub fn optimize_all(function: &Function) -> Option<Function> {
    crate::family::with_facts(function, |facts| apply_one(function, facts))
}

pub(crate) fn apply_one(
    function: &Function,
    facts: &sir_analysis::facts::FactDatabase,
) -> Option<Function> {
    let plan = recognize(function, facts)?;
    let width = type_bits(&plan.accumulator_ty)?;
    if !prove::and_laws(width) {
        return None;
    }
    let max = function.arena.ids().map(|id| id.0).max().unwrap_or(0);
    let mut next = max + 1;
    let mut nodes: Vec<(u64, Node)> = Vec::new();
    let mut chain = push(
        &mut nodes,
        &mut next,
        NodeKind::Constant(ConstantData::u64(1)),
        plan.accumulator_ty.clone(),
        Effects::empty(),
    );
    for index in 0..plan.extent {
        let constant = push(
            &mut nodes,
            &mut next,
            NodeKind::Constant(ConstantData::u64(index as u64)),
            plan.index_ty.clone(),
            Effects::empty(),
        );
        let value = instantiate(
            function,
            &plan,
            plan.invariant,
            constant,
            &mut nodes,
            &mut next,
            0,
        )?;
        chain = push(
            &mut nodes,
            &mut next,
            NodeKind::And {
                lhs: NodeId::new(chain),
                rhs: NodeId::new(value),
            },
            plan.accumulator_ty.clone(),
            Effects::empty(),
        );
    }
    apply(
        function,
        &Patch {
            nodes,
            replacements: vec![(plan.target, chain)],
        },
    )
}

fn recognize(function: &Function, facts: &sir_analysis::facts::FactDatabase) -> Option<AllPlan> {
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
) -> Option<AllPlan> {
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
        (reduction.reduction_kind == "select_reset" || reduction.reduction_kind == "bitwise_and")
            && reduction.invariant_value != counter.invariant_value
    })?;
    let position = carried_inputs
        .iter()
        .position(|id| *id == reduction.variable)?;
    // `select_reset` stores the reset value in the reduction fact; the
    // per-iteration indicator is the matching loop output slot.
    if constant_u64(function, reduction.variable) != Some(1) {
        return None;
    }
    let invariant = *outputs.get(position)?;
    let NodeKind::Select {
        cond,
        true_val,
        false_val,
    } = &function.get_node(invariant)?.kind
    else {
        return None;
    };
    let indicator = (
        constant_u64(function, *true_val),
        constant_u64(function, *false_val),
    );
    if !matches!(indicator, (Some(0), Some(1)) | (Some(1), Some(0))) {
        return None;
    }
    let (element_access, base, index) = find_element_access(function, *cond, counter.variable)?;
    // S2: see `patch::carried_names`. `instantiate` clones the hoisted
    // `Select`, and the arm that names the accumulator is pinned to the
    // identity constant by the `constant_u64(..) == Some(1)` check above,
    // so cloning it is sound. The *predicate* is not pinned: a carried node
    // inside it names the current iteration's value, and cloning it would
    // read the initial value every iteration (`values[i] > i` unrolls to
    // `values[k] > 0`). The element access is a leaf — its index operand
    // names the carry and `instantiate` replaces the whole access.
    if cond_reads_carry(function, *cond, element_access, &carried_names(loop_node)) {
        return None;
    }
    let Type::Array { element, length } = &function.get_node(base)?.ty else {
        return None;
    };
    let extent = *length;
    if extent == 0 || extent > 512 {
        return None;
    }
    // An exact iteration count, not the constant bound the counter is
    // compared against. `loops.rs` reports the bound and never reads the
    // counter's initial value, so it cannot distinguish `i = 0; i < 64`
    // from `i = 5; i < 64`. The two agree today only because the lowerer's
    // extent promotion happens to require the counter to start at 0 and
    // step by 1 — a property of a type inference in `sir_lower`, not a
    // property this family checks. See `sir_analysis::loop_shape`.
    let shape = sir_analysis::loop_shape::checked_counted_loop(function, loop_id)?;
    if shape.trip_count != extent as u64 || shape.counter != counter.variable {
        return None;
    }
    let _ = element;
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
    Some(AllPlan {
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
    plan: &AllPlan,
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
    let mut rec = |id: NodeId| instantiate(function, plan, id, index_local, nodes, next, depth + 1);
    let kind = match &node.kind {
        NodeKind::Constant(data) => NodeKind::Constant(data.clone()),
        NodeKind::Parameter { .. } => return Some(node_id.as_u64()),
        NodeKind::Select {
            cond,
            true_val,
            false_val,
        } => NodeKind::Select {
            cond: NodeId::new(rec(*cond)?),
            true_val: NodeId::new(rec(*true_val)?),
            false_val: NodeId::new(rec(*false_val)?),
        },
        NodeKind::Eq { lhs, rhs } => NodeKind::Eq {
            lhs: NodeId::new(rec(*lhs)?),
            rhs: NodeId::new(rec(*rhs)?),
        },
        NodeKind::Ne { lhs, rhs } => NodeKind::Ne {
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
        NodeKind::Convert {
            operand,
            target_ty,
            kind,
        } => NodeKind::Convert {
            operand: NodeId::new(rec(*operand)?),
            target_ty: target_ty.clone(),
            kind: *kind,
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

/// S2: does `cond` reach loop-carried state? The element access is a leaf
/// (its index operand names the carry, and `instantiate` replaces the
/// access wholesale), so the walk stops there. An unknown node is treated
/// as carrying, because the walk's purpose is to refuse.
fn cond_reads_carry(
    function: &Function,
    root: NodeId,
    element_access: NodeId,
    carried: &HashSet<NodeId>,
) -> bool {
    let mut worklist = vec![root];
    let mut seen = HashSet::new();
    while let Some(id) = worklist.pop() {
        if !seen.insert(id) || id == element_access {
            continue;
        }
        if carried.contains(&id) {
            return true;
        }
        let Some(node) = function.get_node(id) else {
            return true;
        };
        worklist.extend(node.kind.input_nodes());
    }
    false
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
    constant_u64(function, id) == Some(1)
}

fn constant_u64(function: &Function, id: NodeId) -> Option<u64> {
    match function.get_node(id).map(|node| &node.kind) {
        Some(NodeKind::Constant(data)) => data.as_u64(),
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
