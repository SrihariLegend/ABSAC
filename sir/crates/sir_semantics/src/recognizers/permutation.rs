use sir_analysis::facts::FactDatabase;
use sir_nodes::{Function, NodeKind};
use sir_types::NodeId;

use crate::concepts::SemanticConcept;
use crate::region::RecognitionExplanation;
use crate::truth::{TruthParameter, ValueId};

/// Recognizes the two circular-rotation idioms:
///
/// * `ShiftPairLeft` — `(x << k) | (x >> (w - k))`
/// * `ShiftPairRight` — `(x >> k) | (x << (w - k))`
///
/// where `w` is the bit width of `x` and `k` is a runtime amount. The
/// recognizer attaches a `TruthParameter::ShiftPair { width, direction }`
/// so downstream closure rules and role derivation never re-discover the
/// direction from the physical graph.
///
/// Returns a 6-tuple: the concept, explanation, physical nodes, input
/// values, output values, and the ShiftPair parameter.
#[allow(clippy::type_complexity)]
pub fn recognize_shift_pairs(
    func: &Function,
    _analysis: &FactDatabase,
) -> Vec<(
    SemanticConcept,
    RecognitionExplanation,
    Vec<NodeId>,
    Vec<ValueId>,
    Vec<ValueId>,
    TruthParameter,
)> {
    let mut results = Vec::new();

    for node in func.arena.iter() {
        if let NodeKind::Or { lhs, rhs } = &node.kind {
            let or_node = node.id;
            let (l_kind, r_kind) = (
                func.get_node(*lhs).map(|n| &n.kind),
                func.get_node(*rhs).map(|n| &n.kind),
            );
            match (l_kind, r_kind) {
                // (x << k) | (x >> (w - k)) — rotate left
                (Some(NodeKind::Shl { lhs: l1, rhs: k1 }), Some(NodeKind::Shr { lhs: l2, rhs: r2 }))
                    if l1 == l2 =>
                {
                    if let Some((width, sub_lhs, amount)) = check_shift_pair(func, *l1, k1, r2) {
                        let x = *l1;
                        results.push((
                            SemanticConcept::ShiftPairLeft,
                            RecognitionExplanation {
                                concept: SemanticConcept::ShiftPairLeft,
                                triggering_facts: vec![
                                    "Left shift OR right shift of the same value",
                                    "Right shift amount is width minus left shift amount",
                                ],
                            },
                            vec![or_node, *lhs, *rhs, *k1, x, sub_lhs, amount],
                            vec![ValueId::new(x.0)],
                            vec![ValueId::new(or_node.0)],
                            TruthParameter::ShiftPair {
                                width,
                                direction: sir_transform::roles::ShiftDirection::Left,
                            },
                        ));
                    }
                }
                // (x >> k) | (x << (w - k)) — rotate right
                (Some(NodeKind::Shr { lhs: l1, rhs: k1 }), Some(NodeKind::Shl { lhs: l2, rhs: r2 }))
                    if l1 == l2 =>
                {
                    if let Some((width, sub_lhs, amount)) = check_shift_pair(func, *l1, k1, r2) {
                        let x = *l1;
                        results.push((
                            SemanticConcept::ShiftPairRight,
                            RecognitionExplanation {
                                concept: SemanticConcept::ShiftPairRight,
                                triggering_facts: vec![
                                    "Right shift OR left shift of the same value",
                                    "Left shift amount is width minus right shift amount",
                                ],
                            },
                            vec![or_node, *lhs, *rhs, *k1, x, sub_lhs, amount],
                            vec![ValueId::new(x.0)],
                            vec![ValueId::new(or_node.0)],
                            TruthParameter::ShiftPair {
                                width,
                                direction: sir_transform::roles::ShiftDirection::Right,
                            },
                        ));
                    }
                }
                _ => {}
            }
        }
    }

    results
}

