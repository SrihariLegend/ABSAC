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
    /// Return a short description of this node for debugging.
    pub fn describe(&self) -> String {
        format!(
            "{}: {} = {} (effects: {})",
            self.id, self.ty, self.kind, self.effects
        )
    }
}
