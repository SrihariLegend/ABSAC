use sir_transform::ids::DefinitionId;
use sir_types::Span;

use crate::error::RewriteError;
use crate::patch::{ReplacementPatch, ReplacementValue};
use crate::recipe::RewriteRecipe;
use crate::region::RewriteRegion;
use crate::subgraph_builder::SubgraphBuilder;

pub struct TrailingZeroCountRecipe {
    id: DefinitionId,
}

impl TrailingZeroCountRecipe {
    pub fn new(id: DefinitionId) -> Self {
        Self { id }
    }
}

impl RewriteRecipe for TrailingZeroCountRecipe {
    fn definition(&self) -> DefinitionId {
        self.id
    }

    fn name(&self) -> &'static str {
        "TrailingZeroCount"
    }

    fn build_patch(
        &self,
        function: &sir_nodes::Function,
        region: &RewriteRegion,
        mut builder: SubgraphBuilder<'_>,
    ) -> Result<ReplacementPatch, RewriteError> {
        let scalar = region.predicate_scalar()?;
        let result = region.result()?;
        // The loop result is consumed through a projection (field/tuple
        // extract); replace THAT consumer, not the tuple-typed loop
        // node. Leaving the projection in place makes the emitted
        // function return a stale/default value.
        let target = crate::recipes::helpers::find_tuple_extract(function, result)
            .or_else(|| {
                crate::recipes::helpers::find_tuple_consumer(function, result).map(|(id, _)| id)
            })
            .unwrap_or(result);
        let ty = function
            .get_node(target)
            .map(|n| n.ty.clone())
            .unwrap_or(sir_types::Type::u64());
        let tzcnt = builder.trailing_zeros_typed(
            crate::local_id::LocalNodeId::new(scalar.as_u64()),
            ty,
            Span::unknown(),
        );

        Ok(builder.finish(vec![ReplacementValue {
            old: target,
            new: tzcnt,
        }]))
    }
}
