use sir_types::Span;
use crate::error::RewriteError;
use crate::local_id::LocalNodeId;
use crate::region::RewriteRegion;
use crate::subgraph_builder::SubgraphBuilder;

/// Shared helper to emit the initial `pack(board)` operation for bitset reductions.
pub fn emit_pack(
    region: &RewriteRegion,
    builder: &mut SubgraphBuilder,
) -> Result<LocalNodeId, RewriteError> {
    let collection = region.collection()?;
    let packed = builder.pack(
        LocalNodeId::new(collection.as_u64()),
        Span::unknown(),
    );
    Ok(packed)
}
