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
        function: &sir_nodes::Function,
        region: &RewriteRegion,
        mut builder: SubgraphBuilder,
    ) -> Result<ReplacementPatch, RewriteError> {
        let op = region.operator_node()?;
        let lhs_id = region.lhs()?;
        let rhs_id = region.rhs()?;
        let result_id = region.result()?;

        use crate::local_id::LocalNodeId;
        use sir_types::{ConstantData, Span, Type};

        let local_lhs = LocalNodeId::new(lhs_id.as_u64());
        let local_rhs = LocalNodeId::new(rhs_id.as_u64());

        let ty = function
            .get_node(lhs_id)
            .map(|n| n.ty.clone())
            .unwrap_or(sir_types::Type::i32());

        // Determine whether to emit a signed or unsigned `1` constant
        let is_signed = match &ty {
            sir_types::Type::Integer { signed, .. } => *signed,
            _ => true,
        };
        let one_data = if is_signed {
            ConstantData::i32(1)
        } else {
            ConstantData::u32(1)
        };

        let one = builder.constant(one_data, ty.clone(), Span::unknown());
        let mask = builder.sub(local_rhs, one, Span::unknown());
        // Create `lhs & mask`
        let and_op = builder.bitwise_and(local_lhs, mask, Span::unknown());

        Ok(builder.finish(vec![ReplacementValue {
            old: result_id,
            new: and_op,
        }]))
    }
}
