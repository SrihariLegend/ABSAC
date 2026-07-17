use sir_transform::ids::DefinitionId;
use sir_types::{ConstantData, Span, Type};

use crate::error::RewriteError;
use crate::patch::{ReplacementPatch, ReplacementValue};
use crate::recipe::RewriteRecipe;
use crate::recipes::helpers::emit_pack;
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
        let mut result = region.result()?;
        for node in function.arena.iter() {
            if let sir_nodes::NodeKind::TupleExtract { tuple, .. } = &node.kind {
                if *tuple == result {
                    result = node.id;
                    break;
                }
            }
        }
        
        let packed = emit_pack(function, region, &mut builder)?;
        let original_ty = function.get_node(result).unwrap().ty.clone();
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

        Ok(builder.finish(vec![ReplacementValue {
            old: result,
            new: ne_zero,
        }]))
    }
}
