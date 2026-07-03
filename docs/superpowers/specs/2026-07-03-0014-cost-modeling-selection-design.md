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

The mission of ABSAC is transformation to bitwise form. If a rewrite does not degrade performance (`total >= 0`), it should be applied — the goal is bitwise expression, not maximizing performance. Performance is a safeguard against degradation and a tiebreaker between competing bitwise forms.

## Pipeline

```text
Generation       (proposes plans, estimates costs)
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
Plans           (sir_generation)       ← expected_cost attached here
    ↓
Proofs          (sir_verification)
    ↓
Selections      (sir_selection)        ← THIS LAYER
    ↓
Rewrites        (sir_rewrite)
```

Cost estimation happens at generation time. When `sir_generation` creates a `Candidate`, it attaches an `expected_cost: CostProfile`. This is objective data — the generator knows what it proposes. The cost model later assigns *meaning* to that data.

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
```

No dependency on:

```
sir_transform
sir_analysis
sir_semantics
sir_inference
sir_builder
sir_nodes
```

The selector receives `expected_cost` directly from the `Candidate` struct — no transformation registry, no context database, no definition lookups. Selection never rediscovers information.

## Changes to Existing Crates

| Crate | Change |
|-------|--------|
| `sir_types` | Add `CostProfile` struct (new file `cost_profile.rs`). New types are allowed by the architecture freeze. |
| `sir_generation` | `CandidateId` gains `PartialOrd + Ord` derive. `Candidate` gains `expected_cost: CostProfile` field. |
| `sir_selection` | New crate — all selection logic. |
| All other crates | No changes. |

`sir_verification`, `sir_semantics`, `sir_inference`, `sir_rewrite` are unchanged. The original cost profile is computed by the orchestrator (caller) by walking the region's SIR nodes, not by any existing crate.

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

Purely descriptive. Same struct for original region cost and candidate expected cost.

Note: `select_count` reflects SIR's branchless design — `Select` replaces `if`/`else`. There are no branches in SIR.

### Candidate changes (in `sir_generation`)

```rust
pub struct Candidate {
    pub id: CandidateId,
    pub region: RegionId,
    pub context_id: ContextId,
    pub definition_id: DefinitionId,
    pub strategy: ImplementationStrategy,
    pub explanation: CandidateExplanation,
    pub effects: Vec<CandidateEffects>,
    pub expected_cost: CostProfile,        // ← NEW
}
```

The generator populates `expected_cost` at creation time based on the implementation strategy. For example, a `Popcount` candidate always has `instruction_count: 2, select_count: 0, memory_accesses: 1`. This is objective data — the generator knows what it proposes, independent of any cost model.

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

All fields are objective deltas. No policy fields. In the default additive model, `total` equals the sum of the breakdown fields. Weighted models may compute `total` differently — the breakdown remains a factual description of what changed, while `total` reflects the model's policy.

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

Where `SelectionResultOwned` is the owned equivalent for persistent storage:

```rust
pub struct SelectedCandidateOwned {
    pub candidate: Candidate,
    pub proof: Proof,
    pub score: TransformationScore,
}

pub struct SelectionResultOwned {
    pub chosen: Option<SelectedCandidateOwned>,
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
/// In this model, `total` is the unweighted sum of the breakdown fields.
/// Weighted models (e.g., X86CostModel) may compute `total` differently
/// while the `breakdown` still reports the raw deltas.
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
/// The selector does not own a transformation registry or context database.
/// Expected costs come directly from `Candidate.expected_cost`, populated
/// by `sir_generation` at candidate creation time.
pub struct Selector<M: CostModel> {
    cost_model: M,
}

impl<M: CostModel> Selector<M> {
    pub fn new(cost_model: M) -> Self {
        Self { cost_model }
    }

