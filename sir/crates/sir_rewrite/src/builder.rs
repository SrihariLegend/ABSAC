use std::collections::{BTreeMap, BTreeSet};

use sir_nodes::{Function, Node, NodeKind};
use sir_types::NodeId;

use crate::error::RewriteError;
use crate::local_id::LocalNodeId;
use crate::patch::ReplacementPatch;
use crate::plan::RewritePlan;
use crate::region::RewriteRegion;

/// Performs graph surgery: clones the original function, imports a
/// `ReplacementPatch`, reconnects SSA edges, and omits obsolete region nodes.
///
/// `RewriteBuilder` is the sole owner of SSA rewiring, dominance preservation,
/// and use-def repair. No recipe ever touches SSA connectivity.
pub struct RewriteBuilder;

impl RewriteBuilder {
    /// Apply a rewrite plan to a function.
    ///
    /// Clones the original, imports the patch, reconnects SSA, and returns
    /// the rewritten function. The original function is never mutated.
    pub fn apply(function: &Function, plan: RewritePlan) -> Result<Function, RewriteError> {
        let region = &plan.region;
        let patch = &plan.patch;

        // 1. Clone the original function
        let mut rewritten = function.clone();

        // 2. Build LocalNodeId → NodeId mapping
        let id_map = Self::build_id_map(patch, &rewritten);

        // Build the set of original NodeIds to distinguish external references
        // from local references during remapping.
        let original_ids: BTreeSet<NodeId> = rewritten.arena.nodes().keys().copied().collect();

        // 3. Rewrite LocalNodeId references inside the detached arena to NodeId references
        let rewritten_nodes = Self::rewrite_local_refs(patch, &id_map, &original_ids)?;

        // 4. Import the rewritten nodes into the cloned function
        for (local_id, mut node) in rewritten_nodes {
            // Update node.id to the global NodeId
            let global_id = id_map.get(&local_id).ok_or_else(|| {
                RewriteError::InternalInvariantViolation(format!("no mapping for {local_id}"))
            })?;
            node.id = *global_id;
            rewritten.insert_node(node);
        }

        // 5. Reconnect external users: every node outside the region that
        //    references `old` now references `new`
        for replacement in &patch.replacements {
            Self::replace_all_uses(
                &mut rewritten,
                replacement.old,
                *id_map.get(&replacement.new).ok_or_else(|| {
                    RewriteError::InternalInvariantViolation(format!(
                        "no mapping for replacement {}",
                        replacement.new
                    ))
                })?,
            );
        }

        // 6. Run a mark-and-sweep dead code elimination to remove the obsolete loop
        //    and all its internal nodes. Only nodes reachable from the Return node stay.
        if let Some(return_id) = rewritten.return_node {
            let mut reachable = BTreeSet::new();
            let mut worklist = vec![return_id];

            // Always preserve side-effecting nodes, even if they don't feed the return value.
            for node in rewritten.arena.iter() {
                if !node.effects.is_pure() {
                    worklist.push(node.id);
                }
            }

            // Mark
            while let Some(id) = worklist.pop() {
                if reachable.insert(id) {
                    if let Some(node) = rewritten.arena.get(id) {
                        for input in node.kind.input_nodes() {
                            worklist.push(input);
                        }
                    }
                }
            }

            // Sweep
            let all_ids: Vec<NodeId> = rewritten.arena.nodes().keys().copied().collect();
            for id in all_ids {
                if !reachable.contains(&id) {
                    // Skip parameters (sir_verify requires all function parameters to exist)
                    if let Some(node) = rewritten.arena.get(id) {
                        if matches!(node.kind, NodeKind::Parameter { .. }) {
                            continue;
                        }
                    }
                    rewritten.arena.remove(id);
                }
            }
        }

        Ok(rewritten)
    }

    /// Build a mapping from LocalNodeId to fresh global NodeId.
    fn build_id_map(
        patch: &ReplacementPatch,
        function: &Function,
    ) -> BTreeMap<LocalNodeId, NodeId> {
        // Start global IDs after the highest existing ID
        let max_existing = function
            .arena
            .nodes()
            .keys()
            .map(|id| id.as_u64())
            .max()
            .unwrap_or(0);
        let mut next_global = max_existing + 1;

        let mut map = BTreeMap::new();
        for (local_id, _) in &patch.arena {
            map.insert(local_id, NodeId::new(next_global));
            next_global += 1;
        }
        map
    }

