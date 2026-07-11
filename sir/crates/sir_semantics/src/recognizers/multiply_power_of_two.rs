use sir_analysis::facts::FactDatabase;
use sir_nodes::Function;
use sir_types::NodeId;

use crate::concepts::SemanticConcept;
use crate::region::RecognitionExplanation;
use sir_transform::structures::SourceStructure;

pub fn recognize_multiply_power_of_two(
    func: &Function,
    _analysis: &FactDatabase,
) -> Vec<(SemanticConcept, RecognitionExplanation, Vec<NodeId>)> {
    let mut results = Vec::new();

    for node in func.arena.iter() {
        if let sir_nodes::NodeKind::Mul { lhs, rhs } = &node.kind {
            if let Some(rhs_node) = func.get_node(*rhs) {
                if let sir_nodes::NodeKind::Constant(c) = &rhs_node.kind {
                    let is_power_of_two = if let Some(v) = c.as_u64() {
                        v.is_power_of_two()
                    } else if let Some(v) = c.as_i64() {
                        v > 0 && (v as u64).is_power_of_two()
                    } else {
                        false
                    };

                    if is_power_of_two {
                        results.push((
                            SemanticConcept::MultiplyPowerOfTwo,
                            RecognitionExplanation {
                                concept: SemanticConcept::MultiplyPowerOfTwo,
                                triggering_facts: vec![
                                    "Multiplication operation with constant RHS",
                                    "Constant is a power of two",
                                ],
                            },
                            vec![node.id, *lhs, *rhs],
                        ));
                    }
                }
            }
            if let Some(lhs_node) = func.get_node(*lhs) {
                if let sir_nodes::NodeKind::Constant(c) = &lhs_node.kind {
                    let is_power_of_two = if let Some(v) = c.as_u64() {
                        v.is_power_of_two()
                    } else if let Some(v) = c.as_i64() {
                        v > 0 && (v as u64).is_power_of_two()
                    } else {
                        false
                    };

                    if is_power_of_two {
                        results.push((
                            SemanticConcept::MultiplyPowerOfTwo,
                            RecognitionExplanation {
                                concept: SemanticConcept::MultiplyPowerOfTwo,
                                triggering_facts: vec![
                                    "Multiplication operation with constant LHS",
                                    "Constant is a power of two",
                                ],
                            },
                            vec![node.id, *lhs, *rhs],
                        ));
                    }
                }
            }
        }
    }

    results
}

pub fn recognize_multiply_operator(
    func: &Function,
    _analysis: &FactDatabase,
) -> Vec<(
    crate::region::RegionId,
    crate::structure::StructuralDescription,
)> {
    let mut results = Vec::new();

    for node in func.arena.iter() {
        if let sir_nodes::NodeKind::Mul { .. } = &node.kind {
            let desc = crate::structure::StructuralDescription::new(
                crate::region::RegionId::new(0),
                SourceStructure::MultiplyOperator,
            );
            results.push((crate::region::RegionId::new(0), desc));
        }
    }

    results
}
