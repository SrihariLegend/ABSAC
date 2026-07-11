//! Range analysis.
//!
//! Interval arithmetic on integer values. Computes lower/upper bounds
//! and detects special properties: nonzero, power-of-two, alignment.

use sir_nodes::{Function, NodeKind};
use sir_types::{ConstantData, NodeId};
use std::collections::HashMap;

use crate::facts::RangeFact;
use crate::graph;

/// Run range analysis on a function.
///
/// Bottom-up propagation using topological order. Each node's range
/// is computed from its operation and the ranges of its inputs.
pub fn run_ranges(func: &Function) -> HashMap<NodeId, RangeFact> {
    let order = graph::topological_sort(func);
    let mut facts: HashMap<NodeId, RangeFact> = HashMap::new();

    for &id in &order {
        let node = match func.get_node(id) {
            Some(n) => n,
            None => continue,
        };

        let range = match &node.kind {
            NodeKind::Constant(data) => range_of_constant(data),
            _ => compute_range(&node.kind, &node.ty, &facts),
        };

        facts.insert(id, range);
    }

    facts
}

/// Get range for a constant value.
fn range_of_constant(data: &ConstantData) -> RangeFact {
    let (lo, hi) = match data {
        ConstantData::Integer { value, .. } => {
            let parsed: Option<i128> = value.parse().ok();
            (parsed, parsed)
        }
        ConstantData::Bool(b) => {
            let v = if *b { 1i128 } else { 0 };
            (Some(v), Some(v))
        }
        _ => (None, None),
    };

    RangeFact {
        lower: lo,
        upper: hi,
        is_nonzero: lo.map(|v| v != 0).unwrap_or(false),
        is_power_of_two: lo.map(|v| v > 0 && (v & (v - 1)) == 0).unwrap_or(false),
        alignment: lo.and_then(|v| {
            if v == 0 {
                None
            } else {
                Some(v.trailing_zeros() as u64)
            }
        }),
    }
}

/// Return the bit width of an integer type, or 64 as default.
fn type_bit_width(ty: &sir_types::Type) -> usize {
    if let sir_types::Type::Integer { width, .. } = ty {
        width.bits()
    } else {
        64 // default for non-integer input (shouldn't happen for popcount)
    }
}

