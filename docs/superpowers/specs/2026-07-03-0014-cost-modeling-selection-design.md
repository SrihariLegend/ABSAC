# Phase 0014 — Cost Modeling & Transformation Selection Design

**Status:** Research milestone (specification)
**Date:** 2026-07-03
**Depends on:** Phase 0010 (Semantic Reasoning), Phase 0011 (Transformation Planning), Phase 0012 (Equivalence Verification), Phase 0013 (Verified Rewriting)

## Purpose

Phase 0014 introduces the first subjective layer in the compiler — preference over truth.

Previous phases answered:

- What does this program do?
- What representation does it implement?
- What transformations are possible?
- Are those transformations mathematically correct?
- How do we mechanically construct the replacement?

This phase answers only one question:

> **Among the already-proven rewrites, which one should we apply?**

No proving. No graph construction. No analysis. No mutation. Only deterministic ranking.

## Philosophy

Verification establishes correctness. Cost modeling establishes desirability. Those are fundamentally different.

```text
Verification
    proves:
        rewrite is correct

Selection
    decides:
        rewrite is worthwhile
```

A correct rewrite may still be rejected. Proofs are binary — a theorem is proven or it isn't. No proof is "more correct" than another. Score never influences proof (verification happens first, always). Correctness never influences score (only cost deltas matter).

The mission of ABSAC is transformation to bitwise form. If a zero-cost bitwise rewrite exists, it should be applied — the goal is bitwise expression, not performance optimization. Performance is a safeguard against degradation and a tiebreaker between competing bitwise forms.

## Pipeline

```text
Generation       (proposes plans)
    ↓
Verification     (proves plans)
    ↓
Selection        (ranks proven plans)    ← THIS PHASE
    ↓
Rewrite          (executes winner only)
```

Rewrite no longer consumes every verified candidate. It consumes exactly one selected winner.

## Knowledge Pipeline Position

```text
Facts           (sir_analysis)
    ↓
Truths          (sir_semantics)
    ↓
Beliefs         (sir_inference)
    ↓
Contexts        (sir_transform)
    ↓
Plans           (sir_generation)
    ↓
Proofs          (sir_verification)
    ↓
Selections      (sir_selection)          ← THIS LAYER
    ↓
Rewrites        (sir_rewrite)
```

## New Crate

```
sir_selection/
├── Cargo.toml
└── src/
    ├── lib.rs              — module declarations, re-exports
    ├── cost_model.rs        — CostModel trait, DefaultCostModel
    ├── score.rs             — TransformationScore, ScoreBreakdown, CostModelReport
    ├── selector.rs          — Selector, VerifiedCandidate, SelectedCandidate
    └── database.rs          — SelectionDatabase, SelectionResult
```

### Dependencies

```
sir_types
sir_generation
sir_verification
sir_transform
```

No dependency on:

```
sir_analysis
sir_semantics
sir_inference
sir_builder
sir_nodes
```

Selection never rediscovers information.

## Changes to Existing Crates

| Crate | Change |
|-------|--------|
| `sir_types` | Add `CostProfile` struct (new file `cost_profile.rs`). New types are allowed by the architecture freeze. |
| `sir_generation` | `CandidateId` gains `PartialOrd + Ord` derive (for deterministic tie-breaking). |
| `sir_verification` | `TransformationDefinition` trait gains `cost_profile(&self, ctx: &TransformationContext) -> CostProfile`. |
| `sir_selection` | New crate — all selection logic. |
| All other crates | No changes. |

`sir_semantics`, `sir_inference`, `sir_generation`, and `sir_rewrite` are unchanged. The original cost profile is computed by the orchestrator (caller) by walking the region's SIR nodes, not by any existing crate.

## Core Types

### CostProfile (in `sir_types`)

```rust
/// Objective physical characteristics of a computation.
///
/// This type contains no notion of "good" or "bad".
/// Cost models assign meaning to these fields.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CostProfile {
    /// Number of IR instructions.
    pub instruction_count: u32,
    /// Number of Select operations (branchless conditionals).
    pub select_count: u32,
    /// Number of memory accesses (loads + stores).
    pub memory_accesses: u32,
    /// Longest dependency chain in the computation DAG.
    pub critical_path_depth: u32,
}
```

