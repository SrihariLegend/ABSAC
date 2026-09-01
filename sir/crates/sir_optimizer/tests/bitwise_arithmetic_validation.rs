use sir_builder::Builder;
use sir_nodes::NodeKind;
use sir_optimizer::config::OptimizerConfig;
use sir_optimizer::optimizer::Optimizer;
use sir_rewrite::registry::default_registry;
use sir_types::{ConstantData, Span, Type};

// BA001/BA002/BA004 use UNSIGNED operands: the modulo→mask,
// divide→shift and shift-mask identities are only equivalent for
// unsigned (or proven non-negative) values — see the advisor's
// signed div/rem audit. BA003 (multiply) keeps i32: two's-complement
// wrapping makes x * 2^k == x << k for every bit pattern.
fn u32_type() -> Type {
    Type::u32()
}

fn i32_type() -> Type {
    Type::i32()
}

fn unknown_span() -> Span {
    Span::unknown()
}

fn create_ba001_modulo() -> sir_nodes::Function {
    let mut b = Builder::new("ba001_modulo", &[("x", u32_type())], u32_type());
    let x = b.parameter_index(0).unwrap();
    let c = b.constant(ConstantData::u32(16), u32_type(), unknown_span());
    let res = b.rem(x, c, unknown_span()).unwrap();
    b.return_value(res, unknown_span()).unwrap();
    b.build()
}

fn create_ba002_divide() -> sir_nodes::Function {
    let mut b = Builder::new("ba002_divide", &[("x", u32_type())], u32_type());
    let x = b.parameter_index(0).unwrap();
    let c = b.constant(ConstantData::u32(8), u32_type(), unknown_span());
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
    let mut b = Builder::new("ba004_shift_mask", &[("x", u32_type())], u32_type());
    let x = b.parameter_index(0).unwrap();
    let c = b.constant(ConstantData::u32(4), u32_type(), unknown_span());
    let shl = b.shl(x, c, unknown_span()).unwrap();
    let res = b.shr(shl, c, unknown_span()).unwrap();
    b.return_value(res, unknown_span()).unwrap();
    b.build()
}

// ADVISOR P0 VERIFIER QUARANTINE: all four bitwise-arithmetic
// definitions (ModuloAnd, DivideShift, MultiplyShift, ShiftMask) have
// Stub obligations — hardcoded theorems that never bind the actual
// source/candidate operands. None may authorize a rewrite until the
// obligation is built from the actual nodes and discharged concretely.
// These tests now assert the quarantine; the identities themselves
// remain true (for unsigned operands) and are re-enabled by upgrading
// the definitions to ConcreteSolverChecked.

#[test]
fn validate_ba001_modulo() {
    let func = create_ba001_modulo();
    let mut optimizer = Optimizer::new(OptimizerConfig::default(), default_registry());
    let result = optimizer.optimize(&func);

    assert_eq!(result.rewrites_applied, 0, "ModuloAnd is Stub-quarantined");

    let has_and = result
        .function
        .arena
        .iter()
        .any(|n| matches!(n.kind, NodeKind::And { .. }));
    assert!(!has_and, "Must not emit AND while quarantined");
}

#[test]
fn validate_ba002_divide() {
    let func = create_ba002_divide();
    let mut optimizer = Optimizer::new(OptimizerConfig::default(), default_registry());
    let result = optimizer.optimize(&func);

    // Stub-quarantined (advisor P0 verifier audit).
    assert_eq!(result.rewrites_applied, 0, "DivideShift is Stub-quarantined");

    let has_shr = result
        .function
        .arena
        .iter()
        .any(|n| matches!(n.kind, NodeKind::Shr { .. }));
    assert!(!has_shr, "Must not rewrite to shift while quarantined");
}

#[test]
fn validate_ba003_multiply() {
    let func = create_ba003_multiply();
    let mut optimizer = Optimizer::new(OptimizerConfig::default(), default_registry());
    let result = optimizer.optimize(&func);

    // MultiplyShiftDefinition is a trivially-equal stub
    // (Constant(0)==Constant(0)) — quarantined (advisor P0 audit).
    assert_eq!(result.rewrites_applied, 0, "MultiplyShift is Stub-quarantined");

    let has_shl = result
        .function
        .arena
        .iter()
        .any(|n| matches!(n.kind, NodeKind::Shl { .. }));
    assert!(!has_shl, "Must not rewrite to shift while quarantined");
}

#[test]
fn validate_ba004_shift_mask() {
    let func = create_ba004_shift_mask();
    let mut optimizer = Optimizer::new(OptimizerConfig::default(), default_registry());
    let result = optimizer.optimize(&func);

    // ShiftMask definition is Stub-quarantined (advisor P0 audit).
    assert_eq!(result.rewrites_applied, 0);

    let has_and = result
        .function
        .arena
        .iter()
        .any(|n| matches!(n.kind, NodeKind::And { .. }));
    assert!(!has_and, "Must not rewrite to a bitwise AND while quarantined");
}
