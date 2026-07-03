use serde::{Deserialize, Serialize};

use sir_types::{ConstantData, NodeId, Type};

/// The kind of a node in the SIR graph.
///
/// `NodeKind` enumerates every operation the IR can represent. Each variant
/// carries the `NodeId`s of its operands (for dataflow edges) and any
/// immediate data (types, field names, string constants).
///
/// The IR is functional and in SSA form:
/// - Selection uses `Select`, not `If` (branchless by construction).
/// - Loops use `Loop` with explicit carried inputs/outputs, not phi nodes.
/// - `Store` produces `Unit` (it's a side effect, not a value).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum NodeKind {
    // ── Values ──────────────────────────────────────────────
    /// A compile-time constant.
    Constant(ConstantData),
    /// A function parameter. The index refers to the function's `params` list.
    Parameter { index: usize },

    // ── Arithmetic (binary) ─────────────────────────────────
    /// Integer or floating-point addition: `lhs + rhs`.
    Add { lhs: NodeId, rhs: NodeId },
    /// Integer or floating-point subtraction: `lhs - rhs`.
    Sub { lhs: NodeId, rhs: NodeId },
    /// Integer or floating-point multiplication: `lhs * rhs`.
    Mul { lhs: NodeId, rhs: NodeId },
    /// Integer or floating-point division: `lhs / rhs`.
    Div { lhs: NodeId, rhs: NodeId },
    /// Integer remainder: `lhs % rhs`.
    Rem { lhs: NodeId, rhs: NodeId },

    // ── Arithmetic (unary) ──────────────────────────────────
    /// Negation: `-operand`.
    Neg { operand: NodeId },

    // ── Bitwise (binary) ────────────────────────────────────
    /// Bitwise AND: `lhs & rhs`.
    And { lhs: NodeId, rhs: NodeId },
    /// Bitwise OR: `lhs | rhs`.
    Or { lhs: NodeId, rhs: NodeId },
    /// Bitwise XOR: `lhs ^ rhs`.
    Xor { lhs: NodeId, rhs: NodeId },
    /// Left shift: `lhs << rhs`.
    Shl { lhs: NodeId, rhs: NodeId },
    /// Right shift (logical or arithmetic, determined by type signedness): `lhs >> rhs`.
    Shr { lhs: NodeId, rhs: NodeId },
    /// Left rotation: `lhs.rotate_left(rhs)`.
    Rol { lhs: NodeId, rhs: NodeId },
    /// Right rotation: `lhs.rotate_right(rhs)`.
    Ror { lhs: NodeId, rhs: NodeId },

    // ── Bitwise (unary) ─────────────────────────────────────
    /// Bitwise NOT: `!operand`.
    Not { operand: NodeId },
    /// Population count (number of set bits): `operand.count_ones()`.
    Popcount { operand: NodeId },
    /// Count leading zero bits: `operand.leading_zeros()`.
    LeadingZeros { operand: NodeId },
    /// Count trailing zero bits: `operand.trailing_zeros()`.
    TrailingZeros { operand: NodeId },

    // ── Data conversion ─────────────────────────────────────
    /// Pack a boolean array into a bitvector.
    /// Maps `bool[n]` to `bv<n>` where bit i = array[i].
    Pack { array: NodeId },

    // ── Comparisons ─────────────────────────────────────────
    /// Equality: `lhs == rhs`. Result type is always `Bool`.
    Eq { lhs: NodeId, rhs: NodeId },
    /// Inequality: `lhs != rhs`.
    Ne { lhs: NodeId, rhs: NodeId },
    /// Less than: `lhs < rhs`.
    Lt { lhs: NodeId, rhs: NodeId },
    /// Less than or equal: `lhs <= rhs`.
    Le { lhs: NodeId, rhs: NodeId },
    /// Greater than: `lhs > rhs`.
    Gt { lhs: NodeId, rhs: NodeId },
    /// Greater than or equal: `lhs >= rhs`.
    Ge { lhs: NodeId, rhs: NodeId },

    // ── Boolean ─────────────────────────────────────────────
    /// Short-circuit boolean AND: `lhs && rhs`. Both operands must be `Bool`.
    BoolAnd { lhs: NodeId, rhs: NodeId },
    /// Short-circuit boolean OR: `lhs || rhs`.
    BoolOr { lhs: NodeId, rhs: NodeId },
    /// Boolean NOT: `!operand`.
    BoolNot { operand: NodeId },

    // ── Selection ───────────────────────────────────────────
    /// Branchless conditional select: `cond ? true_val : false_val`.
    /// This replaces `if`/`else` constructs.
    Select {
        cond: NodeId,
        true_val: NodeId,
        false_val: NodeId,
    },

    // ── Memory ──────────────────────────────────────────────
    /// Load a value from a pointer.
    Load { ptr: NodeId },
    /// Store a value to a pointer. Returns `Unit`.
    Store { ptr: NodeId, value: NodeId },
    /// Allocate memory of the given type. `count` is the number of elements
    /// (for arrays) and must be an integer.
    Allocate { ty: Type, count: NodeId },
    /// Deallocate memory at the given pointer.
    Deallocate { ptr: NodeId },
    /// Access a named field of a struct.
    FieldAccess {
        base: NodeId,
        field: String,
    },
    /// Access an element of an array or slice by index.
    ArrayAccess {
        base: NodeId,
        index: NodeId,
    },

    // ── Calls ───────────────────────────────────────────────
    /// Call a function (local or known).
    Call {
        callee: NodeId,
        args: Vec<NodeId>,
    },
    /// Call a compiler intrinsic (e.g., `ctpop`, `ctlz`).
    Intrinsic {
        name: String,
        args: Vec<NodeId>,
    },
    /// Call an externally-defined function (FFI).
    ExternalCall {
        name: String,
        args: Vec<NodeId>,
    },

    // ── Loops ───────────────────────────────────────────────
    /// A loop construct. The body is a subgraph of nodes.
    /// `carried_inputs` are values from outside the loop fed into each iteration;
    /// `outputs` are the final values after the loop terminates.
    /// `termination` must be `Bool` — the loop exits when it evaluates to `false`.
    Loop {
        /// Nodes forming the loop body (subgraph).
        body: Vec<NodeId>,
        /// Boolean condition: loop continues while this is `true`.
        termination: NodeId,
        /// Which body nodes become the loop's results after termination.
        outputs: Vec<NodeId>,
        /// Values from outside the loop that feed into each iteration.
        carried_inputs: Vec<NodeId>,
    },
    /// An iterator node producing successive elements from a collection.
    Iterator {
        collection: NodeId,
    },

    // ── Control flow ────────────────────────────────────────
    /// Return a value from the function.
    Return { value: NodeId },
}

