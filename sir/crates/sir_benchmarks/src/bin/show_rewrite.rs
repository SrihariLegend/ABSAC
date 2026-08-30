use sir_builder::Builder;
use sir_optimizer::{Optimizer, OptimizerConfig};
use sir_printer::text::TextPrinter;
use sir_rewrite::registry::default_registry;
use sir_types::{ConstantData, Span, Type};

fn span() -> Span { Span::unknown() }

fn main() {
    let mut b = Builder::new(
        "redis_bitcount_tail",
        &[
            ("buf", Type::Array { element: Box::new(Type::u8()), length: 64 }),
            ("count", Type::u64()),
            ("bitsinbyte", Type::Array { element: Box::new(Type::u64()), length: 256 }),
        ],
        Type::u64(),
    );
    let buf = b.parameter_index(0).unwrap();
    let count = b.parameter_index(1).unwrap();
    let table = b.parameter_index(2).unwrap();
    let one = b.constant(ConstantData::u64(1), Type::u64(), span());
    let i_init = b.constant(ConstantData::u64(0), Type::u64(), span());
    let bits_init = b.constant(ConstantData::u64(0), Type::u64(), span());
    let byte = b.array_access(buf, i_init, Type::u8(), span()).unwrap();
    let to_add = b.array_access(table, byte, Type::u64(), span()).unwrap();
    let next_bits = b.add(bits_init, to_add, span()).unwrap();
    let next_i = b.add(i_init, one, span()).unwrap();
    let cond = b.lt(next_i, count, span()).unwrap();
    let loop_node = b.r#loop(
        &[byte, to_add, next_bits, next_i, cond],
        cond, &[next_i, next_bits], &[i_init, bits_init],
        Type::Tuple { elements: vec![Type::u64(), Type::u64()] }, span(),
    ).unwrap();
    let bits_result = b.tuple_extract(loop_node, 1, Type::u64(), span()).unwrap();
    b.return_value(bits_result, span()).unwrap();
    let func = b.build();

    let printer = TextPrinter::new(false);
    println!("=== BEFORE (real Redis BITCOUNT tail loop) ===");
    println!("{}", printer.function_to_string(&func));

    let config = OptimizerConfig::default();
    let registry = default_registry();
    let optimizer = Optimizer::new(config, registry);
    let result = optimizer.optimize(&func);
    let rewritten = &result.function;

    println!("\n=== AFTER (table lookup replaced with Popcount) ===");
    println!("{}", printer.function_to_string(rewritten));
    println!("\nRewrites applied: {}", result.rewrites_applied);
    println!("Initial nodes: {} -> Final nodes: {}", result.initial_nodes, result.final_nodes);
}
