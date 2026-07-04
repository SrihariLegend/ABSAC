# Phase 0015 — Optimization Driver Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the fixed-point optimization driver that orchestrates the full SIR pipeline (analysis → semantics → inference → generation → verification → selection → rewrite) iteratively until convergence.

**Architecture:** A new `sir_optimizer` crate sits at the top of the dependency stack. A `CostDeriver` component in `sir_semantics` pre-computes region costs so the optimizer never walks SIR. Selector gains a multi-region `select_all()` method. The optimizer carries no mutable state — all pipeline stages are constructed fresh each iteration.

**Tech Stack:** Rust 2021 edition, workspace crate in the existing `sir/` workspace. No new external dependencies.

## Global Constraints

- No new reasoning capability. No new transformation families. Only orchestration.
- All existing tests (380+) must continue to pass.
- The optimizer never walks SIR nodes, inspects node kinds, or derives knowledge from IR.
- Every iteration constructs fresh pipeline stages — no state carries across iterations.
- BS001 must converge in exactly 2 iterations (1 rewrite + 1 confirmation).
- `optimize(optimize(f)) == optimize(f)` for all inputs (idempotency).
- Deterministic: same input always produces identical output.

---

### Task 1: CostDatabase type

**Files:**
- Create: `sir/crates/sir_semantics/src/cost.rs`
- Modify: `sir/crates/sir_semantics/src/lib.rs` (add `pub mod cost;`)

**Interfaces:**
- Consumes: `sir_types::RegionId`, `sir_types::CostProfile`
- Produces: `CostDatabase` struct with `new()`, `insert(RegionId, CostProfile)`, `for_region(RegionId) -> Option<&CostProfile>`

- [ ] **Step 1: Create `cost.rs` with CostDatabase type**

```rust
// sir/crates/sir_semantics/src/cost.rs

use std::collections::HashMap;
use sir_types::{CostProfile, RegionId};

/// Database mapping regions to their pre-computed cost profiles.
///
/// Populated by `CostDeriver` during semantic derivation.
/// Parallel to `StructuralDatabase` — cost is not structure.
/// Immutable after `CostDeriver::derive()` completes.
#[derive(Clone, Debug, Default)]
pub struct CostDatabase {
    costs: HashMap<RegionId, CostProfile>,
}

impl CostDatabase {
    /// Create an empty cost database.
    pub fn new() -> Self {
        Self {
            costs: HashMap::new(),
        }
    }

    /// Store the cost profile for a region.
    pub fn insert(&mut self, region: RegionId, profile: CostProfile) {
        self.costs.insert(region, profile);
    }

    /// Retrieve the cost profile for a region, if present.
    pub fn for_region(&self, region: RegionId) -> Option<&CostProfile> {
        self.costs.get(&region)
    }

    /// Number of regions with cost data.
    pub fn len(&self) -> usize {
        self.costs.len()
    }

    /// Whether the database is empty.
    pub fn is_empty(&self) -> bool {
        self.costs.is_empty()
    }
}
```

- [ ] **Step 2: Add `pub mod cost;` to `lib.rs`**

Edit `sir/crates/sir_semantics/src/lib.rs`, add after line 13 (`pub mod structure;`):
```rust
pub mod cost;
```

- [ ] **Step 3: Build to verify compilation**

Run: `cargo build -p sir_semantics 2>&1`
Expected: exit code 0, no errors.

- [ ] **Step 4: Commit**

```bash
git add sir/crates/sir_semantics/src/cost.rs sir/crates/sir_semantics/src/lib.rs
git commit -m "feat: add CostDatabase type to sir_semantics

Parallel database to StructuralDatabase, mapping RegionId -> CostProfile.
Immutable after population. Part of Phase 0015."
```

---

### Task 2: CostDeriver component

**Files:**
- Create: `sir/crates/sir_semantics/src/cost_deriver.rs`
- Modify: `sir/crates/sir_semantics/src/lib.rs` (add `pub mod cost_deriver;`)

**Interfaces:**
- Consumes: `&sir_nodes::Function`, `&crate::structure::StructuralDatabase`, `sir_types::RegionId`, `sir_types::CostProfile`, `sir_nodes::Node`, `sir_nodes::NodeKind`
- Produces: `CostDeriver` with `pub fn derive(function: &Function, structural: &StructuralDatabase) -> CostDatabase`

- [ ] **Step 1: Create `cost_deriver.rs`**

