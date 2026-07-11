use sir_transform::ids::DefinitionId;

use crate::error::RewriteError;
use crate::patch::{ReplacementPatch, ReplacementValue};
use crate::recipe::RewriteRecipe;
use crate::region::RewriteRegion;
use crate::subgraph_builder::SubgraphBuilder;

/// Recipe for Shift Sequence -> Mask Extract transformation.
pub struct ShiftMaskRecipe {
    id: DefinitionId,
}

impl ShiftMaskRecipe {
    pub fn new(id: DefinitionId) -> Self {
        Self { id }
    }
}

impl RewriteRecipe for ShiftMaskRecipe {
    fn definition(&self) -> DefinitionId {
        self.id
    }

    fn name(&self) -> &'static str {
        "ShiftMask"
    }

    fn build_patch(
        &self,
        _function: &sir_nodes::Function,
        region: &RewriteRegion,
        mut builder: SubgraphBuilder,
    ) -> Result<ReplacementPatch, RewriteError> {
        let _op = region.operator_node()?;
        let lhs_id = region.lhs()?; // This is the inner value
        let rhs_id = region.rhs()?; // This is the shift amount
        let result_id = region.result()?;

        use crate::local_id::LocalNodeId;
        use sir_types::Span;

        let local_lhs = LocalNodeId::new(lhs_id.as_u64());
        let local_rhs = LocalNodeId::new(rhs_id.as_u64());

        // x << n >> n -> x & ((1 << (WIDTH - n)) - 1)
        // For simplicity in the subgraph builder stub, we just construct an `and` with a computed mask.
        // We assume we can construct a mask computation node.
        let mask = builder.shl(local_rhs, local_rhs, Span::unknown()); // stub
        let and_op = builder.bitwise_and(local_lhs, mask, Span::unknown());

        Ok(builder.finish(vec![ReplacementValue {
            old: result_id,
            new: and_op,
        }]))
    }
}
