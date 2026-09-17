use sir_builder::Builder;
use sir_optimizer::{Optimizer, OptimizerConfig};
use sir_rewrite::registry::default_registry;
use sir_types::{ConstantData, Span, Type};

/// Unsigned modulo by power of two → AND: legal (x % 8 == x & 7 for
/// every unsigned x) and verified. Expect a rewrite.
fn build_modulo_power_of_two_unsigned() -> sir_nodes::Function {
    let mut b = Builder::new("mod_pow_2_u", &[("x", Type::u32())], Type::u32());

    let x = b.parameter_index(0).unwrap();
    let divisor = b.constant(ConstantData::u32(8), Type::u32(), Span::unknown());

    let result = b.rem(x, divisor, Span::unknown()).unwrap();
    b.return_value(result, Span::unknown()).unwrap();

    b.build()
}

/// Signed modulo: the arithmetic is defined, but the mask rewrite is
/// NOT equivalent for negative operands (-1 % 8 = -1, -1 & 7 = 7) —
/// the recognizer must refuse the concept and the region must abstain.
fn build_modulo_power_of_two_signed() -> sir_nodes::Function {
    let mut b = Builder::new("mod_pow_2_signed", &[("x", Type::i32())], Type::i32());

    let x = b.parameter_index(0).unwrap();
    let divisor = b.constant(ConstantData::i32(8), Type::i32(), Span::unknown());

    let result = b.rem(x, divisor, Span::unknown()).unwrap();
    b.return_value(result, Span::unknown()).unwrap();

    b.build()
}

/// sdiv by -1: INT_MIN / -1 overflows the result type (trap in C,
/// poison in LLVM). No DefinednessCertificate exists, so the region
/// must generate zero candidates and apply zero rewrites.
fn build_signed_div_minus_one() -> sir_nodes::Function {
    let mut b = Builder::new("sdiv_minus_one", &[("x", Type::i32())], Type::i32());

    let x = b.parameter_index(0).unwrap();
    let minus_one = b.constant(ConstantData::i32(-1), Type::i32(), Span::unknown());

    let result = b.div(x, minus_one, Span::unknown()).unwrap();
    b.return_value(result, Span::unknown()).unwrap();

    b.build()
}

#[test]
fn validate_arithmetic_identities() {
    // ModuloAnd is ConcreteSolverChecked since 2026-09-17: the unsigned
    // x % 8 rewrite is authorized (the obligation binds the actual
    // divisor/operand/width). Signed modulo and signed division still
    // abstain.
    let benchmarks = vec![
        ("ModuloPow2_unsigned", build_modulo_power_of_two_unsigned(), true),
        // Signed modulo: defined arithmetic, but the mask rewrite is
        // NOT equivalent for negative operands (-1 % 8 = -1 vs
        // -1 & 7 = 7) — must abstain.
        ("ModuloSigned_abstains", build_modulo_power_of_two_signed(), false),
        // sdiv INT_MIN / -1 trap case: signed div must never be
        // authorized for transformation.
        ("SignedDiv_abstains", build_signed_div_minus_one(), false),
    ];

    let mut all_passed = true;

    println!("\n=== Arithmetic Identities Validation ===\n");

    for (name, func, expect_rewrite) in benchmarks {
        let optimizer = Optimizer::new(OptimizerConfig::default(), default_registry());
        let result = optimizer.optimize(&func);

        let rewritten = result.rewrites_applied > 0;
        let passed = rewritten == expect_rewrite;

        println!(
            "| {:<22} | {:<9} | {:<9} |",
            name,
            if rewritten { "Yes" } else { "No" },
            if passed { "✓" } else { "✗" }
        );

        if !passed {
            all_passed = false;
        }
    }

    assert!(all_passed, "Arithmetic identity validation failed.");
}
