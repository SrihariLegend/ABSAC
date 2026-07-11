use std::collections::BTreeSet;

use sir_nodes::Function;
use sir_transform::ids::DefinitionId;
use sir_types::NodeId;
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

impl std::fmt::Display for NodeProvenance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} originates from [", self.new_node)?;
        for (i, id) in self.originates_from.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", id)?;
        }
        write!(f, "] via {}", self.recipe)
    }
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

impl std::fmt::Display for GraphDiff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "GraphDiff:")?;
        writeln!(
            f,
            "  removed: {}",
            self.removed_nodes
                .iter()
                .map(|id| format!("{}", id))
                .collect::<Vec<_>>()
                .join(", ")
        )?;
        writeln!(
            f,
            "  added: {}",
            self.added_nodes
                .iter()
                .map(|id| format!("{}", id))
                .collect::<Vec<_>>()
                .join(", ")
        )?;
        write!(f, "  modified edges: {}", self.modified_edges.len())
    }
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

impl std::fmt::Display for EdgeChange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: input {} changed from {} to {}",
            self.from, self.to, self.old_target, self.new_target
        )
    }
}
