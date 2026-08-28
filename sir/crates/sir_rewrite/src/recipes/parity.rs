use sir_transform::ids::DefinitionId;
use sir_types::{ConstantData, Span};

use crate::error::RewriteError;
use crate::patch::{ReplacementPatch, ReplacementValue};
use crate::recipe::RewriteRecipe;
use crate::recipes::helpers::{collection_length, emit_pack, find_tuple_extract, wrap_direct_tuple_return};
use crate::region::RewriteRegion;
use crate::subgraph_builder::SubgraphBuilder;

/// Recipe for the Parity transformation.
///
/// Replaces a boolean-array exclusive loop with:
///   pack(board) → popcount(packed) → (popcount & 1) != 0
pub struct ParityRecipe {
    id: DefinitionId,
}

impl ParityRecipe {
    pub fn new(id: DefinitionId) -> Self {
        Self { id }
    }
}

impl RewriteRecipe for ParityRecipe {
    fn definition(&self) -> DefinitionId {
        self.id
    }

    fn name(&self) -> &'static str {
        "Parity"
    }

    fn build_patch(
        &self,
        function: &sir_nodes::Function,
        region: &RewriteRegion,
        mut builder: SubgraphBuilder,
    ) -> Result<ReplacementPatch, RewriteError> {
        let result = region.result()?;
        let accumulator = region.accumulator().ok().flatten();

        // Prefer replacing the TupleExtract consumer when one exists; when the tuple
        // is returned wholesale the loop's tuple result is rebuilt below.
        let extract = find_tuple_extract(function, result);
        let target = extract.unwrap_or(result);

        let packed = emit_pack(function, region, &mut builder)?;
        // Since parity results in a bool, the popcount type should probably be i32 to do bitwise ops,
        // but we'll use u64 if needed. i32 is safe for popcount.
        let pop_ty = sir_types::Type::i32();
        let pop = builder.popcount(packed, pop_ty.clone(), Span::unknown());

        // Ensure type of popcount result (i32 is default in popcount builder, but let's use what it gives)
        let ty = builder.get_type(pop).unwrap_or(pop_ty);

        let one = builder.constant(ConstantData::i32(1), ty.clone(), Span::unknown());
        let and_one = builder.bitwise_and(pop, one, Span::unknown());

        // Parity is (pop & 1) != 0 which returns a boolean.
        let zero = builder.constant(ConstantData::i32(0), ty, Span::unknown());
        let ne_zero = builder.ne(and_one, zero, Span::unknown());

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
