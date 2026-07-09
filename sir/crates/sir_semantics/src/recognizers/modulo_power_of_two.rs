use sir_analysis::facts::FactDatabase;
use sir_nodes::Function;
use sir_types::{NodeId, ConstantData};

use crate::concepts::SemanticConcept;
use crate::region::RecognitionExplanation;
use sir_transform::structures::SourceStructure;

pub fn recognize_modulo_power_of_two(
    func: &Function,
    _analysis: &FactDatabase,
) -> Vec<(SemanticConcept, RecognitionExplanation, Vec<NodeId>)> {
    let mut results = Vec::new();

    for node in func.arena.iter() {
        if let sir_nodes::NodeKind::Rem { lhs, rhs } = &node.kind {
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
                            SemanticConcept::ModuloPowerOfTwo,
                            RecognitionExplanation {
                                concept: SemanticConcept::ModuloPowerOfTwo,
                                triggering_facts: vec![
                                    "Remainder operation with constant RHS",
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

pub fn recognize_modulo_operator(
    func: &Function,
    _analysis: &FactDatabase,
) -> Vec<(crate::region::RegionId, crate::structure::StructuralDescription)> {
    let mut results = Vec::new();

    for node in func.arena.iter() {
        if let sir_nodes::NodeKind::Rem { .. } = &node.kind {
            let desc = crate::structure::StructuralDescription::new(
                crate::region::RegionId::new(0), // merged later
                SourceStructure::ModuloOperator,
            );
            results.push((crate::region::RegionId::new(0), desc));
        }
    }

    results
}
