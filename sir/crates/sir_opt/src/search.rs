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
    /// Length passed to the `sir_memchr` intrinsic: the scan bound.
    bound: NodeId,
    /// The not-found result the loop returns. Equal to the bound in the
    /// `return n` convention, and `-1` in the conventional index form.
    sentinel: NodeId,
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
        let mut nodes = vec![(local, node)];
        // `sir_memchr` reports a miss as the scan bound. The source may report
        // it as something else -- -1 by convention -- so the miss is translated
        // back to the sentinel. A miss is exactly `result == bound`: a found
        // index is always below the bound, so it can never equal it.
        //
        // This is not cosmetic. Decoupling the sentinel from the bound without
        // this step fired the rewrite and returned the bound where the source
        // returned -1; the referee caught it at 483 of 500 cases.
        let replacement = if plan.sentinel == plan.bound {
            local
        } else {
            let hit_local = max + 2;
            let sel_local = max + 3;
            nodes.push((
                hit_local,
                Node::new(
                    NodeId::new(hit_local),
                    NodeKind::Eq {
                        lhs: NodeId::new(local),
                        rhs: plan.bound,
                    },
                    Type::Bool,
                    Effects::empty(),
                    Span::unknown(),
                ),
            ));
            nodes.push((
                sel_local,
                Node::new(
                    NodeId::new(sel_local),
                    NodeKind::Select {
                        cond: NodeId::new(hit_local),
                        true_val: plan.sentinel,
                        false_val: NodeId::new(local),
                    },
                    Type::u64(),
                    Effects::empty(),
                    Span::unknown(),
                ),
            ));
            sel_local
        };
        apply(
            function,
            &Patch {
                nodes,
                replacements: vec![(plan.target, replacement)],
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
    // The unrolled chain's fall-through value is the not-found sentinel, which
    // is not necessarily the extent: `return -1` returns -1, not the bound.
    // A non-constant sentinel cannot be spelled as a constant here, so this
    // path refuses (fail closed) rather than guessing.
    let sentinel = constant_u64(function, plan.sentinel)?;
    let mut result = push(
        &mut nodes,
        &mut next,
        NodeKind::Constant(ConstantData::u64(sentinel)),
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
    let result_pos = if runtime_result_pos.is_some() {
        runtime_result_pos?
    } else {
        result_pos?
    };
    let index = carried_inputs[counter_pos];
    let found = carried_inputs[found_pos];
    // The carried result is the *not-found sentinel*, which is not the same
    // value as the scan bound: `return n` uses n for both, but `return -1`
    // scans up to n and returns -1. The bound is recovered from the
    // termination below, so the two are no longer conflated here. Conflating
    // them is why the -1 convention was refused outright.
    let sentinel = carried_inputs[result_pos];

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
    if true_val != index || false_val != sentinel {
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

    // A guarded termination (`!found && i < bound`) exits early on a match, so
    // the loop analysis reports no trip count for this shape. The scan bound is
    // recovered structurally instead, from the right-hand side of the
    // termination's `Lt(index, bound)`. It is deliberately not read from the
    // carried result: `return -1` scans to the bound and returns -1, and those
    // are different values. `runtime` now means "the bound is not known at
    // compile time", which is the property that actually selects the vectorized
    // path -- it previously meant "the carried result is not a constant", which
    // answered a different question and refused the -1 convention outright.
    let termination_kind = &function.get_node(*termination)?.kind;
    let is_bound_lt = |id: NodeId| -> bool {
        matches!(
            function.get_node(id).map(|node| &node.kind),
            Some(NodeKind::Lt { lhs, .. }) if *lhs == index
        )
    };
    let lt = match termination_kind {
        NodeKind::Lt { .. } if is_bound_lt(*termination) => *termination,
        NodeKind::BoolAnd { lhs, .. } if is_bound_lt(*lhs) => *lhs,
        NodeKind::BoolAnd { rhs, .. } if is_bound_lt(*rhs) => *rhs,
        _ => return None,
    };
    let NodeKind::Lt {
        rhs: scan_bound_raw,
        ..
    } = &function.get_node(lt)?.kind
    else {
        return None;
    };
    let scan_bound = *scan_bound_raw;
    let runtime = constant_u64(function, scan_bound).is_none();
    let extent = if runtime {
        0
    } else {
        constant_u64(function, scan_bound)? as usize
    };
    if !runtime && extent == 0 {
        return None;
    }
    if runtime && fact.constant_bound.is_some() {
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
        bound: scan_bound,
        sentinel,
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
