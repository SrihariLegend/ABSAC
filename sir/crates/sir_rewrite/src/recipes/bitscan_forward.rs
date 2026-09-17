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
        let result = region.result()?;

        // Prefer replacing the TupleExtract consumer when one exists; when the
        // tuple is returned wholesale the loop's tuple result is rebuilt below
        // with the bitscan result in the index position.
        let extract = crate::recipes::helpers::find_tuple_extract(function, result);
        let target = extract.unwrap_or(result);

        let (packed, _width) = crate::recipes::helpers::emit_pack_from_binding(
            function,
            region,
            "BitScanForward",
            &mut builder,
        )?;

        // Type the bitscan result from the replaced node — except when the
        // replaced node is the tuple-typed loop itself: the result's type is
        // then the tuple element at the index position (typing it as the tuple
        // would corrupt the IR).
        let mut tz_ty = function
            .get_node(target)
            .ok_or_else(|| {
                RewriteError::InternalInvariantViolation(format!(
                    "region result node {target} not found"
                ))
            })?
            .ty
            .clone();
        if let sir_types::Type::Tuple { elements } = &tz_ty {
            if let Some(pos) =
                crate::recipes::helpers::loop_reduction_position(function, target, None)
            {
                tz_ty = elements[pos].clone();
            }
        }
        let tzcnt = builder.trailing_zeros_typed(packed, tz_ty, Span::unknown());

        // Use tzcnt directly, type verification will validate. We don't have a cast operator in builder yet.
        // If type mismatches, the selection/verification phase will reject it.
        let new_value = if extract.is_none() {
            crate::recipes::helpers::wrap_direct_tuple_return(
                function,
                result,
                None,
                crate::recipes::helpers::collection_length(region),
                tzcnt,
                &mut builder,
            )?
        } else {
            tzcnt
        };

        Ok(builder.finish(vec![ReplacementValue {
            old: target,
            new: new_value,
        }]))
    }
}
