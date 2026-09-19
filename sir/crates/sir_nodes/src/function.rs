use serde::{Deserialize, Serialize};

use sir_types::{NodeId, Type};

use crate::{Node, NodeArena};

/// Metadata for a function parameter.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Param {
    /// The parameter's name (for debugging/printing).
    pub name: String,
    /// The parameter's type.
    pub ty: Type,
}

impl Param {
    /// Create a new parameter descriptor.
    pub fn new(name: impl Into<String>, ty: Type) -> Self {
        Self {
            name: name.into(),
            ty,
        }
    }
}

/// A function in SIR form.
///
/// A `Function` owns its node arena and provides methods for adding
/// parameters, inserting nodes, and setting the return value.
///
/// # SSA properties
///
/// - Every `NodeId` in the arena is unique (enforced by `NodeArena::insert`).
/// - Parameters are stored both as `Param` entries in `params` and as
///   `Parameter` nodes in the arena for uniform graph traversal.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Function {
    /// The function's name.
    pub name: String,
    /// The function's parameters (in order).
    pub params: Vec<Param>,
    /// The function's return type.
    pub return_ty: Type,
    /// The arena containing all nodes in this function.
    pub arena: NodeArena,
    /// The optional return node. Set by `set_return`.
    pub return_node: Option<NodeId>,
}

impl Function {
    /// Create a new function with the given name and return type.
    pub fn new(name: impl Into<String>, return_ty: Type) -> Self {
        Self {
            name: name.into(),
            params: Vec::new(),
            return_ty,
            arena: NodeArena::new(),
            return_node: None,
        }
    }
    /// Insert a node into the function's arena.
    ///
    /// Returns `None` on success. Returns `Some(old_node)` if a node with the
    /// same `NodeId` already exists (SSA violation).
    pub fn insert_node(&mut self, node: Node) -> Option<Node> {
        self.arena.insert(node)
    }

    /// Get a reference to a node by ID.
    pub fn get_node(&self, id: NodeId) -> Option<&Node> {
        self.arena.get(id)
    }
    /// Return the number of nodes in the function.
    pub fn node_count(&self) -> usize {
        self.arena.len()
    }
}