/// Recognizes the masked-swap stage idiom: bits `i` and `i + shift` are
/// exchanged for every set bit `i` in `mask` via
/// `(x & mask) << shift | (x >> shift) & mask`.
///
/// One stage performs a single swap distance; a chain of stages composes
/// into a full byte reversal (HD004) or bit reversal (HD005) through the
/// `CombinePermutations` closure rule.
#[allow(clippy::type_complexity)]
pub fn recognize_masked_shift_swaps(
    func: &Function,
    _analysis: &FactDatabase,
) -> Vec<(
    SemanticConcept,
    RecognitionExplanation,
    Vec<NodeId>,
    Vec<ValueId>,
    Vec<ValueId>,
    TruthParameter,
)> {
    let mut results = Vec::new();

    for node in func.arena.iter() {
        if let NodeKind::Or { lhs, rhs } = &node.kind {
            let or_node = node.id;
            let (l_kind, r_kind) = (
                func.get_node(*lhs).map(|n| &n.kind),
                func.get_node(*rhs).map(|n| &n.kind),
            );

            // Try both operand orders: Or(Shl(And(x,m),s), And(Shr(x,s),m)) and
            // Or(And(Shr(x,s),m), Shl(And(x,m),s)).
            let matched = match (l_kind, r_kind) {
                (Some(NodeKind::Shl { lhs: a, rhs: s1 }), Some(NodeKind::And { .. })) => {
                    check_masked_swap(func, *a, *s1, *rhs)
                }
                (Some(NodeKind::And { .. }), Some(NodeKind::Shl { lhs: a, rhs: s1 })) => {
                    check_masked_swap(func, *a, *s1, *lhs)
                }
                _ => None,
            };

            if let Some((x, mask, shift, and_lhs_node, and_rhs_node, shr_node)) = matched {
                // The mask and shift must be compile-time constants so the
                // parameter is exact math, not a runtime value.
                let (Some(mask_val), Some(shift_val)) = (
                    constant_u64(func, &mask),
                    constant_u64(func, &shift),
                ) else {
                    continue;
                };

                results.push((
                    SemanticConcept::MaskedShiftSwap,
                    RecognitionExplanation {
                        concept: SemanticConcept::MaskedShiftSwap,
                        triggering_facts: vec![
                            "Masked left shift OR masked right shift of the same value",
                            "Both sides use the same mask and shift amount",
                        ],
                    },
                    vec![or_node, *lhs, *rhs, x, mask, shift, and_lhs_node, and_rhs_node, shr_node],
                    vec![ValueId::new(x.0)],
                    vec![ValueId::new(or_node.0)],
                    TruthParameter::MaskedShiftSwap {
                        mask: mask_val,
                        shift: shift_val as u32,
                    },
                ));
            }
        }
    }

    results
}

/// Validate the four operands of a masked swap stage and return
/// `(x, mask, shift, and_lhs, and_rhs, shr_node)` on success.
fn check_masked_swap(
    func: &Function,
    shl_lhs: NodeId,
    shl_rhs: NodeId,
    and_node: NodeId,
) -> Option<(NodeId, NodeId, NodeId, NodeId, NodeId, NodeId)> {
    // Left side: Shl(And(x, mask), shift)
    let and_node_l = func.get_node(shl_lhs)?;
    let NodeKind::And { lhs: a, rhs: b } = &and_node_l.kind else {
        return None;
    };
    let x = *a;
    let mask = *b;
    if constant_u64(func, &mask).is_none() {
        return None;
    }
    let shift = shl_rhs;
    if constant_u64(func, &shift).is_none() {
        return None;
    }

    // Right side: And(Shr(x, shift), mask) — same x, same mask, same shift.
    let and2 = func.get_node(and_node)?;
    let NodeKind::And { lhs: c, rhs: d } = &and2.kind else {
        return None;
    };
    let (and_lhs, and_rhs) = (*c, *d);
    let (shr_node, mask2) = match (func.get_node(and_lhs), func.get_node(and_rhs)) {
        (Some(n), _) if matches!(n.kind, NodeKind::Shr { .. }) => (and_lhs, and_rhs),
        (_, Some(n)) if matches!(n.kind, NodeKind::Shr { .. }) => (and_rhs, and_lhs),
        _ => return None,
    };
    let shr = func.get_node(shr_node)?;
    let NodeKind::Shr { lhs: sx, rhs: ss } = &shr.kind else {
        return None;
    };
    if sx != &x {
        return None;
    }
    if ss != &shift {
        return None;
    }
    if mask2 != mask {
        return None;
    }
    if constant_u64(func, &mask2).is_none() {
        return None;
    }

    Some((x, mask, shift, and_lhs, and_rhs, shr_node))
}

