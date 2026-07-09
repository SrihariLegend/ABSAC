use sir_builder::Builder;
use sir_optimizer::{Optimizer, OptimizerConfig};
use sir_rewrite::registry::default_registry;
use sir_types::{ConstantData, Span, Type};

fn build_predicate_reduction() -> sir_nodes::Function {
    let mut b = Builder::new(
        "predicate_reduction",
        &[(
            "array",
            Type::Array {
                element: Box::new(Type::i32()),
                length: 64,
            },
        )],
        Type::i32(),
    );

    let array = b.parameter_index(0).unwrap();
    let i_initial = b.constant(ConstantData::u64(0), Type::u64(), Span::unknown());
    let i_step = b.constant(ConstantData::u64(1), Type::u64(), Span::unknown());
    let limit = b.constant(ConstantData::u64(64), Type::u64(), Span::unknown());
    
    let count_initial = b.constant(ConstantData::i32(0), Type::i32(), Span::unknown());

    let elem = b.array_access(array, i_initial, Type::i32(), Span::unknown()).unwrap();
    let threshold = b.constant(ConstantData::i32(10), Type::i32(), Span::unknown());
    let cond = b.gt(elem, threshold, Span::unknown()).unwrap();

    let count_zero = b.constant(ConstantData::i32(0), Type::i32(), Span::unknown());
    let count_one = b.constant(ConstantData::i32(1), Type::i32(), Span::unknown());
    let inc = b.select(cond, count_one, count_zero, Span::unknown()).unwrap();
    let next_count = b.add(count_initial, inc, Span::unknown()).unwrap();

    let next_i = b.add(i_initial, i_step, Span::unknown()).unwrap();
    let loop_cond = b.lt(i_initial, limit, Span::unknown()).unwrap();

    let loop_node = b.r#loop(
        &[elem, threshold, cond, count_zero, count_one, inc, next_count, next_i, loop_cond],
        loop_cond,
        &[next_count, next_i],
        &[count_initial, i_initial],
        Type::Tuple { elements: vec![Type::i32(), Type::u64()] },
        Span::unknown(),
    ).unwrap();

    b.return_value(loop_node, Span::unknown()).unwrap();
    b.build()
}

#[test]
fn validate_predicate_reductions() {
    let benchmarks = vec![
        ("PredicateCount", build_predicate_reduction()),
    ];

    let mut all_passed = true;

    println!("\n=== Predicate Reduction Validation ===\n");
    println!("| Benchmark | Recognized | Proven | Selected | Rewritten | Expected Output? |");
    println!("| --------- | ---------- | ------ | -------- | --------- | ---------------- |");

    for (name, func) in benchmarks {
        let optimizer = Optimizer::new(OptimizerConfig::default(), default_registry());
        let result = optimizer.optimize(&func);

        let rec = result.iterations_detail.get(0);
        println!("ITERATION RECORD: {:?}", rec);
        let recognized = rec.map(|r| r.truths_discovered > 0).unwrap_or(false);
        let proven = rec.map(|r| r.proofs_succeeded > 0).unwrap_or(false);
        let selected = rec.map(|r| r.candidates_selected > 0).unwrap_or(false);
        let rewritten = result.rewrites_applied > 0;

        let has_loop = result.function.arena.iter().any(|n| matches!(n.kind, sir_nodes::NodeKind::Loop { .. }));
        let expected_output = rewritten && !has_loop;

        if !expected_output {
            all_passed = false;
        }

        println!("| {:<13} | {:<10} | {:<6} | {:<8} | {:<9} | {:<16} |", 
            name,
            if recognized { "✓" } else { "✗" },
            if proven { "✓" } else { "✗" },
            if selected { "✓" } else { "✗" },
            if rewritten { "✓" } else { "✗" },
            if expected_output { "✓" } else { "✗" }
        );
        
        // Print the rewritten SIR for visual confirmation
        if rewritten {
            println!("\n--- {} Rewritten SIR ---", name);
            for node in result.function.arena.iter() {
                println!("{:?}", node.kind);
            }
            println!("----------------------------\n");
        }
    }

    assert!(all_passed, "Predicate reduction validation must complete the pipeline successfully.");
}