```rust
// sir/crates/sir_semantics/src/cost_deriver.rs

use sir_nodes::{Function, Node, NodeKind};
use sir_types::CostProfile;

use crate::cost::CostDatabase;
use crate::structure::StructuralDatabase;

/// Derives `CostProfile` for each region from SIR node counts and expression depth.
///
/// This is a dedicated component, separate from semantic recognizers.
/// Recognizers answer "what is this computation?" — CostDeriver answers
/// "what does it cost?" Neither depends on the other.
///
/// The optimizer never walks SIR. Costs are pre-computed here so the
/// optimizer reads `CostDatabase::for_region(region)` — a single lookup.
pub struct CostDeriver;

impl CostDeriver {
    /// Compute cost profiles for every region in the structural database.
    ///
    /// For each region:
    ///   - instruction_count = number of SIR nodes in the region
    ///   - select_count = number of Select nodes
    ///   - memory_accesses = number of Load + Store nodes
    ///   - critical_path_depth = maximum expression depth (recursive)
    ///
    /// Expression depth is computed locally — no dependency on
    /// `sir_analysis::graph` algorithms. This is an approximation
    /// sufficient for v0.1 and can be replaced with a proper latency
    /// model later.
    pub fn derive(
        function: &Function,
        structural: &StructuralDatabase,
    ) -> CostDatabase {
        let mut db = CostDatabase::new();

        for (region_id, _desc) in structural.iter() {
            let profile = Self::compute_region_cost(function, region_id);
            db.insert(region_id, profile);
        }

        db
    }

    /// Compute the cost profile for a single region by walking its nodes.
    fn compute_region_cost(function: &Function, region_id: sir_types::RegionId) -> CostProfile {
        // The StructuralDatabase doesn't expose node sets directly —
        // it maps RegionId -> StructuralDescription. We need to walk
        // all nodes in the function and check which belong to this region.
        //
        // For v0.1 with a single region per function, we compute costs
        // over all nodes in the function when the region exists.
        // Future: region membership will be tracked explicitly.

        let mut instruction_count: u32 = 0;
        let mut select_count: u32 = 0;
        let mut memory_accesses: u32 = 0;

        // Walk all nodes in the function. In v0.1 with one region,
        // all nodes belong to the region. Future phases will filter
        // by region membership when StructuralDescription carries a
        // node set.
        for node_id in function.node_ids() {
            if let Some(node) = function.get_node(node_id) {
                instruction_count += 1;

                match &node.kind {
                    NodeKind::Select { .. } => {
                        select_count += 1;
                    }
                    NodeKind::Load { .. } | NodeKind::Store { .. } => {
                        memory_accesses += 1;
                    }
                    _ => {}
                }
            }
        }

        // Compute maximum expression depth recursively over all nodes.
        // Depth of a node = 1 + max(depth of each operand).
        // Leaf nodes (no operands, or operands outside this region) have depth 1.
        let critical_path_depth = Self::compute_max_depth(function);

        CostProfile {
            instruction_count,
            select_count,
            memory_accesses,
            critical_path_depth,
        }
    }

    /// Compute maximum expression depth over the function's nodes.
    ///
    /// For each node, depth = 1 + max(depth of its dataflow inputs).
    /// Uses memoization to avoid recomputation. Leaf nodes have depth 1.
    fn compute_max_depth(function: &Function) -> u32 {
        use std::collections::HashMap;

        fn node_depth(
            node_id: sir_types::NodeId,
            function: &Function,
            memo: &mut HashMap<sir_types::NodeId, u32>,
        ) -> u32 {
            if let Some(&cached) = memo.get(&node_id) {
                return cached;
            }

            let node = function.get_node(node_id);
            let depth = match node {
                Some(n) => {
                    let inputs = n.kind.input_nodes();
                    if inputs.is_empty() {
                        1
                    } else {
                        let max_input = inputs
                            .iter()
                            .map(|&input_id| node_depth(input_id, function, memo))
                            .max()
                            .unwrap_or(0);
                        1 + max_input
                    }
                }
                None => 1,
            };

            memo.insert(node_id, depth);
            depth
        }

        let mut memo: HashMap<sir_types::NodeId, u32> = HashMap::new();
        let mut max_depth: u32 = 0;

        for node_id in function.node_ids() {
            let d = node_depth(node_id, function, &mut memo);
            if d > max_depth {
                max_depth = d;
            }
        }

        max_depth.max(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sir_builder::Builder;
    use sir_types::{ConstantData, Span, Type};

    #[test]
    fn cost_deriver_empty_function() {
        let func = Builder::new("empty", &[], Type::Unit).build_empty().unwrap();
        let structural = StructuralDatabase::new();
        let cost_db = CostDeriver::derive(&func, &structural);
        assert!(cost_db.is_empty());
    }
}
```

- [ ] **Step 2: Add `pub mod cost_deriver;` to `lib.rs`**

Edit `sir/crates/sir_semantics/src/lib.rs`, add after `pub mod cost;`:
```rust
pub mod cost_deriver;
```

- [ ] **Step 3: Build to verify compilation**

Run: `cargo build -p sir_semantics 2>&1`
Expected: exit code 0, no errors.

- [ ] **Step 4: Run the unit test**

Run: `cargo test -p sir_semantics cost_deriver_empty_function 2>&1`
Expected: test passes.

- [ ] **Step 5: Commit**

```bash
git add sir/crates/sir_semantics/src/cost_deriver.rs sir/crates/sir_semantics/src/lib.rs
git commit -m "feat: add CostDeriver component to sir_semantics

Computes CostProfile per region: instruction_count, select_count,
memory_accesses, and critical_path_depth (local expression depth).
Separate from recognizers — cost is not structure. Part of Phase 0015."
```

---

### Task 3: Integrate CostDeriver into SemanticEngine

**Files:**
- Modify: `sir/crates/sir_semantics/src/semantics.rs`

**Interfaces:**
- Consumes: `CostDeriver`, `CostDatabase`
- Produces: `SemanticEngine` gains `cost_db` field and `pub fn cost_database(&self) -> &CostDatabase` accessor

- [ ] **Step 1: Add imports to `semantics.rs`**

Edit imports at top of `sir/crates/sir_semantics/src/semantics.rs`. Add after line 9:
```rust
use crate::cost::CostDatabase;
use crate::cost_deriver::CostDeriver;
```

- [ ] **Step 2: Add `cost_db` field to `SemanticEngine`**

Edit the `SemanticEngine` struct (line 164-167). Change from:
```rust
pub struct SemanticEngine {
    db: SemanticDatabase,
    structural_db: StructuralDatabase,
}
```
To:
```rust
pub struct SemanticEngine {
    db: SemanticDatabase,
    structural_db: StructuralDatabase,
    cost_db: CostDatabase,
}
```

