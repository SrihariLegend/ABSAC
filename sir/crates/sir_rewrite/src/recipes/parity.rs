use sir_transform::ids::DefinitionId;
use sir_types::{ConstantData, Span};

use crate::error::RewriteError;
use crate::patch::{ReplacementPatch, ReplacementValue};
use crate::recipe::RewriteRecipe;
use crate::recipes::helpers::{
    authorized_tuple_consumer, binding_collection_extent, binding_target,
    collection_length, emit_pack_from_binding, wrap_direct_tuple_return,
};
use crate::region::RewriteRegion;
use crate::subgraph_builder::SubgraphBuilder;

/// Recipe for the Parity transformation.
///
/// Replaces a boolean-array exclusive loop with:
///   pack(board) → popcount(packed) → (popcount & 1) != 0
pub struct ParityRecipe {
    id: DefinitionId,
}

impl ParityRecipe {
    pub fn new(id: DefinitionId) -> Self {
        Self { id }
    }
}

impl RewriteRecipe for ParityRecipe {
    fn definition(&self) -> DefinitionId {
        self.id
    }

    fn name(&self) -> &'static str {
        "Parity"
    }

    fn build_patch(
        &self,
        function: &sir_nodes::Function,
        region: &RewriteRegion,
        mut builder: SubgraphBuilder,
    ) -> Result<ReplacementPatch, RewriteError> {
        // Scalar Kernighan-parity path: the region is a BitsetIteration
        // loop over a scalar value (no array to pack, and no
        // ProposalBinding — the engine binds collection reductions only),
        // so popcount the set value directly.
        let set_val = region.structural.roles.iter().find_map(|r| {
            if let sir_transform::roles::RegionRoles::SetIteration { set_value, .. } = r {
                Some(*set_value)
            } else {
                None
            }
        });

        let (target, accumulator, packed, whole_value, fallback_bound) =
            if region.binding.is_some() {
                // Collection reduction: consume the authorized binding.
                let map = &crate::recipes::helpers::require_binding(region, "Parity")?.map;
                let target = binding_target(region, "Parity")?;
                let width = binding_collection_extent(function, region, "Parity")?.1;
                let packed = match set_val {
                    Some(set_val) => crate::local_id::LocalNodeId::new(set_val.as_u64()),
                    None => emit_pack_from_binding(function, region, "Parity", &mut builder)?.0,
                };
                (
                    target,
                    Some(map.accumulator),
                    packed,
                    target == map.loop_node,
                    Some(width),
                )
            } else {
                let Some(set_val) = set_val else {
                    return Err(RewriteError::RecipeFailed(
                        "Parity requires an application binding (ProposalBinding) or a \
                         SetIteration scalar role; neither was derived"
                            .to_string(),
                    ));
                };
                let result = region.result()?;
                let accumulator = region.accumulator().ok().flatten();
                // The consumer must read the accumulator slot (PS002).
                let extract = authorized_tuple_consumer(function, result, accumulator)?;
                let target = extract.unwrap_or(result);
                (
                    target,
                    accumulator,
                    crate::local_id::LocalNodeId::new(set_val.as_u64()),
                    extract.is_none(),
                    collection_length(region),
                )
            };

        // The replacement must match the target's type: a Bool slot gets
        // (popcount & 1) != 0; an integer slot gets popcount & 1 directly
        // (e.g. a Kernighan parity counter that a surrounding comparison
        // turns into Bool). When the tuple is returned wholesale there is no
        // typed slot to match, so emit the Bool form (the reduction slot of
        // the rebuilt tuple is Bool).
        let target_ty = if !whole_value {
            function.get_node(target).unwrap().ty.clone()
        } else {
            sir_types::Type::Bool
        };
        let want_bool = target_ty.is_bool();
        let num_ty = if want_bool {
            sir_types::Type::i32()
        } else {
            target_ty
        };
        let pop = builder.popcount(packed, num_ty.clone(), Span::unknown());

        // Build a constant of the numeric type for the & 1 and the != 0 checks.
        fn mk_const(
            builder: &mut SubgraphBuilder,
            value: u64,
            ty: &sir_types::Type,
        ) -> crate::local_id::LocalNodeId {
            let data = match ty {
                sir_types::Type::Integer { signed: true, .. } => ConstantData::i32(value as i32),
                _ => ConstantData::u64(value),
            };
            builder.constant(data, ty.clone(), Span::unknown())
        }
        let one = mk_const(&mut builder, 1, &num_ty);
        let and_one = builder.bitwise_and(pop, one, Span::unknown());

        // Parity is (pop & 1) != 0 which returns a boolean.
        let new_value = if want_bool {
            let zero = mk_const(&mut builder, 0, &num_ty);
            builder.ne(and_one, zero, Span::unknown())
        } else {
            and_one
        };

        let new_value = if whole_value {
            wrap_direct_tuple_return(
                function,
                target,
                accumulator,
                fallback_bound,
                new_value,
                &mut builder,
            )?
        } else {
            new_value
        };

        Ok(builder.finish(vec![ReplacementValue {
            old: target,
            new: new_value,
        }]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sir_nodes::{Function, Node, NodeKind};
    use sir_semantics::structure::StructuralDescription;
    use sir_transform::roles::RegionRoles;
    use sir_transform::structures::SourceStructure;
    use sir_types::{NodeId, RegionId};

    /// A scalar BitsetIteration region: the loop result (node 8) is consumed
    /// by a TupleExtract (node 9); the set value is node 0.
    fn make_scalar_region() -> RewriteRegion {
        let structural = StructuralDescription::new(
            RegionId::new(0),
            SourceStructure::BitMask { width: 64 },
        )
        .with_roles(RegionRoles::SetIteration {
            set_value: NodeId::new(0),
            result: NodeId::new(8),
        });

        RewriteRegion::new(structural)
    }

    fn make_extract_function() -> Function {
        let mut func = Function::new("bk_parity", sir_types::Type::Bool);
        // Region result (loop) node 8, tuple extract node 9 (u64).
        func.arena.insert(Node::new(
            NodeId::new(8),
            NodeKind::Loop {
                body: vec![],
                termination: NodeId::new(7),
                outputs: vec![NodeId::new(3)],
                carried_inputs: vec![NodeId::new(0), NodeId::new(3)],
            },
            sir_types::Type::Tuple {
                elements: vec![sir_types::Type::u64(), sir_types::Type::u64()],
            },
            sir_types::Effects::empty(),
            sir_types::Span::unknown(),
        ));
        func.arena.insert(Node::new(
            NodeId::new(9),
            NodeKind::TupleExtract {
                tuple: NodeId::new(8),
                index: 0,
            },
            sir_types::Type::u64(),
            sir_types::Effects::empty(),
            sir_types::Span::unknown(),
        ));
        func
    }

    #[test]
    fn parity_recipe_emits_scalar_popcount_and_one() {
        let recipe = ParityRecipe::new(DefinitionId::new(6));
        let region = make_scalar_region();
        let builder = SubgraphBuilder::new();
        let func = make_extract_function();

        let patch = recipe.build_patch(&func, &region, builder).unwrap();

        // Replacement targets the tuple extract (node 9), not the loop.
        assert_eq!(patch.replacements.len(), 1);
        assert_eq!(patch.replacements[0].old, NodeId::new(9));

        // The new value is a u64 popcount & 1 (no trailing != 0: the extract's
        // u64 consumer performs the comparison). Find the And node.
        let mut and_seen = false;
        let mut popcount_seen = false;
        for (_, n) in patch.arena.iter() {
            match &n.kind {
                NodeKind::Popcount { .. } => popcount_seen = true,
                NodeKind::And { .. } => {
                    and_seen = true;
                    assert!(n.ty.is_integer(), "And must be integer-typed, got {:?}", n.ty);
                }
                _ => {}
            }
        }
        assert!(popcount_seen, "expected a Popcount node");
        assert!(and_seen, "expected an And (popcount & 1) node");
    }
}
