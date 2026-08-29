use sir_transform::ids::DefinitionId;
use sir_transform::roles::PermutationKind;
use sir_types::Span;

use crate::error::RewriteError;
use crate::local_id::LocalNodeId;
use crate::patch::{ReplacementPatch, ReplacementValue};
use crate::recipe::RewriteRecipe;
use crate::region::RewriteRegion;
use crate::subgraph_builder::SubgraphBuilder;

/// Recipe for the circular-rotation transformation.
///
/// Replaces the entire shift pair `(x << k) | (x >> (w - k))` with a single
/// rotate instruction. The direction is fixed by the candidate's definition
/// id: 310 emits `rol`, 311 emits `ror`. The recipe consumes the
/// `BitPermutation` role (operand + amount) — it never re-discovers the
/// shift pair from the physical graph, and it replaces the top Or result,
/// not any constituent shift.
pub struct RotateRecipe {
    id: DefinitionId,
    /// Emit `rol` when true, `ror` when false.
    left: bool,
}

impl RotateRecipe {
    pub fn new_left(id: DefinitionId) -> Self {
        Self { id, left: true }
    }

    pub fn new_right(id: DefinitionId) -> Self {
        Self { id, left: false }
    }
}

impl RewriteRecipe for RotateRecipe {
    fn definition(&self) -> DefinitionId {
        self.id
    }

    fn name(&self) -> &'static str {
        if self.left {
            "Rotate Left"
        } else {
            "Rotate Right"
        }
    }

    fn build_patch(
        &self,
        _function: &sir_nodes::Function,
        region: &RewriteRegion,
        mut builder: SubgraphBuilder,
    ) -> Result<ReplacementPatch, RewriteError> {
        let result = region.result()?;
        let (operand, kind) = region.permutation()?;
        let amount = match &kind {
            PermutationKind::Circular { amount, .. } => *amount,
            other => {
                return Err(RewriteError::MissingRole {
                    role: format!("Circular permutation kind, found {:?}", other),
                })
            }
        };

        let operand_local = LocalNodeId::new(operand.as_u64());
        let amount_local = LocalNodeId::new(amount.as_u64());

        let rotate = if self.left {
            builder.rol(operand_local, amount_local, Span::unknown())
        } else {
            builder.ror(operand_local, amount_local, Span::unknown())
        };

        Ok(builder.finish(vec![ReplacementValue {
            old: result,
            new: rotate,
        }]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sir_nodes::{Function, Node, NodeKind};
    use sir_semantics::structure::StructuralDescription;
    use sir_transform::roles::{PermutationKind, RegionRoles, ShiftDirection};
    use sir_transform::structures::SourceStructure;
    use sir_types::{NodeId, RegionId};

    fn make_rotate_region(amount: NodeId) -> RewriteRegion {
        let structural = StructuralDescription::new(
            RegionId::new(0),
            SourceStructure::BitPermutation { width: 64 },
        )
        .with_roles(RegionRoles::BitPermutation {
            operand: NodeId::new(0),
            result: NodeId::new(5),
            kind: PermutationKind::Circular {
                direction: ShiftDirection::Left,
                amount,
            },
        });

        RewriteRegion::new(structural)
    }

    fn make_function() -> Function {
        let mut func = Function::new("rotate_left", sir_types::Type::u64());
        // operand node 0, amount node 1, result (Or) node 5.
        for id in [0u64, 1, 5] {
            func.arena.insert(Node::new(
                NodeId::new(id),
                NodeKind::Constant(sir_types::ConstantData::u64(0)),
                sir_types::Type::u64(),
                sir_types::Effects::empty(),
                sir_types::Span::unknown(),
            ));
        }
        func
    }

    #[test]
    fn left_recipe_emits_rol_replacing_top_or() {
        let recipe = RotateRecipe::new_left(DefinitionId::new(310));
        let region = make_rotate_region(NodeId::new(1));
        let builder = SubgraphBuilder::new();
        let func = make_function();

        let patch = recipe.build_patch(&func, &region, builder).unwrap();
        assert_eq!(patch.replacements.len(), 1);
        assert_eq!(patch.replacements[0].old, NodeId::new(5));

        let mut rol_seen = false;
        for (_, n) in patch.arena.iter() {
            if let NodeKind::Rol { lhs, rhs } = &n.kind {
                rol_seen = true;
                assert_eq!(lhs, &NodeId::new(0));
                assert_eq!(rhs, &NodeId::new(1));
            }
        }
        assert!(rol_seen, "expected a Rol node");
    }
}
