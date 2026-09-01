use sir_transform::ids::DefinitionId;
use sir_types::{ConstantData, Span, Type};

use crate::error::RewriteError;
use crate::patch::{ReplacementPatch, ReplacementValue};
use crate::recipe::RewriteRecipe;
use crate::recipes::helpers::{authorized_tuple_consumer, collection_length, emit_pack, wrap_direct_tuple_return};
use crate::region::RewriteRegion;
use crate::subgraph_builder::SubgraphBuilder;

/// Recipe for the Any transformation.
///
/// Replaces a boolean-array disjunctive loop with:
///   pack(board) → (packed != 0)
pub struct AnyRecipe {
    id: DefinitionId,
}

impl AnyRecipe {
    pub fn new(id: DefinitionId) -> Self {
        Self { id }
    }
}

impl RewriteRecipe for AnyRecipe {
    fn definition(&self) -> DefinitionId {
        self.id
    }

    fn name(&self) -> &'static str {
        "Any"
    }

    fn build_patch(
        &self,
        function: &sir_nodes::Function,
        region: &RewriteRegion,
        mut builder: SubgraphBuilder,
    ) -> Result<ReplacementPatch, RewriteError> {
        let result = region.result()?;
        let accumulator = region.accumulator().ok().flatten();

        // Prefer replacing the tuple-slot consumer when one exists; when the
        // tuple is returned wholesale the loop's tuple result is rebuilt
        // below. The consumer must read the ACCUMULATOR slot — a slot the
        // theorem does not cover (e.g. a position/index live-out) refuses
        // the rewrite (PS002 audit: array_find_last was silently rewritten
        // to return a constant).
        let extract = authorized_tuple_consumer(function, result, accumulator)?;
        let target = extract.unwrap_or(result);

        let packed = emit_pack(function, region, &mut builder)?;

        let width = match builder.get_type(packed) {
            Some(Type::BitVector { width }) => width,
            _ => 64, // Default
        };

        // Create zero constant for comparison
        let zero = builder.constant(
            ConstantData::u64(0),
            Type::BitVector { width },
            Span::unknown(),
        );
        let ne_zero = builder.ne(packed, zero, Span::unknown());

        let new_value = if extract.is_none() {
            wrap_direct_tuple_return(
                function,
                result,
                accumulator,
                collection_length(region),
                ne_zero,
                &mut builder,
            )?
        } else {
            ne_zero
        };

        Ok(builder.finish(vec![ReplacementValue {
            old: target,
            new: new_value,
        }]))
    }
}
