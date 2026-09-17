use sir_transform::ids::DefinitionId;
use sir_types::{ConstantData, Span, Type};

use crate::error::RewriteError;
use crate::local_id::LocalNodeId;
use crate::patch::{ReplacementPatch, ReplacementValue};
use crate::recipe::RewriteRecipe;
use crate::region::RewriteRegion;
use crate::subgraph_builder::SubgraphBuilder;

/// Recipe for Shift Sequence -> Mask Extract transformation.
///
/// `(x << k) >> k` (constant `k`, unsigned `x`) extracts the low
/// `W - k` bits: `x & ((1 << (W-k)) - 1)`. The old recipe emitted
/// `x & (k << k)` — a stub with no relation to the identity; this
/// implementation validates the shape and emits the real mask.
pub struct ShiftMaskRecipe {
    id: DefinitionId,
}

impl ShiftMaskRecipe {
    pub fn new(id: DefinitionId) -> Self {
        Self { id }
    }
}

impl RewriteRecipe for ShiftMaskRecipe {
    fn definition(&self) -> DefinitionId {
        self.id
    }

    fn name(&self) -> &'static str {
        "ShiftMask"
    }

    fn build_patch(
        &self,
        function: &sir_nodes::Function,
        region: &RewriteRegion,
        mut builder: SubgraphBuilder,
    ) -> Result<ReplacementPatch, RewriteError> {
        let _op = region.operator_node()?;
        let shl_node = region.lhs()?;
        let result_id = region.result()?;

        let (inner, k_node) = match function.get_node(shl_node).map(|n| &n.kind) {
            Some(sir_nodes::NodeKind::Shl { lhs, rhs }) => (*lhs, *rhs),
            _ => {
                return Err(RewriteError::RecipeFailed(
                    "shift-mask: region lhs is not the inner shift-left".to_string(),
                ))
            }
        };
        let k = match function.get_node(k_node).map(|n| &n.kind) {
            Some(sir_nodes::NodeKind::Constant(data)) => data
                .as_u64()
                .or_else(|| data.as_i64().and_then(|v| u64::try_from(v).ok())),
            _ => None,
        }
        .ok_or_else(|| {
            RewriteError::RecipeFailed("shift-mask: shift amount is not a constant".to_string())
        })?;
        let (width, signed) = match function.get_node(inner).map(|n| &n.ty) {
            Some(Type::Integer { width, signed, .. }) => (width.bits() as usize, *signed),
            _ => {
                return Err(RewriteError::RecipeFailed(
                    "shift-mask: inner value has no integer width".to_string(),
                ))
            }
        };
        if signed {
            return Err(RewriteError::RecipeFailed(
                "shift-mask is only valid for unsigned operands".to_string(),
            ));
        }
        if width == 0 || width > 64 || k >= width as u64 {
            return Err(RewriteError::RecipeFailed(
                "shift-mask amount is outside [0, width)".to_string(),
            ));
        }
        let mask = if k == 0 {
            if width == 64 {
                u64::MAX
            } else {
                (1u64 << width) - 1
            }
        } else {
            (1u64 << (width - k as usize)) - 1
        };
        let mask_data = unsigned_constant(width, mask).ok_or_else(|| {
            RewriteError::RecipeFailed(format!(
                "shift-mask: no unsigned constant representation for width {width}"
            ))
        })?;
        let ty = function.get_node(inner).unwrap().ty.clone();
        let mask_const = builder.constant(mask_data, ty, Span::unknown());
        let and_op = builder.bitwise_and(
            LocalNodeId::new(inner.as_u64()),
            mask_const,
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
