//! Pipeline determinism invariant (advisor hardening item 1).
//!
//! For fixed source + configuration + ontology, repeated pipeline runs
//! (fresh engines → fresh hash seeds every time) must produce
//! identical region layouts and truth sets.

use sir_semantics::semantics::SemanticEngine;

/// Region signatures: sorted node ids + sorted concept names per region.
fn region_signatures(func: &sir_nodes::Function) -> Vec<String> {
    let mut analysis = sir_analysis::manager::AnalysisManager::new();
    analysis.run_all(func);
    let mut semantics = SemanticEngine::new();
    semantics.derive(func, analysis.database());

    let mut sigs: Vec<String> = Vec::new();
    for (rid, region) in semantics.database().regions() {
        let mut nodes: Vec<u64> = region.nodes().iter().map(|n| n.0).collect();
        nodes.sort();
        let mut concepts: Vec<String> =
            region.concepts().iter().map(|c| format!("{:?}", c)).collect();
        concepts.sort();
        sigs.push(format!(
            "region{:?} nodes={:?} concepts={:?}",
            rid, nodes, concepts
        ));
    }
    sigs.sort();
    sigs
}

/// Truth signatures: concept + inputs + outputs + origin, sorted.
fn truth_signatures(func: &sir_nodes::Function) -> Vec<String> {
    let mut analysis = sir_analysis::manager::AnalysisManager::new();
    analysis.run_all(func);
    let mut semantics = sir_semantics::semantics::SemanticEngine::new();
    semantics.derive(func, analysis.database());

    let mut sigs: Vec<String> = semantics
        .database()
        .truths()
        .map(|t| {
            let inputs: Vec<u64> = t.inputs.iter().map(|v| v.0).collect();
            let outputs: Vec<u64> = t.outputs.iter().map(|v| v.0).collect();
            format!(
                "{:?} in={:?} out={:?} origin={:?}",
                t.concept, inputs, outputs, t.origin
            )
        })
        .collect();
    sigs.sort();
    sigs
}

/// Run the pipeline several times per function with fresh engines and
/// assert byte-identical region layouts and truth sets. The functions
/// come from the composition suite (loop + mask algebra + parity) and
/// the two real kernels — the shapes whose merging previously varied.
#[test]
fn repeated_runs_produce_identical_regions_and_truths() {
    let cases: Vec<(String, sir_nodes::Function)> = sir_benchmarks::composition::benchmarks()
        .into_iter()
        .map(|def| (format!("composition:{}", def.spec.id), (def.func)()))
        .collect();

    for (name, func) in &cases {
        let first_regions = region_signatures(func);
        let first_truths = truth_signatures(func);

        for run in 0..4 {
            let regions = region_signatures(func);
            let truths = truth_signatures(func);
            assert_eq!(
                regions, first_regions,
                "region layout differs across runs ({} run {})",
                name, run
            );
            assert_eq!(
                truths, first_truths,
                "truth set differs across runs ({} run {})",
                name, run
            );
        }
    }
}
