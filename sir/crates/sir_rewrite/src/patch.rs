use sir_types::NodeId;

use crate::detached_arena::DetachedArena;
use crate::local_id::LocalNodeId;

/// Maps an original exported SSA value to its replacement in the detached arena.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplacementValue {
    /// The original exported SSA value in the function.
    pub old: NodeId,
    /// The replacement value in the detached arena.
    pub new: LocalNodeId,
}

/// A closed subgraph produced by a `RewriteRecipe`.
///
/// Contains the detached arena with replacement nodes, the roots
/// of the replacement graph, and the explicit old→new SSA mapping.
///
/// Invariant: May reference only nodes created within its own `DetachedArena`
/// and values explicitly provided through the `RewriteRegion` boundary.
/// Must never reference arbitrary nodes in the original `Function`.
#[derive(Clone, Debug)]
pub struct ReplacementPatch {
    /// The detached arena containing all replacement nodes.
    pub arena: DetachedArena,
    /// Roots of the detached subgraph to be imported.
    pub roots: Vec<LocalNodeId>,
    /// Explicit old→new SSA value mappings.
    pub replacements: Vec<ReplacementValue>,
}

impl ReplacementPatch {
    /// Create a new patch.
    pub fn new(
        arena: DetachedArena,
        roots: Vec<LocalNodeId>,
        replacements: Vec<ReplacementValue>,
    ) -> Self {
        Self {
            arena,
            roots,
            replacements,
        }
    }
}
