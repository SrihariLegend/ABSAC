//! Fixed-extent bounded search: first index satisfying a predicate,
//! or the bound sentinel.
//!
//! Equality over a u8 array lowers to the `sir_memchr` library
//! primitive (definition 402). Other predicates lower to an explicit
//! reverse-overwrite select chain (definition 200), which the proof
//! module shows computes the same first match as the source loop's
//! found-flag scan.

use std::collections::HashSet;

use sir_nodes::{Function, Node, NodeKind};
use sir_types::{ConstantData, Effects, NodeId, Span, Type};

use crate::patch::{apply, carried_names, Patch};

struct SearchPlan {
    target: NodeId,
    extent: usize,
    index_ty: Type,
    bound: NodeId,
    predicate: NodeId,
    accesses: Vec<(NodeId, NodeId, Type)>,
    definition: u64,
    memchr: Option<(NodeId, NodeId)>,
    runtime: bool,
    /// S2: the names under which this loop's carried variables are visible
    /// inside its body. `instantiate` refuses to copy one.
    carried: HashSet<NodeId>,
}

/// Optimize a bounded search and report whether definition 402 (fixed
/// `sir_memchr`), 403 (runtime `sir_memchr`), or 200 (select chain)
/// produced the patch.
pub fn optimize_search(function: &Function) -> Option<(Function, u64)> {
    crate::family::with_facts(function, |facts| apply_one(function, facts))
}

pub(crate) fn apply_one(
    function: &Function,
    facts: &sir_analysis::facts::FactDatabase,
) -> Option<(Function, u64)> {
    let plan = recognize(function, facts)?;
    // Measured retention: unrolled select chains beat the memchr helper
    // at extents 4–16 (1.22–1.27 vs 1.50 ns/op in the dispatch benchmark),
    // so equality searches at those extents take definition 200; larger
    // extents use the vectorized primitive (402). The select chain lost
    // to the baseline loop at extent 64 (82ns vs 244ns), so it is refused
    // above 16.
    if plan.definition == 200 && plan.extent > 16 {
        return None;
    }
    if !plan.runtime {
        if !sir_mech::scan::first_match_law() {
            return None;
        }
    }
    let optimized = if plan.definition != 200 {
        let (collection, scalar) = plan.memchr?;
        let max = function.arena.ids().map(|id| id.0).max().unwrap_or(0);
        let local = max + 1;
        let node = Node::new(
            NodeId::new(local),
            NodeKind::Intrinsic {
                name: "sir_memchr".to_string(),
                args: vec![collection, scalar, plan.bound],
            },
            Type::u64(),
            Effects::READ_MEMORY,
            Span::unknown(),
        );
        apply(
            function,
            &Patch {
                nodes: vec![(local, node)],
                replacements: vec![(plan.target, local)],
            },
        )?
    } else {
        build_select_chain(function, &plan)?
    };
    Some((optimized, plan.definition))
}

/// Compatibility entry point: only the equality/`memchr` shape.
pub fn optimize_memchr(function: &Function) -> Option<Function> {
    optimize_search(function)
        .filter(|(_, definition)| *definition == 402)
        .map(|(function, _)| function)
}

fn build_select_chain(function: &Function, plan: &SearchPlan) -> Option<Function> {
    let max = function.arena.ids().map(|id| id.0).max().unwrap_or(0);
    let mut next = max + 1;
    let mut nodes: Vec<(u64, Node)> = Vec::new();
    let mut result = push(
        &mut nodes,
        &mut next,
        NodeKind::Constant(ConstantData::u64(plan.extent as u64)),
        plan.index_ty.clone(),
        Effects::empty(),
    );
    for index in (0..plan.extent).rev() {
        let index_const = push(
            &mut nodes,
            &mut next,
            NodeKind::Constant(ConstantData::u64(index as u64)),
            plan.index_ty.clone(),
            Effects::empty(),
        );
        let predicate = instantiate(
            function,
            plan,
            plan.predicate,
            index_const,
            &mut nodes,
            &mut next,
            0,
        )?;
        result = push(
            &mut nodes,
            &mut next,
            NodeKind::Select {
                cond: NodeId::new(predicate),
                true_val: NodeId::new(index_const),
                false_val: NodeId::new(result),
            },
            plan.index_ty.clone(),
            Effects::empty(),
        );
    }
    apply(
        function,
        &Patch {
            nodes,
            replacements: vec![(plan.target, result)],
        },
    )
}

