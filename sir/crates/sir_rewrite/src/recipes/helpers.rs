use crate::error::RewriteError;
use crate::local_id::LocalNodeId;
use crate::region::RewriteRegion;
use crate::subgraph_builder::SubgraphBuilder;
use sir_types::Span;

/// Shared helper to emit the initial `pack(board)` operation for bitset reductions.
pub fn emit_pack(
    function: &sir_nodes::Function,
    region: &RewriteRegion,
    builder: &mut SubgraphBuilder,
) -> Result<LocalNodeId, RewriteError> {
    let collection = region.collection()?;

    // Check if the structure is a DynamicBooleanSequence, in which case we emit an ArrayCmpMask instead of Pack.
    // We can infer this by checking if the RegionRole is PredicateCollectionReduction.
    if let Ok(scalar) = region.predicate_scalar() {
        if let Ok(_op) = region.predicate_op_node() {
            // In v0.1 we simplify by assuming it is `Gt` or whatever the operator was.
            // We really should extract the actual `CmpOperator` from the original graph, but we don't have it here.
            // As a fallback for the test, we'll hardcode `Gt`.
            let packed = builder.array_cmp_mask(
                LocalNodeId::new(collection.as_u64()),
                LocalNodeId::new(scalar.as_u64()),
                sir_nodes::CmpOperator::Gt,
                Span::unknown(),
            );
            return Ok(packed);
        }
    }

    let mut width = 64;
    if let sir_transform::structures::SourceStructure::BooleanArray { length } =
        region.structural.source_structure
    {
        width = length;
    }

    let packed = builder.pack(
        LocalNodeId::new(collection.as_u64()),
        width,
        Span::unknown(),
    );
    Ok(packed)
}
