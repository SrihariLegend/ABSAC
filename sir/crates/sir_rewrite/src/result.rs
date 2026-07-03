use std::collections::BTreeSet;

use sir_nodes::Function;
use sir_types::NodeId;
use sir_transform::ids::DefinitionId;
use sir_verification::Proof;

/// The result of a successful rewrite.
#[derive(Clone, Debug)]
pub struct RewriteResult {
    /// The rewritten function.
    pub rewritten: Function,
    /// Provenance for every synthetic node.
    pub provenance: Vec<NodeProvenance>,
    /// What changed between original and rewritten.
    pub diff: GraphDiff,
    /// The proof that authorized this rewrite.
    pub proof: Proof,
}

/// Records why a synthetic node exists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeProvenance {
    /// The node in the rewritten function.
    pub new_node: NodeId,
    /// Which original nodes it derives from.
    pub originates_from: Vec<NodeId>,
    /// Which transformation produced it.
    pub recipe: DefinitionId,
}

/// A complete diff between original and rewritten functions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphDiff {
    /// Nodes present in the original but absent from the rewritten function.
    pub removed_nodes: BTreeSet<NodeId>,
    /// Nodes present in the rewritten function but absent from the original.
    pub added_nodes: BTreeSet<NodeId>,
    /// Edges whose target changed between original and rewritten.
    pub modified_edges: Vec<EdgeChange>,
}

/// A single edge change in the graph diff.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EdgeChange {
    /// The source node of the edge.
    pub from: NodeId,
    /// The operand/input index in the source node.
    pub to: usize,
    /// The original target node.
    pub old_target: NodeId,
    /// The new target node.
    pub new_target: NodeId,
}
