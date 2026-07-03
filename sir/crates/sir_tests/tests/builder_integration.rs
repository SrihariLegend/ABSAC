//! Integration tests for the Builder API.
//!
//! These tests exercise the full builder pipeline: construction,
//! type checking, effect computation, and output.

use sir_builder::{BuildError, Builder};
use sir_types::{ConstantData, Span, Type};

fn i32_type() -> Type {
    Type::i32()
}

fn u64_type() -> Type {
    Type::u64()
}

fn unknown_span() -> Span {
    Span::unknown()
}

// ── Milestone 1: Simple arithmetic functions ────────────────

#[test]
fn milestone1_simple_add() {
    let mut b = Builder::new("add", &[("a", i32_type()), ("b", i32_type())], i32_type());
    let a = b.parameter_index(0).unwrap();
    let b_param = b.parameter_index(1).unwrap();
    let sum = b.add(a, b_param, unknown_span()).unwrap();
    b.return_value(sum, unknown_span()).unwrap();
    let func = b.build();

    assert_eq!(func.name, "add");
    assert_eq!(func.params.len(), 2);
    assert_eq!(func.return_ty, i32_type());
    assert!(func.return_node.is_some());
    assert!(func.node_count() >= 4); // 2 params + add + return
}

#[test]
fn milestone1_complex_arithmetic() {
    let mut b = Builder::new("compute", &[("x", i32_type()), ("y", i32_type())], i32_type());
    let x = b.parameter_index(0).unwrap();
    let y = b.parameter_index(1).unwrap();
    let sum = b.add(x, y, unknown_span()).unwrap();
    let diff = b.sub(sum, x, unknown_span()).unwrap();
    let product = b.mul(diff, y, unknown_span()).unwrap();
    let result = b.div(product, x, unknown_span()).unwrap();
    b.return_value(result, unknown_span()).unwrap();
    let func = b.build();
    assert!(func.node_count() > 0);
}

#[test]
fn milestone1_bitwise_operations() {
    let mut b = Builder::new("bitops", &[("a", u64_type()), ("b", u64_type())], u64_type());
    let a = b.parameter_index(0).unwrap();
    let b_param = b.parameter_index(1).unwrap();
    let and_val = b.bit_and(a, b_param, unknown_span()).unwrap();
    let or_val = b.bit_or(and_val, a, unknown_span()).unwrap();
    let xor_val = b.bit_xor(or_val, b_param, unknown_span()).unwrap();
    let not_val = b.bit_not(xor_val, unknown_span()).unwrap();
    b.return_value(not_val, unknown_span()).unwrap();
    let func = b.build();
    assert!(func.node_count() > 0);
}

// ── Milestone 2: if → Select ───────────────────────────────

#[test]
fn milestone2_select_branchless() {
    let mut b = Builder::new(
        "branchless",
        &[("cond", Type::Bool), ("t", i32_type()), ("f", i32_type())],
        i32_type(),
    );
    let cond = b.parameter_index(0).unwrap();
    let true_val = b.parameter_index(1).unwrap();
    let false_val = b.parameter_index(2).unwrap();
    let sel = b.select(cond, true_val, false_val, unknown_span()).unwrap();
    b.return_value(sel, unknown_span()).unwrap();
    let func = b.build();
    assert!(func.node_count() > 0);
}

// ── Milestone 4: Memory operations ─────────────────────────

#[test]
fn milestone4_memory_operations() {
    let mut b = Builder::new("alloc_and_use", &[], i32_type());
    let count = b.constant(ConstantData::u64(1), u64_type(), unknown_span());
    // Allocate space for one i32.
    let ptr = b.allocate(i32_type(), count, unknown_span()).unwrap();
    // Store a value.
    let val = b.constant(ConstantData::i32(42), i32_type(), unknown_span());
    b.store(ptr, val, unknown_span()).unwrap();
    // Load it back.
    let loaded = b.load(ptr, i32_type(), unknown_span()).unwrap();
    // Deallocate.
    b.deallocate(ptr, unknown_span()).unwrap();
    // Return the loaded value.
    b.return_value(loaded, unknown_span()).unwrap();
    let func = b.build();
    assert!(func.node_count() > 0);
}

// ── Milestone 5: Function calls ─────────────────────────────

#[test]
fn milestone5_intrinsic_call() {
    let mut b = Builder::new("use_intrinsic", &[("x", u64_type())], u64_type());
    let x = b.parameter_index(0).unwrap();
    let pop = b.intrinsic(
        "ctpop",
        &[x],
        u64_type(),
        sir_types::Effects::empty(),
        unknown_span(),
    )
    .unwrap();
    b.return_value(pop, unknown_span()).unwrap();
    let func = b.build();
    assert!(func.node_count() > 0);
}

// ── Error handling ─────────────────────────────────────────

#[test]
fn type_mismatch_across_operations() {
    let mut b = Builder::new("bad", &[("x", i32_type()), ("y", Type::f64())], i32_type());
    let x = b.parameter_index(0).unwrap();
    let y = b.parameter_index(1).unwrap();
    // i32 + f64 → error
    assert!(matches!(b.add(x, y, unknown_span()), Err(BuildError::TypeMismatch { .. })));
}

#[test]
fn select_with_non_bool_condition() {
    let mut b = Builder::new("bad_select", &[("x", i32_type())], i32_type());
    let x = b.parameter_index(0).unwrap();
    assert!(b.select(x, x, x, unknown_span()).is_err());
}

#[test]
fn bitwise_on_float() {
    let mut b = Builder::new("bad_bitwise", &[("x", Type::f64())], Type::f64());
    let x = b.parameter_index(0).unwrap();
    assert!(b.bit_and(x, x, unknown_span()).is_err());
}

#[test]
fn load_from_non_pointer() {
    let mut b = Builder::new("bad_load", &[("x", i32_type())], i32_type());
    let x = b.parameter_index(0).unwrap();
    assert!(b.load(x, i32_type(), unknown_span()).is_err());
}