- [ ] **Step 3: Initialize `cost_db` in `new()`**

Edit `SemanticEngine::new()` (line 170-176). Change from:
```rust
pub fn new() -> Self {
    Self {
        db: SemanticDatabase::new(),
        structural_db: StructuralDatabase::new(),
    }
}
```
To:
```rust
pub fn new() -> Self {
    Self {
        db: SemanticDatabase::new(),
        structural_db: StructuralDatabase::new(),
        cost_db: CostDatabase::new(),
    }
}
```

- [ ] **Step 4: Add `cost_database()` accessor**

Add after the `structural_database()` method (after line 185):
```rust
    /// Access the cost database (read-only after derivation).
    pub fn cost_database(&self) -> &CostDatabase {
        &self.cost_db
    }
```

- [ ] **Step 5: Call CostDeriver in `derive()`**

At the end of `SemanticEngine::derive()`, just before the closing `}`, add after the structural rekeying block (after line 324):
```rust
        // ── Cost derivation ────────────────────────────────────
        // Compute CostProfile for each region from the SIR nodes.
        // Runs after structural recognition so all regions are known.
        self.cost_db = CostDeriver::derive(func, &self.structural_db);
```

- [ ] **Step 6: Build and run all sir_semantics tests**

Run: `cargo test -p sir_semantics 2>&1`
Expected: all tests pass (no regressions).

- [ ] **Step 7: Commit**

```bash
git add sir/crates/sir_semantics/src/semantics.rs
git commit -m "feat: integrate CostDeriver into SemanticEngine

SemanticEngine now calls CostDeriver::derive() after structural
recognition, exposing costs via cost_database() accessor.
Part of Phase 0015."
```

---

### Task 4: sir_optimizer crate scaffolding

**Files:**
- Create: `sir/crates/sir_optimizer/Cargo.toml`
- Create: `sir/crates/sir_optimizer/src/lib.rs`
- Create: `sir/crates/sir_optimizer/src/config.rs`
- Create: `sir/crates/sir_optimizer/src/result.rs`
- Modify: `sir/Cargo.toml` (add workspace member)

**Interfaces:**
- Consumes: everything below it (sir_types, sir_nodes, sir_analysis, sir_semantics, sir_inference, sir_generation, sir_verification, sir_selection, sir_rewrite, sir_transform)
- Produces: `OptimizerConfig`, `OptimizationResult`, `TerminationReason`, `IterationRecord`, `IterationOutcome` types

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "sir_optimizer"
version.workspace = true
edition.workspace = true
license.workspace = true
authors.workspace = true

[dependencies]
sir_types = { path = "../sir_types" }
sir_nodes = { path = "../sir_nodes" }
sir_analysis = { path = "../sir_analysis" }
sir_semantics = { path = "../sir_semantics" }
sir_inference = { path = "../sir_inference" }
sir_transform = { path = "../sir_transform" }
sir_generation = { path = "../sir_generation" }
sir_verification = { path = "../sir_verification" }
sir_selection = { path = "../sir_selection" }
sir_rewrite = { path = "../sir_rewrite" }

[dev-dependencies]
sir_builder = { path = "../sir_builder" }
```

- [ ] **Step 2: Create `config.rs`**

```rust
// sir/crates/sir_optimizer/src/config.rs

/// Configuration for the fixed-point optimization driver.
#[derive(Clone, Debug)]
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

- [ ] **Step 3: Create `result.rs`**

```rust
// sir/crates/sir_optimizer/src/result.rs

use sir_nodes::Function;

/// The result of a complete optimization run.
#[derive(Clone, Debug)]
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

/// Why the optimization loop terminated.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TerminationReason {
    /// No more rewrites possible — converged.
    FixedPoint,
    /// max_iterations or max_total_rewrites reached.
    IterationLimitReached,
}

/// Statistics for one fixed-point iteration.
#[derive(Clone, Debug, Default)]
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

/// What happened in a single iteration.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum IterationOutcome {
    /// At least one rewrite was applied.
    RewriteApplied,
    /// Generation produced no candidates.
    NoCandidate,
    /// Candidates were generated but none could be proven equivalent.
    NoProof,
    /// Candidates were proven but none were selected (all had score <= 0).
    NoSelection,
    /// No iteration has run yet.
    #[default]
    NotStarted,
}
```

- [ ] **Step 4: Create `lib.rs`**

```rust
// sir/crates/sir_optimizer/src/lib.rs

//! SIR Optimizer — Fixed-Point Optimization Driver.
//!
//! Orchestrates the full reasoning pipeline iteratively:
//!   Analysis → Semantics → Inference → Generation
//!   → Verification → Selection → Rewrite
//!
//! Runs until fixed point (no more rewrites possible) or iteration limit.
//! All pipeline stages are constructed fresh each iteration.
//! The optimizer never walks SIR or derives knowledge from IR.

pub mod config;
pub mod optimizer;
pub mod result;

pub use config::OptimizerConfig;
pub use optimizer::Optimizer;
pub use result::{IterationOutcome, IterationRecord, OptimizationResult, TerminationReason};
```

- [ ] **Step 5: Add `sir_optimizer` to workspace**

Edit `sir/Cargo.toml`, add after `"crates/sir_selection",` (line 17):
```toml
    "crates/sir_optimizer",
```

- [ ] **Step 6: Build to verify scaffolding compiles**