    /// Rewrite all `LocalNodeId` references inside the detached arena to `NodeId` references.
    fn rewrite_local_refs(
        patch: &ReplacementPatch,
        id_map: &BTreeMap<LocalNodeId, NodeId>,
        original_ids: &BTreeSet<NodeId>,
    ) -> Result<Vec<(LocalNodeId, Node)>, RewriteError> {
        let mut result = Vec::new();

        for (local_id, node) in &patch.arena {
            let mut new_node = node.clone();

            // Rewrite all NodeId operands in the NodeKind
            new_node.kind = Self::rewrite_kind_refs(&node.kind, id_map, original_ids)?;

            result.push((local_id, new_node));
        }

        Ok(result)
    }

    /// Rewrite LocalNodeId→NodeId references within a NodeKind.
    /// NodeKind stores operands as `NodeId`, and during detached construction
    /// we stored `NodeId::new(local_id.as_u64())` as placeholders.
    /// We need to map those back through the id_map.
    fn rewrite_kind_refs(
        kind: &NodeKind,
        id_map: &BTreeMap<LocalNodeId, NodeId>,
        original_ids: &BTreeSet<NodeId>,
    ) -> Result<NodeKind, RewriteError> {
        let resolve = |node_id: &NodeId| -> Result<NodeId, RewriteError> {
            // NodeIds that exist in the original function are external references
            // (e.g., the board parameter). They must be preserved as-is and NOT
            // remapped through the id_map, which only contains local nodes.
            if original_ids.contains(node_id) {
                return Ok(*node_id);
            }
            let local = LocalNodeId::new(node_id.as_u64());
            id_map.get(&local).copied().ok_or_else(|| {
                RewriteError::InternalInvariantViolation(format!("local ID {local} not in id_map"))
            })
        };

        let new_kind = match kind {
            NodeKind::Add { lhs, rhs } => NodeKind::Add {
                lhs: resolve(lhs)?,
                rhs: resolve(rhs)?,
            },
            NodeKind::Sub { lhs, rhs } => NodeKind::Sub {
                lhs: resolve(lhs)?,
                rhs: resolve(rhs)?,
            },
            NodeKind::Mul { lhs, rhs } => NodeKind::Mul {
                lhs: resolve(lhs)?,
                rhs: resolve(rhs)?,
            },
            NodeKind::Div { lhs, rhs } => NodeKind::Div {
                lhs: resolve(lhs)?,
                rhs: resolve(rhs)?,
            },
            NodeKind::Rem { lhs, rhs } => NodeKind::Rem {
                lhs: resolve(lhs)?,
                rhs: resolve(rhs)?,
            },
            NodeKind::Neg { operand } => NodeKind::Neg {
                operand: resolve(operand)?,
            },
            NodeKind::And { lhs, rhs } => NodeKind::And {
                lhs: resolve(lhs)?,
                rhs: resolve(rhs)?,
            },
            NodeKind::Or { lhs, rhs } => NodeKind::Or {
                lhs: resolve(lhs)?,
                rhs: resolve(rhs)?,
            },
            NodeKind::Xor { lhs, rhs } => NodeKind::Xor {
                lhs: resolve(lhs)?,
                rhs: resolve(rhs)?,
            },
            NodeKind::Shl { lhs, rhs } => NodeKind::Shl {
                lhs: resolve(lhs)?,
                rhs: resolve(rhs)?,
            },
            NodeKind::Shr { lhs, rhs } => NodeKind::Shr {
                lhs: resolve(lhs)?,
                rhs: resolve(rhs)?,
            },
            NodeKind::Rol { lhs, rhs } => NodeKind::Rol {
                lhs: resolve(lhs)?,
                rhs: resolve(rhs)?,
            },
            NodeKind::Ror { lhs, rhs } => NodeKind::Ror {
                lhs: resolve(lhs)?,
                rhs: resolve(rhs)?,
            },
            NodeKind::Not { operand } => NodeKind::Not {
                operand: resolve(operand)?,
            },
            NodeKind::Popcount { operand } => NodeKind::Popcount {
                operand: resolve(operand)?,
            },
            NodeKind::LeadingZeros { operand } => NodeKind::LeadingZeros {
                operand: resolve(operand)?,
            },
            NodeKind::TrailingZeros { operand } => NodeKind::TrailingZeros {
                operand: resolve(operand)?,
            },
            NodeKind::Eq { lhs, rhs } => NodeKind::Eq {
                lhs: resolve(lhs)?,
                rhs: resolve(rhs)?,
            },
            NodeKind::Ne { lhs, rhs } => NodeKind::Ne {
                lhs: resolve(lhs)?,
                rhs: resolve(rhs)?,
            },
            NodeKind::Lt { lhs, rhs } => NodeKind::Lt {
                lhs: resolve(lhs)?,
                rhs: resolve(rhs)?,
            },
            NodeKind::Le { lhs, rhs } => NodeKind::Le {
                lhs: resolve(lhs)?,
                rhs: resolve(rhs)?,
            },
            NodeKind::Gt { lhs, rhs } => NodeKind::Gt {
                lhs: resolve(lhs)?,
                rhs: resolve(rhs)?,
            },
            NodeKind::Ge { lhs, rhs } => NodeKind::Ge {
                lhs: resolve(lhs)?,
                rhs: resolve(rhs)?,
            },
            NodeKind::BoolAnd { lhs, rhs } => NodeKind::BoolAnd {
                lhs: resolve(lhs)?,
                rhs: resolve(rhs)?,
            },
            NodeKind::BoolOr { lhs, rhs } => NodeKind::BoolOr {
                lhs: resolve(lhs)?,
                rhs: resolve(rhs)?,
            },
            NodeKind::BoolNot { operand } => NodeKind::BoolNot {
                operand: resolve(operand)?,
            },
            NodeKind::Select {
                cond,
                true_val,
                false_val,
            } => NodeKind::Select {
                cond: resolve(cond)?,
                true_val: resolve(true_val)?,
                false_val: resolve(false_val)?,
            },
            NodeKind::Pack { array } => NodeKind::Pack {
                array: resolve(array)?,
            },
            NodeKind::ArrayCmpMask { array, scalar, op } => NodeKind::ArrayCmpMask {
                array: resolve(array)?,
                scalar: resolve(scalar)?,
                op: *op,
            },
            NodeKind::Return { value } => NodeKind::Return {
                value: resolve(value)?,
            },
            NodeKind::Load { ptr } => NodeKind::Load { ptr: resolve(ptr)? },
            NodeKind::Store { ptr, value } => NodeKind::Store {
                ptr: resolve(ptr)?,
                value: resolve(value)?,
            },
            NodeKind::ArrayAccess { base, index } => NodeKind::ArrayAccess {
                base: resolve(base)?,
                index: resolve(index)?,
            },
            NodeKind::FieldAccess { base, field } => NodeKind::FieldAccess {
                base: resolve(base)?,
                field: field.clone(),
            },
            NodeKind::Call { callee, args } => NodeKind::Call {
                callee: resolve(callee)?,
                args: args
                    .iter()
                    .map(|a| resolve(a))
                    .collect::<Result<Vec<_>, _>>()?,
            },
            NodeKind::Loop {
                body,
                termination,
                outputs,
                carried_inputs,
            } => NodeKind::Loop {
                body: body
                    .iter()
                    .map(|b| resolve(b))
                    .collect::<Result<Vec<_>, _>>()?,
                termination: resolve(termination)?,
                outputs: outputs
                    .iter()
                    .map(|o| resolve(o))
                    .collect::<Result<Vec<_>, _>>()?,
                carried_inputs: carried_inputs
                    .iter()
                    .map(|c| resolve(c))
                    .collect::<Result<Vec<_>, _>>()?,
            },
            // Passthrough -- no NodeId operands to rewrite
            NodeKind::Constant(_) => kind.clone(),
            NodeKind::Parameter { .. } => kind.clone(),
            NodeKind::Allocate { ty, count } => NodeKind::Allocate {
                ty: ty.clone(),
                count: resolve(count)?,
            },
            NodeKind::Deallocate { ptr } => NodeKind::Deallocate { ptr: resolve(ptr)? },
            NodeKind::Iterator { collection } => NodeKind::Iterator {
                collection: resolve(collection)?,
            },
            NodeKind::Intrinsic { name, args } => NodeKind::Intrinsic {
                name: name.clone(),
                args: args
                    .iter()
                    .map(|a| resolve(a))
                    .collect::<Result<Vec<_>, _>>()?,
            },
            NodeKind::ExternalCall { name, args } => NodeKind::ExternalCall {
                name: name.clone(),
                args: args
                    .iter()
                    .map(|a| resolve(a))
                    .collect::<Result<Vec<_>, _>>()?,
            },
        };

        Ok(new_kind)
    }

