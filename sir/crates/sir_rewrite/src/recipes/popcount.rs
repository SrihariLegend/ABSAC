use sir_transform::ids::DefinitionId;
use sir_types::Span;

use crate::error::RewriteError;
use crate::patch::{ReplacementPatch, ReplacementValue};
use crate::recipe::RewriteRecipe;
use crate::recipes::helpers::{authorized_tuple_consumer, collection_length, loop_reduction_position, wrap_direct_tuple_return};
use crate::region::RewriteRegion;
use crate::subgraph_builder::SubgraphBuilder;
use sir_types::Type;

use sir_nodes::NodeKind;
use sir_types::NodeId;

/// Recipe for the Popcount transformation.
///
/// Two strategies depending on collection type:
///
/// - **Array<Bool>** (boolean collection): eliminates the loop entirely.
///   `pack(board) -> popcount(packed)` replaces the loop result.
///
/// - **Array<Integer>** (e.g., byte buffer): preserves the loop but replaces
///   the per-element table-lookup (`bitsinbyte[byte]`) with a native
///   `Popcount(byte)` instruction. This is the Redis BITCOUNT rewrite:
///   `bits += bitsinbyte[buf[i]]` becomes `bits += popcount(buf[i])`.
pub struct PopcountRecipe {
    id: DefinitionId,
}

impl PopcountRecipe {
    pub fn new(id: DefinitionId) -> Self {
        Self { id }
    }

    /// Rewrite for non-boolean collections (e.g., Array<u8> byte buffers).
    ///
    /// The loop body contains a table-lookup pattern:
    ///   byte = ArrayAccess(collection, i)   // read element from buffer
    ///   to_add = ArrayAccess(table, byte)   // lookup popcount in table
    ///   acc = acc + to_add                   // accumulate
    ///
    /// The rewrite replaces the table lookup with a native Popcount:
    ///   to_add = Popcount(byte)              // native popcount instruction
    ///
    /// The loop structure is preserved. Only the table-lookup ArrayAccess
    /// node is replaced.
    fn build_table_lookup_patch(
        &self,
        function: &sir_nodes::Function,
        _region: &RewriteRegion,
        collection: NodeId,
        mut builder: SubgraphBuilder,
    ) -> Result<ReplacementPatch, RewriteError> {
        use crate::local_id::LocalNodeId;

        // Find the Loop node in the function.
        let mut loop_node: Option<&sir_nodes::Node> = None;
        for node in function.arena.iter() {
            if matches!(node.kind, NodeKind::Loop { .. }) {
                loop_node = Some(node);
                break;
            }
        }
        let loop_node = loop_node.ok_or_else(|| RewriteError::RecipeFailed(
            "table-lookup rewrite: no Loop node found".to_string()
        ))?;

        let body_ids: Vec<NodeId> = match &loop_node.kind {
            NodeKind::Loop { body, .. } => body.clone(),
            _ => unreachable!(),
        };

        // Find the buffer-access ArrayAccess: ArrayAccess(collection, i)
        let mut buffer_access_id: Option<NodeId> = None;
        for &id in &body_ids {
            if let Some(node) = function.get_node(id) {
                if let NodeKind::ArrayAccess { base, .. } = &node.kind {
                    if *base == collection {
                        buffer_access_id = Some(id);
                        break;
                    }
                }
            }
        }
        let buffer_access_id = buffer_access_id.ok_or_else(|| RewriteError::RecipeFailed(
            "table-lookup rewrite: no ArrayAccess on collection found in loop body".to_string()
        ))?;

        // Find the table-lookup ArrayAccess: ArrayAccess(table, byte)
        // Its index is the buffer-access node, and its base is NOT the collection.
        let mut table_lookup_id: Option<NodeId> = None;
        for &id in &body_ids {
            if id == buffer_access_id {
                continue;
            }
            if let Some(node) = function.get_node(id) {
                if let NodeKind::ArrayAccess { base, index } = &node.kind {
                    if *index == buffer_access_id && *base != collection {
                        table_lookup_id = Some(id);
                        break;
                    }
                }
            }
        }
        let table_lookup_id = table_lookup_id.ok_or_else(|| RewriteError::RecipeFailed(
            "table-lookup rewrite: no table-lookup ArrayAccess found in loop body".to_string()
        ))?;

        // Type the Popcount to match the table-lookup result (e.g., u64).
        let lookup_ty = function.get_node(table_lookup_id).unwrap().ty.clone();

        // Build Popcount(buffer_access) in the subgraph. The operand references
        // an existing function node, preserved as-is during patch application.
        let byte_local = LocalNodeId::new(buffer_access_id.as_u64());
        let pop = builder.popcount(byte_local, lookup_ty, Span::unknown());

        // Replace the table-lookup ArrayAccess with the Popcount node.
        // All uses (e.g., the Add accumulator) are rewired to Popcount.
        Ok(builder.finish(vec![ReplacementValue {
            old: table_lookup_id,
            new: pop,
        }]))
    }
}