/// Read a constant u64 value from a node.
fn constant_u64(func: &Function, node: &NodeId) -> Option<u64> {
    let n = func.get_node(*node)?;
    if let NodeKind::Constant(c) = &n.kind {
        c.as_u64()
    } else {
        None
    }
}

/// Check the `Sub(width, k)` shape of the complementary shift.
///
/// The amount `k` is used directly as the rotation amount; the subtraction
/// side must be `width - k` where the subtracted constant equals the bit
/// width of the shifted value. Returns `(width, sub_lhs, amount)` on match.
fn check_shift_pair(
    func: &Function,
    x: NodeId,
    k: &NodeId,
    other_shift_rhs: &NodeId,
) -> Option<(u32, NodeId, NodeId)> {
    let sub_node = func.get_node(*other_shift_rhs)?;
    if let NodeKind::Sub { lhs: sub_lhs, rhs: sub_rhs } = &sub_node.kind {
        // The subtracted amount must be the same runtime value as k.
        if sub_rhs != k {
            return None;
        }
        // The width constant must equal the bit width of x.
        let width_const = func.get_node(*sub_lhs)?;
        let NodeKind::Constant(c) = &width_const.kind else {
            return None;
        };
        let width = c.as_u64()? as u32;
        let x_ty = func.get_node(x)?.ty.clone();
        let x_width = type_bits(&x_ty)?;
        if width == x_width as u32 {
            return Some((width, *sub_lhs, *sub_rhs));
        }
    }
    None
}

