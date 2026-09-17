use sir_transform::ids::DefinitionId;
use sir_types::{ConstantData, Span, Type};

use crate::error::RewriteError;
use crate::patch::{ReplacementPatch, ReplacementValue};
use crate::recipe::RewriteRecipe;
use crate::recipes::helpers::{
    binding_target, emit_pack_from_binding, require_binding, wrap_direct_tuple_return,
};
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
        // Binding-only (P0A): roles, observable target and collection
        // extent all come from the authorized ProposalBinding.
        let binding = require_binding(region, "All")?;
        let map = &binding.map;
        let target = binding_target(region, "All")?;
        let (packed, width) = emit_pack_from_binding(function, region, "All", &mut builder)?;

        // SIR constants carry a single u64: an all-ones mask exists only
        // up to 64 elements. Wider All sets refuse loudly instead of
        // emitting a truncated mask.
        if width > 64 {
            return Err(RewriteError::RecipeFailed(format!(
                "All over a {width}-element collection needs a multi-limb \
                 all-ones constant, which SIR constants cannot represent"
            )));
        }

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

        let new_value = if target == map.loop_node {
            // Whole-value observable: rebuild the loop result when it is
            // a tuple (multi-element tuples refuse inside the helper).
            wrap_direct_tuple_return(
                function,
                map.loop_node,
                Some(map.accumulator),
                Some(width),
                eq_mask,
                &mut builder,
            )?
        } else {
            eq_mask
        };

        Ok(builder.finish(vec![ReplacementValue {
            old: target,
            new: new_value,
        }]))
    }
}
