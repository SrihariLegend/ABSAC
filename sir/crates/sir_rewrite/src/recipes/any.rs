use sir_transform::ids::DefinitionId;
use sir_types::{ConstantData, Span, Type};

use crate::error::RewriteError;
use crate::patch::{ReplacementPatch, ReplacementValue};
use crate::recipe::RewriteRecipe;
use crate::region::RewriteRegion;
use crate::subgraph_builder::SubgraphBuilder;
use crate::recipes::helpers::emit_pack;

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
        region: &RewriteRegion,
        mut builder: SubgraphBuilder,
    ) -> Result<ReplacementPatch, RewriteError> {
        let packed = emit_pack(region, &mut builder)?;
        
        let width = match builder.get_type(packed) {
            Some(Type::BitVector { width }) => width,
            _ => 64, // Default
        };

        // Create zero constant for comparison
        let zero = builder.constant(ConstantData::u64(0), Type::BitVector { width }, Span::unknown());
        let ne_zero = builder.ne(packed, zero, Span::unknown());

        let result = region.result()?;
        Ok(builder.finish(vec![ReplacementValue {
            old: result,
            new: ne_zero,
        }]))
    }
}