    /// Replace all uses of `old_id` with `new_id` in the function's arena.
    fn replace_all_uses(function: &mut Function, old_id: NodeId, new_id: NodeId) {
        // Collect all node IDs first, then modify
        let all_ids: Vec<NodeId> = function.arena.nodes().keys().copied().collect();
        for node_id in all_ids {
            if let Some(node) = function.arena.get_mut(node_id) {
                node.kind = Self::replace_in_kind(&node.kind, old_id, new_id);
            }
        }
    }

    /// Replace all occurrences of `old_id` with `new_id` in a NodeKind.
    fn replace_in_kind(kind: &NodeKind, old_id: NodeId, new_id: NodeId) -> NodeKind {
        let r = |id: &NodeId| if *id == old_id { new_id } else { *id };
        let rv = |ids: &[NodeId]| {
            ids.iter()
                .map(|id| if *id == old_id { new_id } else { *id })
                .collect()
        };

        match kind {
            NodeKind::Add { lhs, rhs } => NodeKind::Add {
                lhs: r(lhs),
                rhs: r(rhs),
            },
            NodeKind::Sub { lhs, rhs } => NodeKind::Sub {
                lhs: r(lhs),
                rhs: r(rhs),
            },
            NodeKind::Mul { lhs, rhs } => NodeKind::Mul {
                lhs: r(lhs),
                rhs: r(rhs),
            },
            NodeKind::Div { lhs, rhs } => NodeKind::Div {
                lhs: r(lhs),
                rhs: r(rhs),
            },
            NodeKind::Rem { lhs, rhs } => NodeKind::Rem {
                lhs: r(lhs),
                rhs: r(rhs),
            },
            NodeKind::Neg { operand } => NodeKind::Neg {
                operand: r(operand),
            },
            NodeKind::And { lhs, rhs } => NodeKind::And {
                lhs: r(lhs),
                rhs: r(rhs),
            },
            NodeKind::Or { lhs, rhs } => NodeKind::Or {
                lhs: r(lhs),
                rhs: r(rhs),
            },
            NodeKind::Xor { lhs, rhs } => NodeKind::Xor {
                lhs: r(lhs),
                rhs: r(rhs),
            },
            NodeKind::Shl { lhs, rhs } => NodeKind::Shl {
                lhs: r(lhs),
                rhs: r(rhs),
            },
            NodeKind::Shr { lhs, rhs } => NodeKind::Shr {
                lhs: r(lhs),
                rhs: r(rhs),
            },
            NodeKind::Rol { lhs, rhs } => NodeKind::Rol {
                lhs: r(lhs),
                rhs: r(rhs),
            },
            NodeKind::Ror { lhs, rhs } => NodeKind::Ror {
                lhs: r(lhs),
                rhs: r(rhs),
            },
            NodeKind::Not { operand } => NodeKind::Not {
                operand: r(operand),
            },
            NodeKind::Popcount { operand } => NodeKind::Popcount {
                operand: r(operand),
            },
            NodeKind::LeadingZeros { operand } => NodeKind::LeadingZeros {
                operand: r(operand),
            },
            NodeKind::TrailingZeros { operand } => NodeKind::TrailingZeros {
                operand: r(operand),
            },
            NodeKind::Eq { lhs, rhs } => NodeKind::Eq {
                lhs: r(lhs),
                rhs: r(rhs),
            },
            NodeKind::Ne { lhs, rhs } => NodeKind::Ne {
                lhs: r(lhs),
                rhs: r(rhs),
            },
            NodeKind::Lt { lhs, rhs } => NodeKind::Lt {
                lhs: r(lhs),
                rhs: r(rhs),
            },
            NodeKind::Le { lhs, rhs } => NodeKind::Le {
                lhs: r(lhs),
                rhs: r(rhs),
            },
            NodeKind::Gt { lhs, rhs } => NodeKind::Gt {
                lhs: r(lhs),
                rhs: r(rhs),
            },
            NodeKind::Ge { lhs, rhs } => NodeKind::Ge {
                lhs: r(lhs),
                rhs: r(rhs),
            },
            NodeKind::BoolAnd { lhs, rhs } => NodeKind::BoolAnd {
                lhs: r(lhs),
                rhs: r(rhs),
            },
            NodeKind::BoolOr { lhs, rhs } => NodeKind::BoolOr {
                lhs: r(lhs),
                rhs: r(rhs),
            },
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
            NodeKind::Pack { array } => NodeKind::Pack { array: r(array) },
            NodeKind::ArrayCmpMask { array, scalar, op } => NodeKind::ArrayCmpMask {
                array: r(array),
                scalar: r(scalar),
                op: *op,
            },
            NodeKind::Return { value } => NodeKind::Return { value: r(value) },
            NodeKind::Load { ptr } => NodeKind::Load { ptr: r(ptr) },
            NodeKind::Store { ptr, value } => NodeKind::Store {
                ptr: r(ptr),
                value: r(value),
            },
            NodeKind::ArrayAccess { base, index } => NodeKind::ArrayAccess {
                base: r(base),
                index: r(index),
            },
            NodeKind::FieldAccess { base, field } => NodeKind::FieldAccess {
                base: r(base),
                field: field.clone(),
            },
            NodeKind::Call { callee, args } => NodeKind::Call {
                callee: r(callee),
                args: rv(args),
            },
            NodeKind::Loop {
                body,
                termination,
                outputs,
                carried_inputs,
            } => NodeKind::Loop {
                body: rv(body),
                termination: r(termination),
                outputs: rv(outputs),
                carried_inputs: rv(carried_inputs),
            },
            NodeKind::Allocate { ty, count } => NodeKind::Allocate {
                ty: ty.clone(),
                count: r(count),
            },
            NodeKind::Deallocate { ptr } => NodeKind::Deallocate { ptr: r(ptr) },
            NodeKind::Iterator { collection } => NodeKind::Iterator {
                collection: r(collection),
            },
            NodeKind::Intrinsic { name, args } => NodeKind::Intrinsic {
                name: name.clone(),
                args: rv(args),
            },
            NodeKind::ExternalCall { name, args } => NodeKind::ExternalCall {
                name: name.clone(),
                args: rv(args),
            },
            NodeKind::Constant(_) | NodeKind::Parameter { .. } => kind.clone(),
        }
    }

