# Phase 0015 — Optimization Driver Design

**Status:** Research milestone (specification)
**Date:** 2026-07-04
**Depends on:** Phases 0000–0014 (complete pipeline)

## Purpose

Phase 0015 turns the complete reasoning pipeline into an optimizer. Previous phases built the organs — analysis, semantics, inference, generation, verification, selection, rewrite. This phase builds the circulatory system that makes them work together.

The pipeline today performs:

```text
analyze once → rewrite once → done
```

After Phase 0015:

```text
repeat
    analyze → semantics → inference → generation
    → verification → selection → rewrite
until fixed point
```

No new reasoning capability. No new transformation families. Only orchestration.

## Philosophy

The optimizer is a coordinator, not an analysis pass. It never derives knowledge from SIR. It never counts instructions, walks graphs, or inspects node kinds. It only moves artifacts between stages — each of which owns its own knowledge.

Every iteration constructs fresh pipeline stages from scratch. No state carries across iterations. This guarantees that each iteration starts from the absolute ground truth of the current IR, eliminating an entire class of cache-invalidation bugs.

## Pipeline

```text
Source
    ↓
Builder
    ↓
Analysis
    ↓
Semantics
    ↓
Inference
    ↓
Generation
    ↓
Verification
    ↓
Selection
    ↓
Rewrite
    ↓
Optimizer (fixed-point driver)
    ↓
Optimized SIR
```

The optimizer sits at the top of the library stack. It depends on everything. Nothing depends on it.

## New Crate

```
sir_optimizer/
├── Cargo.toml
└── src/
    ├── lib.rs              — module declarations, re-exports
    ├── optimizer.rs         — Optimizer, optimize(), optimize_iteration()
    ├── config.rs            — OptimizerConfig
    └── result.rs            — OptimizationResult, IterationRecord, enums
```

### Dependencies

Depends on everything below it in the pipeline. No crate depends on `sir_optimizer`.

## Changes to Existing Crates

| Crate | Change |
|-------|--------|
| `sir_semantics` | `StructuralDescription` gains `original_cost: CostProfile` |
| `sir_optimizer` | New crate |
| All other crates | No changes |

`CostProfile` already exists in `sir_transform` (added in Phase 0014). The semantic recognizer that identifies a region's structure also populates its cost — the boolean collection recognizer already knows it's looking at a `bool[64]` traversal loop and can count the operations as part of structural recognition. The optimizer never walks SIR.

## Core Types

### OptimizerConfig

```rust
pub struct OptimizerConfig {
    /// Maximum fixed-point iterations before terminating.
    pub max_iterations: usize,

    /// Stop after this many total rewrites across all iterations.
    /// Safety valve against rewrite oscillation bugs.
    pub max_total_rewrites: Option<usize>,
}

impl Default for OptimizerConfig {
    fn default() -> Self {
        Self {
            max_iterations: 10,
            max_total_rewrites: None,
        }
    }
}
```

### Optimizer

```rust
/// Fixed-point optimization driver.
///
/// Owns configuration, cost model, and recipe registry.
/// All pipeline stages are constructed fresh each iteration —
/// the optimizer carries no mutable state and no caches.
/// Registries are immutable catalogs, not mutable state.
pub struct Optimizer {
    config: OptimizerConfig,
    cost_model: Box<dyn CostModel>,
    recipe_registry: RecipeRegistry,
}

impl Optimizer {
    pub fn new(
        config: OptimizerConfig,
        cost_model: Box<dyn CostModel>,
        recipe_registry: RecipeRegistry,
    ) -> Self;

    /// Run to fixed point.
    ///
    /// Idempotent: if optimize(f) = g, then optimize(g) = g.
    /// Accepts &Function — the optimizer does not consume its input.
    pub fn optimize(&self, function: &Function) -> OptimizationResult;

    /// One full pipeline pass. Private, exposed for unit testing.
    fn optimize_iteration(
        &self,
        function: &Function,
        iteration_number: usize,
    ) -> IterationResult;
}
```

### IterationResult (private)

```rust
struct IterationResult {
    /// The function after this iteration (may be unchanged).
    function: Function,
    /// Statistics for this iteration.
    record: IterationRecord,
    /// Whether the pipeline has converged (no rewrite applied).
    converged: bool,
}
```

### OptimizationResult

```rust
pub struct OptimizationResult {
    /// The optimized function.
    pub function: Function,
    /// Number of fixed-point iterations executed.
    pub iterations: usize,
    /// Total rewrites applied across all iterations.
    pub rewrites_applied: usize,
    /// Per-iteration breakdown.
    pub iterations_detail: Vec<IterationRecord>,
    /// Why optimization stopped.
    pub termination: TerminationReason,
}

pub enum TerminationReason {
    /// No more rewrites possible — converged.
    FixedPoint,
    /// max_iterations or max_total_rewrites reached.
    IterationLimitReached,
}
```

### IterationRecord

```rust
pub struct IterationRecord {
    pub iteration: usize,
    pub facts_discovered: usize,
    pub truths_discovered: usize,
    pub beliefs_inferred: usize,
    pub candidates_generated: usize,
    pub proofs_attempted: usize,
    pub proofs_succeeded: usize,
    pub candidates_selected: usize,
    pub rewrites_applied: usize,
    pub outcome: IterationOutcome,
}

pub enum IterationOutcome {
    /// At least one rewrite was applied.
    RewriteApplied,
    /// Generation produced no candidates.
    NoCandidate,
    /// Candidates existed but none were selected.
    NoSelection,
}
```

Statistics are always collected — no configuration flag. The overhead is negligible (a few integer increments per iteration) and the data is invaluable for research evaluation.

## Convergence Algorithm

