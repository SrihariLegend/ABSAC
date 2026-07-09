use sir_transform::ids::DefinitionId;
use sir_transform::roles::RegionRoles;
use sir_types::Span;

use crate::error::RewriteError;
use crate::patch::{ReplacementPatch, ReplacementValue};
use crate::recipe::RewriteRecipe;
use crate::region::RewriteRegion;
use crate::subgraph_builder::SubgraphBuilder;

/// Recipe for Modulo Power of Two -> Bitwise AND transformation.
pub struct BitwiseAndModuloRecipe {
    id: DefinitionId,
}

impl BitwiseAndModuloRecipe {
    pub fn new(id: DefinitionId) -> Self {
        Self { id }
    }
}

impl RewriteRecipe for BitwiseAndModuloRecipe {
    fn definition(&self) -> DefinitionId {
        self.id
    }

    fn name(&self) -> &'static str {
        "BitwiseAndModulo"
    }

    fn build_patch(
        &self,
        region: &RewriteRegion,
        mut builder: SubgraphBuilder,
    ) -> Result<ReplacementPatch, RewriteError> {
        let op = region.operator_node()?;
        let lhs_id = region.lhs()?;
        let rhs_id = region.rhs()?;
        let result_id = region.result()?;

        use crate::local_id::LocalNodeId;
        use sir_types::{ConstantData, Type, Span};

        let local_lhs = LocalNodeId::new(lhs_id.as_u64());
        let local_rhs = LocalNodeId::new(rhs_id.as_u64());

        // Create `1`
        let one = builder.constant(ConstantData::i32(1), Type::i32(), Span::unknown());
        // Create `rhs - 1`
        let mask = builder.sub(local_rhs, one, Span::unknown());
        // Create `lhs & mask`
        let and_op = builder.bitwise_and(local_lhs, mask, Span::unknown());

        Ok(builder.finish(vec![ReplacementValue {
            old: result_id,
            new: and_op,
        }]))
    }
}
