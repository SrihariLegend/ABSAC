//! emit_vectorized — lower LLVM IR, recognize semantic reduction, emit vectorized C.
//! Usage: emit_vectorized <file.ll> <function_name>

use sir_analysis::manager::AnalysisManager;
use sir_lower::lower_function;
use sir_semantics::semantics::SemanticEngine;
use sir_semantics::truth::{SemanticTruth, Provenance};
use sir_semantics::concepts::SemanticConcept;
use sir_benchmarks::vector_plan::{VectorPlan, VectorOp, VectorPredicate, TailPolicy};
use sir_benchmarks::vector_emit::emit_vectorized;
use sir_nodes::{Function, NodeKind};
use sir_types::NodeId;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: emit_vectorized <file.ll> <function_name>");
        std::process::exit(1);
    }
    let ll_path = &args[1];
    let func_name = &args[2];

    let ll_text = std::fs::read_to_string(ll_path)
        .unwrap_or_else(|e| { eprintln!("error reading {}: {}", ll_path, e); std::process::exit(1); });

    let func = lower_function(&ll_text, func_name)
        .unwrap_or_else(|e| { eprintln!("lower error: {:?}", e); std::process::exit(1); });

    let mut mgr = AnalysisManager::new();
    mgr.run_all(&func);
    let mut engine = SemanticEngine::new();
    engine.derive(&func, mgr.database());
    let db = engine.database();
    let truths: Vec<SemanticTruth> = db.truths().cloned().collect();

    let plan = derive_vector_plan(&func, &truths);

    match plan {
        Some(p) => {
            eprintln!("Recognized: {:?} with predicate {:?}", p.operation, p.predicate);
            println!("#include <stdint.h>");
            println!("#include <stdbool.h>");
            println!("#include <immintrin.h>");
            println!();
            println!("{}", emit_vectorized(&func, &p));
        }
        None => {
            eprintln!("No vectorizable semantic reduction found. Falling back to scalar.");
            println!("{}", sir_benchmarks::emit::emit_c(&func));
        }
    }
}

fn param_name(func: &Function, nid: NodeId) -> String {
    let idx = nid.as_u64() as usize;
    if idx < func.params.len() {
        let p = &func.params[idx];
        if p.name.starts_with('%') {
            format!("p{}", idx)
        } else {
            p.name.clone()
        }
    } else {
        format!("v{}", nid.as_u64())
    }
}

/// Resolve a NodeId to a C expression (constant value or parameter name)
fn resolve_value(func: &Function, nid: NodeId) -> String {
    if let Some(node) = func.arena.get(nid) {
        match &node.kind {
            NodeKind::Constant(data) => {
                if let Some(v) = data.as_u64() {
                    return format!("{}", v);
                }
                "0".to_string()
            }
            NodeKind::Parameter { .. } => param_name(func, nid),
            _ => format!("v{}", nid.as_u64()),
        }
    } else {
        "0".to_string()
    }
}

fn derive_vector_plan(func: &Function, truths: &[SemanticTruth]) -> Option<VectorPlan> {
    let has_cardinality = truths.iter().any(|t| t.concept == SemanticConcept::CardinalityReduction);
    let has_conjunctive = truths.iter().any(|t| t.concept == SemanticConcept::ConjunctiveReduction);
    let has_predicate_map = truths.iter().any(|t| t.concept == SemanticConcept::PredicateMap);

    // Find the loop body nodes to check for comparisons INSIDE the loop
    let loop_body_has_comparison = find_loop_comparison(func);

    // Distinguish Cardinality (has comparison in loop) from Sum (no comparison in loop)
    if has_cardinality && loop_body_has_comparison {
        return derive_cardinality_plan(func, truths);
    }

    if has_cardinality && !loop_body_has_comparison {
        // k43_sum_ascii: sum += buf[i], misidentified as CardinalityReduction
        return derive_sum_plan(func);
    }

    if has_conjunctive {
        return derive_all_plan(func, truths);
    }

    // Fallback: check for AND-reduction pattern (k18_all_equal)
    // The lowered SIR uses Select (v ? 1 : 0) for the AND pattern, not BoolAnd
    if has_predicate_map {
        // Check if the loop body has a Select that implements AND-reduction
        let has_and_pattern = check_and_reduction_pattern(func);
        if has_and_pattern {
            return derive_all_plan(func, truths);
        }
    }

    None
}

