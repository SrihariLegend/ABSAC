//! gate6a_run — generation-agnostic Gate 6A recognition evaluation.
//!
//! Same pipeline and protocol as `gate6a_v3_run` (which stays frozen as
//! the v3 record), extended with the `contained_abstain` expectation:
//! a row where semantic recognition may fire (the concept is true) but
//! authorization containment must still yield zero candidates and zero
//! rewrites (signed-overflow class).
//!
//! Usage: gate6a_run --corpus <corpus.ll> --expectations <expected.csv>

use std::collections::HashMap;

use sir_analysis::manager::AnalysisManager;
use sir_generation::candidate::Candidate;
use sir_generation::generator::CandidateGenerator;
use sir_inference::engine::InferenceEngine;
use sir_lower::{list_functions, lower_function};
use sir_optimizer::{Optimizer, OptimizerConfig};
use sir_rewrite::registry::default_registry;
use sir_semantics::semantics::SemanticEngine;

const REDUCTION_CONCEPTS: [&str; 4] = [
    "CardinalityReduction",
    "SumReduction",
    "ConjunctiveReduction",
    "DisjunctiveReduction",
];

struct Expectation {
    class: String,
    expectation: String,
    expected_lower: String,
    expected_verify: String,
    expected_concepts: Vec<String>,
    key_property: String,
}

fn parse_expectations(path: &str) -> Vec<(String, Expectation)> {
    let text = std::fs::read_to_string(path).expect("read expectations");
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("kernel\t") {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() < 9 {
            continue;
        }
        out.push((
            f[0].to_string(),
            Expectation {
                class: f[1].to_string(),
                expectation: f[2].to_string(),
                expected_lower: f[3].to_string(),
                expected_verify: f[4].to_string(),
                expected_concepts: f[5]
                    .split(';')
                    .filter(|c| *c != "none" && !c.is_empty())
                    .map(|c| c.to_string())
                    .collect(),
                key_property: f[8].to_string(),
            },
        ));
    }
    out
}

struct Observation {
    lowered: bool,
    verified: bool,
    concepts: Vec<String>,
    truths: usize,
    beliefs: usize,
    candidates: usize,
    rewrites: usize,
    error: Option<String>,
}

