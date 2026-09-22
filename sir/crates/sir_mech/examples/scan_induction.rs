//! Evidence CLI for the generic first-match scan theorem.

use std::time::Instant;

fn main() {
    let started = Instant::now();
    let theorem = sir_mech::scan::first_match_theorem();
    println!(
        "first_match_scan proved={} in {:.3}s",
        theorem.is_ok(),
        started.elapsed().as_secs_f64()
    );
    if let Err(error) = theorem {
        eprintln!("{error:?}");
    }
}
