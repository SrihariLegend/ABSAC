use serde::{Deserialize, Serialize};

use sir_types::{Effects, Metadata, NodeId, Span, Type};

use crate::{Node, NodeArena, NodeKind};

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

    /// Add a parameter to the function.
    ///
    /// This creates both a `Param` entry in `self.params` and a `Parameter` node
    /// in the arena. Returns the `NodeId` of the parameter node for use in
    /// subsequent graph construction.
    pub fn add_param(
        &mut self,
        name: impl Into<String>,
        ty: Type,
        span: Span,
    ) -> NodeId {
        let index = self.params.len();
        let name = name.into();
        self.params.push(Param::new(name.clone(), ty.clone()));

        let id = NodeId::new(index as u64);
        let node = Node::with_metadata(
            id,
            NodeKind::Parameter { index },
            ty,
            Effects::empty(),
            {
                let mut meta = Metadata::new();
                meta.insert("param_name", name);
                meta
            },
            span,
        );
        self.arena.insert(node);
        id
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

    /// Set the return node for this function.
    ///
    /// Panics if a return node is already set (use `try_set_return` for a fallible version).
    pub fn set_return(&mut self, node_id: NodeId) {
        assert!(
            self.return_node.is_none(),
            "Return node already set for function '{}'",
            self.name
        );
        self.return_node = Some(node_id);
    }

    /// Try to set the return node. Returns Err if one is already set.
    pub fn try_set_return(&mut self, node_id: NodeId) -> Result<(), &'static str> {
        if self.return_node.is_some() {
            Err("Return node already set")
        } else {
            self.return_node = Some(node_id);
            Ok(())
        }
    }

    /// Return the number of nodes in the function.
    pub fn node_count(&self) -> usize {
        self.arena.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sir_types::{ConstantData, IntegerWidth, OverflowBehavior};

    fn i32_type() -> Type {
        Type::Integer {
            width: IntegerWidth::I32,
            signed: true,
            overflow: OverflowBehavior::Wrapping,
        }
    }

    #[test]
    fn function_creation() {
        let func = Function::new("add", i32_type());
        assert_eq!(func.name, "add");
        assert_eq!(func.return_ty, i32_type());
        assert!(func.params.is_empty());
        assert!(func.arena.is_empty());
        assert!(func.return_node.is_none());
    }

    #[test]
    fn add_parameters() {
        let mut func = Function::new("add", i32_type());
        let a = func.add_param("a", i32_type(), Span::unknown());
        let b = func.add_param("b", i32_type(), Span::unknown());

        assert_eq!(func.params.len(), 2);
        assert_eq!(func.params[0].name, "a");
        assert_eq!(func.params[1].name, "b");
        assert_eq!(a, NodeId::new(0));
        assert_eq!(b, NodeId::new(1));

        // Parameters should be stored as nodes in the arena.
        let param_a = func.get_node(a).unwrap();
        assert!(matches!(param_a.kind, NodeKind::Parameter { index: 0 }));
        assert_eq!(param_a.metadata.get("param_name"), Some("a"));
    }

    #[test]
    fn insert_and_retrieve_node() {
        let mut func = Function::new("test", Type::Unit);
        let node = Node::new(
            NodeId::new(10),
            NodeKind::Constant(ConstantData::i32(42)),
            i32_type(),
            Effects::empty(),
            Span::unknown(),
        );
        assert!(func.insert_node(node).is_none());
        assert_eq!(func.node_count(), 1);
        assert!(func.get_node(NodeId::new(10)).is_some());
    }

    #[test]
    fn set_return() {
        let mut func = Function::new("f", i32_type());
        func.set_return(NodeId::new(5));
        assert_eq!(func.return_node, Some(NodeId::new(5)));
    }

    #[test]
    #[should_panic]
    fn double_set_return_panics() {
        let mut func = Function::new("f", i32_type());
        func.set_return(NodeId::new(1));
        func.set_return(NodeId::new(2)); // panics
    }

    #[test]
    fn try_set_return() {
        let mut func = Function::new("f", i32_type());
        assert!(func.try_set_return(NodeId::new(1)).is_ok());
        assert!(func.try_set_return(NodeId::new(2)).is_err());
    }

    #[test]
    fn serde_roundtrip() {
        let mut func = Function::new("add", i32_type());
        func.add_param("a", i32_type(), Span::unknown());
        func.add_param("b", i32_type(), Span::unknown());

        let json = serde_json::to_string(&func).unwrap();
        let parsed: Function = serde_json::from_str(&json).unwrap();
        assert_eq!(func.name, parsed.name);
        assert_eq!(func.params.len(), parsed.params.len());
        assert_eq!(func.return_ty, parsed.return_ty);
    }
}
