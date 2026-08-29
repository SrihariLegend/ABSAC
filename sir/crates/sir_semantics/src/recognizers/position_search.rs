use sir_analysis::facts::FactDatabase;
use sir_nodes::{Function, NodeKind};
use sir_types::NodeId;

use crate::concepts::SemanticConcept;
use crate::region::RecognitionExplanation;

/// Detect the forward-search termination-condition form:
///
/// ```text
/// is_true = arr[i]                      // predicate evaluates the current element
/// not_found = !is_true                  // negated predicate
/// next_i = i + 1                        // forward index increment
/// cond = not_found && (next_i < bound)  // terminate when found or out of bounds
/// loop outputs = [next_i]               // the resulting search index
/// ```
///
/// This is the PS001 `first_set_bit` shape: the loop continues while the
/// current element is false and the index is in bounds, and exports the
/// incremented index as its result.
///
/// Returns the loop's index-result node (the increment) when the pattern
/// matches, `None` otherwise.
fn detect_forward_termination_search(
    func: &Function,
    body: &[NodeId],
    termination: NodeId,
    outputs: &[NodeId],
    carried_inputs: &[NodeId],
) -> Option<NodeId> {
    // Termination must be `not_found && bounds_check` (either operand order).
    let term = func.get_node(termination)?;
    let (not_found, bounds) = match &term.kind {
        NodeKind::BoolAnd { lhs, rhs } => {
            let l = func.get_node(*lhs)?;
            let r = func.get_node(*rhs)?;
            match (&l.kind, &r.kind) {
                (NodeKind::BoolNot { operand }, _) => (*operand, *rhs),
                (_, NodeKind::BoolNot { operand }) => (*operand, *lhs),
                _ => return None,
            }
        }
        _ => return None,
    };

    // `not_found = !predicate` where the predicate reads the current element.
    if !body.contains(&not_found) {
        return None;
    }
    let predicate = func.get_node(not_found)?;
    let NodeKind::ArrayAccess { base, .. } = &predicate.kind else {
        return None;
    };
    let base_node = func.get_node(*base)?;
    if !matches!(base_node.ty, sir_types::Type::Array { .. }) {
        return None;
    }

    // The bounds check must compare the incremented index in the forward
    // direction: `next_i < bound` (or `<=` / `!=`).
    let bounds_node = func.get_node(bounds)?;
    let (index_node, _bound) = match &bounds_node.kind {
        NodeKind::Lt { lhs, rhs } | NodeKind::Le { lhs, rhs } | NodeKind::Ne { lhs, rhs } => {
            (*lhs, *rhs)
        }
        _ => return None,
    };

    // The index increments forward by one (`i + 1`) and is the loop's search
    // result output.
    if !body.contains(&index_node) || !outputs.contains(&index_node) {
        return None;
    }
    let inc = func.get_node(index_node)?;
    let NodeKind::Add { lhs, rhs } = &inc.kind else {
        return None;
    };
    if !carried_inputs.contains(lhs) {
        return None;
    }
    let step = func.get_node(*rhs)?;
    if !matches!(&step.kind, NodeKind::Constant(c) if c.as_u64() == Some(1)) {
        return None;
    }

    Some(index_node)
}

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
            termination,
            outputs,
            carried_inputs,
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

            // Forward-search termination-condition form (PS001 `first_set_bit`):
            //   is_true = arr[i]; not_found = !is_true; next_i = i + 1;
            //   cond = not_found && (next_i < 64); loop outputs next_i.
            if detect_forward_termination_search(func, body, *termination, outputs, carried_inputs)
                .is_some()
            {
                is_first = true;
            }

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

#[cfg(test)]
mod tests {
    use super::*;
    use sir_analysis::manager::AnalysisManager;
    use sir_builder::Builder;
    use sir_types::{ConstantData, Span, Type};

    fn unknown() -> Span {
        Span::unknown()
    }

    fn bool_array(len: usize) -> Type {
        Type::Array {
            element: Box::new(Type::Bool),
            length: len,
        }
    }

