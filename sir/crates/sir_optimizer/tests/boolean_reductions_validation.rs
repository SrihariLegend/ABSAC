use sir_builder::Builder;
use sir_optimizer::{Optimizer, OptimizerConfig};
use sir_rewrite::registry::default_registry;
use sir_types::{ConstantData, Span, Type};

fn build_benchmark(
    name: &str,
    is_count: bool,
    is_all: bool,
    is_parity: bool,
) -> sir_nodes::Function {
    let return_type = if is_count { Type::i32() } else { Type::Bool };
    let mut b = Builder::new(
        name,
        &[(
            "board",
            Type::Array {
                element: Box::new(Type::Bool),
                length: 64,
            },
        )],
        return_type.clone(),
    );

    let board = b.parameter_index(0).unwrap();
    let i_initial = b.constant(ConstantData::u64(0), Type::u64(), Span::unknown());
    let i_step = b.constant(ConstantData::u64(1), Type::u64(), Span::unknown());
    let limit = b.constant(ConstantData::u64(64), Type::u64(), Span::unknown());

    let accum_initial = if is_count {
        b.constant(ConstantData::i32(0), Type::i32(), Span::unknown())
    } else {
        // any/parity starts false, all starts true
        b.constant(ConstantData::Bool(is_all), Type::Bool, Span::unknown())
    };

    let elem = b
        .array_access(board, i_initial, Type::Bool, Span::unknown())
        .unwrap();

    let mut body = vec![elem];
    let next_accum = if is_count {
        let zero = b.constant(ConstantData::i32(0), Type::i32(), Span::unknown());
        let one = b.constant(ConstantData::i32(1), Type::i32(), Span::unknown());
        let inc = b.select(elem, one, zero, Span::unknown()).unwrap();
        body.push(zero);
        body.push(one);
        body.push(inc);
        b.add(accum_initial, inc, Span::unknown()).unwrap()
    } else if is_parity {
        b.ne(accum_initial, elem, Span::unknown()).unwrap()
    } else if is_all {
        b.bool_and(accum_initial, elem, Span::unknown()).unwrap()
    } else {
        b.bool_or(accum_initial, elem, Span::unknown()).unwrap()
    };
    body.push(next_accum);

    let next_i = b.add(i_initial, i_step, Span::unknown()).unwrap();
    body.push(next_i);
    let cond = b.lt(i_initial, limit, Span::unknown()).unwrap(); // Wait, should it be next_i or i_initial? In BS001 it was `i_initial` for `cond` because loop terminates when `cond` becomes false. No, it terminates when `cond` evaluates false at start of iteration. If we check `i_initial < 64` then it loops.
    body.push(cond);

    let loop_node = b
        .r#loop(
            &body,
            cond,
            &[next_accum, next_i],
            &[accum_initial, i_initial],
            Type::Tuple {
                elements: vec![return_type, Type::u64()],
            },
            Span::unknown(),
        )
        .unwrap();

    b.return_value(loop_node, Span::unknown()).unwrap();
    b.build()
}

#[test]
fn validate_boolean_reductions() {
    let benchmarks = vec![
        ("BS001_Count", build_benchmark("count", true, false, false)),
        ("BS002_Any", build_benchmark("any", false, false, false)),
        ("BS003_All", build_benchmark("all", false, true, false)),
        (
            "BS004_Parity",
            build_benchmark("parity", false, false, true),
        ),
    ];

    let mut all_passed = true;

    println!("\n=== Boolean Reduction Family Validation ===\n");
    println!("| Benchmark | Recognized | Proven | Selected | Rewritten | Expected Output? |");
    println!("| --------- | ---------- | ------ | -------- | --------- | ---------------- |");

    for (name, func) in benchmarks {
        let optimizer = Optimizer::new(OptimizerConfig::default(), default_registry());
        let result = optimizer.optimize(&func);

        let rec = result.iterations_detail.get(0);
        let recognized = rec.map(|r| r.truths_discovered > 0).unwrap_or(false);
        let proven = rec.map(|r| r.proofs_succeeded > 0).unwrap_or(false);
        let selected = rec.map(|r| r.candidates_selected > 0).unwrap_or(false);
        let rewritten = result.rewrites_applied > 0;

        // If not rewritten, maybe it failed in RewriteEngine. Let's see if we can manually test rewrite or print why it failed.
        // Wait, optimizer swallows the error. Let's look at the result.
        if !rewritten {
            println!("Rewriting failed for {}", name);
        }

        let has_loop = result
            .function
            .arena
            .iter()
            .any(|n| matches!(n.kind, sir_nodes::NodeKind::Loop { .. }));
        let expected_output = rewritten && !has_loop;

        if !expected_output {
            all_passed = false;
        }

        println!(
            "| {:<9} | {:<10} | {:<6} | {:<8} | {:<9} | {:<16} |",
            name.split('_').next().unwrap(),
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

    assert!(
        all_passed,
        "All benchmarks must complete the pipeline successfully."
    );
}