impl RewriteRecipe for PopcountRecipe {
    fn definition(&self) -> DefinitionId {
        self.id
    }

    fn name(&self) -> &'static str {
        "Popcount"
    }

    fn build_patch(
        &self,
        function: &sir_nodes::Function,
        region: &RewriteRegion,
        mut builder: SubgraphBuilder,
    ) -> Result<ReplacementPatch, RewriteError> {
        // If the region has a SetIteration role (scalar/BitsetIteration patterns
        // like Brian Kernighan popcount), use the existing Pack+Popcount path.
        let has_set_iteration = region.structural.roles.iter().any(|r| {
            matches!(r, sir_transform::roles::RegionRoles::SetIteration { .. })
        });

        // PredicateCollectionReduction routes through ArrayCmpMask, not table-lookup.
        let has_predicate_collection = region.structural.roles.iter().any(|r| {
            matches!(r, sir_transform::roles::RegionRoles::PredicateCollectionReduction { .. })
        });

        if !has_set_iteration && !has_predicate_collection {
            // Check the collection type. Only route to table-lookup for
            // non-boolean arrays. Bool arrays use the existing Pack path.
            if let Ok(collection) = region.collection() {
                let collection_ty = function.get_node(collection).map(|n| n.ty.clone());
                let is_bool_array = matches!(
                    collection_ty,
                    Some(Type::Array { ref element, .. }) if **element == Type::Bool
                );
                if !is_bool_array {
                    return self.build_table_lookup_patch(function, region, collection, builder);
                }
            }
        }

        // Pack + Popcount strategy (loop elimination): Array<Bool>
        // collections, SetIteration (scalar BK popcount), and
        // PredicateCollectionReduction patterns.
        //
        // Collection reductions consume the authorized ProposalBinding;
        // the scalar SetIteration path has no binding (the engine binds
        // collection reductions only) and is driven by its role.
        let set_val = region.structural.roles.iter().find_map(|r| {
            if let sir_transform::roles::RegionRoles::SetIteration { set_value, .. } = r {
                Some(*set_value)
            } else {
                None
            }
        });

        let (old_result, pop_ty, packed, fallback_bound) = if region.binding.is_some() {
            let map = &crate::recipes::helpers::require_binding(region, "Popcount")?.map;
            let target = crate::recipes::helpers::binding_target(region, "Popcount")?;
            let width = crate::recipes::helpers::binding_collection_extent(
                function,
                region,
                "Popcount",
            )?
            .1;
            // Type the popcount from the authorized reduction position.
            let node_ty = function
                .get_node(target)
                .map(|n| n.ty.clone())
                .unwrap_or(sir_types::Type::Unit);
            let pop_ty = match node_ty {
                Type::Tuple { elements } => elements
                    .get(map.reduction_position)
                    .cloned()
                    .unwrap_or(sir_types::Type::Unit),
                other => other,
            };
            let packed = match set_val {
                Some(set_val) => crate::local_id::LocalNodeId::new(set_val.as_u64()),
                None => crate::recipes::helpers::emit_pack_from_binding(
                    function,
                    region,
                    "Popcount",
                    &mut builder,
                )?
                .0,
            };
            (target, pop_ty, packed, Some(width))
        } else {
            let Some(set_val) = set_val else {
                return Err(RewriteError::RecipeFailed(
                    "Popcount requires an application binding (ProposalBinding) or a \
                     SetIteration scalar role; neither was derived"
                        .to_string(),
                ));
            };
            let mut old_result = region.result()?;
            let accumulator = region.accumulator().ok().flatten();
            // Replace the tuple-slot consumer ONLY when it reads the
            // accumulator slot (PS002 audit).
            if let Some(extract) = authorized_tuple_consumer(function, old_result, accumulator)? {
                old_result = extract;
            }
            let mut pop_ty = function.get_node(old_result).unwrap().ty.clone();
            if let Type::Tuple { elements } = &pop_ty {
                if let Some(pos) = loop_reduction_position(function, old_result, accumulator) {
                    pop_ty = elements[pos].clone();
                }
            }
            (
                old_result,
                pop_ty,
                crate::local_id::LocalNodeId::new(set_val.as_u64()),
                collection_length(region),
            )
        };
        let pop = builder.popcount(packed, pop_ty, Span::unknown());

        let new_value = wrap_direct_tuple_return(
            function,
            old_result,
            region
                .binding
                .as_ref()
                .map(|b| b.map.accumulator),
            fallback_bound,
            pop,
            &mut builder,
        )?;

        Ok(builder.finish(vec![ReplacementValue {
            old: old_result,
            new: new_value,
        }]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sir_semantics::structure::StructuralDescription;
    use sir_transform::roles::RegionRoles;
    use sir_transform::structures::SourceStructure;
    use sir_types::RegionId;

    fn make_test_region() -> RewriteRegion {
        let structural = StructuralDescription::new(
            RegionId::new(0),
            SourceStructure::LogicalSequence { length: 64 },
        )
        .with_roles(RegionRoles::BooleanCollectionReduction {
            collection: sir_types::NodeId::new(10),
            accumulator: None,
            result: sir_types::NodeId::new(20),
        });

        RewriteRegion::new(structural)
    }

    /// A minimal authorized-binding stand-in: one reconstructed
    /// observable at `use_site`, role map pointing at `collection`.
    fn test_binding(
        collection: sir_types::NodeId,
        element_access: sir_types::NodeId,
        predicate_op: Option<sir_nodes::CmpOperator>,
        predicate_scalar: Option<sir_types::NodeId>,
        use_site: sir_types::NodeId,
    ) -> sir_semantics::binding::ProposalBinding {
        use sir_semantics::authorization::IntegerSemantics;
        use sir_semantics::binding::{
            FrameCondition, LiveOutBinding, LiveOutKind, LiveOutSlot, PredicateScalarClass,
            ProposalBinding, ReductionRoleMap,
        };
        ProposalBinding {
            map: ReductionRoleMap {
                region: RegionId::new(0),
                loop_node: sir_types::NodeId::new(8),
                collection,
                element_access,
                induction: None,
                start: None,
                bound: None,
                stride: 1,
                predicate: Some(element_access),
                predicate_op,
                predicate_scalar,
                predicate_scalar_class: PredicateScalarClass::None,
                accumulator: sir_types::NodeId::new(3),
                recurrence: "bitwise_or".to_string(),
                identity: None,
                reduction_position: 0,
                live_ins: vec![],
                effects: sir_types::Effects::empty(),
                integer_semantics: IntegerSemantics::Modular,
            },
            live_outs: vec![LiveOutSlot {
                kind: LiveOutKind::Slot(0),
                binding: LiveOutBinding::Reconstructed { slot: 0 },
                closure: None,
                use_site: Some(use_site),
            }],
            frame: FrameCondition {
                source_reads: vec![],
                source_writes: false,
                has_volatile_or_atomic: false,
                has_calls: false,
                possible_traps: false,
                terminates: true,
                single_normal_exit: true,
                output_count: 2,
                trip_count: None,
            },
        }
    }

    #[test]
    fn popcount_recipe_has_correct_definition_id() {
        let recipe = PopcountRecipe::new(DefinitionId::new(42));
        assert_eq!(recipe.definition(), DefinitionId::new(42));
    }

    #[test]
    fn popcount_recipe_has_correct_name() {
        let recipe = PopcountRecipe::new(DefinitionId::new(0));
        assert_eq!(recipe.name(), "Popcount");
    }

    #[test]
    fn popcount_recipe_produces_patch_with_correct_structure() {
        let recipe = PopcountRecipe::new(DefinitionId::new(0));
        let region = make_test_region().with_binding(test_binding(
            sir_types::NodeId::new(10),
            sir_types::NodeId::new(10),
            None,
            None,
            sir_types::NodeId::new(20),
        ));
        let builder = SubgraphBuilder::new();
        // The region result (node 20) must exist in the function with a scalar
        // type: the recipe reads its type to type the popcount, and the tuple
        // rebuild passes a non-tuple result through unchanged.
        let mut func = sir_nodes::Function::new("test", sir_types::Type::Unit);
        // The collection (node 10) must exist as a bool array parameter so the
        // recipe's type check routes to the Pack+Popcount path.
        func.arena.insert(sir_nodes::Node::new(
            sir_types::NodeId::new(10),
            sir_nodes::NodeKind::Parameter { index: 0 },
            sir_types::Type::Array { element: Box::new(sir_types::Type::Bool), length: 64 },
            sir_types::Effects::empty(),
            sir_types::Span::unknown(),
        ));
        func.arena.insert(sir_nodes::Node::new(
            sir_types::NodeId::new(20),
            sir_nodes::NodeKind::Constant(sir_types::ConstantData::i32(0)),
            sir_types::Type::i32(),
            sir_types::Effects::empty(),
            sir_types::Span::unknown(),
        ));

        let patch = recipe.build_patch(&func, &region, builder).unwrap();

        // The patch contains 2 nodes: Pack + Popcount
        assert_eq!(patch.arena.len(), 2);

        // One replacement: result -> popcount
        assert_eq!(patch.replacements.len(), 1);
        assert_eq!(patch.replacements[0].old, sir_types::NodeId::new(20));
    }

    #[test]
    fn popcount_recipe_fails_without_collection_role() {
        let recipe = PopcountRecipe::new(DefinitionId::new(0));
        // Create a region without roles
        let structural = StructuralDescription::new(
            RegionId::new(0),
            SourceStructure::LogicalSequence { length: 64 },
        );
        let region = RewriteRegion::new(structural);
        let builder = SubgraphBuilder::new();
        let func = sir_nodes::Function::new("test", sir_types::Type::Unit);

        let result = recipe.build_patch(&func, &region, builder);
        assert!(result.is_err());
        match result {
            Err(RewriteError::RecipeFailed(ref msg))
                if msg.contains("requires an application binding") => {}
            other => panic!("expected a binding refusal, got {:?}", other),
        }
    }

    /// P0A: predicate masks must use the binding's TRUE operator. The old
    /// structural `emit_pack` hardcoded `Gt`, so an `==` collection would
    /// have been rewritten as `>`.
    #[test]
    fn popcount_recipe_uses_the_bindings_true_operator() {
        let recipe = PopcountRecipe::new(DefinitionId::new(0));
        let structural = StructuralDescription::new(
            RegionId::new(0),
            SourceStructure::DynamicBooleanSequence { length: 64 },
        )
        .with_roles(RegionRoles::PredicateCollectionReduction {
            collection: sir_types::NodeId::new(10),
            scalar: sir_types::NodeId::new(12),
            operator: sir_types::NodeId::new(13),
            accumulator: None,
            result: sir_types::NodeId::new(20),
        });
        let region = RewriteRegion::new(structural).with_binding(test_binding(
            sir_types::NodeId::new(10),
            sir_types::NodeId::new(10),
            Some(sir_nodes::CmpOperator::Eq),
            Some(sir_types::NodeId::new(12)),
            sir_types::NodeId::new(20),
        ));

        let mut func = sir_nodes::Function::new("test", sir_types::Type::Unit);
        func.arena.insert(sir_nodes::Node::new(
            sir_types::NodeId::new(10),
            sir_nodes::NodeKind::Parameter { index: 0 },
            sir_types::Type::Array {
                element: Box::new(sir_types::Type::u8()),
                length: 64,
            },
            sir_types::Effects::empty(),
            sir_types::Span::unknown(),
        ));
        func.arena.insert(sir_nodes::Node::new(
            sir_types::NodeId::new(12),
            sir_nodes::NodeKind::Constant(sir_types::ConstantData::u8(7)),
            sir_types::Type::u8(),
            sir_types::Effects::empty(),
            sir_types::Span::unknown(),
        ));
        func.arena.insert(sir_nodes::Node::new(
            sir_types::NodeId::new(20),
            sir_nodes::NodeKind::Constant(sir_types::ConstantData::i32(0)),
            sir_types::Type::i32(),
            sir_types::Effects::empty(),
            sir_types::Span::unknown(),
        ));

        let patch = recipe
            .build_patch(&func, &region, SubgraphBuilder::new())
            .unwrap();
        let mut mask_op = None;
        for (_, node) in patch.arena.iter() {
            if let sir_nodes::NodeKind::ArrayCmpMask { op, .. } = &node.kind {
                mask_op = Some(*op);
            }
        }
        assert_eq!(
            mask_op,
            Some(sir_nodes::CmpOperator::Eq),
            "the mask must use the binding's operator, not a hardcoded Gt"
        );
    }

    /// The binding is authoritative: a stale/mutated structural role that
    /// points somewhere else must not silently become the rewrite source.
    #[test]
    fn stale_structural_collection_is_not_a_fallback() {
        let recipe = PopcountRecipe::new(DefinitionId::new(0));
        // Structural role points at the valid collection (10); the
        // binding points at a node that does not exist (99).
        let region = make_test_region().with_binding(test_binding(
            sir_types::NodeId::new(99),
            sir_types::NodeId::new(99),
            None,
            None,
            sir_types::NodeId::new(20),
        ));
        let mut func = sir_nodes::Function::new("test", sir_types::Type::Unit);
        func.arena.insert(sir_nodes::Node::new(
            sir_types::NodeId::new(10),
            sir_nodes::NodeKind::Parameter { index: 0 },
            sir_types::Type::Array {
                element: Box::new(sir_types::Type::Bool),
                length: 64,
            },
            sir_types::Effects::empty(),
            sir_types::Span::unknown(),
        ));
        func.arena.insert(sir_nodes::Node::new(
            sir_types::NodeId::new(20),
            sir_nodes::NodeKind::Constant(sir_types::ConstantData::i32(0)),
            sir_types::Type::i32(),
            sir_types::Effects::empty(),
            sir_types::Span::unknown(),
        ));
        let result = recipe.build_patch(&func, &region, SubgraphBuilder::new());
        match result {
            Err(RewriteError::RecipeFailed(ref msg))
                if msg.contains("no declared array extent") => {}
            other => panic!(
                "a binding whose collection has no extent must refuse, got {:?}",
                other
            ),
        }
    }
}
