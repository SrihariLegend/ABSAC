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
    /// A loop that searches for a specific condition (e.g. FirstTrue).
    PositionSearch {
        /// The collection being searched (e.g. `board`). Optional for scalar searches (TZCNT).
        collection: Option<NodeId>,
        /// The scalar being searched (e.g. `x` in TZCNT). Optional for array searches.
        scalar: Option<NodeId>,
        /// The result node that produces the found index/count.
        result: NodeId,
    },
    /// A bitwise operation corresponding to mask algebra.
    MaskOperation {
        /// The operand representing the bitmask.
        operand: NodeId,
        /// The node representing the final output result to replace.
        result: NodeId,
    },
    /// A loop that iteratively clears bits to count or traverse them.
    SetIteration {
        /// The integer value being iterated over.
        set_value: NodeId,
        /// The loop node itself.
        result: NodeId,
    },
    /// A permutation of bit positions inside a machine word (rotate, byte
    /// swap, bit reversal). Recognized as a whole expression — the recipe
    /// replaces the entire permutation result, never a constituent shift or
    /// mask subexpression.
    BitPermutation {
        /// The value being permuted.
        operand: NodeId,
        /// The node producing the final permuted result (the top Or).
        result: NodeId,
        /// Which permutation family and its parameters (width, direction,
        /// shift amount node for rotates).
        kind: PermutationKind,
    },
}


/// Direction of a circular (rotate) permutation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ShiftDirection {
    /// `(x << k) | (x >> (w - k))` — rotate left.
    Left,
    /// `(x >> k) | (x << (w - k))` — rotate right.
    Right,
}

/// The family of a recognized bit-position permutation, with the parameters
/// the recipe needs to select the instruction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PermutationKind {
    /// A circular rotation by a runtime amount.
    Circular {
        /// Rotation direction.
        direction: ShiftDirection,
        /// The SIR node supplying the rotation amount `k`.
        amount: NodeId,
    },
    /// A byte-order reversal covering `perm_width` of the operand's
    /// `type_width` bits.
    ByteSwap {
        /// Bits the permutation covers (e.g. 16 for a two-byte swap).
        perm_width: u32,
        /// Width of the operand type in the IR (e.g. 32).
        type_width: u32,
    },
    /// A full bit reversal covering `perm_width` of `type_width` bits.
    BitReverse {
        /// Bits the permutation covers (e.g. 8 for an 8-bit reversal).
        perm_width: u32,
        /// Width of the operand type in the IR (e.g. 32).
        type_width: u32,
    },
}