    /// Collect all NodeIds referenced in the region's roles.
    fn collect_role_nodes(region: &RewriteRegion) -> BTreeSet<NodeId> {
        let mut nodes = BTreeSet::new();
        if let Some(roles) = &region.structural.roles {
            match roles {
                sir_transform::roles::RegionRoles::BooleanCollectionReduction {
                    collection,
                    accumulator,
                    result,
                } => {
                    nodes.insert(*collection);
                    if let Some(acc) = accumulator {
                        nodes.insert(*acc);
                    }
                    nodes.insert(*result);
                }
                sir_transform::roles::RegionRoles::PredicateCollectionReduction {
                    collection,
                    scalar,
                    operator,
                    accumulator,
                    result,
                } => {
                    nodes.insert(*collection);
                    nodes.insert(*scalar);
                    nodes.insert(*operator);
                    if let Some(acc) = accumulator {
                        nodes.insert(*acc);
                    }
                    nodes.insert(*result);
                }
                sir_transform::roles::RegionRoles::ArithmeticOperation {
                    operator_node,
                    lhs,
                    rhs,
                    result,
                } => {
                    nodes.insert(*operator_node);
                    nodes.insert(*lhs);
                    nodes.insert(*rhs);
                    nodes.insert(*result);
                }
                sir_transform::roles::RegionRoles::PositionSearch {
                    collection,
                    scalar,
                    result,
                } => {
                    if let Some(c) = collection {
                        nodes.insert(*c);
                    }
                    if let Some(s) = scalar {
                        nodes.insert(*s);
                    }
                    nodes.insert(*result);
                }
            }
        }
        nodes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_is_instantiable() {
        let _builder = RewriteBuilder;
    }
}
