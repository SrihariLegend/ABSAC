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
    let proved = sir_mech::kernighan::kernighan_law(width);
    println!(
        "kernighan width={width} proved={proved} in {:.3}s",
        started.elapsed().as_secs_f64()
    );
}