    /// Build the PS001 forward-search termination-condition form:
    ///
    /// ```text
    /// is_true = arr[i]
    /// not_found = !is_true
    /// next_i = i + 1
    /// cond = not_found && (next_i < 64)
    /// loop output = next_i
    /// ```
    fn build_termination_search(len: u64, bound_op: &str) -> Function {
        let mut b = Builder::new(
            "first_set_bit",
            &[("arr", bool_array(len as usize))],
            Type::Tuple {
                elements: vec![Type::u64()],
            },
        );
        let arr = b.parameter_index(0).unwrap();
        let one = b.constant(ConstantData::u64(1), Type::u64(), unknown());
        let limit = b.constant(ConstantData::u64(len), Type::u64(), unknown());
        let i_init = b.constant(ConstantData::u64(0), Type::u64(), unknown());

        let is_true = b.array_access(arr, i_init, Type::Bool, unknown()).unwrap();
        let not_found = b.bool_not(is_true, unknown()).unwrap();
        let next_i = b.add(i_init, one, unknown()).unwrap();
        let bounds_check = match bound_op {
            "lt" => b.lt(next_i, limit, unknown()).unwrap(),
            "le" => b.le(next_i, limit, unknown()).unwrap(),
            "ne" => b.ne(next_i, limit, unknown()).unwrap(),
            _ => panic!("unknown bound op {bound_op}"),
        };
        let cond = b.bool_and(not_found, bounds_check, unknown()).unwrap();

        let loop_node = b
            .r#loop(
                &[is_true, not_found, next_i, bounds_check, cond],
                cond,
                &[next_i],
                &[i_init],
                Type::Tuple {
                    elements: vec![Type::u64()],
                },
                unknown(),
            )
            .unwrap();

