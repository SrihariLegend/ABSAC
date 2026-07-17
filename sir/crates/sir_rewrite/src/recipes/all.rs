use sir_transform::ids::DefinitionId;
use sir_types::{ConstantData, Span, Type};

use crate::error::RewriteError;
use crate::patch::{ReplacementPatch, ReplacementValue};
use crate::recipe::RewriteRecipe;
use crate::recipes::helpers::emit_pack;
use crate::region::RewriteRegion;
use crate::subgraph_builder::SubgraphBuilder;

/// Recipe for the All transformation.
///
/// Replaces a boolean-array conjunctive loop with:
///   pack(board) → (packed == full_mask)
pub struct AllRecipe {
    id: DefinitionId,
}

impl AllRecipe {
    pub fn new(id: DefinitionId) -> Self {
        Self { id }
    }
}

impl RewriteRecipe for AllRecipe {
    fn definition(&self) -> DefinitionId {
        self.id
    }

    fn name(&self) -> &'static str {
        "All"
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

        let width = match builder.get_type(packed) {
            Some(Type::BitVector { width }) => width,
            _ => 64, // Default
        };

        let full_mask_val = if width == 64 {
            u64::MAX
        } else {
            (1u64 << width) - 1
        };
        let full_mask = builder.constant(
            ConstantData::u64(full_mask_val),
            Type::BitVector { width },
            Span::unknown(),
        );
        let eq_mask = builder.eq(packed, full_mask, Span::unknown());

        Ok(builder.finish(vec![ReplacementValue {
            old: result,
            new: eq_mask,
        }]))
    }
}
