use serde::{Deserialize, Serialize};

use sir_types::{Effects, Metadata, NodeId, Span, Type};

use crate::NodeKind;

/// A single node in the SIR graph.
///
/// Each node has a globally unique `id`, a `kind` determining its operation,
/// a result `ty`, an `effects` bitmask, optional `metadata`, and a source `span`.
///
/// All fields are public to allow direct inspection by verifiers, printers,
/// and optimization passes. For construction, use the `Builder` API from
/// `sir_builder`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub kind: NodeKind,
    pub ty: Type,
    pub effects: Effects,
    pub metadata: Metadata,
    pub span: Span,
}

impl Node {
    /// Create a new node.
    pub fn new(id: NodeId, kind: NodeKind, ty: Type, effects: Effects, span: Span) -> Self {
        Self {
            id,
            kind,
            ty,
            effects,
            metadata: Metadata::new(),
            span,
        }
    }

    /// Create a new node with the given metadata.
    pub fn with_metadata(
        id: NodeId,
        kind: NodeKind,
        ty: Type,
        effects: Effects,
        metadata: Metadata,
        span: Span,
    ) -> Self {
        Self {
            id,
            kind,
            ty,
            effects,
            metadata,
            span,
        }
    }

    /// Return a short description of this node for debugging.
    pub fn describe(&self) -> String {
        format!(
            "{}: {} = {} (effects: {})",
            self.id, self.ty, self.kind, self.effects
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sir_types::ConstantData;

    #[test]
    fn node_construction() {
        let node = Node::new(
            NodeId::new(1),
            NodeKind::Constant(ConstantData::i32(42)),
            sir_types::Type::i32(),
            Effects::empty(),
            Span::unknown(),
        );
        assert_eq!(node.id, NodeId::new(1));
        assert_eq!(node.ty, sir_types::Type::i32());
        assert!(node.effects.is_pure());
    }

    #[test]
    fn node_with_metadata() {
        let mut meta = Metadata::new();
        meta.insert("debug_name", "answer");
        let node = Node::with_metadata(
            NodeId::new(2),
            NodeKind::Constant(ConstantData::boolean(true)),
            sir_types::Type::Bool,
            Effects::empty(),
            meta.clone(),
            Span::unknown(),
        );
        assert_eq!(node.metadata, meta);
        assert_eq!(node.metadata.get("debug_name"), Some("answer"));
    }

    #[test]
    fn node_describe() {
        let node = Node::new(
            NodeId::new(0),
            NodeKind::Parameter { index: 0 },
            sir_types::Type::i64(),
            Effects::empty(),
            Span::unknown(),
        );
        let desc = node.describe();
        assert!(desc.contains("%0"));
        assert!(desc.contains("i64"));
        assert!(desc.contains("Parameter"));
    }

    #[test]
    fn serde_roundtrip() {
        let node = Node::new(
            NodeId::new(3),
            NodeKind::Add {
                lhs: NodeId::new(0),
                rhs: NodeId::new(1),
            },
            sir_types::Type::i32(),
            Effects::empty(),
            Span::unknown(),
        );
        let json = serde_json::to_string(&node).unwrap();
        let parsed: Node = serde_json::from_str(&json).unwrap();
        assert_eq!(node, parsed);
    }
}
