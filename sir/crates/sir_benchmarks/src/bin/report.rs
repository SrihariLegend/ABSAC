use sir_benchmarks::all_benchmarks;
use sir_benchmarks::framework::{BenchmarkSpec, ExpectedKnowledge};
use sir_optimizer::{OptimizationResult, Optimizer, OptimizerConfig};
use sir_rewrite::registry::default_registry;
use sir_semantics::concepts::SemanticConcept;
use sir_semantics::semantics::SemanticEngine;
use sir_semantics::truth::{Provenance, SemanticTruth};
use sir_transform::representation::Representation;
use std::collections::{HashMap, HashSet};

/// A named family of concepts forming a semantic domain.
struct Domain {
    name: &'static str,
    concepts: &'static [SemanticConcept],
}

const DOMAINS: &[Domain] = &[
    Domain {
        name: "Boolean reductions",
        concepts: &[
            SemanticConcept::CardinalityReduction,
            SemanticConcept::DisjunctiveReduction,
            SemanticConcept::ConjunctiveReduction,
            SemanticConcept::ExclusiveReduction,
            SemanticConcept::MembershipTraversal,
            SemanticConcept::LogicalSequence,
            SemanticConcept::FiniteCollection,
            SemanticConcept::PredicateMap,
            SemanticConcept::ElementSequence,
        ],
    },
    Domain {
        name: "Arithmetic identities",
        concepts: &[
            SemanticConcept::ModuloPowerOfTwo,
            SemanticConcept::MultiplyPowerOfTwo,
            SemanticConcept::DividePowerOfTwo,
            SemanticConcept::ShiftMask,
        ],
    },
    Domain {
        name: "Positional search",
        concepts: &[
            SemanticConcept::PositionSearch,
            SemanticConcept::FirstOccurrence,
            SemanticConcept::LastOccurrence,
            SemanticConcept::TrailingZeroSearch,
            SemanticConcept::LeadingZeroSearch,
            SemanticConcept::FindFirst,
            SemanticConcept::LoopUntilZero,
        ],
    },
    Domain {
        name: "Set algebra",
        concepts: &[
            SemanticConcept::FiniteSet,
            SemanticConcept::SetMembership,
            SemanticConcept::SetUnion,
            SemanticConcept::SetDifference,
            SemanticConcept::SetSymmetricDifference,
            SemanticConcept::SetSubset,
            SemanticConcept::SetEquality,
            SemanticConcept::SetEmpty,
            SemanticConcept::SetCardinality,
            SemanticConcept::SetIntersection,
        ],
    },
    Domain {
        name: "Mask algebra",
        concepts: &[
            SemanticConcept::LowestSetBit,
            SemanticConcept::ClearLowestSetBit,
            SemanticConcept::IsZero,
            SemanticConcept::AtMostOneBitSet,
            SemanticConcept::BitsetIteration,
        ],
    },
    Domain {
        name: "Bit permutations",
        concepts: &[],
    },
];

/// Domain coverage: ✓ when every concept in the domain was exercised,
/// Partial when only some were, Missing when none were.
fn domain_status(exercised: &HashSet<String>, concepts: &[SemanticConcept]) -> &'static str {
    let hit = concepts
        .iter()
        .filter(|c| exercised.contains(&format!("{:?}", c)))
        .count();
    if concepts.is_empty() || hit == 0 {
        "Missing"
    } else if hit < concepts.len() {
        "Partial"
    } else {
        "✓"
    }
}

/// Derivation depth of a semantic truth: 0 for physical truths, one more than
/// the deepest ancestor for derived truths. Computed with memoization over one
/// iteration's truth list.
fn truth_depth(truth: &SemanticTruth, truths: &[SemanticTruth], memo: &mut HashMap<usize, usize>) -> usize {
    if let Some(d) = memo.get(&truth.id.0) {
        return *d;
    }
    let d = match &truth.provenance {
        Provenance::Physical { .. } => 0,
        Provenance::Derived { from_truths } => {
            1 + from_truths
                .iter()
                .filter_map(|f| {
                    truths
                        .iter()
                        .find(|t| t.id == *f)
                        .map(|t| truth_depth(t, truths, memo))
                })
                .max()
                .unwrap_or(0)
        }
    };
    memo.insert(truth.id.0, d);
    d
}

/// Average and maximum reasoning depth across all derived truths seen.
fn reasoning_depth(truth_lists: &[Vec<SemanticTruth>]) -> (f64, usize) {
    let mut all_depths: Vec<usize> = Vec::new();
    for truths in truth_lists {
        let mut memo = HashMap::new();
        for t in truths {
            let d = truth_depth(t, truths, &mut memo);
            if matches!(t.provenance, Provenance::Derived { .. }) {
                all_depths.push(d);
            }
        }
    }
    if all_depths.is_empty() {
        return (0.0, 0);
    }
    let sum: usize = all_depths.iter().sum();
    let max = *all_depths.iter().max().unwrap();
    (sum as f64 / all_depths.len() as f64, max)
}

