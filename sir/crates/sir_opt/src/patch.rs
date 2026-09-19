//! Minimal SSA patch applier: local-id graph → fresh global nodes,
//! use rewriting, and dead-code sweep.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use sir_nodes::{Function, Node, NodeKind};
use sir_types::NodeId;

/// A detached replacement graph. Node operands that name another patch
/// node hold that node's local id; any other operand is external.
pub struct Patch {
    pub nodes: Vec<(u64, Node)>,
    pub replacements: Vec<(NodeId, u64)>,
}

pub fn apply(function: &Function, patch: &Patch) -> Option<Function> {
    let max = function.arena.ids().map(|id| id.0).max().unwrap_or(0);
    let local: HashSet<u64> = patch.nodes.iter().map(|(id, _)| *id).collect();
    let mut map: BTreeMap<u64, NodeId> = BTreeMap::new();
    let mut next = max + 1;
    for (local_id, _) in &patch.nodes {
        map.insert(*local_id, NodeId::new(next));
        next += 1;
    }

    let mut rewritten = function.clone();
    for (local_id, node) in &patch.nodes {
        let mut imported = node.clone();
        imported.id = *map.get(local_id)?;
        imported.kind = rewrite_local(&imported.kind, &local, &map)?;
        rewritten.insert_node(imported);
    }
    for (old, local_id) in &patch.replacements {
        let new = *map.get(local_id)?;
        replace_uses(&mut rewritten, *old, new)?;
    }
    for (old, local_id) in &patch.replacements {
        if rewritten.return_node == Some(*old) {
            rewritten.return_node = map.get(local_id).copied();
        }
    }
    // Live-out gate: after use rewriting and return remapping, the
    // replaced node must no longer be referenced anywhere. If it is,
    // some use site was not covered and the patch fails closed.
    for (old, _) in &patch.replacements {
        if rewritten
            .arena
            .iter()
            .any(|node| node.kind.input_nodes().contains(old))
        {
            return None;
        }
    }
    sweep(&mut rewritten);
    // Post-patch structural gate: a patch that produces malformed SIR
    // (dangling references, bad loop shape, type violation, cycles) is
    // rejected before it can reach selection or emission.
    let mut verifier = sir_verify::Verifier::new(&rewritten);
    if verifier.verify() {
        Some(rewritten)
    } else {
        None
    }
}

fn rewrite_local(
    kind: &NodeKind,
    local: &HashSet<u64>,
    map: &BTreeMap<u64, NodeId>,
) -> Option<NodeKind> {
    let adjust = |id: &NodeId| -> Option<NodeId> {
        if local.contains(&id.0) {
            map.get(&id.0).copied()
        } else {
            Some(*id)
        }
    };
    Some(match kind {
        NodeKind::ArrayAccess { base, index } => NodeKind::ArrayAccess {
            base: *base,
            index: adjust(index)?,
        },
        NodeKind::Convert {
            operand,
            target_ty,
            kind,
        } => NodeKind::Convert {
            operand: adjust(operand)?,
            target_ty: target_ty.clone(),
            kind: *kind,
        },
        NodeKind::Add { lhs, rhs } => NodeKind::Add {
            lhs: adjust(lhs)?,
            rhs: adjust(rhs)?,
        },
        NodeKind::Sub { lhs, rhs } => NodeKind::Sub {
            lhs: adjust(lhs)?,
            rhs: adjust(rhs)?,
        },
        NodeKind::Mul { lhs, rhs } => NodeKind::Mul {
            lhs: adjust(lhs)?,
            rhs: adjust(rhs)?,
        },
        NodeKind::And { lhs, rhs } => NodeKind::And {
            lhs: adjust(lhs)?,
            rhs: adjust(rhs)?,
        },
        NodeKind::Or { lhs, rhs } => NodeKind::Or {
            lhs: adjust(lhs)?,
            rhs: adjust(rhs)?,
        },
        NodeKind::Xor { lhs, rhs } => NodeKind::Xor {
            lhs: adjust(lhs)?,
            rhs: adjust(rhs)?,
        },
        NodeKind::Shl { lhs, rhs } => NodeKind::Shl {
            lhs: adjust(lhs)?,
            rhs: adjust(rhs)?,
        },
        NodeKind::Shr { lhs, rhs } => NodeKind::Shr {
            lhs: adjust(lhs)?,
            rhs: adjust(rhs)?,
        },
        NodeKind::Intrinsic { name, args } => NodeKind::Intrinsic {
            name: name.clone(),
            args: args.iter().map(|id| adjust(id)).collect::<Option<Vec<_>>>()?,
        },
        other => other.clone(),
    })
}

fn replace_uses(function: &mut Function, old: NodeId, new: NodeId) -> Option<()> {
    let ids: Vec<NodeId> = function.arena.ids().collect();
    for id in ids {
        let kind = function.arena.get(id)?.kind.clone();
        if !kind.input_nodes().contains(&old) {
            continue;
        }
        let replaced = replace_in_kind(&kind, old, new)?;
        function.arena.get_mut(id)?.kind = replaced;
    }
    Some(())
}

