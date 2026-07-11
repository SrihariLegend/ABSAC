use sir_builder::Builder;
use sir_optimizer::{Optimizer, OptimizerConfig};
use sir_rewrite::registry::default_registry;
use sir_types::{ConstantData, Span, Type};

fn build_modulo_power_of_two() -> sir_nodes::Function {
    let mut b = Builder::new("mod_pow_2", &[("x", Type::i32())], Type::i32());

    let x = b.parameter_index(0).unwrap();
    let divisor = b.constant(ConstantData::i32(8), Type::i32(), Span::unknown());

    let result = b.rem(x, divisor, Span::unknown()).unwrap();
    b.return_value(result, Span::unknown()).unwrap();

    b.build()
}

#[test]
fn validate_arithmetic_identities() {
    let benchmarks = vec![("ModuloPow2", build_modulo_power_of_two(), true)];

    let mut all_passed = true;

    println!("\n=== Arithmetic Identities Validation ===\n");

    for (name, func, expect_rewrite) in benchmarks {
        let optimizer = Optimizer::new(OptimizerConfig::default(), default_registry());
        let result = optimizer.optimize(&func);

        let rewritten = result.rewrites_applied > 0;
        let passed = rewritten == expect_rewrite;

        println!(
            "| {:<20} | {:<9} | {:<9} |",
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
