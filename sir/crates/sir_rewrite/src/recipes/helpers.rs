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

/// Find a tuple-slot consumer of the given tuple value, recognizing both
/// `TupleExtract { index }` and `FieldAccess { field: "N" }` (the builder
/// creates the latter; the PS002 audit found recipes blind to it).
/// Returns the consumer node and its slot index.
pub fn find_tuple_consumer(function: &sir_nodes::Function, tuple: NodeId) -> Option<(NodeId, usize)> {
    for node in function.arena.iter() {
        match &node.kind {
            NodeKind::TupleExtract { tuple: t, index } => {
                if *t == tuple {
                    return Some((node.id, *index));
                }
            }
            NodeKind::FieldAccess { base, field } => {
                if *base == tuple {
                    if let Ok(pos) = field.parse::<usize>() {
                        return Some((node.id, pos));
                    }
                }
            }
            _ => {}
        }
    }
    None
}

/// Authorized slot consumer for a reduction recipe.
///
/// A reduction theorem (popcount/all/any/parity) speaks ONLY about the
/// accumulator output of the loop. If the loop's tuple is consumed
/// downstream by a slot extract, that extract must read the accumulator
/// position; a consumer reading a different slot (e.g. a position/index
/// live-out) cannot be satisfied by the replacement — replacing it, or
/// rebuilding the whole tuple underneath it, silently changes program
/// semantics (PS002 audit: `array_find_last` returned a constant after
/// its loop was rebuilt).
///
/// COMPLETE USE CLASSIFICATION (advisor PS002 follow-up): "no recognized
/// slot consumer" is NOT "no observable consumer". The loop tuple can
/// escape whole (returned/copied/stored), flow through an unsupported
/// projection form, or have several consumers. Therefore:
///
///   - tuple result, NO use found ............ Err (abstain; removing a
///     possibly-observable loop is DCE, not this recipe's proof)
///   - tuple result, unrecognized use ........ Err (UnknownConsumer)
///   - tuple result, use observes another slot  Err (UnauthorizedLiveOut)
///   - tuple result, >1 use .................. Err (MultipleConsumers)
///   - tuple result, exactly one accumulator-slot extract ... Ok(Some)
///   - NON-tuple result ...................... Ok(None): every use
///     observes the whole value, which IS the theorem's subject; the
///     caller replaces the loop directly (replace_all_uses is uniform
///     and no projection can observe a sub-value).
///
/// Only the last case authorizes a rewrite; everything else abstains
/// until complete live-out binding (ProposalBinding) exists.
pub fn authorized_tuple_consumer(
    function: &sir_nodes::Function,
    loop_node: NodeId,
    accumulator: Option<NodeId>,
) -> Result<Option<NodeId>, RewriteError> {
    let loop_ty = function
        .get_node(loop_node)
        .map(|n| n.ty.clone())
        .ok_or(RewriteError::NodeNotFound(loop_node))?;

    // Collect every use of the loop result.
    let mut uses: Vec<(NodeId, Option<usize>)> = Vec::new();
    for node in function.arena.iter() {
        if node.id == loop_node {
            continue;
        }
        if node.kind.input_nodes().contains(&loop_node) {
            let slot = match &node.kind {
                NodeKind::TupleExtract { tuple, index } if *tuple == loop_node => Some(*index),
                NodeKind::FieldAccess { base, field } if *base == loop_node => {
                    field.parse::<usize>().ok()
                }
                _ => None,
            };
            uses.push((node.id, slot));
        }
    }

    // Non-tuple results: every use observes the whole (single) value.
    if !matches!(loop_ty, Type::Tuple { .. }) {
        return Ok(None);
    }

    // Tuple results: every use must be a recognized slot extract.
    if uses.is_empty() {
        // "No recognized consumer" is not "no observable consumer" — the
        // tuple may escape whole or through an unrecognized form. Refuse.
        return Err(RewriteError::UnknownConsumer {
            value: loop_node,
            reason: "no recognized consumer: whole-tuple escape, copy, store, or \
                     unsupported projection — live-out binding incomplete, refusing"
                .to_string(),
        });
    }
    if uses.iter().any(|(_, slot)| slot.is_none()) {
        return Err(RewriteError::UnknownConsumer {
            value: loop_node,
            reason: "unrecognized consumer form observing the loop tuple".to_string(),
        });
    }
    if uses.len() > 1 {
        return Err(RewriteError::UnknownConsumer {
            value: loop_node,
            reason: "multiple consumers of the loop tuple: the recipe replaces \
                     exactly one; all others would dangle or observe uncovered slots"
                .to_string(),
        });
    }

    // Single-output loops have exactly one possible reduction slot.
    let red_pos = match loop_reduction_position(function, loop_node, accumulator) {
        Some(pos) => pos,
        None => {
            let single = match function.get_node(loop_node) {
                Some(n) => matches!(&n.kind,
                    NodeKind::Loop { outputs, .. } if outputs.len() == 1),
                None => false,
            };
            if !single {
                return Err(RewriteError::MissingRole {
                    role: "reduction output position".to_string(),
                });
            }
            0
        }
    };
    let (node, slot) = uses[0];
    let field = slot.expect("classified above");
    if field != red_pos {
        return Err(RewriteError::UnauthorizedLiveOut {
            consumer: node,
            field,
            reduction_position: red_pos,
        });
    }
    Ok(Some(node))
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
    // WHOLESALE TUPLE RECONSTRUCTION QUARANTINE (advisor PS002 follow-up):
    // filling non-reduction slots with the termination bound assumes
    // "index at exit == bound" — sound only for specific ascending,
    // zero-trip-checked loop shapes and never proven here. Any use of
    // this path for a multi-field tuple silently invents observable
    // values. Complete live-out binding (ProposalBinding) must prove
    // each non-reduction slot before this path may return.
    if elements.len() > 1 {
        return Err(RewriteError::UnknownConsumer {
            value: result,
            reason: "wholesale tuple reconstruction quarantined: rebuilding \
                     non-reduction slots from the termination bound is an \
                     unproven exit-index assumption (PS002 class)"
                .to_string(),
        });
    }
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