    /// Select the best candidate from verified options for a single region.
    ///
    /// All candidates in `verified` must belong to the same `region`.
    /// `original_cost` is the pre-computed cost profile of the original
    /// region (computed by the orchestrator by walking SIR region nodes —
    /// the selector never reads SIR).
    ///
    /// For each verified candidate, calls:
    ///   CostModel.evaluate(&candidate, original_cost, &candidate.expected_cost)
    ///
    /// Policy:
    ///   - Filter: total >= 0 (does not degrade performance)
    ///   - Rank: highest total wins
    ///   - Tie: lowest CandidateId wins (deterministic, stable)
    ///   - Empty input: chosen is None
    pub fn select<'a>(
        &self,
        region: RegionId,
        verified: &'a [VerifiedCandidate],
        original_cost: &CostProfile,
    ) -> SelectionResult<'a> { ... }
}
```

### Selection Policy

Deterministic. Always.

| Condition | Result |
|-----------|--------|
| Highest `total` | Winner |
| Tie on `total` | Lowest `CandidateId` wins |
| `total >= 0` | Candidate is accepted (does not degrade) |
| `total < 0` | Candidate is rejected (degradation) |
| All candidates rejected | `chosen` is `None` |
| Empty input | `chosen` is `None` |

Compiler output must be reproducible: same input → same optimization → same binary. Always.

### Tie-breaking invariant

> The selector is deterministic. If multiple candidates receive identical scores, the candidate with the smallest `CandidateId` is selected. `CandidateId` implements `Ord`, guaranteeing a total order.

## Caller Integration

```rust
// Build the selector once
let selector = Selector::new(DefaultCostModel);

// Compute original costs (orchestrator responsibility — walks SIR region nodes)
// Selection never reads SIR.
let original_costs: BTreeMap<RegionId, CostProfile> = regions
    .iter()
    .map(|(rid, nodes)| (*rid, compute_original_cost(function, nodes)))
    .collect();

// Run selection per region
let mut selection_db = SelectionDatabase::new();
for (region_id, verified) in proven_candidates.grouped_by_region() {
    let original = original_costs.get(&region_id).unwrap();
    let result = selector.select(region_id, verified, original);
    selection_db.insert(region_id, result);
}

// Execute only the selected winner for each region
for (region_id, result) in selection_db.iter() {
    if let Some(selected) = result.chosen() {
        engine.rewrite(function, selected.candidate, selected.proof, structural_db)?;
    }
}
```

## BS001 Acceptance Benchmark

```text
Full pipeline: build SIR -> analyze -> semantics -> inference
    -> generation -> verification -> selection -> rewrite
```

1. Four candidates exist (BitIteration, Popcount, PackedBitfield, MaskConstruction) — verified by prior phases
2. Each candidate carries its `expected_cost` set by the generator
3. All four are verified equivalent
4. DefaultCostModel scores each deterministically
5. Selector chooses Popcount (highest score)
6. Only the Popcount recipe is passed to `sir_rewrite`
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
| 2 | `default_model_total_matches_sum` | In the default additive model, total equals the sum of breakdown fields |
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

Note: Test 2 verifies only that the *default* model's total equals the sum. Weighted models are free to compute `total` differently — the breakdown fields are factual deltas, while `total` is policy.

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

The original `CostProfile` is pre-computed by the orchestrator. Expected cost comes from `Candidate.expected_cost`, set at generation time. The `CostModel` only sees the two profiles. No graph traversal inside the selection crate.

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

Each model is a new implementation of the same trait. The `breakdown` fields always report factual deltas; each model decides how to combine them into `total`. No architectural changes required.

## Success Criteria

For the BS001 benchmark:

1. Four mathematically verified candidates exist, each carrying `expected_cost`
2. The cost model deterministically scores each candidate
3. For the default model, every score has a complete breakdown where `sum(deltas) == total`
4. The selector chooses exactly one candidate (Popcount)
5. The choice is reproducible across runs
6. Only the selected rewrite is passed to `sir_rewrite`
7. The resulting optimized SIR is identical across repeated compilations
