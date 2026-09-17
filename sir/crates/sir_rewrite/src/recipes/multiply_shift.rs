use sir_transform::ids::DefinitionId;

use crate::error::RewriteError;
use crate::patch::{ReplacementPatch, ReplacementValue};
use crate::recipe::RewriteRecipe;
use crate::region::RewriteRegion;
use crate::subgraph_builder::SubgraphBuilder;

/// Recipe for Multiply Power of Two -> Shift Left transformation.
pub struct MultiplyShiftRecipe {
    id: DefinitionId,
}

impl MultiplyShiftRecipe {
    pub fn new(id: DefinitionId) -> Self {
        Self { id }
    }
}

impl RewriteRecipe for MultiplyShiftRecipe {
    fn definition(&self) -> DefinitionId {
        self.id
    }

    fn name(&self) -> &'static str {
        "MultiplyShift"
    }

    fn build_patch(
        &self,
        function: &sir_nodes::Function,
        region: &RewriteRegion,
        mut builder: SubgraphBuilder,
    ) -> Result<ReplacementPatch, RewriteError> {
        let _op = region.operator_node()?;
        let lhs_id = region.lhs()?;
        let rhs_id = region.rhs()?;
        let result_id = region.result()?;

        use crate::local_id::LocalNodeId;
        use sir_types::Span;

        // The power-of-two constant may be on EITHER side of the
        // multiplication (`x * 32` and `32 * x` are both valid
        // candidates). The dynamic operand is the one that gets shifted;
        // assuming the RHS is the constant would emit `32 << tzcnt(x)`
        // for the commuted form — a live miscompile the quarantine
        // previously hid.
        let is_constant = |id: sir_types::NodeId| {
            matches!(
                function.get_node(id).map(|n| &n.kind),
                Some(sir_nodes::NodeKind::Constant(_))
            )
        };
        let (dynamic, constant) = match (is_constant(lhs_id), is_constant(rhs_id)) {
            (false, true) => (lhs_id, rhs_id),
            (true, false) => (rhs_id, lhs_id),
            _ => {
                return Err(RewriteError::RecipeFailed(
                    "multiply-shift requires exactly one constant operand".to_string(),
                ))
            }
        };

        // Fail closed on a non-power-of-two constant or an out-of-range
        // shift (the solver only authorizes the checked theorem; the
        // recipe must not manufacture anything else).
        let value = match function.get_node(constant).map(|n| &n.kind) {
            // Signed constants carry the same bit pattern as their
            // unsigned counterpart; accept non-negative signed values.
            Some(sir_nodes::NodeKind::Constant(data)) => data
                .as_u64()
                .or_else(|| data.as_i64().and_then(|v| u64::try_from(v).ok())),
            _ => None,
        }
        .ok_or_else(|| {
            RewriteError::RecipeFailed("multiply-shift constant is not representable".to_string())
        })?;
        if value == 0 || !value.is_power_of_two() {
            return Err(RewriteError::RecipeFailed(
                "multiply-shift constant is not a power of two".to_string(),
            ));
        }
        let width = match function.get_node(dynamic).map(|n| &n.ty) {
            Some(sir_types::Type::Integer { width, .. }) => width.bits(),
            _ => {
                return Err(RewriteError::RecipeFailed(
                    "multiply-shift dynamic operand has no integer width".to_string(),
                ))
            }
        };
        if (value.trailing_zeros() as usize) >= width {
            return Err(RewriteError::RecipeFailed(
                "multiply-shift constant exceeds the operand width".to_string(),
            ));
        }

        let trailing_zeros =
            builder.trailing_zeros(LocalNodeId::new(constant.as_u64()), Span::unknown());
        let shl_op = builder.shl(LocalNodeId::new(dynamic.as_u64()), trailing_zeros, Span::unknown());

        Ok(builder.finish(vec![ReplacementValue {
            old: result_id,
            new: shl_op,
        }]))
    }
}