/// Compute range for a node based on its operation, type, and input ranges.
fn compute_range(
    kind: &NodeKind,
    ty: &sir_types::Type,
    facts: &HashMap<NodeId, RangeFact>,
) -> RangeFact {
    let inputs = graph::dataflow_inputs(kind);
    let input_ranges: Vec<&RangeFact> = inputs.iter().filter_map(|iid| facts.get(iid)).collect();

    let unknown = RangeFact {
        lower: None,
        upper: None,
        is_nonzero: false,
        is_power_of_two: false,
        alignment: None,
    };

    // Operations that produce known ranges regardless of input ranges.
    match kind {
        NodeKind::ArrayCmpMask { .. } => {
            return RangeFact {
                lower: Some(0),
                upper: None, // Can be any bitvector value
                is_nonzero: false,
                is_power_of_two: false,
                alignment: None,
            };
        }
        // Popcount: result is [0, bit_width_of_input_type].
        NodeKind::Popcount { .. } => {
            // For v0.1, the max supported bit width is 64.
            // We use a conservative upper bound of 64 for all popcounts.
            return RangeFact {
                lower: Some(0),
                upper: Some(64),
                is_nonzero: false,
                is_power_of_two: false,
                alignment: None,
            };
        }
        // Comparisons + boolean: result is always 0 or 1.
        NodeKind::Eq { .. }
        | NodeKind::Ne { .. }
        | NodeKind::Lt { .. }
        | NodeKind::Le { .. }
        | NodeKind::Gt { .. }
        | NodeKind::Ge { .. }
        | NodeKind::BoolAnd { .. }
        | NodeKind::BoolOr { .. }
        | NodeKind::BoolNot { .. } => {
            return RangeFact {
                lower: Some(0),
                upper: Some(1),
                is_nonzero: false,
                is_power_of_two: false,
                alignment: None,
            };
        }
        _ => {}
    }

    if input_ranges.is_empty() {
        return unknown;
    }

    match kind {
        // ── Arithmetic ──
        NodeKind::Add { .. } => {
            if input_ranges.len() == 2 {
                let a = input_ranges[0];
                let b = input_ranges[1];
                let lo = a
                    .lower
                    .and_then(|al| b.lower.map(|bl| al.saturating_add(bl)));
                let hi = a
                    .upper
                    .and_then(|au| b.upper.map(|bu| au.saturating_add(bu)));
                RangeFact {
                    lower: lo,
                    upper: hi,
                    is_nonzero: lo.map(|v| v > 0).unwrap_or(false),
                    is_power_of_two: false,
                    alignment: None,
                }
            } else {
                unknown
            }
        }

        NodeKind::Mul { .. } => {
            if input_ranges.len() == 2 {
                let a = input_ranges[0];
                let b = input_ranges[1];
                // Conservative: max of products of bounds.
                let candidates = [
                    a.lower
                        .and_then(|al| b.lower.map(|bl| al.saturating_mul(bl))),
                    a.lower
                        .and_then(|al| b.upper.map(|bu| al.saturating_mul(bu))),
                    a.upper
                        .and_then(|au| b.lower.map(|bl| au.saturating_mul(bl))),
                    a.upper
                        .and_then(|au| b.upper.map(|bu| au.saturating_mul(bu))),
                ];
                let lo = candidates.iter().filter_map(|&v| v).min();
                let hi = candidates.iter().filter_map(|&v| v).max();
                RangeFact {
                    lower: lo,
                    upper: hi,
                    is_nonzero: lo.map(|v| v > 0).unwrap_or(false),
                    is_power_of_two: false,
                    alignment: None,
                }
            } else {
                unknown
            }
        }

        // ── Bitwise AND with constant ──
        NodeKind::And { .. } => {
            if input_ranges.len() == 2 {
                let b = input_ranges[1];
                // AND with a constant: result ≤ min(input, constant).
                let hi = b.upper;
                // If constant is a mask, lower bound is 0.
                let lo = Some(0i128);
                let nonzero = lo.map(|v| v > 0).unwrap_or(false);
                RangeFact {
                    lower: lo,
                    upper: hi,
                    is_nonzero: nonzero,
                    is_power_of_two: false,
                    alignment: None,
                }
            } else {
                unknown
            }
        }

        // ── Shifts ──
        NodeKind::Shl { .. } => {
            if input_ranges.len() == 2 {
                let a = input_ranges[0];
                let b = input_ranges[1];
                if let (Some(av), Some(bv)) = (a.lower, b.lower) {
                    if bv >= 0 && bv < 128 {
                        let shifted = av.checked_shl(bv as u32).unwrap_or(av);
                        RangeFact {
                            lower: Some(shifted),
                            upper: a.upper.and_then(|au| {
                                b.upper.map(|bu| {
                                    if bu >= 0 && bu < 128 {
                                        au.checked_shl(bu as u32).unwrap_or(au)
                                    } else {
                                        au
                                    }
                                })
                            }),
                            is_nonzero: shifted != 0,
                            is_power_of_two: bv == 1 && av == 1,
                            alignment: None,
                        }
                    } else {
                        unknown
                    }
                } else {
                    unknown
                }
            } else {
                unknown
            }
        }

        _ => unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sir_builder::Builder;
    use sir_types::{Span, Type};

    fn i32_type() -> Type {
        Type::i32()
    }
    fn u64_type() -> Type {
        Type::u64()
    }
    fn unknown_span() -> Span {
        Span::unknown()
    }

    #[test]
    fn constant_range() {
        let mut b = Builder::new("f", &[], i32_type());
        let c = b.constant(ConstantData::i32(42), i32_type(), unknown_span());
        b.return_value(c, unknown_span()).unwrap();
        let func = b.build();
        let facts = run_ranges(&func);

        let c_fact = facts.get(&c).unwrap();
        assert_eq!(c_fact.lower, Some(42));
        assert_eq!(c_fact.upper, Some(42));
        assert!(c_fact.is_nonzero);
    }

    #[test]
    fn add_propagates_range() {
        // [0,10] + [5,5] = [5,15]
        let mut b = Builder::new("f", &[], i32_type());
        let a = b.constant(ConstantData::i32(1), i32_type(), unknown_span());
        let b_c = b.constant(ConstantData::i32(2), i32_type(), unknown_span());
        let sum = b.add(a, b_c, unknown_span()).unwrap();
        b.return_value(sum, unknown_span()).unwrap();
        let func = b.build();
        let facts = run_ranges(&func);

        let sum_fact = facts.get(&sum).unwrap();
        assert_eq!(sum_fact.lower, Some(3));
        assert_eq!(sum_fact.upper, Some(3));
    }

    #[test]
    fn comparison_range_is_0_to_1() {
        let mut b = Builder::new("cmp", &[("a", i32_type()), ("b", i32_type())], Type::Bool);
        let a = b.parameter_index(0).unwrap();
        let b_param = b.parameter_index(1).unwrap();
        let eq = b.eq(a, b_param, unknown_span()).unwrap();
        b.return_value(eq, unknown_span()).unwrap();
        let func = b.build();
        let facts = run_ranges(&func);

        let eq_fact = facts.get(&eq).unwrap();
        assert_eq!(eq_fact.lower, Some(0));
        assert_eq!(eq_fact.upper, Some(1));
    }

    #[test]
    fn popcount_range() {
        let mut b = Builder::new("pop", &[("x", u64_type())], i32_type());
        let x = b.parameter_index(0).unwrap();
        let pop = b.popcount(x, unknown_span()).unwrap();
        b.return_value(pop, unknown_span()).unwrap();
        let func = b.build();
        let facts = run_ranges(&func);

        let pop_fact = facts.get(&pop).unwrap();
        assert_eq!(pop_fact.lower, Some(0));
        assert_eq!(pop_fact.upper, Some(64)); // u64 → max popcount is 64
    }

    #[test]
    fn nonzero_power_of_two_detection() {
        let mut b = Builder::new("f", &[], i32_type());
        let c = b.constant(ConstantData::i32(8), i32_type(), unknown_span());
        b.return_value(c, unknown_span()).unwrap();
        let func = b.build();
        let facts = run_ranges(&func);

        let c_fact = facts.get(&c).unwrap();
        assert!(c_fact.is_nonzero);
        assert!(c_fact.is_power_of_two);
    }
}
