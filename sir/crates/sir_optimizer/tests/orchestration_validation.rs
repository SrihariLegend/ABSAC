use sir_builder::Builder;
use sir_optimizer::{Optimizer, OptimizerConfig};
use sir_rewrite::registry::default_registry;
use sir_types::{ConstantData, Span, Type};

/// Build a function with four independent optimization opportunities:
/// 1. Count reduction
/// 2. Any reduction
/// 3. Parity reduction
/// 4. Modulo power of two
///
/// It returns just one of the results (Modulo) to keep it simple, but the arena contains all 4.
fn build_orchestration_function() -> sir_nodes::Function {
    let mut b = Builder::new(
        "orchestration_test",
        &[
            (
                "board",
                Type::Array {
                    element: Box::new(Type::Bool),
                    length: 64,
                },
            ),
            ("x", Type::i32()),
        ],
        Type::i32(),
    );

    let board = b.parameter_index(0).unwrap();
    let x = b.parameter_index(1).unwrap();

    let i_initial = b.constant(ConstantData::u64(0), Type::u64(), Span::unknown());
    let i_step = b.constant(ConstantData::u64(1), Type::u64(), Span::unknown());
    let limit = b.constant(ConstantData::u64(64), Type::u64(), Span::unknown());

    // 1. Count Loop
    let count_initial = b.constant(ConstantData::i32(0), Type::i32(), Span::unknown());
    let c_elem = b.array_access(board, i_initial, Type::Bool, Span::unknown()).unwrap();
    let c_zero = b.constant(ConstantData::i32(0), Type::i32(), Span::unknown());
    let c_one = b.constant(ConstantData::i32(1), Type::i32(), Span::unknown());
    let c_inc = b.select(c_elem, c_one, c_zero, Span::unknown()).unwrap();
    let c_next = b.add(count_initial, c_inc, Span::unknown()).unwrap();
    let c_next_i = b.add(i_initial, i_step, Span::unknown()).unwrap();
    let c_cond = b.lt(i_initial, limit, Span::unknown()).unwrap();
    let _count_loop = b.r#loop(
        &[c_elem, c_zero, c_one, c_inc, c_next, c_next_i, c_cond],
        c_cond,
        &[c_next, c_next_i],
        &[count_initial, i_initial],
        Type::Tuple { elements: vec![Type::i32(), Type::u64()] },
        Span::unknown(),
    ).unwrap();

    // 2. Any Loop
    let any_initial = b.constant(ConstantData::Bool(false), Type::Bool, Span::unknown());
    let a_elem = b.array_access(board, i_initial, Type::Bool, Span::unknown()).unwrap();
    let a_next = b.bool_or(any_initial, a_elem, Span::unknown()).unwrap();
    let a_next_i = b.add(i_initial, i_step, Span::unknown()).unwrap();
    let a_cond = b.lt(i_initial, limit, Span::unknown()).unwrap();
    let _any_loop = b.r#loop(
        &[a_elem, a_next, a_next_i, a_cond],
        a_cond,
        &[a_next, a_next_i],
        &[any_initial, i_initial],
        Type::Tuple { elements: vec![Type::Bool, Type::u64()] },
        Span::unknown(),
    ).unwrap();

    // 3. Parity Loop
    let par_initial = b.constant(ConstantData::Bool(false), Type::Bool, Span::unknown());
    let p_elem = b.array_access(board, i_initial, Type::Bool, Span::unknown()).unwrap();
    let p_next = b.ne(par_initial, p_elem, Span::unknown()).unwrap();
    let p_next_i = b.add(i_initial, i_step, Span::unknown()).unwrap();
    let p_cond = b.lt(i_initial, limit, Span::unknown()).unwrap();
    let _par_loop = b.r#loop(
        &[p_elem, p_next, p_next_i, p_cond],
        p_cond,
        &[p_next, p_next_i],
        &[par_initial, i_initial],
        Type::Tuple { elements: vec![Type::Bool, Type::u64()] },
        Span::unknown(),
    ).unwrap();

    // 4. Modulo Power of Two
    let divisor = b.constant(ConstantData::i32(16), Type::i32(), Span::unknown());
    let mod_res = b.rem(x, divisor, Span::unknown()).unwrap();

    // Use an external call to keep all independent loops alive (prevents DCE)
    let keep_alive = b.external_call(
        "keep_alive",
        &[_count_loop, _any_loop, _par_loop, mod_res],
        Type::i32(),
        sir_types::Effects::IO,
        Span::unknown(),
    ).unwrap();

    b.return_value(keep_alive, Span::unknown()).unwrap();
    let built_func = b.build();
    
    println!("COUNT LOOP EFFECTS: {:?}", built_func.get_node(_count_loop).unwrap().effects);

    built_func
}

#[test]
fn test_orchestration_multiple_rewrites() {
    let func = build_orchestration_function();
    let optimizer = Optimizer::new(OptimizerConfig::default(), default_registry());
    
    let result = optimizer.optimize(&func);

    println!("\n=== Orchestration: Multiple Rewrites ===\n");
    println!("Total Rewrites Applied: {}", result.rewrites_applied);
    for (i, rec) in result.iterations_detail.iter().enumerate() {
        println!("Iteration {}: Found {} candidate(s), {} proven, selected {}, rewrote {}",
            i + 1,
            rec.candidates_generated,
            rec.proofs_succeeded,
            rec.candidates_selected,
            rec.rewrites_applied
        );
        // Print the current arena to see what's happening
        println!("Arena length after iteration: {}", result.function.arena.len());
    }

    // We expect exactly 4 rewrites because there are 4 independent non-overlapping optimizable regions.
    // Iteration 1 should rewrite the highest priority one.
    // Iteration 2 should rewrite the next highest.
    // Iteration 3 ...
    // Iteration 4 ...
    // Iteration 5 should find no remaining candidates and converge.
    assert_eq!(result.rewrites_applied, 4, "Expected exactly 4 rewrites to be applied");
}
