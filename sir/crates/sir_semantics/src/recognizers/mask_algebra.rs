use sir_analysis::facts::FactDatabase;
use sir_nodes::{Function, NodeKind};
use sir_types::{NodeId, ConstantData};

use crate::concepts::SemanticConcept;
use crate::region::RecognitionExplanation;
use crate::truth::ValueId;

/// Recognize mask algebra patterns, such as clearing the lowest set bit.
///
/// Recognizes:
/// - `x & (x - 1)` -> ClearLowestSetBit
/// - `x & -x` -> LowestSetBit
pub fn recognize_mask_algebra(
    func: &Function,
    _analysis: &FactDatabase,
) -> Vec<(SemanticConcept, RecognitionExplanation, Vec<NodeId>, Vec<ValueId>, Vec<ValueId>)> {
    let mut results = Vec::new();

    for node in func.arena.iter() {
        if let NodeKind::And { lhs, rhs } = &node.kind {
            // Check if one operand is `x` and the other is `x - 1`
            let check_sub = |x: NodeId, sub_node: NodeId| -> bool {
                if let Some(sub) = func.get_node(sub_node) {
                    if let NodeKind::Sub { lhs: sub_lhs, rhs: sub_rhs } = &sub.kind {
                        if *sub_lhs == x {
                            if let Some(one) = func.get_node(*sub_rhs) {
                                if let NodeKind::Constant(c) = &one.kind {
                                    if let ConstantData::Integer { value, .. } = c {
                                        if value == "1" {
                                            return true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                false
            };

            // Check if one operand is `x` and the other is `-x`
            let check_neg = |x: NodeId, neg_node: NodeId| -> bool {
                if let Some(neg) = func.get_node(neg_node) {
                    if let NodeKind::Neg { operand } = &neg.kind {
                        if *operand == x {
                            return true;
                        }
                    }
                }
                false
            };

            let mut operand = None;
            if check_sub(*lhs, *rhs) {
                operand = Some(*lhs);
            } else if check_sub(*rhs, *lhs) {
                operand = Some(*rhs);
            }

            if let Some(x) = operand {
                results.push((
                    SemanticConcept::ClearLowestSetBit,
                    RecognitionExplanation {
                        concept: SemanticConcept::ClearLowestSetBit,
                        triggering_facts: vec!["Detected mask algebra pattern: x & (x - 1)"],
                    },
                    vec![node.id, x],
                    vec![ValueId::new(x.0)],       // Input: x
                    vec![ValueId::new(node.id.0)], // Output: x & (x - 1)
                ));
            }

            let mut operand = None;
            if check_neg(*lhs, *rhs) {
                operand = Some(*lhs);
            } else if check_neg(*rhs, *lhs) {
                operand = Some(*rhs);
            }

            if let Some(x) = operand {
                results.push((
                    SemanticConcept::LowestSetBit,
                    RecognitionExplanation {
                        concept: SemanticConcept::LowestSetBit,
                        triggering_facts: vec!["Detected mask algebra pattern: x & -x"],
                    },
                    vec![node.id, x],
                    vec![ValueId::new(x.0)],       // Input: x
                    vec![ValueId::new(node.id.0)], // Output: x & -x
                ));
            }
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use sir_builder::Builder;
    use sir_analysis::manager::AnalysisManager;
    use crate::semantics::SemanticEngine;
    use sir_types::{Type, ConstantData, Span};

    fn unknown_span() -> Span {
        Span::unknown()
    }

    #[test]
    fn recognizes_isolate_lowest_set_bit_pattern() {
        // fn f(x: u64) -> u64 { x & -x }
        let mut b = Builder::new("isolate", &[("x", Type::u64())], Type::u64());
        let x = b.parameter_index(0).unwrap();
        let neg_x = b.neg(x, unknown_span()).unwrap();
        let res = b.bit_and(x, neg_x, unknown_span()).unwrap();
        b.return_value(res, unknown_span()).unwrap();
        let func = b.build();

        let mut analysis = AnalysisManager::new();
        analysis.run_all(&func);

        let recs = recognize_mask_algebra(&func, analysis.database());
        let lowest = recs.iter().find(|r| r.0 == SemanticConcept::LowestSetBit);
        assert!(
            lowest.is_some(),
            "expected a LowestSetBit recognition, got {:?}",
            recs.iter().map(|r| r.0).collect::<Vec<_>>()
        );
        let (_, _, node_ids, inputs, outputs) = lowest.unwrap();
        assert_eq!(inputs, &vec![ValueId::new(x.0)]);
        assert_eq!(outputs, &vec![ValueId::new(res.0)]);
        assert_eq!(node_ids, &vec![res, x]);
    }

    #[test]
    fn recognizes_neg_on_right_side_too() {
        // fn f(x: u64) -> u64 { -x & x } — operand order must not matter.
        let mut b = Builder::new("isolate2", &[("x", Type::u64())], Type::u64());
        let x = b.parameter_index(0).unwrap();
        let neg_x = b.neg(x, unknown_span()).unwrap();
        let res = b.bit_and(neg_x, x, unknown_span()).unwrap();
        b.return_value(res, unknown_span()).unwrap();
        let func = b.build();

        let mut analysis = AnalysisManager::new();
        analysis.run_all(&func);
        let recs = recognize_mask_algebra(&func, analysis.database());
        assert!(
            recs.iter().any(|r| r.0 == SemanticConcept::LowestSetBit),
            "operand order should not matter"
        );
    }

    #[test]
    fn does_not_misclassify_plain_and() {
        // fn f(a: u64, b: u64) -> u64 { a & b } — no mask pattern.
        let mut b = Builder::new("plain_and", &[("a", Type::u64()), ("b", Type::u64())], Type::u64());
        let a = b.parameter_index(0).unwrap();
        let bb = b.parameter_index(1).unwrap();
        let res = b.bit_and(a, bb, unknown_span()).unwrap();
        b.return_value(res, unknown_span()).unwrap();
        let func = b.build();

        let mut analysis = AnalysisManager::new();
        analysis.run_all(&func);
        let recs = recognize_mask_algebra(&func, analysis.database());
        assert!(
            !recs.iter().any(|r| r.0 == SemanticConcept::LowestSetBit),
            "plain AND must not be recognized as LowestSetBit"
        );
    }

    #[test]
    fn end_to_end_semantic_engine_derives_lowest_set_bit() {
        use sir_builder::Builder;
        let mut b = Builder::new("isolate_e2e", &[("x", Type::u64())], Type::u64());
        let x = b.parameter_index(0).unwrap();
        let neg_x = b.neg(x, unknown_span()).unwrap();
        let res = b.bit_and(x, neg_x, unknown_span()).unwrap();
        b.return_value(res, unknown_span()).unwrap();
        let func = b.build();

        let mut analysis = AnalysisManager::new();
        analysis.run_all(&func);
        let mut semantics = SemanticEngine::new();
        semantics.derive(&func, analysis.database());

        let has_lowest = semantics
            .database()
            .regions()
            .any(|(_, r)| r.contains(SemanticConcept::LowestSetBit));
        assert!(has_lowest, "derived regions should contain LowestSetBit");
    }
}
