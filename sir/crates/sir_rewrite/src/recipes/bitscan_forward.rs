use sir_transform::ids::DefinitionId;
use sir_types::Span;

use crate::error::RewriteError;
use crate::patch::{ReplacementPatch, ReplacementValue};
use crate::recipe::RewriteRecipe;
use crate::region::RewriteRegion;
use crate::subgraph_builder::SubgraphBuilder;

pub struct BitScanForwardRecipe {
    id: DefinitionId,
}

impl BitScanForwardRecipe {
    pub fn new(id: DefinitionId) -> Self {
        Self { id }
    }
}

impl RewriteRecipe for BitScanForwardRecipe {
    fn definition(&self) -> DefinitionId {
        self.id
    }

    fn name(&self) -> &'static str {
        "BitScanForward"
    }

    fn build_patch(
        &self,
        function: &sir_nodes::Function,
        region: &RewriteRegion,
        mut builder: SubgraphBuilder<'_>,
    ) -> Result<ReplacementPatch, RewriteError> {
        let packed = crate::recipes::helpers::emit_pack(function, region, &mut builder)?;
        let tzcnt = builder.trailing_zeros(packed, Span::unknown());
        let result = region.result()?;

        // Use tzcnt directly, type verification will validate. We don't have a cast operator in builder yet.
        // If type mismatches, the selection/verification phase will reject it.

        Ok(builder.finish(vec![ReplacementValue {
            old: result,
            new: tzcnt,
        }]))
    }
}
