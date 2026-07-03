//! SIR Rewrite — Verified Graph Rewriting Engine v0.1
//!
//! Executes proven transformations by constructing replacement SIR
//! in a detached arena, then performing transactional graph surgery.
//! Never discovers, never analyses, never proves — only executes.

pub mod local_id;
pub mod detached_arena;
pub mod subgraph_builder;
pub mod region;
pub mod error;

// Temporary stubs — will be replaced in Task 7.
mod patch_stub {
    use crate::detached_arena::DetachedArena;
    use crate::local_id::LocalNodeId;
    use sir_types::NodeId;

    #[derive(Clone, Debug)]
    pub struct ReplacementValue {
        pub old: NodeId,
        pub new: LocalNodeId,
    }

    #[derive(Clone, Debug)]
    pub struct ReplacementPatch {
        pub arena: DetachedArena,
        pub roots: Vec<LocalNodeId>,
        pub replacements: Vec<ReplacementValue>,
    }
}
