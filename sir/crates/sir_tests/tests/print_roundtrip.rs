//! Integration tests for pretty-printing and JSON roundtrip.

use sir_builder::Builder;
use sir_printer::{JsonPrinter, TextPrinter};
use sir_types::{ConstantData, Span, Type};

fn i32_type() -> Type {
    Type::i32()
}

fn unknown_span() -> Span {
    Span::unknown()
}

fn build_add_function() -> sir_nodes::Function {
    let mut b = Builder::new("add", &[("a", i32_type()), ("b", i32_type())], i32_type());
    let a = b.parameter_index(0).unwrap();
    let b_param = b.parameter_index(1).unwrap();
    let sum = b.add(a, b_param, unknown_span()).unwrap();
    b.return_value(sum, unknown_span()).unwrap();
    b.build()
}

fn build_select_function() -> sir_nodes::Function {
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
    b.build()
}

// ── Text printer (compact) ─────────────────────────────────

#[test]
fn compact_add_function_format() {
    let func = build_add_function();
    let printer = TextPrinter::new(true);
    let output = printer.function_to_string(&func);
    assert!(output.contains("Function add"));
    assert!(output.contains("Parameter a"));
    assert!(output.contains("Parameter b"));
    assert!(output.contains("Add"));
    assert!(output.contains("Return"));
}

#[test]
fn compact_select_function_format() {
    let func = build_select_function();
    let printer = TextPrinter::new(true);
    let output = printer.function_to_string(&func);
    assert!(output.contains("Function max"));
    assert!(output.contains("Select"));
}

// ── Text printer (detailed) ────────────────────────────────

#[test]
fn detailed_add_function_format() {
    let func = build_add_function();
    let printer = TextPrinter::new(false);
    let output = printer.function_to_string(&func);
    assert!(output.contains("Function add (params:"));
    assert!(output.contains("returns:"));
    assert!(output.contains("Add"));
    assert!(output.contains("Return"));
    assert!(output.contains("%"));
}

// ── JSON roundtrip ─────────────────────────────────────────

#[test]
fn json_roundtrip_add_function() {
    let func = build_add_function();
    let json = JsonPrinter::function_to_string(&func).unwrap();
    let parsed = JsonPrinter::function_from_str(&json).unwrap();
    assert_eq!(func.name, parsed.name);
    assert_eq!(func.params.len(), parsed.params.len());
    assert_eq!(func.return_ty, parsed.return_ty);
    assert_eq!(func.node_count(), parsed.node_count());
}

#[test]
fn json_roundtrip_select_function() {
    let func = build_select_function();
    let json = JsonPrinter::function_to_string(&func).unwrap();
    let parsed = JsonPrinter::function_from_str(&json).unwrap();
    assert_eq!(func.name, parsed.name);
    assert_eq!(func.params.len(), parsed.params.len());
    assert_eq!(func.node_count(), parsed.node_count());
}

#[test]
fn json_roundtrip_module() {
    let mut module = sir_nodes::Module::new("test_mod");
    module.add_function(build_add_function());
    module.add_function(build_select_function());

    let json = JsonPrinter::module_to_string(&module).unwrap();
    let parsed = JsonPrinter::module_from_str(&json).unwrap();
    assert_eq!(module.name, parsed.name);
    assert_eq!(module.function_count(), parsed.function_count());
}

// ── Node-level printing ─────────────────────────────────────

#[test]
fn print_constant_node() {
    let printer = TextPrinter::new(false);
    let node = sir_nodes::Node::new(
        sir_types::NodeId::new(0),
        sir_nodes::NodeKind::Constant(ConstantData::i32(42)),
        i32_type(),
        sir_types::Effects::empty(),
        unknown_span(),
    );
    let output = printer.node_to_string(&node);
    assert!(output.contains("Constant"));
    assert!(output.contains("i32"));
}
