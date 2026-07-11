//! Integration tests: builder → verify → analysis → assert facts.
//!
//! These tests exercise the full pipeline for each SIR milestone.

use crate::facts::EscapeKind;
use crate::manager::AnalysisManager;
use sir_builder::Builder;
use sir_types::{ConstantData, Span, Type};
use sir_verify::Verifier;

fn i32_type() -> Type {
    Type::i32()
}
fn u64_type() -> Type {
    Type::u64()
}
fn unknown_span() -> Span {
    Span::unknown()
}

// ── Milestone 1: Simple add function ───────────────────────

#[test]
fn milestone1_add_analysis() {
    let mut b = Builder::new("add", &[("a", i32_type()), ("b", i32_type())], i32_type());
    let a = b.parameter_index(0).unwrap();
    let b_param = b.parameter_index(1).unwrap();
    let sum = b.add(a, b_param, unknown_span()).unwrap();
    b.return_value(sum, unknown_span()).unwrap();
    let func = b.build();

    // Verify.
    let mut v = Verifier::new(&func);
    assert!(v.verify(), "{:?}", v.errors());

    // Analyze.
    let mut mgr = AnalysisManager::new();
    mgr.run_all(&func);
    let db = mgr.database();

    // UseDef: each parameter has a user (the add), sum has users (return).
    let a_def = db.use_def.get(&a).unwrap();
    assert_eq!(a_def.use_count, 1);
    assert!(!a_def.is_dead);

    let sum_def = db.use_def.get(&sum).unwrap();
    assert!(!sum_def.is_dead);
    assert_eq!(sum_def.use_count, 1);

    // Purity: all nodes are pure.
    for fact in db.purity.values() {
        assert!(fact.subgraph_is_pure);
    }

    // Constants: parameters are Top, no constants found.
    assert!(db.constants.get(&a).unwrap().value.is_top());
    assert!(db.constants.get(&b_param).unwrap().value.is_top());

    // Dominance: all nodes have dominators.
    assert!(!db.dominance.is_empty());
}

// ── Milestone 2: Select (branchless if) ───────────────────

#[test]
fn milestone2_select_analysis() {
    let mut b = Builder::new(
        "max",
        &[("cond", Type::Bool), ("x", i32_type()), ("y", i32_type())],
        i32_type(),
    );
    let cond = b.parameter_index(0).unwrap();
    let x = b.parameter_index(1).unwrap();
    let y = b.parameter_index(2).unwrap();
    let sel = b.select(cond, x, y, unknown_span()).unwrap();
    b.return_value(sel, unknown_span()).unwrap();
    let func = b.build();

    let mut v = Verifier::new(&func);
    assert!(v.verify(), "{:?}", v.errors());

    let mut mgr = AnalysisManager::new();
    mgr.run_all(&func);
    let db = mgr.database();

    // UseDef: select has 3 inputs (cond, x, y), 1 user (return).
    let sel_fact = db.use_def.get(&sel).unwrap();
    assert_eq!(sel_fact.definitions.len(), 3);

    // Purity: select is pure.
    assert!(db.purity.get(&sel).unwrap().subgraph_is_pure);
}

// ── Milestone 4: Memory operations ─────────────────────────

#[test]
fn milestone4_memory_analysis() {
    let mut b = Builder::new("mem", &[], i32_type());
    let count = b.constant(ConstantData::u64(1), u64_type(), unknown_span());
    let ptr = b.allocate(i32_type(), count, unknown_span()).unwrap();
    let val = b.constant(ConstantData::i32(42), i32_type(), unknown_span());
    b.store(ptr, val, unknown_span()).unwrap();
    let loaded = b.load(ptr, i32_type(), unknown_span()).unwrap();
    b.return_value(loaded, unknown_span()).unwrap();
    let func = b.build();

    let mut v = Verifier::new(&func);
    assert!(v.verify(), "{:?}", v.errors());

    let mut mgr = AnalysisManager::new();
    mgr.run_all(&func);
    let db = mgr.database();

    // Purity: load is ReadsMemory, not subgraph pure.
    assert!(!db.purity.get(&loaded).unwrap().subgraph_is_pure);

    // Alias: ptr has allocation site.
    assert!(db.aliases.get(&ptr).unwrap().allocation_site.is_some());

    // Escape: loaded escapes (returned).
    assert_eq!(db.escapes.get(&loaded).unwrap().kind, EscapeKind::Returned);
}

// ── Milestone 5: Intrinsic call ────────────────────────────