fn main() {
    println!("ABSAC Benchmark Status\n======================");

    let benchmarks = all_benchmarks();
    let total = benchmarks.len();

    let mut optimized = 0;
    let mut optimize_failed = 0;
    let mut expected_failures = 0;
    let mut correctly_declined = 0;
    let mut failed_names: Vec<&str> = Vec::new();

    let mut total_initial_nodes = 0;
    let mut total_final_nodes = 0;
    let mut total_truths = 0;

    // Ontology metrics, derived from the actual run.
    let mut exercised_concepts: HashSet<String> = HashSet::new();
    let mut exercised_representations: HashSet<String> = HashSet::new();
    let mut all_truth_lists: Vec<Vec<SemanticTruth>> = Vec::new();

    for def in &benchmarks {
        let config = OptimizerConfig::default();
        let registry = default_registry();
        let optimizer = Optimizer::new(config, registry);
        let result: OptimizationResult = optimizer.optimize(&(def.func)());

        for record in &result.iterations_detail {
            exercised_concepts.extend(record.concepts_discovered.iter().cloned());
            exercised_representations.extend(record.representations_inferred.iter().cloned());
            all_truth_lists.push(record.truths.clone());
        }

        match def.spec.expected {
            ExpectedKnowledge::Optimizes { .. } => {
                if result.rewrites_applied > 0 {
                    optimized += 1;
                    total_initial_nodes += result.initial_nodes;
                    total_final_nodes += result.final_nodes;
                    total_truths += result.max_truths;
                } else {
                    // Expected to optimize but the pipeline did not rewrite:
                    // count it as a failure rather than a success.
                    optimize_failed += 1;
                    failed_names.push(def.spec.name);
                }
            }
            ExpectedKnowledge::MissingKnowledge { .. } => expected_failures += 1,
            ExpectedKnowledge::NonOptimizable { .. } => correctly_declined += 1,
            ExpectedKnowledge::ProvenanceGraph { .. } => {}
        }
    }

    // ── Ontology coverage (derived) ────────────────────────
    let closure_rules = SemanticEngine::new().closure_rule_count();
    let concepts_implemented = SemanticConcept::ALL.len();
    let (avg_depth, max_depth) = reasoning_depth(&all_truth_lists);

    println!(
        "\nOntology Coverage (Architectural Metrics)\n========================================="
    );
    println!(
        "Concepts implemented:     {}   ({} SemanticConcept variants)",
        concepts_implemented, concepts_implemented
    );
    println!(
        "Concepts exercised:       {}   (across the {} benchmark suite)",
        exercised_concepts.len(),
        total
    );
    println!("Closure rules:            {}", closure_rules);
    println!("Average reasoning depth:  {:.1}", avg_depth);
    println!("Maximum reasoning depth:  {}", max_depth);
    println!();

    println!("\nSemantic domains (coverage of each domain's concept family)\n");
    for domain in DOMAINS {
        println!(
            "  {:<24} {}",
            domain.name,
            domain_status(&exercised_concepts, domain.concepts)
        );
    }

    println!("\nRepresentations\n");
    for rep in [
        Representation::BitSet,
        Representation::BitwiseArithmetic,
        Representation::BitScan,
        Representation::MaskAlgebra,
    ] {
        let present = exercised_representations.contains(&format!("{:?}", rep));
        println!("  {:<24} {}", format!("{:?}", rep), if present { "✓" } else { "Missing" });
    }

    // ── Benchmark status ───────────────────────────────────
    println!("\nBenchmarks:             {}", total);
    println!();
    println!("Optimized:              {}", optimized);
    println!("Optimize failures:       {}", optimize_failed);
    println!("Expected failures:       {}", expected_failures);
    println!("Correctly declined:      {}", correctly_declined);
    if !failed_names.is_empty() {
        println!("\nExpected to optimize but failed:");
        for name in &failed_names {
            println!("  - {}", name);
        }
    }

    println!("\nSemantic Compression\n");
    println!("  Total Initial IR nodes:   {}", total_initial_nodes);
    println!("  Total Semantic truths:    {}", total_truths);
    println!("  Total Final IR nodes:     {}", total_final_nodes);
    if total_initial_nodes > 0 {
        let ratio = total_final_nodes as f64 / total_initial_nodes as f64;
        println!("  Compression ratio:        {:.2}x", 1.0 / ratio);
    }
}
