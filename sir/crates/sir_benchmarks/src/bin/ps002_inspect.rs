//! PS002 end-to-end inspection (advisor directive): what exactly does the
//! Any rewrite replace, and does the returned value still mean "last index"?
//!
//! Prints:
//! - source IR (loop body, live-outs, return)
//! - recognized roles (collection/accumulator/result/predicate)
//! - the candidate + definition that proved
//! - the final function IR after optimization

use sir_optimizer::config::OptimizerConfig;
use sir_optimizer::optimizer::Optimizer;
use sir_rewrite::registry::default_registry;

fn main() {
    for def in sir_benchmarks::positional_search::benchmarks() {
        if def.spec.id != "PS002" {
            continue;
        }
        let func = (def.func)();
        println!("=== SOURCE IR ===");
        let printer = sir_printer::text::TextPrinter::new(false);
        println!("{}", printer.function_to_string(&func));

        let optimizer = Optimizer::new(OptimizerConfig::default(), default_registry());
        let result = optimizer.optimize(&func);
        println!("=== REWRITES: {} ===", result.rewrites_applied);
        // Inspect the return chain: what value is returned now?
        let ret = result.function.return_node.expect("function has a return");
        if let Some(ret_node) = result.function.get_node(ret) {
            if let sir_nodes::NodeKind::Return { value } = &ret_node.kind {
                println!("RETURN operand: {}", value);
                if let Some(v) = result.function.get_node(*value) {
                    println!("  kind={:?} ty={:?}", v.kind, v.ty);
                }
            }
        }
        println!("=== FINAL IR ===");
        println!("{}", printer.function_to_string(&result.function));
    }
}