fn observe(ll_text: &str, kernel: &str) -> Observation {
    let mut obs = Observation {
        lowered: false,
        verified: false,
        concepts: Vec::new(),
        truths: 0,
        beliefs: 0,
        candidates: 0,
        rewrites: 0,
        error: None,
    };
    let func = match lower_function(ll_text, kernel) {
        Ok(f) => {
            obs.lowered = true;
            f
        }
        Err(e) => {
            obs.error = Some(format!("lower: {e}"));
            return obs;
        }
    };
    let mut verifier = sir_verify::Verifier::new(&func);
    obs.verified = verifier.verify();
    if !obs.verified {
        obs.error = Some("verify: SIR failed structural verification".to_string());
        return obs;
    }
    let mut analysis = AnalysisManager::new();
    analysis.run_all(&func);
    let mut semantics = SemanticEngine::new();
    semantics.derive(&func, analysis.database());
    obs.truths = semantics.database().truths().count();
    for (_, region) in semantics.database().regions() {
        for c in region.concepts() {
            obs.concepts.push(format!("{:?}", c));
        }
    }
    let mut inference = InferenceEngine::new();
    inference.infer(semantics.database(), semantics.structural_database());
    for (_, ctxs) in inference.context_database().contexts() {
        obs.beliefs += ctxs.len();
    }
    let authorizations = sir_semantics::authorization::derive_authorizations(
        &func,
        analysis.database(),
        semantics.database(),
    );
    let mut generator = CandidateGenerator::new();
    generator.generate(
        inference.context_database(),
        semantics.database(),
        &authorizations,
        &func,
    );
    let candidates: Vec<Candidate> = generator.database().all_candidates().cloned().collect();
    obs.candidates = candidates.len();
    let optimizer = Optimizer::new(OptimizerConfig::default(), default_registry());
    let result = optimizer.optimize(&func);
    obs.rewrites = result.rewrites_applied;
    obs
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut corpus = String::new();
    let mut expectations_path = String::new();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--corpus" => {
                corpus = args.get(i + 1).cloned().unwrap_or_default();
                i += 2;
            }
            "--expectations" => {
                expectations_path = args.get(i + 1).cloned().unwrap_or_default();
                i += 2;
            }
            other => {
                eprintln!("unknown argument '{other}'");
                std::process::exit(2);
            }
        }
    }
    if corpus.is_empty() || expectations_path.is_empty() {
        eprintln!("usage: gate6a_run --corpus <corpus.ll> --expectations <expected.csv>");
        std::process::exit(2);
    }

    let ll_text = std::fs::read_to_string(&corpus).expect("read corpus");
    let expectations = parse_expectations(&expectations_path);
    let corpus_functions: Vec<String> = list_functions(&ll_text);
    let by_name: HashMap<&str, &Expectation> = expectations
        .iter()
        .map(|(k, e)| (k.as_str(), e))
        .collect();

    println!("# Gate 6A fresh blind recognition evaluation");
    println!("# corpus={corpus}");
    println!("# registry=default; config=OptimizerConfig::default");

    let mut positives = 0usize;
    let mut positives_recognized = 0usize;
    let mut recall_misses = 0usize;
    let mut known_gap_misses = 0usize;
    let mut negatives = 0usize;
    let mut contained_rows = 0usize;
    let mut false_positive_recognitions = 0usize;
    let mut unsafe_candidates = 0usize;
    let mut unsafe_rewrites = 0usize;
    let mut row_failures = 0usize;

    for kernel in &corpus_functions {
        let Some(expect) = by_name.get(kernel.as_str()) else {
            println!("\nKERNEL {kernel} UNCLASSIFIED (not in expectations) — FAIL");
            row_failures += 1;
            continue;
        };
        let obs = observe(&ll_text, kernel);
        let reduction_concepts: Vec<String> = obs
            .concepts
            .iter()
            .filter(|c| REDUCTION_CONCEPTS.contains(&c.as_str()))
            .cloned()
            .collect();

        println!(
            "\nKERNEL {kernel} class={} expectation={}",
            expect.class, expect.expectation
        );
        println!("  key_property: {}", expect.key_property);
        println!(
            "  observed: lowered={} verify={} truths={} concepts={:?} beliefs={} cands={} rewrites={}",
            obs.lowered, obs.verified, obs.truths, obs.concepts, obs.beliefs, obs.candidates, obs.rewrites
        );
        if let Some(err) = &obs.error {
            println!("  note: {err}");
        }

        if expect.class == "N" {
            negatives += 1;
            if expect.expectation == "contained_abstain" {
                contained_rows += 1;
                let mut ok = true;
                if obs.candidates > 0 {
                    unsafe_candidates += 1;
                    ok = false;
                    println!(
                        "  SAFETY FAIL: {} candidate(s) authorized on a contained_abstain row",
                        obs.candidates
                    );
                }
                if obs.rewrites > 0 {
                    unsafe_rewrites += 1;
                    ok = false;
                    println!(
                        "  SAFETY FAIL: {} rewrite(s) applied on a contained_abstain row",
                        obs.rewrites
                    );
                }
                println!(
                    "  recognition recorded (allowed): reduction_concepts={reduction_concepts:?}"
                );
                if ok {
                    println!("  RESULT contained (PASS)");
                } else {
                    row_failures += 1;
                    println!("  RESULT CONTAINMENT VIOLATION (FAIL)");
                }
                continue;
            }
            let mut ok = true;
            if !reduction_concepts.is_empty() {
                false_positive_recognitions += 1;
                ok = false;
                println!(
                    "  SAFETY FAIL: reduction concept on a negative: {reduction_concepts:?}"
                );
            }
            if obs.candidates > 0 {
                unsafe_candidates += 1;
                ok = false;
                println!(
                    "  SAFETY FAIL: {} candidate(s) generated on a negative",
                    obs.candidates
                );
            }
            if obs.rewrites > 0 {
                unsafe_rewrites += 1;
                ok = false;
                println!(
                    "  SAFETY FAIL: {} rewrite(s) applied on a negative",
                    obs.rewrites
                );
            }
            if ok {
                println!("  RESULT safe_abstain (PASS)");
            } else {
                row_failures += 1;
                println!("  RESULT SAFETY VIOLATION (FAIL)");
            }
            continue;
        }

        positives += 1;
        let recognized = expect
            .expected_concepts
            .iter()
            .all(|c| obs.concepts.contains(c));
        if recognized {
            positives_recognized += 1;
            println!("  RESULT recognized {:?} (PASS)", expect.expected_concepts);
            continue;
        }
        if expect.expectation == "recall_gap_known" {
            known_gap_misses += 1;
            println!(
                "  RESULT recall miss (KNOWN OPEN GAP, recorded; lowered={} verified={})",
                obs.lowered, obs.verified
            );
            continue;
        }
        recall_misses += 1;
        row_failures += 1;
        println!(
            "  RESULT RECALL MISS (FAIL): expected {:?}; lowered={} verified={} (expected lower={} verify={})",
            expect.expected_concepts, obs.lowered, obs.verified, expect.expected_lower, expect.expected_verify
        );
    }

    println!(
        "\nGATE6A_SUMMARY positives={positives} recognized={positives_recognized} recall_misses={recall_misses} known_gap_misses={known_gap_misses}"
    );
    println!(
        "GATE6A_SAFETY negatives={negatives} contained_rows={contained_rows} false_positive_recognitions={false_positive_recognitions} unsafe_candidates={unsafe_candidates} unsafe_rewrites={unsafe_rewrites}"
    );
    println!(
        "GATE6A_VERDICT {}",
        if row_failures == 0 { "PASS" } else { "FAIL" }
    );
    if row_failures > 0 {
        std::process::exit(1);
    }
}
