//! emit_c — emits compilable C from ABSAC's rewritten SIR.
//!
//! Usage: cargo run -p sir_benchmarks --bin emit_c -- <corpus_id>
//!
//! Looks up the corpus entry by ID, builds the corresponding SIR function,
//! runs the ABSAC optimizer, and emits the rewritten function as C to stdout.
//! This is the Rust side of the perft benchmark harness.

use sir_benchmarks::real_kernels;
use sir_optimizer::{Optimizer, OptimizerConfig};
use sir_rewrite::registry::default_registry;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: emit_c <corpus_id>");
        std::process::exit(1);
    }
    let corpus_id = &args[1];

    // Find the kernel by corpus ID.
    let func = match real_kernels::build_by_id(corpus_id) {
        Some(f) => f,
        None => {
            eprintln!("emit_c: unknown corpus id '{}'", corpus_id);
            std::process::exit(1);
        }
    };

    // Run ABSAC optimization. The optimizer prints debug info to stdout,
    // so we redirect stdout to /dev/null during optimization, then restore
    // it to emit only the C source.
    let result = {
        let dev_null = std::fs::OpenOptions::new()
            .write(true)
            .open("/dev/null")
            .unwrap();
        let null_fd = std::os::fd::AsRawFd::as_raw_fd(&dev_null);
        let saved_fd = unsafe { libc::dup(1) };
        unsafe { libc::dup2(null_fd, 1); }

        let config = OptimizerConfig::default();
        let registry = default_registry();
        let optimizer = Optimizer::new(config, registry);
        let result = optimizer.optimize(&func);

        // Restore stdout
        unsafe {
            libc::dup2(saved_fd, 1);
            libc::close(saved_fd);
        }
        drop(dev_null);
        result
    };

    // Emit the rewritten function as C.
    let c_source = sir_benchmarks::emit::emit_c(&result.function);
    print!("{}", c_source);
}
