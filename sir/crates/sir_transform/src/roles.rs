use sir_types::NodeId;

/// Semantic roles assigned by pattern recognizers during semantic analysis.
///
/// Each variant corresponds to a recognized computation pattern.
/// The recognizer records which SIR nodes fill each role.
/// Downstream phases (rewrite) consume these roles without rediscovering them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegionRoles {
    /// A loop that iterates over a boolean array and counts matching elements.
    /// Recognized as: MembershipTraversal + CardinalityReduction.
    BooleanCollectionReduction {
        /// The boolean array being iterated (e.g., `board` in BS001).
        collection: NodeId,
        /// The accumulator carrying the running count (None if zero-initialized).
        accumulator: Option<NodeId>,
        /// The final count produced by the region.
        result: NodeId,
    },
    PredicateCollectionReduction {
        /// The array being iterated.
        collection: NodeId,
        /// The scalar value being compared against.
        scalar: NodeId,
        /// The comparison operation used.
        operator: sir_types::NodeId, // To identify the node, actually we just need the operator.
        /// The accumulator carrying the running count.
        accumulator: Option<NodeId>,
        /// The final count produced by the region.
        result: NodeId,
    },
    /// An arithmetic expression recognized as having an optimized form.
    ArithmeticOperation {
        /// The node representing the operator.
        operator_node: NodeId,
        /// The left operand.
        lhs: NodeId,
        /// The right operand (e.g. the constant divisor).
        rhs: NodeId,
        /// The node representing the final output result to replace.
        result: NodeId,
    },
}
