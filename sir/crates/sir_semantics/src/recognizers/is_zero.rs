use sir_analysis::facts::FactDatabase;
use sir_nodes::{Function, NodeKind};
use sir_types::NodeId;

use crate::concepts::SemanticConcept;
use crate::region::RecognitionExplanation;
use crate::truth::ValueId;

/// Recognizes `x == 0` operations.
pub fn recognize_is_zero(
    func: &Function,
    _analysis: &FactDatabase,
) -> Vec<(SemanticConcept, RecognitionExplanation, Vec<NodeId>, Vec<ValueId>, Vec<ValueId>)> {
    let mut recognized = Vec::new();

    for node in func.arena.iter() {
        let node_id = node.id;
        if let NodeKind::Eq { lhs, rhs } = &node.kind {
            let mut matched = false;
            let mut input_val = None;
            
            // Check if rhs is constant 0
            if let Some(rhs_node) = func.arena.get(*rhs) {
                if let NodeKind::Constant(value) = &rhs_node.kind {
                    if let Some(v) = value.as_u64() {
                        if v == 0 {
                            matched = true;
                            input_val = Some(*lhs);
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
                                input_val = Some(*rhs);
                            }
                        }
                    }
                }
            }

            if matched {
                let explanation = RecognitionExplanation {
                    concept: SemanticConcept::IsZero,
                    triggering_facts: vec!["Equality comparison with constant zero"],
                };
                let input_vid = ValueId::new(input_val.unwrap().0);
                let output_vid = ValueId::new(node_id.0);
                recognized.push((
                    SemanticConcept::IsZero, 
                    explanation, 
                    vec![node_id],
                    vec![input_vid],
                    vec![output_vid]
                ));
            }
        }
    }

    recognized
}
