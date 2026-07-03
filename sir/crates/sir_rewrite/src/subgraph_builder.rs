use sir_nodes::{Node, NodeKind};
use sir_types::{ConstantData, Effects, NodeId, Span, Type};

use crate::detached_arena::DetachedArena;
use crate::local_id::LocalNodeId;
use crate::patch::{ReplacementPatch, ReplacementValue};

/// Builds replacement SIR in a detached arena.
///
/// Mirrors `sir_builder::Builder` in API surface but targets `LocalNodeId`
/// instead of global `NodeId`. Every method corresponds 1:1 with a `NodeKind`
/// variant — no hidden synthesis, no macros, no DSLs.
///
/// Consumed by value in `RewriteRecipe::build_patch`. Recipes call
/// constructor methods to build the replacement subgraph, then seal
/// the builder with `finish()` to produce a `ReplacementPatch`.
pub struct SubgraphBuilder {
    arena: DetachedArena,
    next_local_id: u64,
}

impl SubgraphBuilder {
    /// Create a new subgraph builder with an empty detached arena.
    pub fn new() -> Self {
        Self {
            arena: DetachedArena::new(),
            next_local_id: 0,
        }
    }

    // ── Internal helpers ────────────────────────────────────────

    fn next_id(&mut self) -> LocalNodeId {
        let id = LocalNodeId::new(self.next_local_id);
        self.next_local_id += 1;
        id
    }

    fn alloc_node(
        &mut self,
        kind: NodeKind,
        ty: Type,
        effects: Effects,
        _span: Span,
    ) -> LocalNodeId {
        let local_id = self.next_id();
        // The Node's internal `id` field is a placeholder — LocalNodeId is authoritative.
        let node = Node::new(sir_types::NodeId::new(local_id.as_u64()), kind, ty, effects, _span);
        self.arena.insert(local_id, node);
        local_id
    }

    #[allow(dead_code)]
    fn compute_effects(kind: &NodeKind) -> Effects {
        match kind {
            NodeKind::Load { .. } => Effects::READ_MEMORY,
            NodeKind::Store { .. } => Effects::WRITE_MEMORY,
            NodeKind::Allocate { .. } => Effects::ALLOCATE,
            NodeKind::Deallocate { .. } => Effects::WRITE_MEMORY,
            NodeKind::Iterator { .. } => Effects::READ_MEMORY,
            NodeKind::Call { .. } => Effects::READ_MEMORY | Effects::WRITE_MEMORY,
            NodeKind::Loop { .. } => {
                Effects::READ_MEMORY | Effects::WRITE_MEMORY
            }
            _ => Effects::empty(),
        }
    }

    fn get_type(&self, id: LocalNodeId) -> Option<Type> {
        self.arena.get(id).map(|n| n.ty.clone())
    }

    // ── Finalization ────────────────────────────────────────────

    /// Seal the builder and produce a `ReplacementPatch`.
    /// Consumes self — the builder cannot be used after finishing.
    pub fn finish(self, replacements: Vec<ReplacementValue>) -> ReplacementPatch {
        let roots: Vec<LocalNodeId> = self.arena.nodes().map(|n| {
            // Every node in the arena is a potential root.
            // In practice, roots are the nodes not referenced by other nodes
            // in the same arena — but for v0.1 we include all nodes.
            LocalNodeId::new(n.id.as_u64())
        }).collect();

        ReplacementPatch {
            arena: self.arena,
            roots,
            replacements,
        }
    }

    // ── Value nodes ─────────────────────────────────────────────

    /// Create a constant node.
    pub fn constant(&mut self, data: ConstantData, ty: Type, span: Span) -> LocalNodeId {
        self.alloc_node(NodeKind::Constant(data), ty, Effects::empty(), span)
    }

    // ── Arithmetic (binary) ─────────────────────────────────────

    pub fn add(&mut self, lhs: LocalNodeId, rhs: LocalNodeId, span: Span) -> LocalNodeId {
        let ty = self.get_type(lhs).unwrap_or(Type::i32());
        self.alloc_node(NodeKind::Add { lhs: NodeId::new(lhs.as_u64()), rhs: NodeId::new(rhs.as_u64()) }, ty, Effects::empty(), span)
    }

