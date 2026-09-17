use sir_transform::ids::DefinitionId;
use sir_types::{ConstantData, Span, Type};

use crate::error::RewriteError;
use crate::local_id::LocalNodeId;
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
        let _op = region.operator_node()?;
        let lhs_id = region.lhs()?;
        let rhs_id = region.rhs()?;
        let result_id = region.result()?;

        // The divisor must be the constant RHS and a power of two; the
        // identity is only valid for unsigned operands.
        let constant = match function.get_node(rhs_id).map(|n| &n.kind) {
            Some(sir_nodes::NodeKind::Constant(data)) => data
                .as_u64()
                .or_else(|| data.as_i64().and_then(|v| u64::try_from(v).ok())),
            _ => None,
        }
        .ok_or_else(|| {
            RewriteError::RecipeFailed("modulo-and divisor is not a constant".to_string())
        })?;
        if constant == 0 || !constant.is_power_of_two() {
            return Err(RewriteError::RecipeFailed(
                "modulo-and divisor is not a power of two".to_string(),
            ));
        }
        let (width, signed) = match function.get_node(lhs_id).map(|n| &n.ty) {
            Some(Type::Integer { width, signed, .. }) => (width.bits() as usize, *signed),
            _ => {
                return Err(RewriteError::RecipeFailed(
                    "modulo-and operand has no integer width".to_string(),
                ))
            }
        };
        if signed {
            return Err(RewriteError::RecipeFailed(
                "modulo-and is only valid for unsigned operands".to_string(),
            ));
        }
        let mask_data = unsigned_constant(width, constant - 1).ok_or_else(|| {
            RewriteError::RecipeFailed(format!(
                "no unsigned constant representation for width {width}"
            ))
        })?;
        let ty = function.get_node(lhs_id).unwrap().ty.clone();
        let mask = builder.constant(mask_data, ty, Span::unknown());
        let and_op = builder.bitwise_and(
            LocalNodeId::new(lhs_id.as_u64()),
            mask,
            Span::unknown(),
        );

        Ok(builder.finish(vec![ReplacementValue {
            old: result_id,
            new: and_op,
        }]))
    }
}

fn unsigned_constant(width: usize, value: u64) -> Option<ConstantData> {
    Some(match width {
        8 => ConstantData::u8(value as u8),
        16 => ConstantData::u16(value as u16),
        32 => ConstantData::u32(value as u32),
        64 => ConstantData::u64(value),
        _ => return None,
    })
}
