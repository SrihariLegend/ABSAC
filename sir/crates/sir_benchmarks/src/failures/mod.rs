// Expected Failures: Captured cases where ABSAC does not optimize.
// They act as artifacts of future work (e.g., missing representation, missing concept).

pub mod mask_algebra;
pub mod bit_permutations;

use crate::framework::BenchmarkDef;

pub fn benchmarks() -> Vec<BenchmarkDef> {
    let mut all = Vec::new();
    all.extend(mask_algebra::benchmarks());
    all.extend(bit_permutations::benchmarks());
    all
}