Purely descriptive. Same struct for original region cost and candidate expected cost. The `diff` method lives here.

Note: `select_count` reflects SIR's branchless design — `Select` replaces `if`/`else`. There are no branches in SIR.

### VerifiedCandidate (in `sir_selection`)

```rust
/// A candidate that has passed verification.
/// Carries everything needed for selection and rewriting.
#[derive(Clone, Debug)]
pub struct VerifiedCandidate {
    pub candidate: Candidate,
    pub proof: Proof,
}
```

Makes impossible states impossible — you cannot pass an unverified candidate to the selector.

### TransformationScore (in `sir_selection`)

```rust
#[derive(Clone, Debug)]
pub struct TransformationScore {
    pub candidate: CandidateId,
    pub strategy: ImplementationStrategy,
    pub total: i64,
    pub breakdown: ScoreBreakdown,
}
```

### ScoreBreakdown (in `sir_selection`)

```rust
#[derive(Clone, Debug)]
pub struct ScoreBreakdown {
    pub instruction_delta: i64,  // positive = fewer instructions
    pub select_delta: i64,       // positive = fewer Select ops
    pub memory_delta: i64,       // positive = fewer memory accesses
    pub depth_delta: i64,        // positive = shallower critical path
}
```

All fields are objective deltas. No policy fields. The invariant `total == instruction_delta + select_delta + memory_delta + depth_delta` is enforced by tests.

### SelectedCandidate (in `sir_selection`)

```rust
/// The selector's output — no reconstruction needed by the caller.
pub struct SelectedCandidate<'a> {
    pub candidate: &'a Candidate,
    pub proof: &'a Proof,
    pub score: TransformationScore,
}
```

### SelectionResult (in `sir_selection`)

```rust
pub struct SelectionResult<'a> {
    pub chosen: Option<SelectedCandidate<'a>>,
    pub rejected: Vec<CandidateId>,
    pub report: CostModelReport,
}
```

`chosen` is `Option` — tie-breaking resolves to a single winner via `CandidateId` ordering, so there is always 0 or 1 result.

### SelectionDatabase (in `sir_selection`)

```rust
pub struct SelectionDatabase {
    results: BTreeMap<RegionId, SelectionResultOwned>,
}
```

Where `SelectionResultOwned` is the owned equivalent of `SelectionResult<'a>` for persistent storage:

```rust
pub struct SelectionResultOwned {
    pub chosen: Option<(Candidate, Proof, TransformationScore)>,
    pub rejected: Vec<CandidateId>,
    pub report: CostModelReport,
}
```

### CostModelReport (in `sir_selection`)

```rust
pub struct CostModelReport {
    pub region: RegionId,
    pub scores: Vec<TransformationScore>,  // sorted highest first
}
```

Display format:

```
Region 5
  Popcount         +70
  BitIteration      +66
  MaskConstruction  +63
  PackedBitfield    +60
  Winner: Popcount
```

## CostModel Trait

```rust
/// Assigns desirability to proven rewrites.
///
/// This is the first subjective layer in the compiler.
/// After verification, there may be multiple provably correct rewrites.
/// The CostModel determines which is *preferred* — it does not determine
/// which is correct (that is verification's job).
pub trait CostModel {
    /// Evaluate a proven candidate and return its score.
    ///
    /// Deltas are computed as: original - expected.
    /// Positive deltas mean improvement.
    fn evaluate(
        &self,
        candidate: &Candidate,
        original: &CostProfile,
        expected: &CostProfile,
    ) -> TransformationScore;
}
```

No `Proof` parameter — the cost model does not care how a candidate was proven. No SIR access. Pure function of profiles.

### DefaultCostModel

