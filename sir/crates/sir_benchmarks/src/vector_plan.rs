//! Vector plan — target-independent representation of vectorized semantic reductions.

use sir_types::NodeId;

#[derive(Debug, Clone)]
pub struct VectorPlan {
    pub operation: VectorOp,
    pub buffer_name: String,
    pub length_name: String,
    pub predicate: Option<VectorPredicate>,
    pub vector_width: usize,
    pub tail: TailPolicy,
}

#[derive(Debug, Clone)]
pub enum VectorOp {
    Cardinality,
    All,
    Sum,
}

/// A predicate with resolved value names (C expressions).
#[derive(Debug, Clone)]
pub enum VectorPredicate {
    Equal(String),      // value to compare against
    MaskedEqual { mask: String, target: String },
    Identity,
}

#[derive(Debug, Clone)]
pub enum TailPolicy {
    Scalar,
    NarrowerVector,
}