fn replace_in_kind(kind: &NodeKind, old: NodeId, new: NodeId) -> Option<NodeKind> {
    let r = |id: &NodeId| if *id == old { new } else { *id };
    let bin = |lhs: &NodeId, rhs: &NodeId| (r(lhs), r(rhs));
    Some(match kind {
        NodeKind::Return { value } => NodeKind::Return { value: r(value) },
        NodeKind::Convert {
            operand,
            target_ty,
            kind,
        } => NodeKind::Convert {
            operand: r(operand),
            target_ty: target_ty.clone(),
            kind: *kind,
        },
        NodeKind::Add { lhs, rhs } => {
            let (lhs, rhs) = bin(lhs, rhs);
            NodeKind::Add { lhs, rhs }
        }
        NodeKind::Sub { lhs, rhs } => {
            let (lhs, rhs) = bin(lhs, rhs);
            NodeKind::Sub { lhs, rhs }
        }
        NodeKind::Mul { lhs, rhs } => {
            let (lhs, rhs) = bin(lhs, rhs);
            NodeKind::Mul { lhs, rhs }
        }
        NodeKind::Div { lhs, rhs } => {
            let (lhs, rhs) = bin(lhs, rhs);
            NodeKind::Div { lhs, rhs }
        }
        NodeKind::Rem { lhs, rhs } => {
            let (lhs, rhs) = bin(lhs, rhs);
            NodeKind::Rem { lhs, rhs }
        }
        NodeKind::And { lhs, rhs } => {
            let (lhs, rhs) = bin(lhs, rhs);
            NodeKind::And { lhs, rhs }
        }
        NodeKind::Or { lhs, rhs } => {
            let (lhs, rhs) = bin(lhs, rhs);
            NodeKind::Or { lhs, rhs }
        }
        NodeKind::Xor { lhs, rhs } => {
            let (lhs, rhs) = bin(lhs, rhs);
            NodeKind::Xor { lhs, rhs }
        }
        NodeKind::Shl { lhs, rhs } => {
            let (lhs, rhs) = bin(lhs, rhs);
            NodeKind::Shl { lhs, rhs }
        }
        NodeKind::Shr { lhs, rhs } => {
            let (lhs, rhs) = bin(lhs, rhs);
            NodeKind::Shr { lhs, rhs }
        }
        NodeKind::Neg { operand } => NodeKind::Neg {
            operand: r(operand),
        },
        NodeKind::Not { operand } => NodeKind::Not {
            operand: r(operand),
        },
        NodeKind::Eq { lhs, rhs } => {
            let (lhs, rhs) = bin(lhs, rhs);
            NodeKind::Eq { lhs, rhs }
        }
        NodeKind::Ne { lhs, rhs } => {
            let (lhs, rhs) = bin(lhs, rhs);
            NodeKind::Ne { lhs, rhs }
        }
        NodeKind::Lt { lhs, rhs } => {
            let (lhs, rhs) = bin(lhs, rhs);
            NodeKind::Lt { lhs, rhs }
        }
        NodeKind::Le { lhs, rhs } => {
            let (lhs, rhs) = bin(lhs, rhs);
            NodeKind::Le { lhs, rhs }
        }
        NodeKind::Gt { lhs, rhs } => {
            let (lhs, rhs) = bin(lhs, rhs);
            NodeKind::Gt { lhs, rhs }
        }
        NodeKind::Ge { lhs, rhs } => {
            let (lhs, rhs) = bin(lhs, rhs);
            NodeKind::Ge { lhs, rhs }
        }
        NodeKind::BoolAnd { lhs, rhs } => {
            let (lhs, rhs) = bin(lhs, rhs);
            NodeKind::BoolAnd { lhs, rhs }
        }
        NodeKind::BoolOr { lhs, rhs } => {
            let (lhs, rhs) = bin(lhs, rhs);
            NodeKind::BoolOr { lhs, rhs }
        }
        NodeKind::BoolNot { operand } => NodeKind::BoolNot {
            operand: r(operand),
        },
        NodeKind::Select {
            cond,
            true_val,
            false_val,
        } => NodeKind::Select {
            cond: r(cond),
            true_val: r(true_val),
            false_val: r(false_val),
        },
        NodeKind::ArrayAccess { base, index } => NodeKind::ArrayAccess {
            base: r(base),
            index: r(index),
        },
        NodeKind::Intrinsic { name, args } => NodeKind::Intrinsic {
            name: name.clone(),
            args: args.iter().map(r).collect(),
        },
        NodeKind::Call { callee, args } => NodeKind::Call {
            callee: r(callee),
            args: args.iter().map(r).collect(),
        },
        NodeKind::ExternalCall { name, args } => NodeKind::ExternalCall {
            name: name.clone(),
            args: args.iter().map(r).collect(),
        },
        _ => return None,
    })
}

fn sweep(function: &mut Function) {
    let Some(return_id) = function.return_node else {
        return;
    };
    let mut reachable = BTreeSet::new();
    let mut worklist = vec![return_id];
    for node in function.arena.iter() {
        if !node.effects.is_pure() {
            worklist.push(node.id);
        }
    }
    while let Some(id) = worklist.pop() {
        if reachable.insert(id) {
            if let Some(node) = function.arena.get(id) {
                for input in node.kind.input_nodes() {
                    worklist.push(input);
                }
            }
        }
    }
    let ids: Vec<NodeId> = function.arena.ids().collect();
    for id in ids {
        if reachable.contains(&id) {
            continue;
        }
        if function
            .arena
            .get(id)
            .is_some_and(|node| matches!(node.kind, NodeKind::Parameter { .. }))
        {
            continue;
        }
        function.arena.remove(id);
    }
}