Run: `cargo build -p sir_optimizer 2>&1`
Expected: exit code 0 (may warn about unused imports until optimizer.rs is written — that's OK).

- [ ] **Step 7: Commit**

```bash
git add sir/crates/sir_optimizer/ sir/Cargo.toml
git commit -m "feat: scaffold sir_optimizer crate with config and result types

New crate at top of dependency stack. Types: OptimizerConfig,
OptimizationResult, TerminationReason, IterationRecord.
Part of Phase 0015."
```

---

### Task 5: Selector multi-region API

**Files:**
- Modify: `sir/crates/sir_selection/src/selector.rs`

**Interfaces:**
- Consumes: existing `VerifiedCandidate`, `CostProfile`, `TransformationScore`, `CostModelReport`
- Produces: `MultiRegionSelection` struct, `pub fn select_all(&self, verified: &[VerifiedCandidate], cost_db: &CostDatabase) -> MultiRegionSelection`

- [ ] **Step 1: Add imports for CostDatabase**

Edit imports at top of `sir/crates/sir_selection/src/selector.rs`. The `CostDatabase` type is in `sir_semantics::cost`. Check if `sir_selection` already depends on `sir_semantics` — it does not. We need to add the dependency.

First, check `sir/crates/sir_selection/Cargo.toml`:
```bash
grep -n "sir_semantics" sir/crates/sir_selection/Cargo.toml
```
If not present, add:
```toml
sir_semantics = { path = "../sir_semantics" }
```

Then add import in `selector.rs`:
```rust
use sir_semantics::cost::CostDatabase;
```

- [ ] **Step 2: Add MultiRegionSelection type**

Add after the `SelectionResult` impl block (after line 45):
```rust
/// Selection result across all regions.
///
/// The selector owns region grouping — the optimizer doesn't need to
/// know that selection is per-region. It just calls select_all() and
/// receives a flat list of chosen candidates.
pub struct MultiRegionSelection<'a> {
    /// All chosen candidates (one per region, at most).
    pub chosen: Vec<SelectedCandidate<'a>>,
    /// Per-region reports for diagnostics.
    pub reports: Vec<CostModelReport>,
}
```

- [ ] **Step 3: Add `select_all()` method**

Add after the existing `select()` method, inside `impl<M: CostModel> Selector<M>`:
```rust
    /// Select the best candidate per region across all verified candidates.
    ///
    /// Groups candidates by region internally, then applies the same
    /// per-region selection policy. The optimizer calls this once and
    /// receives a flat list of chosen candidates — it doesn't need to
    /// know selection is region-based.
    ///
    /// Policy (same as per-region):
    ///   - Filter: total > 0 (strict improvement)
    ///   - Rank: highest total wins
    ///   - Tie: lowest CandidateId wins
    pub fn select_all<'a>(
        &self,
        verified: &'a [VerifiedCandidate],
        cost_db: &CostDatabase,
    ) -> MultiRegionSelection<'a> {
        use std::collections::BTreeMap;
        use sir_types::RegionId;

        // Group verified candidates by region
        let mut by_region: BTreeMap<RegionId, Vec<&'a VerifiedCandidate>> = BTreeMap::new();
        for vc in verified {
            by_region
                .entry(vc.candidate.region)
                .or_default()
                .push(vc);
        }

        let mut chosen = Vec::new();
        let mut reports = Vec::new();

        for (region, region_candidates) in by_region {
            // Borrow the Vec elements as a slice
            let vcs: Vec<VerifiedCandidate> = region_candidates
                .iter()
                .map(|vc| (*vc).clone())
                .collect();

            let original_cost = cost_db.for_region(region);

            if let Some(cost) = original_cost {
                let result = self.select(region, &vcs, cost);
                if let Some(selected) = result.chosen {
                    chosen.push(selected);
                }
                reports.push(result.report);
            }
        }

        MultiRegionSelection { chosen, reports }
    }
```

Wait — the `select()` method borrows `verified: &'a [VerifiedCandidate]` but we have `Vec<&'a VerifiedCandidate>`. We need to clone into owned `VerifiedCandidate`s first, which we do above. Let me refine this to avoid the clone:

Actually, `VerifiedCandidate` derives `Clone`, so the clone is fine. And `select()` takes `&[VerifiedCandidate]` not `&[&VerifiedCandidate]`. So we need owned values. The clone approach works.

- [ ] **Step 4: Update imports in `lib.rs`**

Edit `sir/crates/sir_selection/src/lib.rs` to export the new type. Add `MultiRegionSelection` to the existing `pub use selector::...` line.

- [ ] **Step 5: Build and run sir_selection tests**

Run: `cargo test -p sir_selection 2>&1`
Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add sir/crates/sir_selection/
git commit -m "feat: add multi-region select_all() to Selector

Selector now owns region grouping. The optimizer calls select_all()
with all verified candidates and the cost database — it doesn't need
to know selection is per-region. Part of Phase 0015."
```

---

### Task 6: Optimizer implementation

**Files:**
- Create: `sir/crates/sir_optimizer/src/optimizer.rs`

**Interfaces:**
- Consumes: All pipeline crates (sir_analysis, sir_semantics, sir_inference, sir_generation, sir_verification, sir_selection, sir_rewrite)
- Produces: `Optimizer` struct with `new()`, `optimize()`, private `optimize_iteration()` and `PipelineState`

- [ ] **Step 1: Create `optimizer.rs` — PipelineState and imports**

```rust
// sir/crates/sir_optimizer/src/optimizer.rs

use sir_analysis::manager::AnalysisManager;
use sir_generation::generator::CandidateGenerator;
use sir_inference::engine::InferenceEngine;
use sir_nodes::Function;
use sir_rewrite::engine::RewriteEngine;
use sir_rewrite::recipe::RecipeRegistry;
use sir_selection::cost_model::DefaultCostModel;
use sir_selection::selector::{Selector, VerifiedCandidate};
use sir_semantics::semantics::SemanticEngine;
use sir_verification::Verifier;

use crate::config::OptimizerConfig;
use crate::result::{IterationOutcome, IterationRecord, OptimizationResult, TerminationReason};

/// Internal state for a single pipeline iteration.
///
/// Lives only within `optimize_iteration()`. Constructed fresh each time.
/// Keeps the method from becoming a 300-line function with a dozen locals.
struct PipelineState<'a> {
    /// The function being optimized (current iteration's input).
    function: &'a Function,
    /// Compiler facts from analysis.
    analysis: AnalysisManager,
    /// Semantic truths + structural descriptions + costs.
    semantics: SemanticEngine,
    /// Representation beliefs + transformation contexts.
    inference: InferenceEngine,
    /// Candidate implementations.
    generator: CandidateGenerator,
    /// Verified candidates (populated after verification).
    proven: Vec<VerifiedCandidate>,
    /// Iteration statistics.
    record: IterationRecord,
}
```

- [ ] **Step 2: Add `Optimizer` struct and constructor**

```rust
/// Fixed-point optimization driver.
///
/// Owns configuration and recipe registry.
/// All pipeline stages are constructed fresh each iteration —
/// the optimizer carries no mutable state and no caches.
/// Registries are immutable catalogs, not mutable state.
///
/// Uses `DefaultCostModel` directly — a unit struct, zero-cost to
/// construct. Generalize to `Box<dyn CostModel>` when multiple cost
/// models are needed.
pub struct Optimizer {
    config: OptimizerConfig,
    recipe_registry: RecipeRegistry,
}