```text
current = input_function

for iteration in 1..=max_iterations:
    result = optimize_iteration(current, iteration)

    if result.converged:
        return OptimizationResult {
            function: result.function,
            termination: FixedPoint,
            ...
        }

    if total_rewrites >= max_total_rewrites:
        return OptimizationResult {
            termination: IterationLimitReached,
            ...
        }

    current = result.function

return OptimizationResult {
    termination: IterationLimitReached,
    ...
}
```

## Single Iteration

```text
optimize_iteration(function, n):

    // 1. Analysis
    analysis = AnalysisManager::new()
    analysis.run_all(function)

    // 2. Semantics
    semantics = SemanticEngine::new()
    semantics.derive(function, analysis.database())
    // StructuralDescription now carries original_cost: CostProfile

    // 3. Inference
    inference = InferenceEngine::new()
    inference.infer(semantics.database(), semantics.structural_database())

    // 4. Generation
    generator = CandidateGenerator::new()
    generator.generate(inference.context_database(), semantics.database())

    if no candidates:
        return converged(function, NoCandidate)

    // 5. Verification
    verifier = Verifier::new()
    obligations = verifier.build_obligations(candidates, contexts)
    proven = []
    for each obligation:
        result = verifier.verify(obligation, context)
        if Proven(proof):
            proven.push(VerifiedCandidate { candidate, proof })

    if proven.is_empty():
        return converged(function, NoCandidate)

    // 6. Selection (per region)
    selector = Selector::new(DefaultCostModel)
    for each region with proven candidates:
        original_cost = structural_db.region(region).original_cost
        result = selector.select(region, &proven, &original_cost)
        if result.chosen:
            selected.push(result.chosen)

    if no selection:
        return converged(function, NoSelection)

    // 7. Rewrite (only selected winners)
    engine = RewriteEngine::new(recipe_registry)
    current = function
    for each selected:
        rewrite_result = engine.rewrite(current, candidate, proof, structural_db)
        current = rewrite_result.rewritten

    return continued(current, RewriteApplied, record)
```

The optimizer never walks SIR nodes. The `original_cost` comes from `StructuralDescription`, populated by the semantic recognizer. Context-to-candidate associations come from the obligation database. The optimizer purely moves artifacts between stages.

## Invariants

### Fixed-point invariant

> Every successful rewrite must eliminate the specific optimization opportunity that produced it. A subsequent iteration must never rediscover the identical candidate for the rewritten region.

This is stronger than "the optimizer converges." It specifies *why* it converges. It also serves as a regression criterion as new transformation families are added.

### Determinism

> `optimize(f)` always produces the same `OptimizationResult` for the same input `f`. No randomness. No instability. Compiler output must be reproducible.

### Database freshness

> Every iteration constructs fresh `AnalysisManager`, `SemanticEngine`, `InferenceEngine`, `CandidateGenerator`, `Verifier`, `Selector`, and `RewriteEngine`. No stage state survives between iterations.

### No knowledge derivation

> The optimizer never counts instructions, walks SIR graphs, inspects node kinds, or derives information from the IR. All knowledge comes from the pipeline stages it orchestrates.

## Test Strategy

| Tier | Test | Verifies |
|------|------|----------|
| 1 | `optimize_empty_function` | No regions → FixedPoint in 1 iteration, output == input |
| 2 | `optimize_bs001_converges` | Iteration 1 rewrites, iteration 2 confirms convergence |
| 3 | `optimize_is_idempotent` | `optimize(optimize(f)) == optimize(f)` |
| 4 | `optimize_iteration_limit` | Hits `max_iterations`, returns `IterationLimitReached` |
| 5 | `optimize_already_optimal` | Input with no candidates → `FixedPoint` in 1 iteration |
| 6 | `iteration_record_populated` | Statistics non-zero when rewrites occur |
| 7 | `deterministic_result` | Same input → same output |
| 8 | `iteration_monotonicity` | Rewrite count never increases after converging |
| 9 | `database_freshness` | Every iteration constructs fresh stages |
| 10 | `no_candidate_termination` | Iteration with zero generated candidates → `NoCandidate` outcome |
| 11 | `no_selection_termination` | Iteration with candidates but none selected → `NoSelection` outcome |

## BS001 Acceptance Benchmark

```
Iteration 1:
    Build SIR (board_scan function)
    Analysis → Semantics → Inference → Generation
    → Verification → Selection → Rewrite
    Result: loop replaced with popcount. RewriteApplied.

Iteration 2:
    Analysis → Semantics → ... → Generation
    Result: no candidates (popcount is already optimal).
    Outcome: NoCandidate. Converged.

optimize(f) returns:
    iterations: 2
    rewrites_applied: 1
    termination: FixedPoint
```

## Explicit Non-Goals

- Front-end lowering (Rust/C → SIR)
- CLI or executable binary
- Multi-function or module-level optimization
- Parallel or incremental compilation
- Pass ordering or phase ordering (single fixed pipeline)
- Profile-guided optimization

## Extensibility

The optimizer is designed to be transparent to new transformation families. Adding a new recognizer, definition, or recipe requires no changes to the optimizer — it discovers the new capability through the existing pipeline interfaces. The fixed-point loop naturally handles multi-step transformations (e.g., popcount → constant propagation → dead code elimination) as each step creates new optimization opportunities for the next iteration.

## Success Criteria

1. BS001 converges in exactly 2 iterations (1 rewrite + 1 confirmation)
2. `optimize(optimize(f)) == optimize(f)` for all inputs
3. Same input always produces identical output
4. Statistics are populated and non-zero when rewrites occur
5. The optimizer never directly inspects SIR nodes
6. All existing tests (380+) continue to pass
