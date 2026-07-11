//! SIR Nodes — Graph data structures for Semantic IR.
//!
//! This crate defines the NodeKind enum, Node struct, NodeArena storage,
//! Function, and Module types that form the core IR representation.

pub mod arena;
pub mod function;
pub mod module;
pub mod node;
pub mod node_kind;

pub use arena::*;
pub use function::*;
pub use module::*;
pub use node::*;
pub use node_kind::*;