```rust
/// Simple additive cost model.
///
/// Every reduced instruction, Select operation, memory access,
/// and dependency level contributes equally (+1).
///
/// No architecture-specific weighting is performed.
/// These values are illustrative for the default
/// architecture-independent model and are not intended
/// to represent real hardware performance.
pub struct DefaultCostModel;

impl CostModel for DefaultCostModel {
    fn evaluate(
        &self,
        candidate: &Candidate,
        original: &CostProfile,
        expected: &CostProfile,
    ) -> TransformationScore {
        let instruction_delta = original.instruction_count as i64 - expected.instruction_count as i64;
        let select_delta = original.select_count as i64 - expected.select_count as i64;
        let memory_delta = original.memory_accesses as i64 - expected.memory_accesses as i64;
        let depth_delta = original.critical_path_depth as i64 - expected.critical_path_depth as i64;

        let total = instruction_delta + select_delta + memory_delta + depth_delta;

        TransformationScore {
            candidate: candidate.id,
            strategy: candidate.strategy,
            total,
            breakdown: ScoreBreakdown {
                instruction_delta,
                select_delta,
                memory_delta,
                depth_delta,
            },
        }
    }
}
```

## Selector

```rust
/// Deterministic selection of the best verified candidate.
///
/// Owns the transformation registry (for cost profile lookup)
/// and the cost model (for scoring).
pub struct Selector<M: CostModel> {
    registry: TransformationRegistry,
    cost_model: M,
}

impl<M: CostModel> Selector<M> {
    pub fn new(registry: TransformationRegistry, cost_model: M) -> Self {
        Self { registry, cost_model }
    }

    /// Select the best candidate from verified options.
    ///
    /// For each verified candidate:
    ///   1. Look up TransformationDefinition by candidate.definition_id
    ///   2. Call definition.cost_profile(&ctx) -> expected CostProfile
    ///   3. CostModel.evaluate(candidate, &original, &expected) -> TransformationScore
    ///
    /// `original_costs` maps RegionId to the original region's CostProfile.
    /// These are pre-computed by the orchestrator (caller) by walking region
    /// SIR nodes — the selector never reads SIR.
    ///
    /// Policy:
    ///   - Filter: total >= 0 (bitwise form is the goal; performance is a safeguard)
    ///   - Rank: highest total wins
    ///   - Tie: lowest CandidateId wins (deterministic, stable)
    ///   - Empty input: None
    pub fn select<'a>(
        &self,
        verified: &'a [VerifiedCandidate],
        context_db: &TransformationContextDatabase,
        original_costs: &BTreeMap<RegionId, CostProfile>,
    ) -> SelectionResult<'a> { ... }
}
```

### Selection Policy

Deterministic. Always.

| Condition | Result |
|-----------|--------|
| Highest `total` | Winner |
| Tie on `total` | Lowest `CandidateId` wins |
| `total >= 0` | Candidate is accepted (bitwise form achieved) |
| `total < 0` | Candidate is rejected (degradation) |
| All candidates rejected | `chosen` is `None` |
| Empty input | `chosen` is `None` |

Compiler output must be reproducible: same input → same optimization → same binary. Always.

### Tie-breaking invariant

> The selector is deterministic. If multiple candidates receive identical scores, the candidate with the smallest `CandidateId` is selected. `CandidateId` implements `Ord`, guaranteeing a total order.

## Caller Integration

```rust
// Build the selector once
let mut registry = TransformationRegistry::new();
registry.register(Box::new(PopcountDefinition::new(DefinitionId::new(0))));
let selector = Selector::new(registry, DefaultCostModel);

// Compute original costs (orchestrator responsibility — walks SIR region nodes)
let mut original_costs = BTreeMap::new();
for (region_id, nodes) in &regions {
    original_costs.insert(*region_id, compute_original_cost(function, nodes));
}

// Run selection
let result = selector.select(&verified_candidates, context_db, &original_costs);

// Execute only the winner
if let Some(selected) = result.chosen {
    engine.rewrite(function, selected.candidate, selected.proof, structural_db)?;
}
```

