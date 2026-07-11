// sir/crates/sir_semantics/src/cost_deriver.rs

use sir_nodes::{Function, NodeKind};
use sir_types::CostProfile;

use crate::cost::CostDatabase;
use crate::structure::StructuralDatabase;

/// Derives `CostProfile` for each region from SIR node counts and expression depth.
///
/// This is a dedicated component, separate from semantic recognizers.
/// Recognizers answer "what is this computation?" — CostDeriver answers
/// "what does it cost?" Neither depends on the other.
///
/// The optimizer never walks SIR. Costs are pre-computed here so the
/// optimizer reads `CostDatabase::for_region(region)` — a single lookup.
pub struct CostDeriver;

impl CostDeriver {
    /// Compute cost profiles for every region in the structural database.
    ///
    /// For each region:
    ///   - instruction_count = number of SIR nodes in the region
    ///   - select_count = number of Select nodes
    ///   - memory_accesses = number of Load + Store nodes
    ///   - critical_path_depth = maximum expression depth (recursive)
    ///
    /// Expression depth is computed locally — no dependency on
    /// `sir_analysis::graph` algorithms. This is an approximation
    /// sufficient for v0.1 and can be replaced with a proper latency
    /// model later.
    pub fn derive(function: &Function, structural: &StructuralDatabase) -> CostDatabase {
        let mut db = CostDatabase::new();

        for (region_id, _desc) in structural.regions() {
            let profile = Self::compute_region_cost(function, region_id);
            db.insert(region_id, profile);
        }

        db
    }

    /// Compute the cost profile for a single region by walking its nodes.
    fn compute_region_cost(function: &Function, _region_id: sir_types::RegionId) -> CostProfile {
        // The StructuralDatabase doesn't expose node sets directly —
        // it maps RegionId -> StructuralDescription. We need to walk
        // all nodes in the function and check which belong to this region.
        //
        // For v0.1 with a single region per function, we compute costs
        // over all nodes in the function when the region exists.
        // Future: region membership will be tracked explicitly.

        let mut instruction_count: u32 = 0;
        let mut select_count: u32 = 0;
        let mut memory_accesses: u32 = 0;

        // Walk all nodes in the function via arena iteration.
        // In v0.1 with one region, all nodes belong to the region.
        // Future phases will filter by region membership when
        // StructuralDescription carries a node set.
        for node in function.arena.iter() {
            instruction_count += 1;

            match &node.kind {
                NodeKind::Select { .. } => {
                    select_count += 1;
                }
                NodeKind::Load { .. } | NodeKind::Store { .. } => {
                    memory_accesses += 1;
                }
                _ => {}
            }
        }

        // Compute maximum expression depth recursively over all nodes.
        // Depth of a node = 1 + max(depth of each operand).
        // Leaf nodes (no operands, or operands outside this region) have depth 1.
        let critical_path_depth = Self::compute_max_depth(function);

        CostProfile {
            instruction_count,
            select_count,
            memory_accesses,
            critical_path_depth,
        }
    }

    /// Compute maximum expression depth over the function's nodes.
    ///
    /// For each node, depth = 1 + max(depth of its dataflow inputs).
    /// Uses memoization to avoid recomputation. Leaf nodes have depth 1.
    fn compute_max_depth(function: &Function) -> u32 {
        use std::collections::HashMap;

        fn node_depth(
            node_id: sir_types::NodeId,
            function: &Function,
            memo: &mut HashMap<sir_types::NodeId, u32>,
        ) -> u32 {
            if let Some(&cached) = memo.get(&node_id) {
                return cached;
            }

            let depth = match function.get_node(node_id) {
                Some(node) => {
                    let inputs = node.kind.input_nodes();
                    if inputs.is_empty() {
                        1
                    } else {
                        let max_input = inputs
                            .iter()
                            .map(|&input_id| node_depth(input_id, function, memo))
                            .max()
                            .unwrap_or(0);
                        1 + max_input
                    }
                }
                None => 1,
            };

            memo.insert(node_id, depth);
            depth
        }

        let mut memo: HashMap<sir_types::NodeId, u32> = HashMap::new();
        let mut max_depth: u32 = 0;

        for node in function.arena.iter() {
            let d = node_depth(node.id, function, &mut memo);
            if d > max_depth {
                max_depth = d;
            }
        }

        max_depth.max(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sir_builder::Builder;
    use sir_types::Type;

    #[test]
    fn cost_deriver_empty_function() {
        let func = Builder::new("empty", &[], Type::Unit).build();
        let structural = StructuralDatabase::new();
        let cost_db = CostDeriver::derive(&func, &structural);
        assert!(cost_db.is_empty());
    }
}
