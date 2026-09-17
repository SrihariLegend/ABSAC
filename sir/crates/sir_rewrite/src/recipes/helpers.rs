use crate::error::RewriteError;
use crate::local_id::LocalNodeId;
use crate::region::RewriteRegion;
use crate::subgraph_builder::SubgraphBuilder;
use sir_nodes::NodeKind;
use sir_semantics::binding::{
    LiveOutBinding, LiveOutKind, ProposalBinding, ReductionRoleMap,
};
use sir_types::{ConstantData, NodeId, Span, Type};

/// The authorized application binding, or a fail-closed refusal.
///
/// Reduction recipes must consume this artifact instead of re-deriving
/// roles from the structural description (P0A: the canonical binder is
/// the ONE legitimate role scan).
pub fn require_binding<'a>(
    region: &'a RewriteRegion,
    recipe: &str,
) -> Result<&'a ProposalBinding, RewriteError> {
    region.binding.as_ref().ok_or_else(|| {
        RewriteError::RecipeFailed(format!(
            "{recipe} requires an application binding (ProposalBinding); none was derived"
        ))
    })
}

/// The single reconstructed observable target a reduction recipe may
/// replace, taken from the authorized binding's live-out classification.
///
/// Dead slots carry use-closure evidence and get no replacement; any
/// other binding state refuses. Shared by every reduction recipe so the
/// observable interface has one implementation.
pub fn binding_target(region: &RewriteRegion, recipe: &str) -> Result<NodeId, RewriteError> {
    let binding = require_binding(region, recipe)?;
    let mut target: Option<NodeId> = None;
    for observable in &binding.live_outs {
        match &observable.binding {
            LiveOutBinding::Reconstructed { .. } => {
                let Some(site) = observable.use_site else {
                    return Err(RewriteError::RecipeFailed(format!(
                        "{recipe}: reconstructed observable has no replacement site"
                    )));
                };
                if target.is_some() {
                    return Err(RewriteError::RecipeFailed(format!(
                        "{recipe}: binding classifies more than one reconstructed observable"
                    )));
                }
                target = Some(match observable.kind {
                    LiveOutKind::WholeValue => binding.map.loop_node,
                    LiveOutKind::Slot(_) => site,
                });
            }
            LiveOutBinding::Dead { .. } => {}
            _ => {
                return Err(RewriteError::RecipeFailed(format!(
                    "{recipe}: unsupported live-out binding state"
                )));
            }
        }
    }
    target.ok_or_else(|| {
        RewriteError::RecipeFailed(format!(
            "{recipe}: binding classifies no reconstructed observable"
        ))
    })
}

/// The collection extent from the authorized role map: the collection's
/// own declared array type, never a guess.
pub fn binding_collection_extent(
    function: &sir_nodes::Function,
    region: &RewriteRegion,
    recipe: &str,
) -> Result<(NodeId, usize, Type), RewriteError> {
    let map = &require_binding(region, recipe)?.map;
    match function.get_node(map.collection).map(|n| &n.ty) {
        Some(Type::Array { length, element }) => {
            Ok((map.collection, *length, (**element).clone()))
        }
        _ => Err(RewriteError::RecipeFailed(format!(
            "{recipe}: collection %{} has no declared array extent",
            map.collection.0
        ))),
    }
}

/// D4: the implicit element-truthiness predicate is certified by the
/// role map itself — the accumulator combines the RAW element access
/// (`predicate == element_access`), the recurrence is a disjunction
/// whose nonzero value means "some element is nonzero", and the element
/// type is a non-Boolean integer the mask comparison is defined over.
pub fn implicit_element_nonzero(map: &ReductionRoleMap, element_ty: &Type) -> bool {
    map.predicate_op.is_none()
        && map.predicate == Some(map.element_access)
        && map.recurrence == "bitwise_or"
        && !element_ty.is_bool()
        && zero_of_type(element_ty).is_some()
}

