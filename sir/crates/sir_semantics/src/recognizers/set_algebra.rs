use sir_analysis::facts::FactDatabase;
use sir_nodes::{Function, NodeKind};
use sir_types::{NodeId, Type};

use crate::concepts::SemanticConcept;
use crate::region::RecognitionExplanation;

/// Helper to determine if a node represents a value derived from a finite set
/// (e.g. an element loaded from a boolean array).
fn is_set_element(func: &Function, start: NodeId) -> bool {
    let mut stack = vec![start];
    let mut visited = std::collections::BTreeSet::new();

    while let Some(current) = stack.pop() {
        if !visited.insert(current) {
            continue;
        }

        let node = match func.get_node(current) {
            Some(n) => n,
            None => continue,
        };

        match &node.kind {
            NodeKind::ArrayAccess { base, .. } => {
                if let Some(base_node) = func.get_node(*base) {
                    if let Type::Array { element, .. } = &base_node.ty {
                        if matches!(element.as_ref(), &Type::Bool) {
                            return true;
                        }
                    }
                }
            }
            NodeKind::Load { ptr } => {
                stack.push(*ptr);
            }
            NodeKind::BoolNot { operand } => {
                stack.push(*operand);
            }
            NodeKind::BoolAnd { lhs, rhs } |
            NodeKind::BoolOr { lhs, rhs } => {
                stack.push(*lhs);
                stack.push(*rhs);
            }
            NodeKind::Xor { lhs, rhs } |
            NodeKind::Eq { lhs, rhs } |
            NodeKind::Ne { lhs, rhs } => {
                if matches!(node.ty, Type::Bool) {
                    stack.push(*lhs);
                    stack.push(*rhs);
                }
            }
            NodeKind::Select { true_val, false_val, .. } => {
                stack.push(*true_val);
                stack.push(*false_val);
            }
            _ => {}
        }
    }
    
    false
}

