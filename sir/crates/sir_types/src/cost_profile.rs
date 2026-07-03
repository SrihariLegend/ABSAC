/// Objective physical characteristics of a computation.
///
/// This type contains no notion of "good" or "bad".
/// Cost models assign meaning to these fields.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CostProfile {
    /// Number of IR instructions.
    pub instruction_count: u32,
    /// Number of Select operations (branchless conditionals).
    pub select_count: u32,
    /// Number of memory accesses (loads + stores).
    pub memory_accesses: u32,
    /// Longest dependency chain in the computation DAG.
    pub critical_path_depth: u32,
}