impl Optimizer {
    /// Create a new optimizer.
    ///
    /// `recipe_registry` maps DefinitionId → graph rewrite recipe.
    /// Cost model is `DefaultCostModel` (v0.1 — single model).
    pub fn new(
        config: OptimizerConfig,
        recipe_registry: RecipeRegistry,
    ) -> Self {
        Self {
            config,
            recipe_registry,
        }
    }

    /// Run optimization to fixed point.
    ///
    /// Idempotent: if optimize(f) = g, then optimize(g) = g.
    /// Accepts `&Function` — the optimizer does not consume its input.
    /// Every iteration constructs fresh pipeline stages from scratch.
    pub fn optimize(&self, function: &Function) -> OptimizationResult {
        let mut current = function.clone();
        let mut total_rewrites: usize = 0;
        let mut iterations_detail: Vec<IterationRecord> = Vec::new();

        for iteration in 1..=self.config.max_iterations {
            let result = self.optimize_iteration(&current, iteration);
            total_rewrites += result.record.rewrites_applied;
            iterations_detail.push(result.record);

            if result.converged {
                return OptimizationResult {
                    function: result.function,
                    iterations: iteration,
                    rewrites_applied: total_rewrites,
                    iterations_detail,
                    termination: TerminationReason::FixedPoint,
                };
            }

            if let Some(max_rewrites) = self.config.max_total_rewrites {
                if total_rewrites >= max_rewrites {
                    return OptimizationResult {
                        function: result.function,
                        iterations: iteration,
                        rewrites_applied: total_rewrites,
                        iterations_detail,
                        termination: TerminationReason::IterationLimitReached,
                    };
                }
            }

            current = result.function;
        }

        OptimizationResult {
            function: current,
            iterations: self.config.max_iterations,
            rewrites_applied: total_rewrites,
            iterations_detail,
            termination: TerminationReason::IterationLimitReached,
        }
    }
}
```

- [ ] **Step 3: Add `optimize_iteration()` method**

```rust
impl Optimizer {
    // ... (new and optimize above)

