use sir_transform::ids::DefinitionId;
use sir_types::Span;

use crate::error::RewriteError;
use crate::patch::{ReplacementPatch, ReplacementValue};
use crate::recipe::RewriteRecipe;
use crate::region::RewriteRegion;
use crate::subgraph_builder::SubgraphBuilder;

/// Rewrite recipe for `ClearLowestSetBit`.
/// Expected to map to an intrinsic like `llvm.ctpop` but for blsr: `Intrinsic("blsr")`.
pub struct ClearLowestSetBitRecipe {
    id: DefinitionId,
}

impl ClearLowestSetBitRecipe {
    pub fn new(id: DefinitionId) -> Self {
        Self { id }
    }
}

impl RewriteRecipe for ClearLowestSetBitRecipe {
    fn definition(&self) -> DefinitionId {
        self.id
    }

    fn name(&self) -> &'static str {
        "ClearLowestSetBit"
    }

    fn build_patch(
        &self,
        function: &sir_nodes::Function,
        region: &RewriteRegion,
        mut builder: SubgraphBuilder,
    ) -> Result<ReplacementPatch, RewriteError> {
        let (operand, result_node) = region.mask_operation()?;
        
        let original_ty = function.get_node(result_node).unwrap().ty.clone();
        
        use crate::local_id::LocalNodeId;
        let local_operand = LocalNodeId::new(operand.as_u64());
        let blsr = builder.intrinsic("blsr".to_string(), vec![local_operand], original_ty, Span::unknown());

        Ok(builder.finish(vec![ReplacementValue {
            old: result_node,
            new: blsr,
        }]))
    }
}
