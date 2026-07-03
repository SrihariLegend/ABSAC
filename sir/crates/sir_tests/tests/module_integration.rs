//! Integration tests for Module-level operations.

use sir_builder::Builder;
use sir_nodes::Module;
use sir_printer::JsonPrinter;
use sir_types::{Span, Type};

fn i32_type() -> Type {
    Type::i32()
}

fn unknown_span() -> Span {
    Span::unknown()
}

fn build_add() -> sir_nodes::Function {
    let mut b = Builder::new("add", &[("a", i32_type()), ("b", i32_type())], i32_type());
    let a = b.parameter_index(0).unwrap();
    let b_param = b.parameter_index(1).unwrap();
    let sum = b.add(a, b_param, unknown_span()).unwrap();
    b.return_value(sum, unknown_span()).unwrap();
    b.build()
}

fn build_sub() -> sir_nodes::Function {
    let mut b = Builder::new("sub", &[("a", i32_type()), ("b", i32_type())], i32_type());
    let a = b.parameter_index(0).unwrap();
    let b_param = b.parameter_index(1).unwrap();
    let diff = b.sub(a, b_param, unknown_span()).unwrap();
    b.return_value(diff, unknown_span()).unwrap();
    b.build()
}

fn build_mul() -> sir_nodes::Function {
    let mut b = Builder::new("mul", &[("a", i32_type()), ("b", i32_type())], i32_type());
    let a = b.parameter_index(0).unwrap();
    let b_param = b.parameter_index(1).unwrap();
    let prod = b.mul(a, b_param, unknown_span()).unwrap();
    b.return_value(prod, unknown_span()).unwrap();
    b.build()
}

#[test]
fn module_with_multiple_functions() {
    let mut m = Module::new("arithmetic");
    m.add_function(build_add());
    m.add_function(build_sub());
    m.add_function(build_mul());

    assert_eq!(m.function_count(), 3);
    assert!(m.get_function("add").is_some());
    assert!(m.get_function("sub").is_some());
    assert!(m.get_function("mul").is_some());
    assert!(m.get_function("div").is_none());
}

#[test]
fn module_json_roundtrip() {
    let mut m = Module::new("math");
    m.add_function(build_add());
    m.add_function(build_sub());

    let json = JsonPrinter::module_to_string(&m).unwrap();
    let parsed = JsonPrinter::module_from_str(&json).unwrap();

    assert_eq!(m.name, parsed.name);
    assert_eq!(m.function_count(), parsed.function_count());
    assert!(parsed.get_function("add").is_some());
    assert!(parsed.get_function("sub").is_some());
}
