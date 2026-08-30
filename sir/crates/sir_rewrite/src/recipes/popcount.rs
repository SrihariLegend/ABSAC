use sir_transform::ids::DefinitionId;
use sir_types::Span;

use crate::error::RewriteError;
use crate::patch::{ReplacementPatch, ReplacementValue};
use crate::recipe::RewriteRecipe;
use crate::recipes::helpers::{collection_length, find_tuple_extract, loop_reduction_position, wrap_direct_tuple_return};
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

        // Existing Pack + Popcount strategy (loop elimination).
        // Used for: Array<Bool> collections, SetIteration (scalar BK popcount),
        // and PredicateCollectionReduction patterns.
        let mut old_result = region.result()?;
        let accumulator = region.accumulator().ok().flatten();

        // Prefer replacing the TupleExtract consumer when one exists; when the tuple
        // is returned wholesale the loop's tuple result is rebuilt below.
        if let Some(extract) = find_tuple_extract(function, old_result) {
            old_result = extract;
        }

        // Type the popcount from the replaced node.
        let mut pop_ty = function.get_node(old_result).unwrap().ty.clone();
        if let Type::Tuple { elements } = &pop_ty {
            if let Some(pos) = loop_reduction_position(function, old_result, accumulator) {
                pop_ty = elements[pos].clone();
            }
        }

        let packed = if let Some(set_val) = region.structural.roles.iter().find_map(|r| {
            if let sir_transform::roles::RegionRoles::SetIteration { set_value, .. } = r {
                Some(*set_value)
            } else {
                None
            }
        }) {
            use crate::local_id::LocalNodeId;
            LocalNodeId::new(set_val.as_u64())
        } else {
            crate::recipes::helpers::emit_pack(function, region, &mut builder)?
        };
        let pop = builder.popcount(packed, pop_ty, Span::unknown());

        let new_value = wrap_direct_tuple_return(
            function,
            old_result,
            accumulator,
            collection_length(region),
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
        let region = make_test_region();
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
            Err(RewriteError::MissingRole { .. }) => {} // expected
            other => panic!("expected MissingRole, got {:?}", other),
        }
    }
}
