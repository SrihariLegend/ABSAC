use sir_analysis::facts::FactDatabase;
use sir_nodes::{Function, NodeKind};
use sir_types::NodeId;

use crate::concepts::SemanticConcept;
use crate::region::RecognitionExplanation;
use crate::truth::ValueId;

/// Recognizes loops that terminate when a variable reaches zero.
pub fn recognize_loop_until_zero(
    func: &Function,
    _analysis: &FactDatabase,
) -> Vec<(
    SemanticConcept,
    RecognitionExplanation,
    Vec<NodeId>,
    Vec<ValueId>,
    Vec<ValueId>,
)> {
    let mut recognized = Vec::new();

    for node in func.arena.iter() {
        if let NodeKind::Loop {
            termination,
            carried_inputs,
            body,
            ..
        } = &node.kind
        {
            if let Some(term_node) = func.arena.get(*termination) {
                if let NodeKind::Ne { lhs, rhs } = &term_node.kind {
                    let mut matched = false;
                    let mut zero_checked_var = None;

                    // Check if rhs is constant 0
                    if let Some(rhs_node) = func.arena.get(*rhs) {
                        if let NodeKind::Constant(value) = &rhs_node.kind {
                            if let Some(v) = value.as_u64() {
                                if v == 0 {
                                    matched = true;
                                    zero_checked_var = Some(*lhs);
                                }
                            }
                        }
                    }

                    // Check if lhs is constant 0
                    if !matched {
                        if let Some(lhs_node) = func.arena.get(*lhs) {
                            if let NodeKind::Constant(value) = &lhs_node.kind {
                                if let Some(v) = value.as_u64() {
                                    if v == 0 {
                                        matched = true;
                                        zero_checked_var = Some(*rhs);
                                    }
                                }
                            }
                        }
                    }

                    if matched {
                        if let Some(var) = zero_checked_var {
                            // Verify that this variable is part of the loop (e.g. it's computed in the body or carried)
                            // We use the node ID as the value it operates on
                            let explanation = RecognitionExplanation {
                                concept: SemanticConcept::LoopUntilZero,
                                triggering_facts: vec![
                                    "Loop termination is an inequality with constant zero",
                                ],
                            };
                            let input_vid = ValueId::new(var.0);
                            let output_vid = ValueId::new(node.id.0);

                            // The concept relates the loop to the variable being checked against zero
                            let mut nodes = body.clone();
                            nodes.push(node.id);
                            nodes.push(*termination);

                            recognized.push((
                                SemanticConcept::LoopUntilZero,
                                explanation,
                                nodes,
                                vec![input_vid],
                                vec![output_vid],
                            ));
                        }
                    }
                }
            }
        }
    }

    recognized
}
