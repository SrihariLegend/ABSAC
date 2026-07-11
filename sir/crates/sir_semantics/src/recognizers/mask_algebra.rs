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
        }
    }

    results
}
