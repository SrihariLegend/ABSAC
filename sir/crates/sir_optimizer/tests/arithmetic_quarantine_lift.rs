//! MultiplyShift quarantine lift, end to end through the optimizer.
//!
//! The verifier now proves `x * C == x << log2(C)` from the actual
//! constant/width with the concrete solver, so AR003 rewrites. These
//! tests pin the rewrite, the commuted-operand recipe fix (`32 * x`
//! must shift x, not the constant), and the still-quarantined
//! neighbours.

use sir_builder::Builder;
use sir_optimizer::{Optimizer, OptimizerConfig};
use sir_rewrite::registry::default_registry;
use sir_types::{ConstantData, NodeId, Span, Type};

fn span() -> Span {
    Span::unknown()
}

fn multiply_function(name: &str, lhs_is_constant: bool) -> sir_nodes::Function {
    let mut b = Builder::new(name, &[("x", Type::u32())], Type::u32());
    let x = b.parameter_index(0).unwrap();
    let c = b.constant(ConstantData::u32(32), Type::u32(), span());
    let mul = if lhs_is_constant {
        b.mul(c, x, span()).unwrap()
    } else {
        b.mul(x, c, span()).unwrap()
    };
    b.return_value(mul, span()).unwrap();
    b.build()
}

fn signed_multiply_function(name: &str, lhs_is_constant: bool) -> sir_nodes::Function {
    let mut b = Builder::new(name, &[("x", Type::i32())], Type::i32());
    let x = b.parameter_index(0).unwrap();
    let c = b.constant(ConstantData::i32(32), Type::i32(), span());
    let mul = if lhs_is_constant {
        b.mul(c, x, span()).unwrap()
    } else {
        b.mul(x, c, span()).unwrap()
    };
    b.return_value(mul, span()).unwrap();
    b.build()
}

fn optimize(func: &sir_nodes::Function) -> sir_optimizer::OptimizationResult {
    Optimizer::new(OptimizerConfig::default(), default_registry()).optimize(func)
}

fn shift_lhs(result: &sir_optimizer::OptimizationResult) -> Option<NodeId> {
    result.function.arena.iter().find_map(|node| {
        if let sir_nodes::NodeKind::Shl { lhs, .. } = &node.kind {
            Some(*lhs)
        } else {
            None
        }
    })
}

#[test]
fn multiply_by_power_of_two_rewrites_through_the_solver() {
    let result = optimize(&multiply_function("mul_x_c", false));
    assert_eq!(
        result.rewrites_applied, 1,
        "x * 32 must be authorized by the concrete-solver proof and rewritten"
    );
    assert_eq!(
        shift_lhs(&result),
        Some(NodeId::new(0)),
        "the shifted operand must be x (the parameter), not the constant"
    );
}

#[test]
fn commuted_multiply_shifts_the_dynamic_operand() {
    // `32 * x`: a recipe that assumed the RHS is the constant would emit
    // `32 << tzcnt(x)`. The dynamic operand must be shifted in both
    // orders.
    let result = optimize(&multiply_function("mul_c_x", true));
    assert_eq!(result.rewrites_applied, 1);
    assert_eq!(
        shift_lhs(&result),
        Some(NodeId::new(0)),
        "commuted multiply must still shift x, never the constant"
    );
}

#[test]
fn non_power_of_two_multiply_is_not_rewritten() {
    let mut b = Builder::new("mul_12", &[("x", Type::u32())], Type::u32());
    let x = b.parameter_index(0).unwrap();
    let c = b.constant(ConstantData::u32(12), Type::u32(), span());
    let mul = b.mul(x, c, span()).unwrap();
    b.return_value(mul, span()).unwrap();
    let func = b.build();
    let result = optimize(&func);
    assert_eq!(
        result.rewrites_applied, 0,
        "x * 12 has no shift identity and must not rewrite"
    );
}

#[test]
fn signed_multiply_by_power_of_two_rewrites() {
    let result = optimize(&signed_multiply_function("smul_x_c", false));
    assert_eq!(
        result.rewrites_applied, 1,
        "signed x * 32 is bit-pattern identical to x << 5 and the solver proves it"
    );
    assert_eq!(shift_lhs(&result), Some(NodeId::new(0)));
}

#[test]
fn signed_commuted_multiply_shifts_the_dynamic_operand() {
    let result = optimize(&signed_multiply_function("smul_c_x", true));
    assert_eq!(result.rewrites_applied, 1);
    assert_eq!(shift_lhs(&result), Some(NodeId::new(0)));
}

#[test]
fn divide_power_of_two_stays_quarantined() {
    // DivideShift (id 101) is still a Stub: the boundary must hold.
    let mut b = Builder::new("div_16", &[("x", Type::u32())], Type::u32());
    let x = b.parameter_index(0).unwrap();
    let c = b.constant(ConstantData::u32(16), Type::u32(), span());
    let div = b.div(x, c, span()).unwrap();
    b.return_value(div, span()).unwrap();
    let func = b.build();
    let result = optimize(&func);
    assert_eq!(
        result.rewrites_applied, 0,
        "DivideShift is still Stub-quarantined; it must not rewrite"
    );
}

/// Recorded blocker (2026-09-17): `CircularPermutation` regions infer the
/// BitPermutation representation but produce ZERO candidates today —
/// for constant AND variable amounts. Rotate definitions are therefore
/// held Stub even though their bindings are concrete. When the
/// generation/authorization gap is fixed, replace this with a rewrite +
/// native-execution test.
#[test]
fn circular_permutation_generates_no_candidates_today() {
    let mut b = Builder::new("rotl3", &[("x", Type::u32())], Type::u32());
    let x = b.parameter_index(0).unwrap();
    let three = b.constant(ConstantData::u32(3), Type::u32(), span());
    let width = b.constant(ConstantData::u32(32), Type::u32(), span());
    let diff = b.sub(width, three, span()).unwrap();
    let shl = b.shl(x, three, span()).unwrap();
    let shr = b.shr(x, diff, span()).unwrap();
    let res = b.bit_or(shl, shr, span()).unwrap();
    b.return_value(res, span()).unwrap();
    let func = b.build();
    let result = optimize(&func);
    assert_eq!(
        result.iterations_detail[0].candidates_generated, 0,
        "constant-amount rotation candidate generation is the recorded blocker"
    );
    assert_eq!(result.rewrites_applied, 0, "held Stub definitions authorize nothing");

    // Variable amount (HD003 shape): same zero-candidate outcome.
    let mut b = Builder::new("rotl_k", &[("x", Type::u32()), ("k", Type::u32())], Type::u32());
    let x = b.parameter_index(0).unwrap();
    let k = b.parameter_index(1).unwrap();
    let width = b.constant(ConstantData::u32(32), Type::u32(), span());
    let diff = b.sub(width, k, span()).unwrap();
    let shl = b.shl(x, k, span()).unwrap();
    let shr = b.shr(x, diff, span()).unwrap();
    let res = b.bit_or(shl, shr, span()).unwrap();
    b.return_value(res, span()).unwrap();
    let result = optimize(&b.build());
    assert_eq!(result.iterations_detail[0].candidates_generated, 0);
    assert_eq!(result.rewrites_applied, 0);
}