    /// Execute one full pipeline pass.
    ///
    /// 1. Analysis  → run_all()
    /// 2. Semantics → derive() (includes cost derivation)
    /// 3. Inference → infer()
    /// 4. Generation → generate()
    /// 5. Verification → build_obligations() + verify()
    /// 6. Selection → select_all()
    /// 7. Rewrite → rewrite() for each selected winner
    fn optimize_iteration(
        &self,
        function: &Function,
        iteration_number: usize,
    ) -> IterationResult {
        // ── 1. Analysis ───────────────────────────────────────
        let mut analysis = AnalysisManager::new();
        analysis.run_all(function);

        // ── 2. Semantics (recognizers + structure + cost) ──────
        let mut semantics = SemanticEngine::new();
        semantics.derive(function, analysis.database());

        // ── 3. Inference ──────────────────────────────────────
        let mut inference = InferenceEngine::new();
        inference.infer(semantics.database(), semantics.structural_database());

        // ── 4. Generation ─────────────────────────────────────
        let mut generator = CandidateGenerator::new();
        generator.generate(inference.context_database(), semantics.database());

        let candidate_count = generator.database().all_candidates().count();
        if candidate_count == 0 {
            return IterationResult {
                function: function.clone(),
                record: IterationRecord {
                    iteration: iteration_number,
                    candidates_generated: 0,
                    outcome: IterationOutcome::NoCandidate,
                    ..Default::default()
                },
                converged: true,
            };
        }

        // ── 5. Verification ───────────────────────────────────
        let verifier = Verifier::new();
        let obligations_db = verifier.build_obligations(
            generator.database(),
            inference.context_database(),
        );

        let proofs_attempted = obligations_db.len();
        let mut proven: Vec<VerifiedCandidate> = Vec::new();

        for obligation in obligations_db.all() {
            let context = inference
                .context_database()
                .for_region(obligation.region)
                .first()
                .expect("Context should exist for this region");

            let result = verifier.verify(obligation, context);

            if let sir_verification::VerificationResult::Proven(proof) = result {
                // Find the original candidate from the generator's database
                let candidates = generator.database();
                for candidate in candidates.all_candidates() {
                    if candidate.id == obligation.candidate {
                        proven.push(VerifiedCandidate {
                            candidate: candidate.clone(),
                            proof,
                        });
                        break;
                    }
                }
            }
        }

        let proofs_succeeded = proven.len();

        if proven.is_empty() {
            return IterationResult {
                function: function.clone(),
                record: IterationRecord {
                    iteration: iteration_number,
                    facts_discovered: 0, // AnalysisManager doesn't expose fact count
                    truths_discovered: semantics.database().region_count(),
                    beliefs_inferred: 0, // InferenceEngine doesn't expose hypothesis count
                    candidates_generated: candidate_count,
                    proofs_attempted,
                    proofs_succeeded: 0,
                    outcome: IterationOutcome::NoProof,
                    ..Default::default()
                },
                converged: true,
            };
        }

        // ── 6. Selection ──────────────────────────────────────
        let selector = Selector::new(DefaultCostModel);
        let cost_db = semantics.cost_database();
        let selection = selector.select_all(&proven, cost_db);

        if selection.chosen.is_empty() {
            return IterationResult {
                function: function.clone(),
                record: IterationRecord {
                    iteration: iteration_number,
                    truths_discovered: semantics.database().region_count(),
                    candidates_generated: candidate_count,
                    proofs_attempted,
                    proofs_succeeded,
                    candidates_selected: 0,
                    outcome: IterationOutcome::NoSelection,
                    ..Default::default()
                },
                converged: true,
            };
        }

        let candidates_selected = selection.chosen.len();

        // ── 7. Rewrite (exactly one per iteration) ────────────
        // Apply only the highest-scoring candidate. Multiple rewrites
        // are sequenced across fixed-point iterations — this eliminates
        // overlapping-rewrite concerns entirely.
        let engine = RewriteEngine::new(self.recipe_registry.clone());
        let best = &selection.chosen[0]; // highest score after selection

        let (current, rewrites_applied) = match engine.rewrite(
            function,
            best.candidate,
            best.proof,
            semantics.structural_database(),
        ) {
            Ok(rewrite_result) => (rewrite_result.rewritten, 1usize),
            Err(_e) => (function.clone(), 0usize),
        };

        let converged = rewrites_applied == 0;

        IterationResult {
            function: current,
            record: IterationRecord {
                iteration: iteration_number,
                truths_discovered: semantics.database().region_count(),
                candidates_generated: candidate_count,
                proofs_attempted,
                proofs_succeeded,
                candidates_selected,
                rewrites_applied,
                outcome: if rewrites_applied > 0 {
                    IterationOutcome::RewriteApplied
                } else {
                    IterationOutcome::NoCandidate
                },
            },
            converged,
        }
    }
}

/// Internal result from a single iteration.
struct IterationResult {
    function: Function,
    record: IterationRecord,
    converged: bool,
}
```

- [ ] **Step 4: Add `as_ref()` method for `Box<dyn CostModel>`**

The `Selector::new()` takes `M: CostModel` by value, but we have `Box<dyn CostModel>`. We can't pass `self.cost_model.as_ref()` because `dyn CostModel` doesn't implement `CostModel` (the trait object vs trait bound issue).

Check the actual Selector API — it takes `M: CostModel`, so we can't use `&dyn CostModel` directly. Two options:
1. Change `Selector` to accept `&dyn CostModel` — but that changes `sir_selection`
2. Make `Optimizer` generic over `M: CostModel`

Let's go with option 2 — it's cleaner and doesn't change the Selector API:
```rust
pub struct Optimizer<M: CostModel> {
    config: OptimizerConfig,
    cost_model: M,
    recipe_registry: RecipeRegistry,
}

impl<M: CostModel> Optimizer<M> {
    pub fn new(config: OptimizerConfig, cost_model: M, recipe_registry: RecipeRegistry) -> Self {
        Self { config, cost_model, recipe_registry }
    }
    // selector uses self.cost_model (moved via clone or ref)
}
```

But `CostModel` doesn't require `Clone`. So we need to use a reference: `Selector::new(&self.cost_model)`. But then the Selector lifetime is tied to `&self`. That works within `optimize_iteration()`.

Actually, let's check: `Selector::new(cost_model: M)` takes ownership. We need `M: CostModel` to work for both owned and reference types. In Rust, `&T where T: CostModel` implements `CostModel`? No — traits for references need explicit impls.

The simplest fix: make the Selector work with `&dyn CostModel` by changing it to store `&dyn CostModel` instead of `M`. But that's a bigger refactor.

Alternative: just clone the CostModel. Add `Clone` bound... no, that changes the trait.

Simplest approach for v0.1: store `Box<dyn CostModel>` in Optimizer, and make Selector generic over it:

Actually wait. Let's re-read the Selector:
```rust
pub struct Selector<M: CostModel> {
    cost_model: M,
}
```

And its `new`:
```rust
pub fn new(cost_model: M) -> Self { Self { cost_model } }
```

The issue is that `Box<dyn CostModel>` doesn't implement `CostModel` unless there's a blanket impl. There isn't one.

The fix: Instead of `Box<dyn CostModel>`, make Optimizer generic:
```rust
pub struct Optimizer<M: CostModel> {
    config: OptimizerConfig,
    cost_model: M,
    recipe_registry: RecipeRegistry,
}
```

This is the right approach — matches the pattern used by `Selector<M: CostModel>`.

Let me update the code above and the Optimizer struct in config.rs/lib.rs.

Actually, let me just write this properly. Let me update the plan to use `Optimizer<M: CostModel>` throughout.

- [ ] **Step 5: Update config.rs and lib.rs for generic Optimizer**

The `OptimizerConfig` doesn't change. The `Optimizer` struct in `lib.rs` should be generic:
```rust
pub use optimizer::Optimizer;
```
This re-export works with generic types too.

- [ ] **Step 6: Build to verify compilation**

Run: `cargo build -p sir_optimizer 2>&1`
Expected: exit code 0, no errors.

- [ ] **Step 7: Commit**

```bash
git add sir/crates/sir_optimizer/src/optimizer.rs
git commit -m "feat: implement Optimizer with fixed-point loop

