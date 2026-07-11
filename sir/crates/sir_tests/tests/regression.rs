//! Regression and edge-case tests.

use sir_builder::Builder;
use sir_printer::JsonPrinter;
use sir_types::{ConstantData, Effects, Span, Type};
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

// ── Edge cases ─────────────────────────────────────────────

#[test]
fn empty_function_with_no_params() {
    // A function that returns a constant — minimal valid function.
    let mut b = Builder::new("answer", &[], i32_type());
    let c = b.constant(ConstantData::i32(42), i32_type(), unknown_span());
    b.return_value(c, unknown_span()).unwrap();
    let func = b.build();
    assert_eq!(func.params.len(), 0);
    assert_eq!(func.node_count(), 2); // constant + return
    let mut v = Verifier::new(&func);
    assert!(v.verify(), "{:?}", v.errors());
}

#[test]
fn function_with_many_parameters() {
    let names: Vec<String> = (0..16).map(|i| format!("p{i}")).collect();
    let params: Vec<(&str, Type)> = names.iter().map(|n| (n.as_str(), i32_type())).collect();
    let mut b = Builder::new("many_params", &params, i32_type());
    let first = b.parameter_index(0).unwrap();
    b.return_value(first, unknown_span()).unwrap();
    let func = b.build();
    assert_eq!(func.params.len(), 16);
    let mut v = Verifier::new(&func);
    assert!(v.verify(), "{:?}", v.errors());
}

#[test]
fn nested_selects() {
    // A ? (B ? X : Y) : Z  — nested select for chained conditions.
    let mut b = Builder::new(
        "nested_select",
        &[
            ("a", Type::Bool),
            ("b", Type::Bool),
            ("x", i32_type()),
            ("y", i32_type()),
            ("z", i32_type()),
        ],
        i32_type(),
    );
    let a = b.parameter_index(0).unwrap();
    let cond_b = b.parameter_index(1).unwrap();
    let x = b.parameter_index(2).unwrap();
    let y = b.parameter_index(3).unwrap();
    let z = b.parameter_index(4).unwrap();

    let inner = b.select(cond_b, x, y, unknown_span()).unwrap();
    let outer = b.select(a, inner, z, unknown_span()).unwrap();
    b.return_value(outer, unknown_span()).unwrap();
    let func = b.build();

    let mut v = Verifier::new(&func);
    assert!(v.verify(), "{:?}", v.errors());
}

#[test]
fn all_comparison_operators() {
    let mut b = Builder::new(
        "cmp_all",
        &[("a", i32_type()), ("b", i32_type())],
        Type::Bool,
    );
    let a = b.parameter_index(0).unwrap();
    let b_param = b.parameter_index(1).unwrap();

    // Build a chain: (a == b) && (a < b) && (a <= b) && (a > b) && (a >= b) && (a != b)
    // This exercises all comparison and boolean operators.
    let eq = b.eq(a, b_param, unknown_span()).unwrap();
    let lt = b.lt(a, b_param, unknown_span()).unwrap();
    let le = b.le(a, b_param, unknown_span()).unwrap();
    let gt = b.gt(a, b_param, unknown_span()).unwrap();
    let ge = b.ge(a, b_param, unknown_span()).unwrap();
    let ne = b.ne(a, b_param, unknown_span()).unwrap();

    let r1 = b.bool_and(eq, lt, unknown_span()).unwrap();
    let r2 = b.bool_and(le, gt, unknown_span()).unwrap();
    let r3 = b.bool_and(ge, ne, unknown_span()).unwrap();
    let r4 = b.bool_and(r1, r2, unknown_span()).unwrap();
    let r5 = b.bool_and(r4, r3, unknown_span()).unwrap();

    b.return_value(r5, unknown_span()).unwrap();
    let func = b.build();

    let mut v = Verifier::new(&func);
    assert!(v.verify(), "{:?}", v.errors());
}

#[test]
fn json_roundtrip_preserves_effects() {
    let mut b = Builder::new("with_effects", &[], i32_type());
    let count = b.constant(ConstantData::u64(1), u64_type(), unknown_span());
    let ptr = b.allocate(i32_type(), count, unknown_span()).unwrap();
    let val = b.constant(ConstantData::i32(10), i32_type(), unknown_span());
    b.store(ptr, val, unknown_span()).unwrap();
    let loaded = b.load(ptr, i32_type(), unknown_span()).unwrap();
    b.return_value(loaded, unknown_span()).unwrap();
    let func = b.build();

    // Verify effects exist before serialization.
    for node in &func.arena {
        if matches!(node.kind, sir_nodes::NodeKind::Load { .. }) {
            assert!(node.effects.contains(Effects::READ_MEMORY));
        }
        if matches!(node.kind, sir_nodes::NodeKind::Store { .. }) {
            assert!(node.effects.contains(Effects::WRITE_MEMORY));
        }
    }

    // Roundtrip.
    let json = JsonPrinter::function_to_string(&func).unwrap();
    let parsed = JsonPrinter::function_from_str(&json).unwrap();

    // Verify effects preserved.
    for node in &parsed.arena {
        if matches!(node.kind, sir_nodes::NodeKind::Load { .. }) {
            assert!(node.effects.contains(Effects::READ_MEMORY));
        }
        if matches!(node.kind, sir_nodes::NodeKind::Store { .. }) {
            assert!(node.effects.contains(Effects::WRITE_MEMORY));
        }
    }
}

#[test]
fn constants_of_all_widths() {
    let types_to_test = vec![
        Type::i8(),
        Type::i16(),
        Type::i32(),
        Type::i64(),
        Type::u8(),
        Type::u16(),
        Type::u32(),
        Type::u64(),
        Type::f32(),
        Type::f64(),
        Type::Bool,
        Type::Unit,
    ];

    for ty in &types_to_test {
        let func = {
            let mut b = Builder::new("const_func", &[], ty.clone());
            let c = match ty {
                t if t.is_integer() => b.constant(ConstantData::i32(0), t.clone(), unknown_span()),
                t if t.is_float() => b.constant(ConstantData::f64(0.0), t.clone(), unknown_span()),
                t if t.is_bool() => {
                    b.constant(ConstantData::boolean(false), t.clone(), unknown_span())
                }
                _ => b.constant(ConstantData::Unit, ty.clone(), unknown_span()),
            };
            b.return_value(c, unknown_span()).unwrap();
            b.build()
        };

        let mut v = Verifier::new(&func);
        assert!(v.verify(), "failed for type {ty:?}: {:?}", v.errors());

        let json = JsonPrinter::function_to_string(&func).unwrap();
        let parsed = JsonPrinter::function_from_str(&json).unwrap();
        assert_eq!(func.name, parsed.name);
    }
}
