use sir_transform::ids::DefinitionId;
use sir_types::Span;

use crate::error::RewriteError;
use crate::patch::{ReplacementPatch, ReplacementValue};
use crate::recipe::RewriteRecipe;
use crate::region::RewriteRegion;
use crate::subgraph_builder::SubgraphBuilder;

pub struct BitScanReverseRecipe {
    id: DefinitionId,
}

impl BitScanReverseRecipe {
    pub fn new(id: DefinitionId) -> Self {
        Self { id }
    }
}

impl RewriteRecipe for BitScanReverseRecipe {
    fn definition(&self) -> DefinitionId {
        self.id
    }

    fn name(&self) -> &'static str {
        "BitScanReverse"
    }

    fn build_patch(
        &self,
        function: &sir_nodes::Function,
        region: &RewriteRegion,
        mut builder: SubgraphBuilder<'_>,
    ) -> Result<ReplacementPatch, RewriteError> {
        let packed = crate::recipes::helpers::emit_pack(function, region, &mut builder)?;
        let lzcnt = builder.leading_zeros(packed, Span::unknown());

        let result = region.result()?;

        Ok(builder.finish(vec![ReplacementValue {
            old: result,
            new: lzcnt,
        }]))
    }
}
