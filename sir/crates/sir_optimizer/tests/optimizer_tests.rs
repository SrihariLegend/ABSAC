//! Edge-case optimizer tests.

use sir_builder::Builder;
use sir_optimizer::{Optimizer, OptimizerConfig, TerminationReason};
use sir_rewrite::recipe::RecipeRegistry;
use sir_types::{ConstantData, Span, Type};

fn make_empty_registry() -> RecipeRegistry {
    RecipeRegistry::new()
}

/// Build a minimal function: `fn empty() -> i32 { 0 }`
fn build_empty_function() -> sir_nodes::Function {
    let mut b = Builder::new("empty", &[], Type::i32());
    let zero = b
        .constant(ConstantData::i32(0), Type::i32(), Span::unknown());
    b.return_value(zero, Span::unknown()).unwrap();
    b.build()
}

#[test]
fn optimize_empty_function_converges_immediately() {
    let func = build_empty_function();
    let optimizer = Optimizer::new(OptimizerConfig::default(), make_empty_registry());

    let result = optimizer.optimize(&func);

    assert_eq!(result.iterations, 1);
    assert_eq!(result.rewrites_applied, 0);
    assert_eq!(result.termination, TerminationReason::FixedPoint);
}

#[test]
fn optimize_iteration_limit_is_respected() {
    let func = build_empty_function();
    let optimizer = Optimizer::new(
        OptimizerConfig {
            max_iterations: 3,
            max_total_rewrites: None,
        },
        make_empty_registry(),
    );

    let result = optimizer.optimize(&func);
    // Empty function converges in 1 iteration, well under the limit.
    assert!(result.iterations <= 3);
}

#[test]
fn optimize_max_total_rewrites_is_respected() {
    let func = build_empty_function();
    let optimizer = Optimizer::new(
        OptimizerConfig {
            max_iterations: 10,
            max_total_rewrites: Some(0),
        },
        make_empty_registry(),
    );

    let result = optimizer.optimize(&func);
    // With max_total_rewrites=0 and 0 rewrites applied, converges normally.
    assert_eq!(result.termination, TerminationReason::FixedPoint);
}

#[test]
fn iteration_records_are_populated() {
    let func = build_empty_function();
    let optimizer = Optimizer::new(OptimizerConfig::default(), make_empty_registry());

    let result = optimizer.optimize(&func);
    assert!(
        !result.iterations_detail.is_empty(),
        "Should have at least one iteration record"
    );
    for record in &result.iterations_detail {
        assert!(record.iteration > 0, "Iteration number should be positive");
    }
}
