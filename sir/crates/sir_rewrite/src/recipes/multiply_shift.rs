use sir_transform::ids::DefinitionId;

use crate::error::RewriteError;
use crate::patch::{ReplacementPatch, ReplacementValue};
use crate::recipe::RewriteRecipe;
use crate::region::RewriteRegion;
use crate::subgraph_builder::SubgraphBuilder;

/// Recipe for Multiply Power of Two -> Shift Left transformation.
pub struct MultiplyShiftRecipe {
    id: DefinitionId,
}

impl MultiplyShiftRecipe {
    pub fn new(id: DefinitionId) -> Self {
        Self { id }
    }
}

impl RewriteRecipe for MultiplyShiftRecipe {
    fn definition(&self) -> DefinitionId {
        self.id
    }

    fn name(&self) -> &'static str {
        "MultiplyShift"
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

        let trailing_zeros = builder.trailing_zeros(local_rhs, Span::unknown());
        let shl_op = builder.shl(local_lhs, trailing_zeros, Span::unknown());

        Ok(builder.finish(vec![ReplacementValue {
            old: result_id,
            new: shl_op,
        }]))
    }
}