        b.return_value(loop_node, unknown()).unwrap();
        b.build()
    }

    fn recognized_concepts(func: &Function) -> Vec<SemanticConcept> {
        let mut analysis = AnalysisManager::new();
        analysis.run_all(func);
        recognize_position_search(func, analysis.database())
            .into_iter()
            .map(|r| r.0)
            .collect()
    }

    fn find_loop_id(func: &Function) -> NodeId {
        func.arena
            .iter()
            .find(|n| matches!(n.kind, sir_nodes::NodeKind::Loop { .. }))
            .map(|n| n.id)
            .expect("expected a loop node")
    }

    #[test]
    fn recognizes_termination_search_as_first_occurrence() {
        // The canonical PS001 shape: predicate on the current element, negated
        // as the continuation condition, forward `+1` index, `next_i < bound`
        // bounds termination, index exported as the loop output.
        let func = build_termination_search(64, "lt");
        let concepts = recognized_concepts(&func);
        assert!(
            concepts.contains(&SemanticConcept::FirstOccurrence),
            "expected FirstOccurrence, got {concepts:?}"
        );
        assert!(!concepts.contains(&SemanticConcept::LastOccurrence));
    }

    #[test]
    fn recognizes_forward_index_progression() {
        // `next_i = i + 1` with a `<=` bounds check still steps forward.
        let func = build_termination_search(64, "le");
        let concepts = recognized_concepts(&func);
        assert!(
            concepts.contains(&SemanticConcept::FirstOccurrence),
            "expected FirstOccurrence for forward `<=` bounds, got {concepts:?}"
        );

        // A `!=` bound also terminates the forward scan.
        let func = build_termination_search(64, "ne");
        let concepts = recognized_concepts(&func);
        assert!(
            concepts.contains(&SemanticConcept::FirstOccurrence),
            "expected FirstOccurrence for `!=` bounds, got {concepts:?}"
        );
    }

    #[test]
    fn recognizes_bounds_termination() {
        // Bounds termination (in-bounds check ANDed into the continuation
        // condition) is what distinguishes a bounded scan; it must still
        // recognize as FirstOccurrence.
        let func = build_termination_search(32, "lt");
        let concepts = recognized_concepts(&func);
        assert!(
            concepts.contains(&SemanticConcept::FirstOccurrence),
            "expected FirstOccurrence with bounds termination, got {concepts:?}"
        );
    }

    #[test]
    fn termination_search_has_correct_result_role() {
        // End-to-end semantic derivation must attach a PositionSearch role whose
        // result is the loop node (the exported search index) and whose
        // collection is the boolean array parameter — never an internal
        // predicate, increment, or condition node.
        let func = build_termination_search(64, "lt");
        let loop_id = find_loop_id(&func);
        let arr_id = func
            .arena
            .iter()
            .find(|n| matches!(n.kind, sir_nodes::NodeKind::Parameter { .. }))
            .map(|n| n.id)
            .expect("expected the array parameter");

        let mut analysis = AnalysisManager::new();
        analysis.run_all(&func);
        let mut semantics = crate::semantics::SemanticEngine::new();
        semantics.derive(&func, analysis.database());

        let rid = semantics
            .database()
            .regions()
            .find(|(_, r)| r.contains(SemanticConcept::FirstOccurrence))
            .map(|(rid, _)| rid)
            .expect("expected a FirstOccurrence region");

        let desc = semantics
            .structural_database()
            .region(rid)
            .expect("expected a structural description");
        let role = desc
            .roles
            .iter()
            .find_map(|r| match r {
                sir_transform::roles::RegionRoles::PositionSearch {
                    collection,
                    scalar,
                    result,
                } => Some((*collection, *scalar, *result)),
                _ => None,
            })
            .expect("expected a PositionSearch role");

        assert_eq!(role.2, loop_id, "result must be the loop node");
        assert_eq!(
            role.0,
            Some(arr_id),
            "collection must be the array parameter"
        );
        assert_eq!(role.1, None, "no scalar for an array search");
    }

    #[test]
    fn does_not_recognize_count_loop() {
        // A pure count loop terminates on `i < limit` only — no negated
        // predicate ANDed into the condition — so it is not a search.
        let mut b = Builder::new(
            "count",
            &[("arr", bool_array(64))],
            Type::Tuple {
                elements: vec![Type::u64(), Type::u64()],
            },
        );
        let arr = b.parameter_index(0).unwrap();
        let zero = b.constant(ConstantData::u64(0), Type::u64(), unknown());
        let one = b.constant(ConstantData::u64(1), Type::u64(), unknown());
        let limit = b.constant(ConstantData::u64(64), Type::u64(), unknown());
        let i_init = b.constant(ConstantData::u64(0), Type::u64(), unknown());

        let next_i = b.add(i_init, one, unknown()).unwrap();
        let is_true = b.array_access(arr, i_init, Type::Bool, unknown()).unwrap();
        let to_add = b.select(is_true, one, zero, unknown()).unwrap();
        let next_count = b.add(i_init, to_add, unknown()).unwrap();
        let cond = b.lt(next_i, limit, unknown()).unwrap();
        let loop_node = b
            .r#loop(
                &[next_i, is_true, to_add, next_count, cond],
                cond,
                &[next_i, next_count],
                &[i_init, i_init],
                Type::Tuple {
                    elements: vec![Type::u64(), Type::u64()],
                },
                unknown(),
            )
            .unwrap();
        b.return_value(loop_node, unknown()).unwrap();
        let func = b.build();

        let concepts = recognized_concepts(&func);
        assert!(
            !concepts.contains(&SemanticConcept::FirstOccurrence),
            "count loop must not be FirstOccurrence, got {concepts:?}"
        );
    }

    #[test]
    fn does_not_recognize_backward_search_as_first_occurrence() {
        // A backward scan (`i - 1`, `i >= 0` bounds) is a last-occurrence
        // search, not a first-occurrence search.
        let mut b = Builder::new(
            "last_set_bit",
            &[("arr", bool_array(64))],
            Type::Tuple {
                elements: vec![Type::u64()],
            },
        );
        let arr = b.parameter_index(0).unwrap();
        let one = b.constant(ConstantData::u64(1), Type::u64(), unknown());
        let zero = b.constant(ConstantData::u64(0), Type::u64(), unknown());
        let i_init = b.constant(ConstantData::u64(63), Type::u64(), unknown());

        let is_true = b.array_access(arr, i_init, Type::Bool, unknown()).unwrap();
        let not_found = b.bool_not(is_true, unknown()).unwrap();
        let next_i = b.sub(i_init, one, unknown()).unwrap();
        let bounds_check = b.ge(i_init, zero, unknown()).unwrap();
        let cond = b.bool_and(not_found, bounds_check, unknown()).unwrap();
        let loop_node = b
            .r#loop(
                &[is_true, not_found, next_i, bounds_check, cond],
                cond,
                &[next_i],
                &[i_init],
                Type::Tuple {
                    elements: vec![Type::u64()],
                },
                unknown(),
            )
            .unwrap();
        b.return_value(loop_node, unknown()).unwrap();
        let func = b.build();

        let concepts = recognized_concepts(&func);
        assert!(
            !concepts.contains(&SemanticConcept::FirstOccurrence),
            "backward scan must not be FirstOccurrence, got {concepts:?}"
        );
        // (LastOccurrence recognition for the termination-condition form is
        // PS002 territory and out of scope here — the important invariant is
        // that this shape never becomes FirstOccurrence.)
    }

    #[test]
    fn does_not_recognize_search_without_index_output() {
        // The continuation condition has the search shape, but the loop does
        // not export the incremented index — there is no search result to
        // bitscan, so it must not be a FirstOccurrence.
        let mut b = Builder::new(
            "any_early_exit",
            &[("arr", bool_array(64))],
            Type::Tuple {
                elements: vec![Type::Bool],
            },
        );
        let arr = b.parameter_index(0).unwrap();
        let one = b.constant(ConstantData::u64(1), Type::u64(), unknown());
        let limit = b.constant(ConstantData::u64(64), Type::u64(), unknown());
        let i_init = b.constant(ConstantData::u64(0), Type::u64(), unknown());
        let found_init = b.constant(ConstantData::boolean(false), Type::Bool, unknown());

        let is_true = b.array_access(arr, i_init, Type::Bool, unknown()).unwrap();
        let new_found = b.bool_or(found_init, is_true, unknown()).unwrap();
        let not_found = b.bool_not(new_found, unknown()).unwrap();
        let next_i = b.add(i_init, one, unknown()).unwrap();
        let bounds_check = b.lt(next_i, limit, unknown()).unwrap();
        let cond = b.bool_and(not_found, bounds_check, unknown()).unwrap();
        let loop_node = b
            .r#loop(
                &[is_true, new_found, not_found, next_i, bounds_check, cond],
                cond,
                &[new_found],
                &[found_init],
                Type::Tuple {
                    elements: vec![Type::Bool],
                },
                unknown(),
            )
            .unwrap();
        b.return_value(loop_node, unknown()).unwrap();
        let func = b.build();

        let concepts = recognized_concepts(&func);
        assert!(
            !concepts.contains(&SemanticConcept::FirstOccurrence),
            "loop without an index output must not be FirstOccurrence, got {concepts:?}"
        );
    }

    #[test]
    fn does_not_recognize_non_element_predicate() {
        // The negated predicate must evaluate the current array element; a
        // loop that negates an accumulated flag is an early-exit scan, not a
        // position search.
        let mut b = Builder::new(
            "any_accum",
            &[("arr", bool_array(64))],
            Type::Tuple {
                elements: vec![Type::Bool],
            },
        );
        let arr = b.parameter_index(0).unwrap();
        let one = b.constant(ConstantData::u64(1), Type::u64(), unknown());
        let limit = b.constant(ConstantData::u64(64), Type::u64(), unknown());
        let i_init = b.constant(ConstantData::u64(0), Type::u64(), unknown());
        let found_init = b.constant(ConstantData::boolean(false), Type::Bool, unknown());

        let is_true = b.array_access(arr, i_init, Type::Bool, unknown()).unwrap();
        let _not_elem = b.bool_not(is_true, unknown()).unwrap();
        let next_i = b.add(i_init, one, unknown()).unwrap();
        let bounds_check = b.lt(next_i, limit, unknown()).unwrap();
        // Termination negates the accumulated flag, not the element predicate.
        let not_found = b.bool_not(found_init, unknown()).unwrap();
        let cond = b.bool_and(not_found, bounds_check, unknown()).unwrap();
        let loop_node = b
            .r#loop(
                &[is_true, _not_elem, next_i, bounds_check, not_found, cond],
                cond,
                &[next_i],
                &[i_init],
                Type::Tuple {
                    elements: vec![Type::u64()],
                },
                unknown(),
            )
            .unwrap();
        b.return_value(loop_node, unknown()).unwrap();
        let func = b.build();

        let concepts = recognized_concepts(&func);
        assert!(
            !concepts.contains(&SemanticConcept::FirstOccurrence),
            "non-element predicate must not be FirstOccurrence, got {concepts:?}"
        );
    }
}