/// The canonical zero constant of an integer type (None for types
/// without a constructor — those refuse rather than guess).
pub fn zero_of_type(ty: &Type) -> Option<ConstantData> {
    use sir_types::IntegerWidth;
    match ty {
        Type::Integer { width, signed, .. } => Some(match (width, signed) {
            (IntegerWidth::I8, false) => ConstantData::u8(0),
            (IntegerWidth::I8, true) => ConstantData::i8(0),
            (IntegerWidth::I16, false) => ConstantData::u16(0),
            (IntegerWidth::I16, true) => ConstantData::i16(0),
            (IntegerWidth::I32, false) => ConstantData::u32(0),
            (IntegerWidth::I32, true) => ConstantData::i32(0),
            (IntegerWidth::I64, false) => ConstantData::u64(0),
            (IntegerWidth::I64, true) => ConstantData::i64(0),
            (IntegerWidth::I128, _) => return None,
        }),
        _ => None,
    }
}

/// Emit the initial mask/board read for a reduction, entirely from the
/// authorized role map:
///
/// - predicate collection → `array_cmp_mask(collection, scalar, TRUE op)`
///   (the operator is the binding's role, never hardcoded);
/// - boolean collection → `pack(collection)`;
/// - integer collection reducing raw elements with an OR recurrence →
///   the certified `x[i] != 0` mask with a synthesized canonical zero.
///
/// Returns the packed node and the collection width.
pub fn emit_pack_from_binding(
    function: &sir_nodes::Function,
    region: &RewriteRegion,
    recipe: &str,
    builder: &mut SubgraphBuilder,
) -> Result<(LocalNodeId, usize), RewriteError> {
    let (collection, width, element_ty) =
        binding_collection_extent(function, region, recipe)?;
    let map = &require_binding(region, recipe)?.map;
    let span = Span::unknown();
    let collection_local = LocalNodeId::new(collection.as_u64());
    let packed = match (map.predicate_op, map.predicate_scalar) {
        (Some(op), Some(scalar)) => builder.array_cmp_mask(
            collection_local,
            LocalNodeId::new(scalar.as_u64()),
            op,
            span,
        ),
        (None, None) if element_ty == Type::Bool => {
            builder.pack(collection_local, width, span)
        }
        (None, None) if implicit_element_nonzero(map, &element_ty) => {
            let zero_data = zero_of_type(&element_ty).ok_or_else(|| {
                RewriteError::RecipeFailed(format!(
                    "no canonical zero constant for element type {element_ty:?}"
                ))
            })?;
            let element_zero = builder.constant(zero_data, element_ty.clone(), span);
            builder.array_cmp_mask(
                collection_local,
                element_zero,
                sir_nodes::CmpOperator::Ne,
                span,
            )
        }
        (None, None) => {
            return Err(RewriteError::RecipeFailed(format!(
                "{recipe}: integer collection without a certified element predicate: \
                 the recurrence does not reduce the raw elements"
            )));
        }
        _ => {
            return Err(RewriteError::RecipeFailed(format!(
                "{recipe}: predicate roles inconsistent (op without scalar or vice versa)"
            )));
        }
    };
    Ok((packed, width))
}

/// Emit `pack(collection)` for a POSITION-SEARCH region.
///
/// Position searches are authorized by their own certificate (X06), not
/// by the reduction binding: their live-out is the position slot, which
/// the reduction binding correctly refuses to classify. The role map is
/// still the single authorized source of the collection and its extent
/// (the array's declared type) — no graph re-discovery.
pub fn emit_pack_for_position_search(
    function: &sir_nodes::Function,
    region: &RewriteRegion,
    recipe: &str,
    builder: &mut SubgraphBuilder,
) -> Result<(LocalNodeId, usize), RewriteError> {
    let collection = region
        .structural
        .roles
        .iter()
        .find_map(|role| match role {
            sir_transform::roles::RegionRoles::PositionSearch {
                collection: Some(collection),
                ..
            } => Some(*collection),
            _ => None,
        })
        .ok_or_else(|| {
            RewriteError::RecipeFailed(format!(
                "{recipe}: no PositionSearch collection role"
            ))
        })?;
    let length = match function.get_node(collection).map(|n| &n.ty) {
        Some(Type::Array { element, length }) if **element == Type::Bool => *length,
        _ => {
            return Err(RewriteError::RecipeFailed(format!(
                "{recipe}: collection %{} is not a fixed-length boolean array",
                collection.0
            )))
        }
    };
    let packed = builder.pack(
        LocalNodeId::new(collection.as_u64()),
        length,
        Span::unknown(),
    );
    Ok((packed, length))
}

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

// `emit_pack` (which hardcoded `CmpOperator::Gt` for predicate
// collections) is gone: every pack site now goes through
// `emit_pack_from_binding`, which takes the TRUE operator from the
// authorized role map.