/// Recognize set algebra operations over boolean arrays or bitvectors.
pub fn recognize_set_algebra(
    func: &Function,
    _analysis: &FactDatabase,
) -> Vec<(SemanticConcept, RecognitionExplanation, Vec<NodeId>)> {
    let mut results = Vec::new();

    for node in func.arena.iter() {
        // Recognize FiniteSet data structures
        match &node.ty {
            Type::Array { element, .. } if matches!(element.as_ref(), &Type::Bool) => {
                let mut related = vec![node.id];
                // Gather array accesses to ensure merging works
                for consumer in func.arena.iter() {
                    if let NodeKind::ArrayAccess { base, .. } = &consumer.kind {
                        if *base == node.id {
                            related.push(consumer.id);
                        }
                    }
                }
                
                results.push((
                    SemanticConcept::FiniteSet,
                    RecognitionExplanation {
                        concept: SemanticConcept::FiniteSet,
                        triggering_facts: vec!["Boolean array represents a finite set"],
                    },
                    related,
                ));
            }
            // We deliberately DO NOT recognize `Type::BitVector` as a `FiniteSet` unconditionally.
            // A BitVector is a *representation*, whereas FiniteSet is a semantic domain.
            // An integer variable might just be a mathematical integer, not a FiniteSet.
            // If the user wants ABSAC to treat it as a set, it must be inferred from the context
            // or the domain.
            _ => {}
        }

        // Recognize Set Operations
        match &node.kind {
            // Set Membership: checking an element in a boolean array
            NodeKind::ArrayAccess { base, .. } => {
                if let Some(base_node) = func.get_node(*base) {
                    if let Type::Array { element, .. } = &base_node.ty {
                        if matches!(element.as_ref(), &Type::Bool) {
                            results.push((
                                SemanticConcept::SetMembership,
                                RecognitionExplanation {
                                    concept: SemanticConcept::SetMembership,
                                    triggering_facts: vec!["Array access on boolean array maps to set membership"],
                                },
                                vec![node.id, *base],
                            ));
                        }
                    }
                }
            }
            
            // Logic mapping to Set Intersections, Unions, etc.
            // But ONLY if the operands derive from a Set (e.g. pointwise over boolean arrays)
            NodeKind::BoolAnd { lhs, rhs } => {
                if is_set_element(func, *lhs) || is_set_element(func, *rhs) {
                    let mut is_difference = false;
                    
                    // Check if rhs is BoolNot
                    if let Some(rhs_node) = func.get_node(*rhs) {
                        if matches!(rhs_node.kind, NodeKind::BoolNot { .. }) {
                            is_difference = true;
                        }
                    }
                    
                    // Check if lhs is BoolNot (for difference the other way)
                    if !is_difference {
                        if let Some(lhs_node) = func.get_node(*lhs) {
                            if matches!(lhs_node.kind, NodeKind::BoolNot { .. }) {
                                is_difference = true;
                            }
                        }
                    }

                    if is_difference {
                        results.push((
                            SemanticConcept::SetDifference,
                            RecognitionExplanation {
                                concept: SemanticConcept::SetDifference,
                                triggering_facts: vec!["Pointwise Logical AND with NOT maps to set difference"],
                            },
                            vec![node.id, *lhs, *rhs],
                        ));
                    } else {
                        results.push((
                            SemanticConcept::SetIntersection,
                            RecognitionExplanation {
                                concept: SemanticConcept::SetIntersection,
                                triggering_facts: vec!["Pointwise Logical AND maps to set intersection"],
                            },
                            vec![node.id, *lhs, *rhs],
                        ));
                    }
                }
            }
            NodeKind::BoolOr { lhs, rhs } => {
                if is_set_element(func, *lhs) || is_set_element(func, *rhs) {
                    results.push((
                        SemanticConcept::SetUnion,
                        RecognitionExplanation {
                            concept: SemanticConcept::SetUnion,
                            triggering_facts: vec!["Pointwise Logical OR maps to set union"],
                        },
                        vec![node.id, *lhs, *rhs],
                    ));
                }
            }
            NodeKind::Xor { lhs, rhs } => {
                if matches!(node.ty, Type::Bool) && (is_set_element(func, *lhs) || is_set_element(func, *rhs)) {
                    results.push((
                        SemanticConcept::SetSymmetricDifference,
                        RecognitionExplanation {
                            concept: SemanticConcept::SetSymmetricDifference,
                            triggering_facts: vec!["Pointwise Logical XOR maps to symmetric difference"],
                        },
                        vec![node.id, *lhs, *rhs],
                    ));
                }
            }
            NodeKind::Ne { lhs, rhs } => {
                if matches!(node.ty, Type::Bool) && (is_set_element(func, *lhs) || is_set_element(func, *rhs)) {
                    results.push((
                        SemanticConcept::SetSymmetricDifference,
                        RecognitionExplanation {
                            concept: SemanticConcept::SetSymmetricDifference,
                            triggering_facts: vec!["Pointwise Logical Inequality maps to symmetric difference"],
                        },
                        vec![node.id, *lhs, *rhs],
                    ));
                }
            }
            NodeKind::Eq { lhs, rhs } => {
                if matches!(node.ty, Type::Bool) && (is_set_element(func, *lhs) || is_set_element(func, *rhs)) {
                    results.push((
                        SemanticConcept::SetEquality,
                        RecognitionExplanation {
                            concept: SemanticConcept::SetEquality,
                            triggering_facts: vec!["Pointwise Logical Equality maps to set equality"],
                        },
                        vec![node.id, *lhs, *rhs],
                    ));
                }
            }
            // We deliberately omit unconditional mappings from BitVector arithmetic (And, Or, Xor, Not)
            // to Set concepts. BitVector arithmetic is an implementation of Set semantics, but the
            // operation itself is purely mathematical/hardware representation.
            _ => {}
        }
    }

    results
}
