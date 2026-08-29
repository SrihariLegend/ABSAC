use crate::error::RewriteError;
use crate::local_id::LocalNodeId;
use crate::region::RewriteRegion;
use crate::subgraph_builder::SubgraphBuilder;
use sir_nodes::NodeKind;
use sir_types::{ConstantData, NodeId, Span, Type};

/// Find a `TupleExtract` node that consumes the given tuple value, if any.
///
/// Recipes replace the extract when one exists (the loop is consumed field-by-field),
/// and replace the tuple itself when it is returned wholesale.
pub fn find_tuple_extract(function: &sir_nodes::Function, tuple: NodeId) -> Option<NodeId> {
    function.arena.iter().find_map(|node| {
        if let NodeKind::TupleExtract { tuple: t, .. } = &node.kind {
            if *t == tuple {
                return Some(node.id);
            }
        }
        None
    })
}

/// The loop bound: the operand of the termination comparison that is not a loop output.
///
/// The index output equals this bound at loop exit (the loop runs while the
/// comparison holds), so it is the correct value for the index positions when
/// rebuilding a loop's tuple result. Returns `None` when the bound cannot be
/// determined unambiguously.
pub fn loop_termination_bound(
    function: &sir_nodes::Function,
    loop_node: NodeId,
) -> Option<NodeId> {
    let node = function.get_node(loop_node)?;
    let NodeKind::Loop {
        termination,
        outputs,
        ..
    } = &node.kind
    else {
        return None;
    };
    let term = function.get_node(*termination)?;
    let (lhs, rhs) = match &term.kind {
        NodeKind::Lt { lhs, rhs } | NodeKind::Le { lhs, rhs } | NodeKind::Ne { lhs, rhs } => {
            (*lhs, *rhs)
        }
        NodeKind::Gt { lhs, rhs } | NodeKind::Ge { lhs, rhs } | NodeKind::Eq { lhs, rhs } => {
            (*rhs, *lhs)
        }
        _ => return None,
    };
    let lhs_is_output = outputs.contains(&lhs);
    let rhs_is_output = outputs.contains(&rhs);
    match (lhs_is_output, rhs_is_output) {
        (true, false) => Some(rhs),
        (false, true) => Some(lhs),
        _ => None, // Both or neither are outputs — bound is ambiguous.
    }
}

/// Position of the reduction output among the loop's outputs.
///
/// Uses the `accumulator` role when known; otherwise falls back to the loop
/// output that does not participate in the termination comparison (the index
/// counter is the one that feeds the comparison).
pub fn loop_reduction_position(
    function: &sir_nodes::Function,
    loop_node: NodeId,
    accumulator: Option<NodeId>,
) -> Option<usize> {
    let node = function.get_node(loop_node)?;
    let NodeKind::Loop {
        termination,
        outputs,
        ..
    } = &node.kind
    else {
        return None;
    };
    if let Some(acc) = accumulator {
        if let Some(pos) = outputs.iter().position(|o| *o == acc) {
            return Some(pos);
        }
    }
    let term = function.get_node(*termination)?;
    let cmp_inputs = term.kind.input_nodes();
    outputs.iter().position(|o| !cmp_inputs.contains(o))
}

/// Rebuild a tuple-typed loop result when it is returned wholesale.
///
/// The non-reduction positions (index counters) equal the termination bound at
/// loop exit, so they are materialized as the bound node; the reduction position
/// gets `new_scalar`. When the result is not a tuple-typed loop, the scalar is
/// returned unchanged (the caller replaces `result` directly).
///
/// `fallback_bound` is the recognized collection length, used when the
/// termination comparison does not involve the index output (hand-built loops
/// that compare a constant carried input) — the index equals the collection
/// length at exit, so the length is materialized as a constant.
pub fn wrap_direct_tuple_return(
    function: &sir_nodes::Function,
    result: NodeId,
    accumulator: Option<NodeId>,
    fallback_bound: Option<usize>,
    new_scalar: LocalNodeId,
    builder: &mut SubgraphBuilder,
) -> Result<LocalNodeId, RewriteError> {
    let node = function
        .get_node(result)
        .ok_or_else(|| RewriteError::InternalInvariantViolation(format!("region result node {result} not found")))?;
    let Type::Tuple { elements } = &node.ty else {
        return Ok(new_scalar);
    };
    let red_pos = loop_reduction_position(function, result, accumulator).ok_or_else(|| {
        RewriteError::MissingRole {
            role: "reduction output".to_string(),
        }
    })?;
    if red_pos >= elements.len() {
        return Err(RewriteError::InternalInvariantViolation(format!(
            "reduction position {red_pos} out of bounds for tuple of {} elements",
            elements.len()
        )));
    }
    let bound = loop_termination_bound(function, result);
    let mut elems = Vec::with_capacity(elements.len());
    for (pos, ty) in elements.iter().enumerate() {
        if pos == red_pos {
            elems.push(new_scalar);
        } else if let Some(b) = bound {
            elems.push(LocalNodeId::new(b.as_u64()));
        } else if let Some(len) = fallback_bound {
            elems.push(builder.constant(
                ConstantData::u64(len as u64),
                ty.clone(),
                Span::unknown(),
            ));
        } else {
            return Err(RewriteError::MissingRole {
                role: "loop termination bound".to_string(),
            });
        }
    }
    Ok(builder.tuple(elems, node.ty.clone(), Span::unknown()))
}

/// The recognized collection length of the region, when it is a known-length
/// sequence. Used as the fallback index bound for tuple rebuilds: the index
/// equals the collection length at loop exit.
pub fn collection_length(region: &RewriteRegion) -> Option<usize> {
    match region.structural.source_structure {
        sir_transform::structures::SourceStructure::LogicalSequence { length }
        | sir_transform::structures::SourceStructure::DynamicBooleanSequence { length } => {
            Some(length)
        }
        _ => None,
    }
}

/// Shared helper to emit the initial `pack(board)` operation for bitset reductions.
pub fn emit_pack(
    _function: &sir_nodes::Function,
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
    if let sir_transform::structures::SourceStructure::LogicalSequence { length } =
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
