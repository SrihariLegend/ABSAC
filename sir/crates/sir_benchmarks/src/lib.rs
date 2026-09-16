pub mod framework;
pub mod emit;
pub mod real_kernels;
pub mod hackers_delight;
pub mod boolean_reductions;
pub mod positional_search;
pub mod failures;
pub mod non_optimizable;
pub mod composition;
pub mod graph;
pub mod vector_plan;
pub mod vector_derive;
pub mod vector_emit;
// NOTE: the prose-only `compositional_proof` module and the
// `gate4_prove` binary were retired when Gate 4B was actually closed by
// the mechanical checker in `sir_gate4b` (see docs/GATE4B_PROOF.md).
// They reported `Proven` from hardcoded strings and must not be used as
// evidence.
pub mod gate5a;
pub mod gate6b;

use framework::BenchmarkDef;

pub fn all_benchmarks() -> Vec<BenchmarkDef> {
    let mut all = Vec::new();
    all.extend(hackers_delight::benchmarks());
    all.extend(boolean_reductions::benchmarks());
    all.extend(positional_search::benchmarks());
    all.extend(failures::benchmarks());
    all.extend(non_optimizable::benchmarks());
    all.extend(composition::benchmarks());
    all.extend(graph::benchmarks());
    all
}
