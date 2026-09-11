use sir_semantics::binding::{LiveOutBinding, LiveOutKind};
use sir_transform::ids::DefinitionId;
use sir_types::{ConstantData, Span, Type};

use crate::error::RewriteError;
use crate::local_id::LocalNodeId;
use crate::patch::{ReplacementPatch, ReplacementValue};
use crate::recipe::RewriteRecipe;
use crate::region::RewriteRegion;
use crate::subgraph_builder::SubgraphBuilder;

/// Recipe for the Any transformation.
///
/// Replaces a boolean-array disjunctive loop with:
///   pack(board) → (packed != 0)
pub struct AnyRecipe {
    id: DefinitionId,
}

impl AnyRecipe {
    pub fn new(id: DefinitionId) -> Self {
        Self { id }
    }
}

impl RewriteRecipe for AnyRecipe {
    fn definition(&self) -> DefinitionId {
        self.id
    }

    fn name(&self) -> &'static str {
        "Any"
    }

    fn build_patch(
        &self,
        function: &sir_nodes::Function,
        region: &RewriteRegion,
        mut builder: SubgraphBuilder,
    ) -> Result<ReplacementPatch, RewriteError> {
        // ── ROLE-MAP-ONLY (advisor invariant 6) ─────────────────
        // Once ProposalBinding succeeds, no stage scans the source to
        // guess semantic roles. The Any recipe consumes ONLY the
        // binding's role map and classified observables; there is no
        // global rediscovery of collection/accumulator/slot.
        let binding = region.binding.as_ref().ok_or_else(|| {
            RewriteError::RecipeFailed(
                "Any requires an application binding (ProposalBinding); none was derived"
                    .to_string(),
            )
        })?;
        let map = &binding.map;

        // The classified observable interface: exactly one Reconstructed
        // live-out determines the replacement target. Dead slots carry
        // closure evidence and get no replacement (DCE removes them);
        // any other binding state refuses.
        let mut target: Option<sir_types::NodeId> = None;
        for observable in &binding.live_outs {
            match &observable.binding {
                LiveOutBinding::Reconstructed { .. } => {
                    let Some(site) = observable.use_site else {
                        return Err(RewriteError::RecipeFailed(
                            "reconstructed observable has no replacement site".to_string(),
                        ));
                    };
                    if target.is_some() {
                        return Err(RewriteError::RecipeFailed(
                            "binding classifies more than one reconstructed observable".to_string(),
                        ));
                    }
                    // The whole-value observable is replaced at the loop
                    // node itself; a slot observable at its projection.
                    target = Some(match observable.kind {
                        LiveOutKind::WholeValue => map.loop_node,
                        LiveOutKind::Slot(_) => site,
                    });
                }
                LiveOutBinding::Dead { .. } => {
                    // Unobserved slot: no replacement site; the rewrite
                    // deletes the loop and this slot with it. Evidence
                    // (direct users + function fingerprint) was checked
                    // against THIS function version at derivation.
                }
                _ => {
                    return Err(RewriteError::RecipeFailed(
                        "unsupported live-out binding state".to_string(),
                    ));
                }
            }
        }
        let target = target.ok_or_else(|| {
            RewriteError::RecipeFailed("binding classifies no reconstructed observable".to_string())
        })?;

        // ── Candidate ──────────────────────────────────────────
        // Collection comes from the role map. The extent comes from
        // the collection's own type — a missing extent refuses (no
        // guessed width). A predicate collection emits
        // array_cmp_mask(collection, scalar, TRUE op from the binding);
        // a plain boolean collection emits pack(collection). The op is
        // a role — never hardcoded, never rediscovered.
        let collection = map.collection;
        let (width, element_ty) = match function.get_node(collection).map(|n| &n.ty) {
            Some(Type::Array { length, element }) => (*length, (**element).clone()),
            _ => {
                return Err(RewriteError::RecipeFailed(format!(
                    "collection %{} has no declared array extent",
                    collection.0
                )));
            }
        };
        let span = Span::unknown();
        let packed = match (map.predicate_op, map.predicate_scalar) {
            (Some(op), Some(scalar)) => builder.array_cmp_mask(
                LocalNodeId::new(collection.as_u64()),
                LocalNodeId::new(scalar.as_u64()),
                op,
                span,
            ),
            (None, None) if element_ty == Type::Bool => {
                builder.pack(LocalNodeId::new(collection.as_u64()), width, span)
            }
            (None, None) if implicit_element_nonzero(map, &element_ty) => {
                // D4: integer collection whose certified recurrence
                // reduces the RAW elements (`acc |= x[i]`). The Any
                // object is `any(x[i] != 0)`; the canonical candidate
                // is the vectorized `x[i] != 0` mask, nonzero-checked.
                // The zero scalar is the mathematical zero of the
                // element type — synthesized here, never inferred
                // from the source.
                let zero_data = zero_of_type(&element_ty).ok_or_else(|| {
                    RewriteError::RecipeFailed(format!(
                        "no canonical zero constant for element type {element_ty:?}"
                    ))
                })?;
                let element_zero =
                    builder.constant(zero_data, element_ty.clone(), Span::unknown());
                builder.array_cmp_mask(
                    LocalNodeId::new(collection.as_u64()),
                    element_zero,
                    sir_nodes::CmpOperator::Ne,
                    span,
                )
            }
            (None, None) => {
                return Err(RewriteError::RecipeFailed(
                    "integer collection without a certified element predicate: \
                     the recurrence does not reduce the raw elements, so the \
                     nonzero byte cannot be re-expressed as an element mask"
                        .to_string(),
                ));
            }
            _ => {
                return Err(RewriteError::RecipeFailed(
                    "predicate roles inconsistent (op without scalar or vice versa)".to_string(),
                ));
            }
        };

        let zero = builder.constant(
            ConstantData::u64(0),
            Type::BitVector { width },
            Span::unknown(),
        );
        let ne_zero = builder.ne(packed, zero, Span::unknown());

        Ok(builder.finish(vec![ReplacementValue {
            old: target,
            new: ne_zero,
        }]))
    }
}

/// D4: the implicit element-truthiness predicate is certified by the
/// role map itself — the accumulator combines the RAW element access
/// (`predicate == element_access`), the recurrence is the disjunction
/// whose nonzero value means "some element is nonzero", and the
/// element type is a non-Boolean integer the mask comparison is
/// defined over. Anything else (a projected element such as
/// `x[i] >> 7`, or a non-OR recurrence whose zero test means something
/// else) refuses.
pub fn implicit_element_nonzero(
    map: &sir_semantics::binding::ReductionRoleMap,
    element_ty: &Type,
) -> bool {
    map.predicate_op.is_none()
        && map.predicate == Some(map.element_access)
        && map.recurrence == "bitwise_or"
        && !element_ty.is_bool()
        && zero_of_type(element_ty).is_some()
}

/// The canonical zero constant of an integer type (None for types
/// without a constructor — those refuse rather than guess).
fn zero_of_type(ty: &Type) -> Option<ConstantData> {
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
            _ => return None,
        }),
        _ => None,
    }
}
