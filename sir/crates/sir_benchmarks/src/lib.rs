pub mod framework;
pub mod hackers_delight;
pub mod boolean_reductions;
pub mod positional_search;
pub mod failures;
pub mod non_optimizable;

use framework::BenchmarkDef;

pub fn all_benchmarks() -> Vec<BenchmarkDef> {
    let mut all = Vec::new();
    all.extend(hackers_delight::benchmarks());
    all.extend(boolean_reductions::benchmarks());
    all.extend(positional_search::benchmarks());
    all.extend(failures::benchmarks());
    all.extend(non_optimizable::benchmarks());
    all
}
