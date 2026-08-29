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
}
