use sir_transform::ids::DefinitionId;
use sir_types::Span;

use crate::error::RewriteError;
use crate::patch::{ReplacementPatch, ReplacementValue};
use crate::recipe::RewriteRecipe;
use crate::region::RewriteRegion;
use crate::subgraph_builder::SubgraphBuilder;

pub struct LeadingZeroCountRecipe {
    id: DefinitionId,
}

impl LeadingZeroCountRecipe {
    pub fn new(id: DefinitionId) -> Self {
        Self { id }
    }
}

impl RewriteRecipe for LeadingZeroCountRecipe {
    fn definition(&self) -> DefinitionId {
        self.id
    }

    fn name(&self) -> &'static str {
        "LeadingZeroCount"
    }

    fn build_patch(
        &self,
        _function: &sir_nodes::Function,
        region: &RewriteRegion,
        mut builder: SubgraphBuilder<'_>,
    ) -> Result<ReplacementPatch, RewriteError> {
        let scalar = region.predicate_scalar()?;
        let lzcnt = builder.leading_zeros(
            crate::local_id::LocalNodeId::new(scalar.as_u64()),
            Span::unknown(),
        );

        let result = region.result()?;

        Ok(builder.finish(vec![ReplacementValue {
            old: result,
            new: lzcnt,
        }]))
    }
}
