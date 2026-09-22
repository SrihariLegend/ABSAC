//! Evidence CLI for the kernel-checked Kernighan law.
//!
//! Run with `cargo run -p sir_mech --example kernighan_induction -- 32`.

use std::time::Instant;

fn main() {
    let width: u32 = std::env::args()
        .nth(1)
        .and_then(|arg| arg.parse().ok())
        .unwrap_or(32);
    let started = Instant::now();
    let outcome = sir_mech::kernighan::kernighan_theorem(width);
    println!(
        "kernighan width={width} proved={} in {:.3}s",
        outcome.is_ok(),
        started.elapsed().as_secs_f64()
    );
    if let Err(error) = outcome {
        eprintln!("{error:?}");
    }
}