fn recognize(function: &Function, facts: &sir_analysis::facts::FactDatabase) -> Option<SearchPlan> {
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
) -> Option<SearchPlan> {
    let loop_node = function.get_node(loop_id)?;
    if !(loop_node.effects - Effects::READ_MEMORY).is_empty() {
        return None;
    }
    let NodeKind::Loop {
        body,
        outputs,
        carried_inputs,
        termination,
    } = &loop_node.kind
    else {
        return None;
    };
    for id in body
        .iter()
        .chain(outputs)
        .chain(carried_inputs)
        .chain(std::iter::once(termination))
        .copied()
    {
        if !(function.get_node(id)?.effects - Effects::READ_MEMORY).is_empty() {
            return None;
        }
    }
    if carried_inputs.len() != 3 || outputs.len() != 3 {
        return None;
    }
    let fact = facts.loops.get(&loop_id)?;

    let mut found_pos = None;
    let mut counter_pos = None;
    let mut result_pos = None;
    let mut runtime_result_pos = None;
    for (position, carried) in carried_inputs.iter().enumerate() {
        match &function.get_node(*carried)?.kind {
            NodeKind::Constant(ConstantData::Bool(false)) => found_pos = Some(position),
            NodeKind::Constant(data) if data.as_u64() == Some(0) => counter_pos = Some(position),
            NodeKind::Constant(data) if data.as_u64().is_some() => result_pos = Some(position),
            _ => runtime_result_pos = Some(position),
        }
    }
    let found_pos = found_pos?;
    let counter_pos = counter_pos?;
    let runtime = runtime_result_pos.is_some();
    let result_pos = if runtime {
        runtime_result_pos?
    } else {
        result_pos?
    };
    let index = carried_inputs[counter_pos];
    let found = carried_inputs[found_pos];
    let bound = carried_inputs[result_pos];
    let extent = if runtime {
        0
    } else {
        constant_u64(function, bound)? as usize
    };
    // A guarded termination (`!found && i < bound`) can exit early on the
    // match, so the loop analysis reports no trip count for it. The scan
    // bound is established structurally below instead: `bound_guard`
    // requires the zero-initialized counter, stepped by exactly one, to be
    // compared `Lt` against the carried extent.
    if !runtime && extent == 0 {
        return None;
    }
    if runtime && fact.trip_count.is_some() {
        return None;
    }

    let match_node = function.get_node(outputs[found_pos])?;
    let NodeKind::BoolOr {
        lhs: found_side,
        rhs: match_side,
    } = &match_node.kind
    else {
        return None;
    };
    if *found_side != found {
        return None;
    }
    let predicate = *match_side;

    let select = function.get_node(outputs[result_pos])?;
    let NodeKind::Select {
        cond,
        true_val,
        false_val,
    } = select.kind
    else {
        return None;
    };
    if true_val != index || false_val != bound {
        return None;
    }
    let NodeKind::BoolAnd {
        lhs: cond_lhs,
        rhs: cond_rhs,
    } = &function.get_node(cond)?.kind
    else {
        return None;
    };
    let negated_found = if is_not_found(function, *cond_lhs, found) {
        *cond_lhs
    } else if is_not_found(function, *cond_rhs, found) {
        *cond_rhs
    } else {
        return None;
    };
    let other = if negated_found == *cond_lhs {
        *cond_rhs
    } else {
        *cond_lhs
    };
    if other != predicate {
        return None;
    }

    let step = function.get_node(outputs[counter_pos])?;
    let NodeKind::Add { lhs, rhs } = step.kind else {
        return None;
    };
    if !(lhs == index && is_one(function, rhs) || rhs == index && is_one(function, lhs)) {
        return None;
    }

    let termination_kind = &function.get_node(*termination)?.kind;
    let bound_guard = match termination_kind {
        NodeKind::Lt { lhs, rhs }
            if *lhs == index
                && (*rhs == bound
                    || (!runtime && constant_u64(function, *rhs) == Some(extent as u64))) =>
        {
            true
        }
        NodeKind::BoolAnd { lhs, rhs } => {
            let lt = if matches!(&function.get_node(*lhs)?.kind, NodeKind::Lt { .. }) {
                *lhs
            } else {
                *rhs
            };
            matches!(
                &function.get_node(lt)?.kind,
                NodeKind::Lt { lhs, rhs }
                    if *lhs == index
                        && (*rhs == bound
                            || (!runtime
                                && constant_u64(function, *rhs) == Some(extent as u64)))
            )
        }
        _ => false,
    };
    if !bound_guard {
        return None;
    }

    let mut accesses = Vec::new();
    let mut worklist = vec![predicate];
    let mut seen = std::collections::HashSet::new();
    while let Some(id) = worklist.pop() {
        if !seen.insert(id) {
            continue;
        }
        let node = function.get_node(id)?;
        if let NodeKind::ArrayAccess {
            base,
            index: access_index,
        } = &node.kind
        {
            if *access_index != index {
                continue;
            }
            let base_ty = &function.get_node(*base)?.ty;
            let (element, length) = match base_ty {
                Type::Array { element, length } => (element.as_ref().clone(), Some(*length)),
                Type::Pointer { pointee, .. } if **pointee == Type::u8() => (Type::u8(), None),
                _ => return None,
            };
            if !runtime {
                if let Some(length) = length {
                    if length != extent {
                        return None;
                    }
                }
            }
            accesses.push((id, *base, element));
        }
        worklist.extend(node.kind.input_nodes());
    }
    if accesses.is_empty() || accesses.len() > 2 {
        return None;
    }

    let memchr = memchr_contract(function, predicate, &accesses);
    if runtime && memchr.is_none() {
        return None;
    }
    let definition = if runtime {
        if memchr.is_some() {
            403
        } else {
            return None;
        }
    } else if memchr.is_some() && extent > 16 {
        402
    } else {
        200
    };

    let target = function
        .arena
        .iter()
        .find(|node| {
            matches!(
                &node.kind,
                NodeKind::TupleExtract { tuple, index: slot }
                    if *tuple == loop_id && *slot == result_pos
            )
        })?
        .id;
    Some(SearchPlan {
        target,
        extent,
        index_ty: function.get_node(index)?.ty.clone(),
        bound,
        predicate,
        accesses,
        definition,
        memchr,
        runtime,
        carried: carried_names(loop_node),
    })
}