/// Bit width of an integer or bitvector type.
fn type_bits(ty: &sir_types::Type) -> Option<usize> {
    match ty {
        sir_types::Type::Integer { width, .. } => Some(width.bits()),
        sir_types::Type::BitVector { width } => Some(*width),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sir_transform::roles::ShiftDirection;
    use sir_builder::Builder;
    use sir_types::{ConstantData, Span, Type};

    fn unknown_span() -> Span {
        Span::unknown()
    }

    #[test]
    fn recognizes_rotate_left() {
        let mut b = Builder::new("rotate_left", &[("x", Type::u64()), ("k", Type::u64())], Type::u64());
        let x = b.parameter_index(0).unwrap();
        let k = b.parameter_index(1).unwrap();
        let sixty_four = b.constant(ConstantData::u64(64), Type::u64(), unknown_span());
        let shl = b.shl(x, k, unknown_span()).unwrap();
        let diff = b.sub(sixty_four, k, unknown_span()).unwrap();
        let shr = b.shr(x, diff, unknown_span()).unwrap();
        let res = b.bit_or(shl, shr, unknown_span()).unwrap();
        b.return_value(res, unknown_span()).unwrap();
        let func = b.build();

        let recs = recognize_shift_pairs(&func, &FactDatabase::default());
        assert_eq!(recs.len(), 1);
        let (concept, _exp, node_ids, inputs, outputs, param) = &recs[0];
        assert_eq!(*concept, SemanticConcept::ShiftPairLeft);
        assert!(node_ids.contains(&res));
        assert_eq!(inputs, &vec![ValueId::new(x.0)]);
        assert_eq!(outputs, &vec![ValueId::new(res.0)]);
        match param {
            TruthParameter::ShiftPair { width, direction } => {
                assert_eq!(*width, 64);
                assert_eq!(*direction, ShiftDirection::Left);
            }
            _ => panic!("expected ShiftPair parameter"),
        }
    }

    #[test]
    fn recognizes_rotate_right() {
        let mut b = Builder::new("rotate_right", &[("x", Type::u64()), ("k", Type::u64())], Type::u64());
        let x = b.parameter_index(0).unwrap();
        let k = b.parameter_index(1).unwrap();
        let sixty_four = b.constant(ConstantData::u64(64), Type::u64(), unknown_span());
        let shr = b.shr(x, k, unknown_span()).unwrap();
        let diff = b.sub(sixty_four, k, unknown_span()).unwrap();
        let shl = b.shl(x, diff, unknown_span()).unwrap();
        let res = b.bit_or(shr, shl, unknown_span()).unwrap();
        b.return_value(res, unknown_span()).unwrap();
        let func = b.build();

        let recs = recognize_shift_pairs(&func, &FactDatabase::default());
        assert_eq!(recs.len(), 1);
        let (concept, _exp, _node_ids, _inputs, _outputs, param) = &recs[0];
        assert_eq!(*concept, SemanticConcept::ShiftPairRight);
        match param {
            TruthParameter::ShiftPair { width, direction } => {
                assert_eq!(*width, 64);
                assert_eq!(*direction, ShiftDirection::Right);
            }
            _ => panic!("expected ShiftPair parameter"),
        }
    }

    #[test]
    fn rejects_unrelated_shifts() {
        // (x << k) | (y >> k): different values, no width subtraction.
        let mut b = Builder::new("unrelated", &[("x", Type::u64()), ("y", Type::u64()), ("k", Type::u64())], Type::u64());
        let x = b.parameter_index(0).unwrap();
        let y = b.parameter_index(1).unwrap();
        let k = b.parameter_index(2).unwrap();
        let shl = b.shl(x, k, unknown_span()).unwrap();
        let shr = b.shr(y, k, unknown_span()).unwrap();
        let res = b.bit_or(shl, shr, unknown_span()).unwrap();
        b.return_value(res, unknown_span()).unwrap();
        let func = b.build();

        let recs = recognize_shift_pairs(&func, &FactDatabase::default());
        assert!(recs.is_empty());
    }

    #[test]
    fn recognizes_masked_swap_stage() {
        // HD004 shape: ((x & 0xFF) << 8) | ((x >> 8) & 0xFF)
        let mut b = Builder::new("byte_swap_16", &[("x", Type::u32())], Type::u32());
        let x = b.parameter_index(0).unwrap();
        let mask = b.constant(ConstantData::u32(0xFF), Type::u32(), unknown_span());
        let eight = b.constant(ConstantData::u32(8), Type::u32(), unknown_span());

        let low = b.bit_and(x, mask, unknown_span()).unwrap();
        let low_shifted = b.shl(low, eight, unknown_span()).unwrap();
        let high = b.shr(x, eight, unknown_span()).unwrap();
        let high_masked = b.bit_and(high, mask, unknown_span()).unwrap();
        let res = b.bit_or(low_shifted, high_masked, unknown_span()).unwrap();
        b.return_value(res, unknown_span()).unwrap();
        let func = b.build();

        let recs = recognize_masked_shift_swaps(&func, &FactDatabase::default());
        assert_eq!(recs.len(), 1);
        let (concept, _exp, node_ids, inputs, outputs, param) = &recs[0];
        assert_eq!(*concept, SemanticConcept::MaskedShiftSwap);
        assert!(node_ids.contains(&res));
        assert_eq!(inputs, &vec![ValueId::new(x.0)]);
        assert_eq!(outputs, &vec![ValueId::new(res.0)]);
        match param {
            TruthParameter::MaskedShiftSwap { mask, shift } => {
                assert_eq!(*mask, 0xFF);
                assert_eq!(*shift, 8);
            }
            _ => panic!("expected MaskedShiftSwap parameter"),
        }
    }

    #[test]
    fn masked_swap_requires_constant_mask_and_shift() {
        // Same shape but a runtime shift amount: must not match.
        let mut b = Builder::new("dynamic", &[("x", Type::u32()), ("s", Type::u32())], Type::u32());
        let x = b.parameter_index(0).unwrap();
        let s = b.parameter_index(1).unwrap();
        let mask = b.constant(ConstantData::u32(0xFF), Type::u32(), unknown_span());

        let low = b.bit_and(x, mask, unknown_span()).unwrap();
        let low_shifted = b.shl(low, s, unknown_span()).unwrap();
        let high = b.shr(x, s, unknown_span()).unwrap();
        let high_masked = b.bit_and(high, mask, unknown_span()).unwrap();
        let res = b.bit_or(low_shifted, high_masked, unknown_span()).unwrap();
        b.return_value(res, unknown_span()).unwrap();
        let func = b.build();

        let recs = recognize_masked_shift_swaps(&func, &FactDatabase::default());
        assert!(recs.is_empty());
    }
}
