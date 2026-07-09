use sir_transform::ids::DefinitionId;
use sir_types::{ConstantData, Span, Type};

use crate::error::RewriteError;
use crate::patch::{ReplacementPatch, ReplacementValue};
use crate::recipe::RewriteRecipe;
use crate::region::RewriteRegion;
use crate::subgraph_builder::SubgraphBuilder;
use crate::recipes::helpers::emit_pack;

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
        region: &RewriteRegion,
        mut builder: SubgraphBuilder,
    ) -> Result<ReplacementPatch, RewriteError> {
        let packed = emit_pack(region, &mut builder)?;
        let pop = builder.popcount(packed, Span::unknown());
        
        // Ensure type of popcount result (i32 is default in popcount builder, but let's use what it gives)
        let ty = builder.get_type(pop).unwrap_or(Type::i32());
        
        let one = builder.constant(ConstantData::i32(1), ty.clone(), Span::unknown());
        let and_one = builder.bitwise_and(pop, one, Span::unknown());

        // Parity is (pop & 1) != 0 which returns a boolean.
        let zero = builder.constant(ConstantData::i32(0), ty, Span::unknown());
        let ne_zero = builder.ne(and_one, zero, Span::unknown());

        let result = region.result()?;
        Ok(builder.finish(vec![ReplacementValue {
            old: result,
            new: ne_zero,
        }]))
    }
}
