//! emit_c_ll — lowers a function from a .ll file via the auto-lowerer,
//! runs ABSAC, and emits rewritten C to stdout.
//!
//! Usage: cargo run -p sir_benchmarks --bin emit_c_ll -- <file.ll> <function_name>

use sir_lower::lower_function;
use sir_optimizer::{Optimizer, OptimizerConfig};
use sir_rewrite::registry::default_registry;
use sir_benchmarks::emit;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: emit_c_ll <file.ll> <function_name>");
        std::process::exit(1);
    }
    let ll_path = &args[1];
    let func_name = &args[2];

    let ll_text = match std::fs::read_to_string(ll_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error reading {}: {}", ll_path, e);
            std::process::exit(1);
        }
    };

    // Lower from LLVM IR to SIR
    let func = match lower_function(&ll_text, func_name) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("lower error: {}", e);
            std::process::exit(1);
        }
    };

    // Run ABSAC optimization (suppress debug output)
    let result = {
        let dev_null = std::fs::OpenOptions::new()
            .write(true).open("/dev/null").unwrap();
        let null_fd = std::os::fd::AsRawFd::as_raw_fd(&dev_null);
        let saved_fd = unsafe { libc::dup(1) };
        unsafe { libc::dup2(null_fd, 1); }

        let config = OptimizerConfig::default();
        let registry = default_registry();
        let optimizer = Optimizer::new(config, registry);
        let result = optimizer.optimize(&func);

        unsafe {
            libc::dup2(saved_fd, 1);
            libc::close(saved_fd);
        }
        drop(dev_null);
        result
    };

    // Emit the rewritten function as C
    let c_source = emit::emit_c(&result.function);
    print!("{}", c_source);
}
