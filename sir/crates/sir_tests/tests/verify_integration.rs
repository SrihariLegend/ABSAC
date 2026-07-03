//! Integration tests for the Verifier with builder-constructed functions.

use sir_builder::Builder;
use sir_verify::Verifier;
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

// ── Valid functions pass verification ──────────────────────

#[test]
fn simple_add_passes_verification() {
    let mut b = Builder::new("add", &[("a", i32_type()), ("b", i32_type())], i32_type());
    let a = b.parameter_index(0).unwrap();
    let b_param = b.parameter_index(1).unwrap();
    let sum = b.add(a, b_param, unknown_span()).unwrap();
    b.return_value(sum, unknown_span()).unwrap();
    let func = b.build();

    let mut v = Verifier::new(&func);
    assert!(v.verify(), "expected valid function: {:?}", v.errors());
}

#[test]
fn bitwise_function_passes_verification() {
    let mut b = Builder::new("xor_eq", &[("a", u64_type()), ("b", u64_type())], Type::Bool);
    let a = b.parameter_index(0).unwrap();
    let b_param = b.parameter_index(1).unwrap();
    let xor_val = b.bit_xor(a, b_param, unknown_span()).unwrap();
    let zero = b.constant(ConstantData::u64(0), u64_type(), unknown_span());
    let cmp = b.eq(xor_val, zero, unknown_span()).unwrap();
    b.return_value(cmp, unknown_span()).unwrap();
    let func = b.build();

    let mut v = Verifier::new(&func);
    assert!(v.verify(), "expected valid function: {:?}", v.errors());
}

#[test]
fn select_function_passes_verification() {
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
    assert!(v.verify(), "expected valid function: {:?}", v.errors());
}

#[test]
fn memory_function_passes_verification() {
    let mut b = Builder::new("mem", &[], i32_type());
    let count = b.constant(ConstantData::u64(16), u64_type(), unknown_span());
    let ptr = b.allocate(i32_type(), count, unknown_span()).unwrap();
    let val = b.constant(ConstantData::i32(7), i32_type(), unknown_span());
    b.store(ptr, val, unknown_span()).unwrap();
    let loaded = b.load(ptr, i32_type(), unknown_span()).unwrap();
    b.return_value(loaded, unknown_span()).unwrap();
    let func = b.build();

    let mut v = Verifier::new(&func);
    assert!(v.verify(), "expected valid function: {:?}", v.errors());
}

// ── Invalid functions fail correctly ───────────────────────

#[test]
fn missing_return_fails() {
    let func = sir_nodes::Function::new("empty", Type::Unit);
    let mut v = Verifier::new(&func);
    assert!(!v.verify());
    assert_eq!(v.errors().len(), 1);
    assert!(matches!(
        v.errors()[0],
        sir_verify::VerificationError::MissingReturn
    ));
}

#[test]
fn return_type_mismatch_detected() {
    let mut b = Builder::new("bad_ret", &[("x", i32_type())], Type::Bool);
    let x = b.parameter_index(0).unwrap();
    b.return_value(x, unknown_span()).unwrap(); // returns i32, expected Bool
    let func = b.build();
    let mut v = Verifier::new(&func);
    assert!(!v.verify());
}