The orchestrator (caller, not a crate) is responsible for computing the original `CostProfile` for each region by walking the region's SIR nodes and counting instructions, Select operations, memory effects, and dependency depth. This is done once before calling `select()`. Selection never reads SIR.

## BS001 Acceptance Benchmark

```text
Full pipeline: build SIR -> analyze -> semantics -> inference
    -> generation -> verification -> selection -> rewrite
```

1. Four candidates exist (BitIteration, Popcount, PackedBitfield, MaskConstruction) — verified by prior phases
2. All four are verified equivalent
3. DefaultCostModel scores each deterministically
4. Selector chooses Popcount (highest score)
5. Only the Popcount recipe is passed to `sir_rewrite`
6. Rewritten SIR passes `sir_verify`
7. Repeated runs produce identical results

Illustrative BS001 scores (DefaultCostModel):

| Candidate | Instr Δ | Select Δ | Mem Δ | Depth Δ | Total |
|-----------|---------|----------|-------|---------|-------|
| Popcount | +4 | +1 | +63 | +2 | **+70** |
| BitIteration | +2 | 0 | +63 | +1 | +66 |
| MaskConstruction | 0 | +1 | +62 | 0 | +63 |
| PackedBitfield | -2 | +1 | +62 | -1 | +60 |

Winner: Popcount.

These values are illustrative for the default architecture-independent model and are not intended to represent real hardware performance.

## Test Strategy

| Tier | Test | Verifies |
|------|------|----------|
| 1 | `cost_profile_diff` | Delta computation is correct |
| 2 | `score_breakdown_sum` | Breakdown fields sum to `total` |
| 3 | `selector_highest_wins` | Max score is chosen |
| 4 | `selector_tie_lowest_id` | Equal scores → lowest `CandidateId` wins |
| 5 | `selector_empty_input` | Empty → `None` |
| 6 | `selector_all_negative` | All totals < 0 → `None` |
| 7 | `selector_zero_wins` | `total == 0` → accepted, lowest ID selected |
| 8 | `selector_positive_beats_zero` | `+5` beats `0` |
| 9 | `selector_positive_beats_negative` | `+1` beats `-1` |
| 10 | `deterministic_selection` | Same inputs → same decision |
| 11 | `report_formatting` | `CostModelReport` Display matches expected format |
| 12 | `bs001_selection` | Full pipeline: Popcount selected |

## Selection Invariants

### Correctness never influences score

Proofs are binary. A theorem is proven or it isn't. No proof is "more correct." The score reflects only cost deltas.

### Score never influences proof

Verification happens first. Always. Selection only sees candidates that have already been proven.

### Selection never mutates SIR

Pure function. Selection reads profiles and returns a decision. It never touches the IR.

### Same inputs → same decision

Deterministic. No randomness. No instability. Compiler output must be reproducible.

### Selection never reads SIR

The original `CostProfile` is pre-computed by the orchestrator. The `CostModel` only sees the profiles. No graph traversal inside the selection crate.

## Explicit Non-Goals

This phase does **not** implement:

- CPU-specific tuning
- Auto benchmarking
- Profile-guided optimization (PGO)
- JIT compilation
- Machine learning
- Reinforcement learning
- Search algorithms
- Exploration
- Speculative execution
- Multi-objective optimization

## Extensibility

The `CostModel` trait naturally allows future models without touching any previous phase:

```
DefaultCostModel       — v0.1 additive model
X86CostModel           — x86 instruction weighting
ARM64CostModel         — ARM instruction weighting
ProfileGuidedCostModel — runtime profile input
EnergyCostModel        — power consumption weighting
CodeSizeCostModel      — minimal binary size
```

Each model is a new implementation of the same trait. No architectural changes required.

## Success Criteria

For the BS001 benchmark:

1. Four mathematically verified candidates exist
2. The cost model deterministically scores each candidate
3. Every score has a complete breakdown where `sum(deltas) == total`
4. The selector chooses exactly one candidate (Popcount)
5. The choice is reproducible across runs
6. Only the selected rewrite is passed to `sir_rewrite`
7. The resulting optimized SIR is identical across repeated compilations
