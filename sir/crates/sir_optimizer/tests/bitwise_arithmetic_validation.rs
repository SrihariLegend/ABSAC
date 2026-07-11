use sir_builder::Builder;
use sir_nodes::NodeKind;
use sir_optimizer::config::OptimizerConfig;
use sir_optimizer::optimizer::Optimizer;
use sir_rewrite::registry::default_registry;
use sir_types::{ConstantData, Span, Type};

fn i32_type() -> Type {
    Type::i32()
}

fn unknown_span() -> Span {
    Span::unknown()
}

fn create_ba001_modulo() -> sir_nodes::Function {
    let mut b = Builder::new("ba001_modulo", &[("x", i32_type())], i32_type());
    let x = b.parameter_index(0).unwrap();
    let c = b.constant(ConstantData::i32(16), i32_type(), unknown_span());
    let res = b.rem(x, c, unknown_span()).unwrap();
    b.return_value(res, unknown_span()).unwrap();
    b.build()
}

fn create_ba002_divide() -> sir_nodes::Function {
    let mut b = Builder::new("ba002_divide", &[("x", i32_type())], i32_type());
    let x = b.parameter_index(0).unwrap();
    let c = b.constant(ConstantData::i32(8), i32_type(), unknown_span());
    let res = b.div(x, c, unknown_span()).unwrap();
    b.return_value(res, unknown_span()).unwrap();
    b.build()
}

fn create_ba003_multiply() -> sir_nodes::Function {
    let mut b = Builder::new("ba003_multiply", &[("x", i32_type())], i32_type());
    let x = b.parameter_index(0).unwrap();
    let c = b.constant(ConstantData::i32(32), i32_type(), unknown_span());
    let res = b.mul(x, c, unknown_span()).unwrap();
    b.return_value(res, unknown_span()).unwrap();
    b.build()
}

fn create_ba004_shift_mask() -> sir_nodes::Function {
    let mut b = Builder::new("ba004_shift_mask", &[("x", i32_type())], i32_type());
    let x = b.parameter_index(0).unwrap();
    let c = b.constant(ConstantData::i32(4), i32_type(), unknown_span());
    let shl = b.shl(x, c, unknown_span()).unwrap();
    let res = b.shr(shl, c, unknown_span()).unwrap();
    b.return_value(res, unknown_span()).unwrap();
    b.build()
}

#[test]
fn validate_ba001_modulo() {
    let func = create_ba001_modulo();
    let mut optimizer = Optimizer::new(OptimizerConfig::default(), default_registry());
    let result = optimizer.optimize(&func);

    assert_eq!(result.rewrites_applied, 1);

    let has_and = result
        .function
        .arena
        .iter()
        .any(|n| matches!(n.kind, NodeKind::And { .. }));
    assert!(has_and, "Should have been rewritten to a bitwise AND");
}

#[test]
fn validate_ba002_divide() {
    let func = create_ba002_divide();
    let mut optimizer = Optimizer::new(OptimizerConfig::default(), default_registry());
    let result = optimizer.optimize(&func);

    assert_eq!(result.rewrites_applied, 1);

    let has_shr = result
        .function
        .arena
        .iter()
        .any(|n| matches!(n.kind, NodeKind::Shr { .. }));
    assert!(has_shr, "Should have been rewritten to a shift right");
}

#[test]
fn validate_ba003_multiply() {
    let func = create_ba003_multiply();
    let mut optimizer = Optimizer::new(OptimizerConfig::default(), default_registry());
    let result = optimizer.optimize(&func);

    assert_eq!(result.rewrites_applied, 1);

    let has_shl = result
        .function
        .arena
        .iter()
        .any(|n| matches!(n.kind, NodeKind::Shl { .. }));
    assert!(has_shl, "Should have been rewritten to a shift left");
}

#[test]
fn validate_ba004_shift_mask() {
    let func = create_ba004_shift_mask();
    let mut optimizer = Optimizer::new(OptimizerConfig::default(), default_registry());
    let result = optimizer.optimize(&func);

    assert_eq!(result.rewrites_applied, 1);

    let has_and = result
        .function
        .arena
        .iter()
        .any(|n| matches!(n.kind, NodeKind::And { .. }));
    assert!(has_and, "Should have been rewritten to a bitwise AND");
}