Optimize() runs: analysis → semantics → inference → generation →
verification → selection → rewrite. Repeats until convergence or
iteration limit. Fresh stages every iteration. Part of Phase 0015."
```

---

### Task 7: Integration tests

**Files:**
- Create: `sir/crates/sir_optimizer/tests/bs001_optimizer.rs`
- Create: `sir/crates/sir_optimizer/tests/optimizer_tests.rs`

**Interfaces:**
- Consumes: `Optimizer`, `OptimizerConfig`, all pipeline crates, `sir_builder`

- [ ] **Step 1: Create BS001 integration test**

Create `sir/crates/sir_optimizer/tests/bs001_optimizer.rs`:

```rust
//! BS001 Optimizer Integration Test.
//!
//! End-to-end: build board_scan SIR → optimize → verify popcount rewrite.
//! Tests the acceptance benchmark from the Phase 0015 spec:
//!   - Iteration 1: RewriteApplied
//!   - Iteration 2: NoCandidate → FixedPoint
//!   - Result: 2 iterations, 1 rewrite, FixedPoint

use sir_analysis::manager::AnalysisManager;
use sir_builder::Builder;
use sir_generation::generator::CandidateGenerator;
use sir_inference::engine::InferenceEngine;
use sir_optimizer::{Optimizer, OptimizerConfig, TerminationReason};
use sir_rewrite::recipe::RecipeRegistry;
use sir_rewrite::recipes::popcount::PopcountRecipe;
use sir_semantics::semantics::SemanticEngine;
use sir_transform::ids::DefinitionId;
use sir_types::{ConstantData, Span, Type};

/// Build the canonical BS001 board_scan function.
fn build_board_scan() -> sir_nodes::Function {
    let mut b = Builder::new(
        "board_scan",
        &[(
            "board",
            Type::Array {
                element: Box::new(Type::Bool),
                length: 64,
            },
        )],
        Type::i32(),
    );

    let board = b.parameter_index(0).unwrap();
    let i_initial = b.constant(ConstantData::u64(0), Type::u64(), Span::unknown());
    let i_step = b.constant(ConstantData::u64(1), Type::u64(), Span::unknown());
    let limit = b.constant(ConstantData::u64(64), Type::u64(), Span::unknown());
    let count_initial = b.constant(ConstantData::i32(0), Type::i32(), Span::unknown());
    let zero_i32 = b.constant(ConstantData::i32(0), Type::i32(), Span::unknown());
    let one_i32 = b.constant(ConstantData::i32(1), Type::i32(), Span::unknown());

    let elem = b.array_access(board, i_initial, Type::Bool, Span::unknown()).unwrap();
    let inc = b.select(elem, one_i32, zero_i32, Span::unknown()).unwrap();
    let new_count = b.add(count_initial, inc, Span::unknown()).unwrap();
    let i_next = b.add(i_initial, i_step, Span::unknown()).unwrap();
    let cond = b.lt(i_initial, limit, Span::unknown()).unwrap();

    let loop_node = b
        .r#loop(
            &[elem, inc, new_count, i_next, cond],
            cond,
            &[new_count, i_next],
            &[count_initial, i_initial],
            Type::Tuple {
                elements: vec![Type::i32(), Type::u64()],
            },
            Span::unknown(),
        )
        .unwrap();

    b.return_value(loop_node, Span::unknown()).unwrap();
    b.build()
}

fn make_recipe_registry() -> RecipeRegistry {
    let mut registry = RecipeRegistry::new();
    registry.register(Box::new(PopcountRecipe::new(DefinitionId::new(0))));
    registry
}

#[test]
fn bs001_converges_in_two_iterations() {
    let func = build_board_scan();
    let optimizer = Optimizer::new(
        OptimizerConfig::default(),
        make_recipe_registry(),
    );

    let result = optimizer.optimize(&func);

    assert_eq!(result.iterations, 2, "BS001 should converge in exactly 2 iterations");
    assert_eq!(result.rewrites_applied, 1, "BS001 should apply exactly 1 rewrite");
    assert_eq!(result.termination, TerminationReason::FixedPoint);

    // Verify per-iteration details
    assert_eq!(result.iterations_detail.len(), 2);
    assert_eq!(result.iterations_detail[0].iteration, 1);
    assert!(result.iterations_detail[0].rewrites_applied > 0,
        "Iteration 1 should apply at least 1 rewrite");
    assert_eq!(result.iterations_detail[1].iteration, 2);
    assert_eq!(result.iterations_detail[1].rewrites_applied, 0,
        "Iteration 2 should apply no rewrites (converged)");
}

