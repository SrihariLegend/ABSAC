use sir_builder::Builder;
use sir_optimizer::{Optimizer, OptimizerConfig};
use sir_rewrite::registry::default_registry;
use sir_types::{ConstantData, Effects, Span, Type};

fn build_non_boolean_array() -> sir_nodes::Function {
    let mut b = Builder::new(
        "non_boolean_count",
        &[(
            "board",
            Type::Array {
                element: Box::new(Type::u8()),
                length: 64,
            },
        )],
        Type::i32(),
    );

    let board = b.parameter_index(0).unwrap();
    let i_initial = b.constant(ConstantData::u64(0), Type::u64(), Span::unknown());
    let i_step = b.constant(ConstantData::u64(1), Type::u64(), Span::unknown());
    let limit = b.constant(ConstantData::u64(64), Type::u64(), Span::unknown());
    let accum_initial = b.constant(ConstantData::i32(0), Type::i32(), Span::unknown());

    let elem = b.array_access(board, i_initial, Type::u8(), Span::unknown()).unwrap();
    let zero_u8 = b.constant(ConstantData::u8(0), Type::u8(), Span::unknown());
    let cond = b.eq(elem, zero_u8, Span::unknown()).unwrap(); // true if 0

    let zero_i32 = b.constant(ConstantData::i32(0), Type::i32(), Span::unknown());
    let one_i32 = b.constant(ConstantData::i32(1), Type::i32(), Span::unknown());
    let inc = b.select(cond, one_i32, zero_i32, Span::unknown()).unwrap();
    let next_accum = b.add(accum_initial, inc, Span::unknown()).unwrap();

    let next_i = b.add(i_initial, i_step, Span::unknown()).unwrap();
    let loop_cond = b.lt(i_initial, limit, Span::unknown()).unwrap();

    let loop_node = b.r#loop(
        &[elem, zero_u8, cond, zero_i32, one_i32, inc, next_accum, next_i, loop_cond],
        loop_cond,
        &[next_accum, next_i],
        &[accum_initial, i_initial],
        Type::Tuple { elements: vec![Type::i32(), Type::u64()] },
        Span::unknown(),
    ).unwrap();

    b.return_value(loop_node, Span::unknown()).unwrap();
    b.build()
}

fn build_side_effect_interrupt() -> sir_nodes::Function {
    let mut b = Builder::new(
        "side_effect_count",
        &[(
            "board",
            Type::Array {
                element: Box::new(Type::Bool),
                length: 64,
            },
        )],
        Type::i32(),
    );

    let board = b.parameter_index(0).unwrap();
    let i_initial = b.constant(ConstantData::u64(0), Type::u64(), Span::unknown());
    let i_step = b.constant(ConstantData::u64(1), Type::u64(), Span::unknown());
    let limit = b.constant(ConstantData::u64(64), Type::u64(), Span::unknown());
    let accum_initial = b.constant(ConstantData::i32(0), Type::i32(), Span::unknown());

    let elem = b.array_access(board, i_initial, Type::Bool, Span::unknown()).unwrap();
    let zero = b.constant(ConstantData::i32(0), Type::i32(), Span::unknown());
    let one = b.constant(ConstantData::i32(1), Type::i32(), Span::unknown());
    let inc = b.select(elem, one, zero, Span::unknown()).unwrap();
    let next_accum = b.add(accum_initial, inc, Span::unknown()).unwrap();

    // Observable side effect
    let call_node = b.external_call(
        "observe", 
        &[next_accum], 
        Type::Unit, 
        Effects::IO, 
        Span::unknown()
    ).unwrap();

    let next_i = b.add(i_initial, i_step, Span::unknown()).unwrap();
    let loop_cond = b.lt(i_initial, limit, Span::unknown()).unwrap();

    let loop_node = b.r#loop(
        &[elem, zero, one, inc, next_accum, call_node, next_i, loop_cond],
        loop_cond,
        &[next_accum, next_i],
        &[accum_initial, i_initial],
        Type::Tuple { elements: vec![Type::i32(), Type::u64()] },
        Span::unknown(),
    ).unwrap();

    b.return_value(loop_node, Span::unknown()).unwrap();
    b.build()
}

