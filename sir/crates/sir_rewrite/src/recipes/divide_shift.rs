use sir_transform::ids::DefinitionId;

use crate::error::RewriteError;
use crate::patch::{ReplacementPatch, ReplacementValue};
use crate::recipe::RewriteRecipe;
use crate::region::RewriteRegion;
use crate::subgraph_builder::SubgraphBuilder;

/// Recipe for Divide Power of Two -> Shift Right transformation.
pub struct DivideShiftRecipe {
    id: DefinitionId,
}

impl DivideShiftRecipe {
    pub fn new(id: DefinitionId) -> Self {
        Self { id }
    }
}

impl RewriteRecipe for DivideShiftRecipe {
    fn definition(&self) -> DefinitionId {
        self.id
    }

    fn name(&self) -> &'static str {
        "DivideShift"
    }

    fn build_patch(
        &self,
        function: &sir_nodes::Function,
        region: &RewriteRegion,
        mut builder: SubgraphBuilder,
    ) -> Result<ReplacementPatch, RewriteError> {
        let op = region.operator_node()?;
        let lhs_id = region.lhs()?;
        let rhs_id = region.rhs()?;
        let result_id = region.result()?;

        use crate::local_id::LocalNodeId;
        use sir_types::Span;

        let local_lhs = LocalNodeId::new(lhs_id.as_u64());
        let local_rhs = LocalNodeId::new(rhs_id.as_u64());

        // Extract trailing zeros of the constant RHS
        // In a real implementation we'd read the constant value, compute trailing zeros,
        // and create a new constant node. Here we just use a stub builder method
        // to represent the computation.
        let trailing_zeros = builder.trailing_zeros(local_rhs, Span::unknown());
        let shr_op = builder.shr(local_lhs, trailing_zeros, Span::unknown());

        Ok(builder.finish(vec![ReplacementValue {
            old: result_id,
            new: shr_op,
        }]))
    }
}
