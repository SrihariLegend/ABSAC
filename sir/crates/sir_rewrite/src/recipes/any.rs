use sir_transform::ids::DefinitionId;
use sir_types::{ConstantData, Span, Type};

use crate::error::RewriteError;
use crate::patch::{ReplacementPatch, ReplacementValue};
use crate::recipe::RewriteRecipe;
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
        // ── ROLE-MAP-ONLY (advisor invariant 6) ─────────────────
        // Once ProposalBinding succeeds, no stage scans the source to
        // guess semantic roles. The recipe consumes ONLY the binding's
        // role map and classified observables through the shared
        // helpers (one implementation for every reduction recipe).
        let target = crate::recipes::helpers::binding_target(region, "Any")?;
        let (packed, width) =
            crate::recipes::helpers::emit_pack_from_binding(function, region, "Any", &mut builder)?;

        let zero = builder.constant(
            ConstantData::u64(0),
            Type::BitVector { width },
            Span::unknown(),
        );
        let ne_zero = builder.ne(packed, zero, Span::unknown());

        Ok(builder.finish(vec![ReplacementValue {
            old: target,
            new: ne_zero,
        }]))
    }
}