    pub fn sub(&mut self, lhs: LocalNodeId, rhs: LocalNodeId, span: Span) -> LocalNodeId {
        let ty = self.get_type(lhs).unwrap_or(Type::i32());
        self.alloc_node(NodeKind::Sub { lhs: NodeId::new(lhs.as_u64()), rhs: NodeId::new(rhs.as_u64()) }, ty, Effects::empty(), span)
    }

    pub fn mul(&mut self, lhs: LocalNodeId, rhs: LocalNodeId, span: Span) -> LocalNodeId {
        let ty = self.get_type(lhs).unwrap_or(Type::i32());
        self.alloc_node(NodeKind::Mul { lhs: NodeId::new(lhs.as_u64()), rhs: NodeId::new(rhs.as_u64()) }, ty, Effects::empty(), span)
    }

    // ── Bitwise (unary) ─────────────────────────────────────────

    pub fn bit_not(&mut self, operand: LocalNodeId, span: Span) -> LocalNodeId {
        let ty = self.get_type(operand).unwrap_or(Type::i32());
        self.alloc_node(NodeKind::Not { operand: NodeId::new(operand.as_u64()) }, ty, Effects::empty(), span)
    }

    pub fn popcount(&mut self, operand: LocalNodeId, span: Span) -> LocalNodeId {
        let ty = self.get_type(operand).unwrap_or(Type::i32());
        self.alloc_node(NodeKind::Popcount { operand: NodeId::new(operand.as_u64()) }, ty, Effects::empty(), span)
    }

    // ── Pack ────────────────────────────────────────────────────

    pub fn pack(&mut self, array: LocalNodeId, span: Span) -> LocalNodeId {
        let width = match self.get_type(array) {
            Some(Type::Array { length, .. }) => length,
            _ => 64, // default for BS001
        };
        self.alloc_node(
            NodeKind::Pack { array: NodeId::new(array.as_u64()) },
            Type::BitVector { width },
            Effects::empty(),
            span,
        )
    }

    // ── Select ──────────────────────────────────────────────────

    pub fn select(
        &mut self,
        cond: LocalNodeId,
        true_val: LocalNodeId,
        false_val: LocalNodeId,
        span: Span,
    ) -> LocalNodeId {
        let ty = self.get_type(true_val).unwrap_or(Type::i32());
        self.alloc_node(
            NodeKind::Select {
                cond: NodeId::new(cond.as_u64()),
                true_val: NodeId::new(true_val.as_u64()),
                false_val: NodeId::new(false_val.as_u64()),
            },
            ty,
            Effects::empty(),
            span,
        )
    }

    // ── Return ──────────────────────────────────────────────────

    pub fn return_value(&mut self, value: LocalNodeId, span: Span) -> LocalNodeId {
        self.alloc_node(
            NodeKind::Return { value: NodeId::new(value.as_u64()) },
            Type::Unit,
            Effects::empty(),
            span,
        )
    }
}

impl Default for SubgraphBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_simple_pack_popcount_chain() {
        let mut b = SubgraphBuilder::new();
        // Simulate building: pack(array) -> popcount(packed)
        // We can't reference external nodes (that's the recipe's job via rewrite region),
        // so we test internal construction with constants.
        let c = b.constant(ConstantData::u64(0), Type::BitVector { width: 64 }, Span::unknown());
        let pop = b.popcount(c, Span::unknown());

        // Verify the arena has nodes
        let arena = &b.finish(vec![]).arena;
        assert_eq!(arena.len(), 2);
        assert!(arena.contains(c));
        assert!(arena.contains(pop));
    }

    #[test]
    fn popcount_of_bitvector() {
        let mut b = SubgraphBuilder::new();
        let bv = LocalNodeId::new(0);
        // We can't type-check without arena lookup, but construction succeeds
        let pop = b.popcount(bv, Span::unknown());
        assert_eq!(pop.as_u64(), 0); // first allocated node
    }
}