fn build_existing_implementation() -> sir_nodes::Function {
    let mut b = Builder::new(
        "existing_popcount",
        &[(
            "board",
            Type::Array {
                element: Box::new(Type::Bool),
                length: 64,
            },
        )],
        Type::i32(),
    );

    let board = b.parameter_index(0).unwrap();
    let packed = b.pack(board, Span::unknown()).unwrap(); // u64
    let pop = b.popcount(packed, Span::unknown()).unwrap(); // i32
    
    b.return_value(pop, Span::unknown()).unwrap();
    b.build()
}

fn build_mixed_reductions() -> sir_nodes::Function {
    let mut b = Builder::new(
        "mixed_reductions",
        &[(
            "board",
            Type::Array {
                element: Box::new(Type::Bool),
                length: 64,
            },
        )],
        Type::Tuple { elements: vec![Type::i32(), Type::Bool] },
    );

    let board = b.parameter_index(0).unwrap();
    let i_initial = b.constant(ConstantData::u64(0), Type::u64(), Span::unknown());
    let i_step = b.constant(ConstantData::u64(1), Type::u64(), Span::unknown());
    let limit = b.constant(ConstantData::u64(64), Type::u64(), Span::unknown());
    
    let count_initial = b.constant(ConstantData::i32(0), Type::i32(), Span::unknown());
    let any_initial = b.constant(ConstantData::Bool(false), Type::Bool, Span::unknown());

    let elem = b.array_access(board, i_initial, Type::Bool, Span::unknown()).unwrap();
    
    // Count logic
    let zero = b.constant(ConstantData::i32(0), Type::i32(), Span::unknown());
    let one = b.constant(ConstantData::i32(1), Type::i32(), Span::unknown());
    let inc = b.select(elem, one, zero, Span::unknown()).unwrap();
    let next_count = b.add(count_initial, inc, Span::unknown()).unwrap();

    // Any logic
    let next_any = b.bool_or(any_initial, elem, Span::unknown()).unwrap();

    let next_i = b.add(i_initial, i_step, Span::unknown()).unwrap();
    let loop_cond = b.lt(i_initial, limit, Span::unknown()).unwrap();

    let loop_node = b.r#loop(
        &[elem, zero, one, inc, next_count, next_any, next_i, loop_cond],
        loop_cond,
        &[next_count, next_any, next_i],
        &[count_initial, any_initial, i_initial],
        Type::Tuple { elements: vec![Type::i32(), Type::Bool, Type::u64()] },
        Span::unknown(),
    ).unwrap();

    b.return_value(loop_node, Span::unknown()).unwrap();
    b.build()
}

#[test]
fn validate_boolean_boundaries() {
    let benchmarks = vec![
        ("BS005_NonBoolean", build_non_boolean_array(), false),
        ("BS006_InterruptingSideEffects", build_side_effect_interrupt(), false),
        ("BS007_ExistingImpl", build_existing_implementation(), false),
        ("BS008_MixedReductions", build_mixed_reductions(), false),
    ];

    println!("\n=== Boolean Reduction Boundary Validation ===\n");
    println!("| Benchmark | Rewritten | Expected? |");
    println!("| --------- | --------- | --------- |");

    let mut all_passed = true;

    for (name, func, expect_rewrite) in benchmarks {
        let optimizer = Optimizer::new(OptimizerConfig::default(), default_registry());
        let result = optimizer.optimize(&func);

        let rewritten = result.rewrites_applied > 0;
        let passed = rewritten == expect_rewrite;

        println!("| {:<20} | {:<9} | {:<9} |", 
            name.split('_').next().unwrap(),
            if rewritten { "Yes" } else { "No" },
            if passed { "✓" } else { "✗" }
        );

        if !passed {
            all_passed = false;
            println!("  => FAILED: Expected rewrite={}, but got rewrite={}", expect_rewrite, rewritten);
        }
    }

    assert!(all_passed, "Boundary conditions failed.");
}