/// Check if the loop body contains a comparison (Eq/Ne) that's NOT the loop termination check
fn find_loop_comparison(func: &Function) -> bool {
    // Find the Loop node
    for node in func.arena.iter() {
        if let NodeKind::Loop { body, termination, .. } = &node.kind {
            // Collect all body subgraph nodes
            let mut body_nodes = Vec::new();
            for &bid in body {
                body_nodes.extend(collect_subgraph(func, bid));
            }
            // Check for Eq/Ne in body (excluding the termination check)
            for &bid in &body_nodes {
                if let Some(bn) = func.arena.get(bid) {
                    if matches!(&bn.kind, NodeKind::Eq { .. } | NodeKind::Ne { .. }) {
                        if bid != *termination {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

/// Collect all nodes reachable from a starting node (subgraph)
fn collect_subgraph(func: &Function, start: NodeId) -> Vec<NodeId> {
    let mut visited = std::collections::HashSet::new();
    let mut stack = vec![start];
    while let Some(nid) = stack.pop() {
        if visited.contains(&nid) { continue; }
        visited.insert(nid);
        if let Some(node) = func.arena.get(nid) {
            for input in node.kind.input_nodes() {
                stack.push(input);
            }
        }
    }
    visited.into_iter().collect()
}

/// Check if the loop implements an AND-reduction pattern
/// (Select with true_val = carried accumulator, false_val = 0/false)
fn check_and_reduction_pattern(func: &Function) -> bool {
    for node in func.arena.iter() {
        if let NodeKind::Loop { body, .. } = &node.kind {
            let mut body_nodes = Vec::new();
            for &bid in body {
                body_nodes.extend(collect_subgraph(func, bid));
            }
            for &bid in &body_nodes {
                if let Some(bn) = func.arena.get(bid) {
                    if let NodeKind::Select { cond, true_val, false_val } = &bn.kind {
                        // Check if cond is an Eq and false_val is 0
                        if let Some(cn) = func.arena.get(*cond) {
                            if matches!(&cn.kind, NodeKind::Eq { .. }) {
                                if let Some(fn_) = func.arena.get(*false_val) {
                                    if let NodeKind::Constant(data) = &fn_.kind {
                                        if data.as_u64() == Some(0) {
                                            return true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

fn derive_cardinality_plan(func: &Function, truths: &[SemanticTruth]) -> Option<VectorPlan> {
    let pred_truth = truths.iter().find(|t| t.concept == SemanticConcept::PredicateMap)?;
    let buf_name = param_name(func, NodeId::new(0));
    let len_name = param_name(func, NodeId::new(1));
    let predicate = extract_predicate(func, &pred_truth.provenance);

    Some(VectorPlan {
        operation: VectorOp::Cardinality,
        buffer_name: buf_name,
        length_name: len_name,
        predicate,
        vector_width: 32,
        tail: TailPolicy::NarrowerVector,
    })
}

fn derive_sum_plan(func: &Function) -> Option<VectorPlan> {
    Some(VectorPlan {
        operation: VectorOp::Sum,
        buffer_name: param_name(func, NodeId::new(0)),
        length_name: param_name(func, NodeId::new(1)),
        predicate: Some(VectorPredicate::Identity),
        vector_width: 32,
        tail: TailPolicy::NarrowerVector,
    })
}

fn derive_all_plan(func: &Function, truths: &[SemanticTruth]) -> Option<VectorPlan> {
    let pred_truth = truths.iter().find(|t| t.concept == SemanticConcept::PredicateMap)?;
    let buf_name = param_name(func, NodeId::new(0));
    let len_name = param_name(func, NodeId::new(1));
    let predicate = extract_predicate(func, &pred_truth.provenance);

    Some(VectorPlan {
        operation: VectorOp::All,
        buffer_name: buf_name,
        length_name: len_name,
        predicate,
        vector_width: 32,
        tail: TailPolicy::NarrowerVector,
    })
}

fn extract_predicate(func: &Function, provenance: &Provenance) -> Option<VectorPredicate> {
    let nodes = match provenance {
        Provenance::Physical { nodes } => nodes,
        _ => return None,
    };

    for &nid in nodes {
        if let Some(node) = func.arena.get(nid) {
            match &node.kind {
                NodeKind::Eq { lhs, rhs } => {
                    // Check for masked equality: (x & mask) == target
                    if let Some(lhs_node) = func.arena.get(*lhs) {
                        if let NodeKind::And { lhs: and_lhs, rhs: and_rhs } = &lhs_node.kind {
                            let mask = resolve_value(func, *and_rhs);
                            let target = resolve_value(func, *rhs);
                            return Some(VectorPredicate::MaskedEqual { mask, target });
                        }
                    }
                    // Simple equality: x == val
                    let target = resolve_value(func, *rhs);
                    return Some(VectorPredicate::Equal(target));
                }
                _ => {}
            }
        }
    }
    None
}
