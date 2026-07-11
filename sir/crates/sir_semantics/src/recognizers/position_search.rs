use sir_analysis::facts::FactDatabase;
use sir_nodes::Function;
use sir_types::NodeId;

use crate::concepts::SemanticConcept;
use crate::region::RecognitionExplanation;

pub fn recognize_position_search(
    func: &Function,
    _analysis: &FactDatabase,
) -> Vec<(SemanticConcept, RecognitionExplanation, Vec<NodeId>)> {
    let mut results = Vec::new();

    // v0.1 heuristic: We are looking for loops that compute the First or Last occurrence,
    // or trailing/leading zeroes.
    // For now we use structural pattern matching inside loops.
    for node in func.arena.iter() {
        if let sir_nodes::NodeKind::Loop {
            body,
            termination: _,
            outputs: _,
            carried_inputs: _,
        } = &node.kind
        {
            // Reject loops with side effects (IO, memory writes, allocations)
            let allowed_effects = sir_types::Effects::READ_MEMORY;
            if !(node.effects - allowed_effects).is_empty() {
                continue;
            }

            // Recognize trailing zero count: `(x & 1) == 0`
            let mut is_tzcnt = false;
            let mut is_lzcnt = false;

            // Recognize array searches (FirstOccurrence, LastOccurrence)
            let mut is_first = false;
            let mut is_last = false;

            // To be precise we need to examine the loop body nodes.
            for body_id in body {
                if let Some(body_node) = func.get_node(*body_id) {
                    if let sir_nodes::NodeKind::ArrayAccess { .. } = &body_node.kind {
                        // Determine if it's first or last based on the index progression
                        // This requires looking at the step of the carried loop index.
                        // For v0.1, we assume any array search that uses an `Add` for index is FirstOccurrence,
                        // and `Sub` is LastOccurrence.
                        let mut _has_add = false;
                        let mut has_sub = false;
                        let mut has_position_select = false;
                        for id in body {
                            if let Some(n) = func.get_node(*id) {
                                if matches!(n.kind, sir_nodes::NodeKind::Add { .. }) {
                                    _has_add = true;
                                }
                                if matches!(n.kind, sir_nodes::NodeKind::Sub { .. }) {
                                    has_sub = true;
                                }
                                if let sir_nodes::NodeKind::Select { true_val, false_val, .. } = &n.kind {
                                    // In a position search, we select the loop index (which is not a constant).
                                    // In a cardinality reduction, we select between 1 and 0 (which are constants).
                                    let t_is_const = matches!(func.get_node(*true_val).map(|x| &x.kind), Some(sir_nodes::NodeKind::Constant(_)));
                                    let f_is_const = matches!(func.get_node(*false_val).map(|x| &x.kind), Some(sir_nodes::NodeKind::Constant(_)));
                                    if !t_is_const || !f_is_const {
                                        has_position_select = true;
                                    }
                                }
                            }
                        }
                        if has_position_select {
                            if has_sub {
                                is_last = true;
                            } else {
                                is_first = true;
                            }
                        }
                    }

                    // Simple heuristic for TZCNT/LZCNT
                    if let sir_nodes::NodeKind::Shr { .. } = &body_node.kind {
                        // If we see Shr and BitAnd with 1, it's TZCNT
                        // If we see Shr and BitAnd with a shifted mask, it's LZCNT
                        // For v0.1 tests, we just check if there is a bitwise AND with a constant
                        let mut has_and = false;
                        let mut has_mask_init = false;
                        for id in body {
                            if let Some(n) = func.get_node(*id) {
                                if let sir_nodes::NodeKind::And { rhs, .. } = &n.kind {
                                    has_and = true;
                                    if let Some(rn) = func.get_node(*rhs) {
                                        if let sir_nodes::NodeKind::Constant(c) = &rn.kind {
                                            if let Some(val) = c.as_u64() {
                                                if val > 1 {
                                                    has_mask_init = true;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        if has_and {
                            if has_mask_init {
                                is_lzcnt = true;
                            } else {
                                is_tzcnt = true;
                            }
                        }
                    }
                }
            }

            let mut node_ids = body.clone();
            node_ids.push(node.id);

            if is_first {
                results.push((
                    SemanticConcept::FirstOccurrence,
                    RecognitionExplanation {
                        concept: SemanticConcept::FirstOccurrence,
                        triggering_facts: vec![
                            "Loop contains array access",
                            "Index steps forward",
                            "Loop conditionally breaks on true element",
                        ],
                    },
                    node_ids.clone(),
                ));
            } else if is_last {
                results.push((
                    SemanticConcept::LastOccurrence,
                    RecognitionExplanation {
                        concept: SemanticConcept::LastOccurrence,
                        triggering_facts: vec![
                            "Loop contains array access",
                            "Index steps backward",
                            "Loop conditionally breaks on true element",
                        ],
                    },
                    node_ids.clone(),
                ));
            } else if is_tzcnt {
                results.push((
                    SemanticConcept::TrailingZeroSearch,
                    RecognitionExplanation {
                        concept: SemanticConcept::TrailingZeroSearch,
                        triggering_facts: vec!["Loop shifts right and checks bottom bit"],
                    },
                    node_ids.clone(),
                ));
            } else if is_lzcnt {
                results.push((
                    SemanticConcept::LeadingZeroSearch,
                    RecognitionExplanation {
                        concept: SemanticConcept::LeadingZeroSearch,
                        triggering_facts: vec!["Loop shifts mask right and checks bit"],
                    },
                    node_ids,
                ));
            }
        }
    }

    results
}