impl NodeKind {
    /// Return a human-readable name for this node kind.
    pub fn kind_name(&self) -> &'static str {
        match self {
            NodeKind::Constant(_) => "Constant",
            NodeKind::Parameter { .. } => "Parameter",
            NodeKind::Add { .. } => "Add",
            NodeKind::Sub { .. } => "Sub",
            NodeKind::Mul { .. } => "Mul",
            NodeKind::Div { .. } => "Div",
            NodeKind::Rem { .. } => "Rem",
            NodeKind::Neg { .. } => "Neg",
            NodeKind::And { .. } => "And",
            NodeKind::Or { .. } => "Or",
            NodeKind::Xor { .. } => "Xor",
            NodeKind::Shl { .. } => "Shl",
            NodeKind::Shr { .. } => "Shr",
            NodeKind::Rol { .. } => "Rol",
            NodeKind::Ror { .. } => "Ror",
            NodeKind::Not { .. } => "Not",
            NodeKind::Popcount { .. } => "Popcount",
            NodeKind::LeadingZeros { .. } => "LeadingZeros",
            NodeKind::TrailingZeros { .. } => "TrailingZeros",
            NodeKind::Pack { .. } => "Pack",
            NodeKind::Eq { .. } => "Eq",
            NodeKind::Ne { .. } => "Ne",
            NodeKind::Lt { .. } => "Lt",
            NodeKind::Le { .. } => "Le",
            NodeKind::Gt { .. } => "Gt",
            NodeKind::Ge { .. } => "Ge",
            NodeKind::BoolAnd { .. } => "BoolAnd",
            NodeKind::BoolOr { .. } => "BoolOr",
            NodeKind::BoolNot { .. } => "BoolNot",
            NodeKind::Select { .. } => "Select",
            NodeKind::Load { .. } => "Load",
            NodeKind::Store { .. } => "Store",
            NodeKind::Allocate { .. } => "Allocate",
            NodeKind::Deallocate { .. } => "Deallocate",
            NodeKind::FieldAccess { .. } => "FieldAccess",
            NodeKind::ArrayAccess { .. } => "ArrayAccess",
            NodeKind::Call { .. } => "Call",
            NodeKind::Intrinsic { .. } => "Intrinsic",
            NodeKind::ExternalCall { .. } => "ExternalCall",
            NodeKind::Loop { .. } => "Loop",
            NodeKind::Iterator { .. } => "Iterator",
            NodeKind::Return { .. } => "Return",
        }
    }

    /// Collect all NodeId references from this node kind.
    /// Used by the verifier for reference checking and cycle detection.
    pub fn input_nodes(&self) -> Vec<NodeId> {
        match self {
            NodeKind::Constant(_) => vec![],
            NodeKind::Parameter { .. } => vec![],
            NodeKind::Add { lhs, rhs }
            | NodeKind::Sub { lhs, rhs }
            | NodeKind::Mul { lhs, rhs }
            | NodeKind::Div { lhs, rhs }
            | NodeKind::Rem { lhs, rhs }
            | NodeKind::And { lhs, rhs }
            | NodeKind::Or { lhs, rhs }
            | NodeKind::Xor { lhs, rhs }
            | NodeKind::Shl { lhs, rhs }
            | NodeKind::Shr { lhs, rhs }
            | NodeKind::Rol { lhs, rhs }
            | NodeKind::Ror { lhs, rhs }
            | NodeKind::Eq { lhs, rhs }
            | NodeKind::Ne { lhs, rhs }
            | NodeKind::Lt { lhs, rhs }
            | NodeKind::Le { lhs, rhs }
            | NodeKind::Gt { lhs, rhs }
            | NodeKind::Ge { lhs, rhs }
            | NodeKind::BoolAnd { lhs, rhs }
            | NodeKind::BoolOr { lhs, rhs } => vec![*lhs, *rhs],
            NodeKind::Neg { operand }
            | NodeKind::Not { operand }
            | NodeKind::Popcount { operand }
            | NodeKind::LeadingZeros { operand }
            | NodeKind::TrailingZeros { operand }
            | NodeKind::BoolNot { operand }
            | NodeKind::Load { ptr: operand }
            | NodeKind::Deallocate { ptr: operand } => vec![*operand],
            NodeKind::Pack { array } => vec![*array],
            NodeKind::Select {
                cond,
                true_val,
                false_val,
            } => vec![*cond, *true_val, *false_val],
            NodeKind::Store { ptr, value } => vec![*ptr, *value],
            NodeKind::Allocate { count, .. } => vec![*count],
            NodeKind::FieldAccess { base, .. } => vec![*base],
            NodeKind::ArrayAccess { base, index } => vec![*base, *index],
            NodeKind::Call { callee, args } => {
                let mut v = vec![*callee];
                v.extend(args);
                v
            }
            NodeKind::Intrinsic { args, .. } | NodeKind::ExternalCall { args, .. } => args.clone(),
            NodeKind::Loop {
                body,
                termination,
                outputs,
                carried_inputs,
            } => {
                let mut v = body.clone();
                v.push(*termination);
                v.extend(outputs);
                v.extend(carried_inputs);
                v
            }
            NodeKind::Iterator { collection } => vec![*collection],
            NodeKind::Return { value } => vec![*value],
        }
    }
}

