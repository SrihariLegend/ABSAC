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
        let (packed, _width) = crate::recipes::helpers::emit_pack_for_position_search(
            function,
            region,
            "BitScanReverse",
            &mut builder,
        )?;
        let result = region.result()?;
        let consumer = crate::recipes::helpers::find_tuple_consumer(function, result);
        let target = consumer.map(|(id, _)| id).unwrap_or(result);
        // Emit the bit-scan-reverse intrinsic (highest set index,
        // width sentinel for zero) — the target the corrected
        // obligation `LastTrue == BitScanReverse(Pack)` names. The
        // definition is still held Stub pending reverse counted-loop
        // trip-count support and a sound reverse kernel, so this path
        // cannot be authorized yet.
        let mut ty = function
            .get_node(target)
            .map(|n| n.ty.clone())
            .unwrap_or(sir_types::Type::u64());
        if let sir_types::Type::Tuple { elements } = &ty {
            if let Some(pos) =
                crate::recipes::helpers::loop_reduction_position(function, target, None)
            {
                ty = elements[pos].clone();
            }
        }
        let bsr = builder.bit_scan_reverse_typed(packed, ty, Span::unknown());

        Ok(builder.finish(vec![ReplacementValue {
            old: target,
            new: bsr,
        }]))
    }
}