fn memchr_contract(
    function: &Function,
    predicate: NodeId,
    accesses: &[(NodeId, NodeId, Type)],
) -> Option<(NodeId, NodeId)> {
    let [(access, base, element_ty)] = accesses else {
        return None;
    };
    if *element_ty != Type::u8() {
        return None;
    }
    let NodeKind::Eq { lhs, rhs } = &function.get_node(predicate)?.kind else {
        return None;
    };
    let scalar = if *lhs == *access {
        *rhs
    } else if *rhs == *access {
        *lhs
    } else {
        return None;
    };
    if function.get_node(scalar)?.ty != Type::u8() {
        return None;
    }
    Some((*base, scalar))
}

fn instantiate(
    function: &Function,
    plan: &SearchPlan,
    node_id: NodeId,
    index_const: u64,
    nodes: &mut Vec<(u64, Node)>,
    next: &mut u64,
    depth: usize,
) -> Option<u64> {
    if depth > 32 {
        return None;
    }
    if let Some((_, base, element_ty)) = plan
        .accesses
        .iter()
        .find(|(access, _, _)| *access == node_id)
    {
        return Some(push(
            nodes,
            next,
            NodeKind::ArrayAccess {
                base: *base,
                index: NodeId::new(index_const),
            },
            element_ty.clone(),
            Effects::READ_MEMORY,
        ));
    }
    // S2: see `patch::carried_names`. The select-chain path re-instantiates
    // the predicate once per index; a carried node names the current
    // iteration's value inside the body and the initial value outside it,
    // so copying one would read the initial value at every index. The
    // access substitution above is the sanctioned exception. The memchr
    // path (402/403) does not re-instantiate the predicate at all, so this
    // refusal only reaches the select-chain shape (200).
    if plan.carried.contains(&node_id) {
        return None;
    }
    let node = function.get_node(node_id)?;
    let mut rec = |id: NodeId| instantiate(function, plan, id, index_const, nodes, next, depth + 1);
    let kind = match &node.kind {
        NodeKind::Constant(data) => NodeKind::Constant(data.clone()),
        NodeKind::Parameter { .. } => return Some(node_id.as_u64()),
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

fn is_one(function: &Function, id: NodeId) -> bool {
    constant_u64(function, id) == Some(1)
}

fn constant_u64(function: &Function, id: NodeId) -> Option<u64> {
    match function.get_node(id).map(|node| &node.kind) {
        Some(NodeKind::Constant(data)) => data.as_u64(),
        _ => None,
    }
}

fn is_not_found(function: &Function, id: NodeId, found: NodeId) -> bool {
    matches!(
        function.get_node(id).map(|node| &node.kind),
        Some(NodeKind::BoolNot { operand }) if *operand == found
    )
}