impl std::fmt::Display for NodeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.kind_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nid(v: u64) -> NodeId {
        NodeId::new(v)
    }

    #[test]
    fn kind_name_for_all_variants() {
        assert_eq!(
            NodeKind::Add {
                lhs: nid(0),
                rhs: nid(1)
            }
            .kind_name(),
            "Add"
        );
        assert_eq!(NodeKind::Constant(ConstantData::Unit).kind_name(), "Constant");
        assert_eq!(NodeKind::Parameter { index: 0 }.kind_name(), "Parameter");
        assert_eq!(NodeKind::Select { cond: nid(0), true_val: nid(1), false_val: nid(2) }.kind_name(), "Select");
        assert_eq!(
            NodeKind::Loop {
                body: vec![],
                termination: nid(0),
                outputs: vec![],
                carried_inputs: vec![],
            }
            .kind_name(),
            "Loop"
        );
    }

    #[test]
    fn input_nodes_binary() {
        let add = NodeKind::Add {
            lhs: nid(10),
            rhs: nid(20),
        };
        assert_eq!(add.input_nodes(), vec![nid(10), nid(20)]);
    }

    #[test]
    fn input_nodes_unary() {
        let neg = NodeKind::Neg { operand: nid(5) };
        assert_eq!(neg.input_nodes(), vec![nid(5)]);
    }

    #[test]
    fn input_nodes_constant_is_empty() {
        let c = NodeKind::Constant(ConstantData::i32(42));
        assert!(c.input_nodes().is_empty());
    }

    #[test]
    fn input_nodes_select() {
        let sel = NodeKind::Select {
            cond: nid(0),
            true_val: nid(1),
            false_val: nid(2),
        };
        assert_eq!(sel.input_nodes(), vec![nid(0), nid(1), nid(2)]);
    }

    #[test]
    fn input_nodes_call() {
        let call = NodeKind::Call {
            callee: nid(100),
            args: vec![nid(1), nid(2), nid(3)],
        };
        assert_eq!(call.input_nodes(), vec![nid(100), nid(1), nid(2), nid(3)]);
    }

    #[test]
    fn input_nodes_return() {
        let ret = NodeKind::Return { value: nid(42) };
        assert_eq!(ret.input_nodes(), vec![nid(42)]);
    }

    #[test]
    fn serde_roundtrip() {
        let kind = NodeKind::Add {
            lhs: nid(0),
            rhs: nid(1),
        };
        let json = serde_json::to_string(&kind).unwrap();
        let parsed: NodeKind = serde_json::from_str(&json).unwrap();
        assert_eq!(kind, parsed);
    }
}