#[test]
fn milestone5_intrinsic_analysis() {
    let mut b = Builder::new("pop", &[("x", u64_type())], i32_type());
    let x = b.parameter_index(0).unwrap();
    let pop = b.popcount(x, unknown_span()).unwrap();
    b.return_value(pop, unknown_span()).unwrap();
    let func = b.build();

    let mut v = Verifier::new(&func);
    assert!(v.verify(), "{:?}", v.errors());

    let mut mgr = AnalysisManager::new();
    mgr.run_all(&func);
    let db = mgr.database();

    // UseDef: pop has 1 input (x), 1 user (return).
    let pop_fact = db.use_def.get(&pop).unwrap();
    assert_eq!(pop_fact.use_count, 1);

    // Range: popcount result is [0, 128].
    let pop_range = db.ranges.get(&pop).unwrap();
    assert_eq!(pop_range.lower, Some(0));
    assert_eq!(pop_range.upper, Some(64)); // u64 → max popcount is 64
}

// ── Constant folding ───────────────────────────────────────

#[test]
fn constant_folding_pipeline() {
    let mut b = Builder::new("fold", &[], i32_type());
    let c1 = b.constant(ConstantData::i32(10), i32_type(), unknown_span());
    let c2 = b.constant(ConstantData::i32(20), i32_type(), unknown_span());
    let sum = b.add(c1, c2, unknown_span()).unwrap();
    b.return_value(sum, unknown_span()).unwrap();
    let func = b.build();

    let mut mgr = AnalysisManager::new();
    mgr.run_all(&func);
    let db = mgr.database();

    assert!(db.constants.get(&sum).unwrap().value.is_constant());
}

// ── Diamond pattern ────────────────────────────────────────

#[test]
fn diamond_pattern_analysis() {
    // x -> s1 = x+x, s2 = x+x, s3 = s1+s2
    let mut b = Builder::new("diamond", &[("x", i32_type())], i32_type());
    let x = b.parameter_index(0).unwrap();
    let s1 = b.add(x, x, unknown_span()).unwrap();
    let s2 = b.add(x, x, unknown_span()).unwrap();
    let s3 = b.add(s1, s2, unknown_span()).unwrap();
    b.return_value(s3, unknown_span()).unwrap();
    let func = b.build();

    let mut mgr = AnalysisManager::new();
    mgr.run_all(&func);
    let db = mgr.database();

    // Value numbering: s1 and s2 are congruent (same op, same inputs).
    assert_eq!(
        db.value_numbers.get(&s1).unwrap().congruence_class,
        db.value_numbers.get(&s2).unwrap().congruence_class
    );

    // s3 dominates itself.
    assert!(db.dominance.get(&s3).unwrap().dominators.contains(&s3));
}

// ── Full pipeline stats ────────────────────────────────────

#[test]
fn full_pipeline_produces_stats() {
    let mut b = Builder::new(
        "compute",
        &[("a", i32_type()), ("b", i32_type())],
        i32_type(),
    );
    let a = b.parameter_index(0).unwrap();
    let b_param = b.parameter_index(1).unwrap();
    let s1 = b.add(a, b_param, unknown_span()).unwrap();
    let s2 = b.mul(s1, a, unknown_span()).unwrap();
    b.return_value(s2, unknown_span()).unwrap();
    let func = b.build();

    let mut mgr = AnalysisManager::new();
    mgr.run_all(&func);
    let stats = mgr.stats();

    assert!(stats.total_runs > 0);
    assert!(stats.cache_misses > 0);
    assert_eq!(stats.cache_hits, 0);

    // Second run: all cache hits.
    mgr.run_all(&func);
    let stats2 = mgr.stats();
    assert!(stats2.cache_hits > 0);
}

// ── Empty function ─────────────────────────────────────────

#[test]
fn empty_function_analysis_does_not_crash() {
    let func = sir_nodes::Function::new("empty", Type::Unit);
    let mut mgr = AnalysisManager::new();
    mgr.run_all(&func);
    let db = mgr.database();

    // All fact stores should be empty.
    assert!(db.is_empty());
}

// ── Mixed bitwise and boolean ──────────────────────────────

#[test]
fn bitwise_boolean_pipeline() {
    let mut b = Builder::new("bools", &[("x", u64_type()), ("y", u64_type())], Type::Bool);
    let x = b.parameter_index(0).unwrap();
    let y = b.parameter_index(1).unwrap();
    let xor_val = b.bit_xor(x, y, unknown_span()).unwrap();
    let zero = b.constant(ConstantData::u64(0), u64_type(), unknown_span());
    let eq = b.eq(xor_val, zero, unknown_span()).unwrap();
    b.return_value(eq, unknown_span()).unwrap();
    let func = b.build();

    let mut v = Verifier::new(&func);
    assert!(v.verify(), "{:?}", v.errors());

    let mut mgr = AnalysisManager::new();
    mgr.run_all(&func);
    let db = mgr.database();

    // Eq result range is [0, 1].
    assert!(db.ranges.get(&eq).unwrap().lower.is_some());
    // All pure.
    assert!(db.purity.get(&eq).unwrap().subgraph_is_pure);
}