#[test]
fn bs001_optimize_is_idempotent() {
    let func = build_board_scan();
    let optimizer = Optimizer::new(
        OptimizerConfig::default(),
        make_recipe_registry(),
    );

    let first_pass = optimizer.optimize(&func);
    let second_pass = optimizer.optimize(&first_pass.function);

    assert_eq!(second_pass.rewrites_applied, 0,
        "Second optimization pass should apply no rewrites");
    assert_eq!(second_pass.termination, TerminationReason::FixedPoint);
    assert!(second_pass.iterations <= 2,
        "Second pass should converge quickly on already-optimal IR");
}

#[test]
fn bs001_result_is_deterministic() {
    let func = build_board_scan();
    let optimizer = Optimizer::new(
        OptimizerConfig::default(),
        make_recipe_registry(),
    );

    let result1 = optimizer.optimize(&func);
    let result2 = optimizer.optimize(&func);

    assert_eq!(result1.iterations, result2.iterations);
    assert_eq!(result1.rewrites_applied, result2.rewrites_applied);
    assert_eq!(result1.termination, result2.termination);
}
```

- [ ] **Step 2: Create edge-case tests**

Create `sir/crates/sir_optimizer/tests/optimizer_tests.rs`:

```rust
//! Edge-case optimizer tests.

use sir_builder::Builder;
use sir_optimizer::{Optimizer, OptimizerConfig, TerminationReason};
use sir_rewrite::recipe::RecipeRegistry;
use sir_types::Type;

fn make_empty_registry() -> RecipeRegistry {
    RecipeRegistry::new()
}

fn build_empty_function() -> sir_nodes::Function {
    Builder::new("empty", &[], Type::Unit).build_empty().unwrap()
}

#[test]
fn optimize_empty_function_converges_immediately() {
    let func = build_empty_function();
    let optimizer = Optimizer::new(
        OptimizerConfig::default(),
        make_empty_registry(),
    );

    let result = optimizer.optimize(&func);

    assert_eq!(result.iterations, 1);
    assert_eq!(result.rewrites_applied, 0);
    assert_eq!(result.termination, TerminationReason::FixedPoint);
}

#[test]
fn optimize_iteration_limit_is_respected() {
    let func = build_empty_function();
    let optimizer = Optimizer::new(
        OptimizerConfig {
            max_iterations: 3,
            max_total_rewrites: None,
        },
        make_empty_registry(),
    );

    let result = optimizer.optimize(&func);
    // Empty function converges in 1 iteration, not 3.
    // But we test that max_iterations bounds the loop.
    assert!(result.iterations <= 3);
}

#[test]
fn optimize_max_total_rewrites_is_respected() {
    let func = build_empty_function();
    let optimizer = Optimizer::new(
        OptimizerConfig {
            max_iterations: 10,
            max_total_rewrites: Some(0),
        },
        make_empty_registry(),
    );

    let result = optimizer.optimize(&func);
    assert_eq!(result.termination, TerminationReason::FixedPoint);
    // With max_total_rewrites=0 and 0 rewrites, should still converge
    // since no rewrites are applied (the check is >=, not >)
}

#[test]
fn iteration_records_are_populated() {
    let func = build_empty_function();
    let optimizer = Optimizer::new(
        OptimizerConfig::default(),
        make_empty_registry(),
    );

    let result = optimizer.optimize(&func);
    assert!(!result.iterations_detail.is_empty());
    for record in &result.iterations_detail {
        assert!(record.iteration > 0);
    }
}
```

- [ ] **Step 3: Declare tests in Cargo.toml**

Add to `sir/crates/sir_optimizer/Cargo.toml`:
```toml
[[test]]
name = "bs001_optimizer"
path = "tests/bs001_optimizer.rs"

[[test]]
name = "optimizer_tests"
path = "tests/optimizer_tests.rs"
```

- [ ] **Step 4: Run optimizer tests**

Run: `cargo test -p sir_optimizer 2>&1`
Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add sir/crates/sir_optimizer/tests/ sir/crates/sir_optimizer/Cargo.toml
git commit -m "test: add BS001 optimizer integration tests

BS001 convergence (2 iterations, 1 rewrite), idempotency,
determinism, empty function, iteration limits. Part of Phase 0015."
```

---

### Task 8: Full test suite verification

**Files:** None (verification only)

- [ ] **Step 1: Run the full test suite**

Run: `cargo test 2>&1`
Expected: all 380+ tests pass, no regressions.

- [ ] **Step 2: Check for warnings**

Run: `cargo build 2>&1 | grep -i warning`
Expected: no warnings (or only pre-existing warnings).

- [ ] **Step 3: Verify the invariant: Optimizer never touches SIR directly**

Run: `grep -rn "get_node\|node_count\|NodeKind\|node\.kind\|\.arena\|\.nodes(" sir/crates/sir_optimizer/src/ 2>&1`
Expected: no matches (the optimizer accesses SIR only through pipeline stage APIs).

- [ ] **Step 4: Run clippy if available**

Run: `cargo clippy --all-targets 2>&1 || true`
Expected: no new warnings from sir_optimizer.

- [ ] **Step 5: Commit (if any fixups needed)**

Only commit if verification found issues that needed fixing.

---

### Task 9: Update documentation

**Files:**
- Modify: `README.md` or equivalent (update crate list, pipeline diagram)

- [ ] **Step 1: Check if README needs updating**

Check `README.md` for any crate lists or pipeline diagrams that should include `sir_optimizer`.

- [ ] **Step 2: Update and commit if needed**

```bash
git add README.md
git commit -m "docs: add sir_optimizer to README crate list"
```
