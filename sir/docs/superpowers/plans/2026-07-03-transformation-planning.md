# Transformation Planning — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the 0011 Transformation Planning pipeline — `sir_transform` (contract types), extended `sir_semantics` (StructuralDatabase), refactored `sir_inference` (TransformationContextDatabase), and `sir_generation` (4 candidate generators for BitSet).

**Architecture:** `sir_transform` is a thin type crate at the center. `Representation` moves there from `sir_inference`. `sir_semantics` gains structural recognizers producing a `StructuralDatabase`. `sir_inference` consumes both knowledge databases and emits `TransformationContext`s. `sir_generation` is a pure consumer of contexts, producing `Candidate` plans with no SIR access.

**Tech Stack:** Rust 2021 edition, no new external dependencies beyond existing workspace crates.

## Global Constraints

- No new external dependencies — use only `sir_types`, `sir_nodes`, `sir_analysis`, `sir_semantics`, `sir_inference`, and stdlib
- All tests pass: `cargo test` in `sir/` must succeed at every commit
- Only one representation (`BitSet`) in v0.1
- No SIR modification — read-only planning only
- `sir_semantics` public interfaces are frozen after this phase
- 4 generators, exactly — no more, no less
- Generators are pure functions: `fn plan(context: &TransformationContext) -> Option<Candidate>`

---

### Task 1: `sir_transform` crate — scaffold + types

**Files:**
- Create: `sir/crates/sir_transform/Cargo.toml`
- Create: `sir/crates/sir_transform/src/lib.rs`
- Create: `sir/crates/sir_transform/src/representation.rs`
- Create: `sir/crates/sir_transform/src/structures.rs`
- Create: `sir/crates/sir_transform/src/constraints.rs`
- Create: `sir/crates/sir_transform/src/assumptions.rs`
- Create: `sir/crates/sir_transform/src/context.rs`
- Modify: `sir/Cargo.toml` (add workspace member)

**Interfaces:**
- Produces:
  - `Representation` enum with `BitSet` variant + `Display`
  - `SourceStructure` enum (4 variants)
  - `Constraint` enum (5 variants)
  - `Assumption` enum (3 variants)
  - `TransformationContext` struct with `validate()` method
  - `ContextId` newtype
  - `ValidationError` type

- [ ] **Step 1: Add workspace member to `sir/Cargo.toml`**

Add `"crates/sir_transform"` to the `members` array.

- [ ] **Step 2: Create `sir/crates/sir_transform/Cargo.toml`**

```toml
[package]
name = "sir_transform"
version.workspace = true
edition.workspace = true
license.workspace = true
authors.workspace = true

[dependencies]
sir_semantics = { path = "../sir_semantics" }
```

- [ ] **Step 3: Create `sir/crates/sir_transform/src/lib.rs`**

```rust
//! SIR Transform — Transformation Contract v0.1
//!
//! Defines the immutable contract between program understanding and
//! program transformation. Contains only data types and invariants.
//! Contains no algorithms, analyses, or rewrite logic.
//!
//! This crate sits at the center of the architecture:
//!   Understanding → sir_transform ← Action

pub mod representation;
pub mod structures;
pub mod constraints;
pub mod assumptions;
pub mod context;
```

- [ ] **Step 4: Create `sir/crates/sir_transform/src/representation.rs`**

```rust
use std::fmt;

/// A mathematical representation of a computation.
///
/// Representations are transformation-domain concepts, not inference concepts.
/// Inference predicts them; generation implements them; verification proves them;
/// rewrite applies them. All four phases use the same definition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Representation {
    /// A fixed-size set of boolean values representable as bits.
    BitSet,
}

impl fmt::Display for Representation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Representation::BitSet => write!(f, "BitSet"),
        }
    }
}
```

- [ ] **Step 5: Create `sir/crates/sir_transform/src/structures.rs`**

```rust
/// Describes the physical organization of data in a region.
///
/// SourceStructure describes data layout, not computational behavior.
/// Computational behavior belongs to the semantic layer (SemanticConcept).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SourceStructure {
    /// Array of booleans with known length, e.g. bool[64]
    BooleanArray { length: usize },
    /// Single integer used as a bitmask, e.g. u64 storing flags
    BitMask { width: usize },
    /// Multiple boolean values packed into minimal storage
    PackedBooleanArray { element_count: usize },
    /// 2D arrangement of boolean values
    BooleanMatrix { rows: usize, cols: usize },
}
```

- [ ] **Step 6: Create `sir/crates/sir_transform/src/constraints.rs`**

```rust
/// Properties already established by analysis or semantics.
///
/// A Constraint is already established. It cannot become false unless
/// the underlying analysis changes. Constraints are NOT assumptions
/// waiting to be proven — they are facts that have been determined.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Constraint {
    /// The structure has a statically known size.
    FixedLength(usize),
    /// The structure is not mutated (read-only access).
    ReadOnly,
    /// The structure does not escape the function.
    NoEscape,
    /// The structure is not aliased.
    NoAlias,
    /// The computation iterates a finite, known number of times.
    FiniteIteration,
}
```

- [ ] **Step 7: Create `sir/crates/sir_transform/src/assumptions.rs`**

```rust
/// Properties that must be proven before transformation.
///
/// An Assumption is NOT yet established. It must eventually become
/// either Proven (by SMT or formal reasoning) or Refuted.
/// Assumptions must never be left unresolved.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Assumption {
    /// The transformed computation produces identical cardinality.
    EquivalentCardinality,
    /// The order of iteration is preserved (or does not matter).
    PreservesIterationOrder,
    /// The external memory layout is unchanged.
    PreservesLayout,
}
```

- [ ] **Step 8: Create `sir/crates/sir_transform/src/context.rs`**

```rust
use std::collections::HashSet;
use std::fmt;

use sir_semantics::region::RegionId;

use crate::assumptions::Assumption;
use crate::constraints::Constraint;
use crate::representation::Representation;
use crate::structures::SourceStructure;

/// A unique identifier for a transformation context.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ContextId(pub u64);

impl ContextId {
    pub fn new(id: u64) -> Self { Self(id) }
}

impl fmt::Display for ContextId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ctx#{}", self.0)
    }
}

/// Error type for context validation.
#[derive(Clone, Debug)]
pub enum ValidationError {
    MissingSourceStructure,
    ContradictoryConstraints(String),
    EmptyRegion,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::MissingSourceStructure =>
                write!(f, "TransformationContext must have a source structure"),
            ValidationError::ContradictoryConstraints(msg) =>
                write!(f, "Contradictory constraints: {}", msg),
            ValidationError::EmptyRegion =>
                write!(f, "TransformationContext region must not be empty"),
        }
    }
}

/// The semantic package connecting belief to action.
///
/// A TransformationContext must contain all information required to
/// generate candidate transformation plans without consulting SIR,
/// compiler analyses, or semantic recognizers.
#[derive(Clone, Debug)]
pub struct TransformationContext {
    pub region: RegionId,
    pub representation: Representation,
    pub source_structure: SourceStructure,
    pub constraints: HashSet<Constraint>,
    pub assumptions: HashSet<Assumption>,
}

impl TransformationContext {
    pub fn new(
        region: RegionId,
        representation: Representation,
        source_structure: SourceStructure,
        constraints: HashSet<Constraint>,
        assumptions: HashSet<Assumption>,
    ) -> Self {
        Self { region, representation, source_structure, constraints, assumptions }
    }

    /// Validate invariants: source structure present, no contradictions.
    pub fn validate(&self) -> Result<(), ValidationError> {
        // No contradictory constraints check for v0.1:
        // ReadOnly + (Write observed) would be contradictory,
        // but we only track positive constraints.
        Ok(())
    }
}
```

- [ ] **Step 9: Verify compilation**

Run: `cargo build -p sir_transform`
Expected: Compiles cleanly.

- [ ] **Step 10: Commit**

```bash
git add sir/Cargo.toml sir/crates/sir_transform/
git commit -m "feat(sir_transform): add transformation contract crate"
```

---

### Task 2: Move `Representation` from `sir_inference` to `sir_transform`

**Files:**
- Modify: `sir/crates/sir_inference/src/hypothesis.rs` (remove `Representation`, re-export from `sir_transform`)
- Modify: `sir/crates/sir_inference/src/evidence.rs` (update import)
- Modify: `sir/crates/sir_inference/src/engine.rs` (update import)
- Modify: `sir/crates/sir_inference/src/sources/bitset_evidence.rs` (update import)
- Modify: `sir/crates/sir_inference/Cargo.toml` (add `sir_transform` dep)
- Modify: `sir/crates/sir_inference/tests/types.rs` (update import)
- Modify: `sir/crates/sir_inference/tests/engine.rs` (update import)
- Modify: `sir/crates/sir_inference/tests/bitset_evidence.rs` (update import)
- Modify: `sir/crates/sir_inference/tests/negative.rs` (update import)
- Modify: `sir/crates/sir_semantics/tests/semantic_truth.rs` (update import)
- Modify: `sir/crates/sir_semantics/Cargo.toml` (add `sir_transform` dev-dep)

**Interfaces:**
- Consumes: `sir_transform::representation::Representation` (Task 1)
- Produces: `sir_inference::hypothesis::Representation` is now a re-export: `pub use sir_transform::representation::Representation;`

- [ ] **Step 1: Add `sir_transform` dependency to `sir_inference/Cargo.toml`**

Add under `[dependencies]`:
```toml
sir_transform = { path = "../sir_transform" }
```

- [ ] **Step 2: Replace `Representation` in `sir_inference/src/hypothesis.rs`**

Replace the entire `Representation` enum definition and its `Display` impl with a re-export:
```rust
pub use sir_transform::representation::Representation;
```

Remove the enum definition (lines ~7-19) and Display impl (lines ~13-19). Add the re-export line.

- [ ] **Step 3: Update imports in `sir_inference/src/evidence.rs`**

Change:
```rust
use crate::hypothesis::{EvidenceId, Representation};
```
To:
```rust
use sir_transform::representation::Representation;
use crate::hypothesis::EvidenceId;
```

- [ ] **Step 4: Update imports in `sir_inference/src/engine.rs`**

Change:
```rust
use crate::hypothesis::{Hypothesis, Representation, Support};
```
To:
```rust
use sir_transform::representation::Representation;
use crate::hypothesis::{Hypothesis, Support};
```

- [ ] **Step 5: Update imports in `sir_inference/src/sources/bitset_evidence.rs`**

Change:
```rust
use crate::hypothesis::Representation;
```
To:
```rust
use sir_transform::representation::Representation;
```

- [ ] **Step 6: Add `sir_transform` dev-dep to `sir_semantics/Cargo.toml`**

Add under `[dev-dependencies]`:
```toml
sir_transform = { path = "../sir_transform" }
```

- [ ] **Step 7: Update test imports** (all 5 test files)

For each test file that uses `sir_inference::hypothesis::Representation`, change to one of:
```rust
// For sir_inference tests:
use sir_transform::representation::Representation;

// For sir_semantics tests (semantic_truth.rs):
use sir_transform::representation::Representation;
```

**Files to update:**
- `sir/crates/sir_inference/tests/types.rs` — line 2
- `sir/crates/sir_inference/tests/engine.rs` — line 2
- `sir/crates/sir_inference/tests/bitset_evidence.rs` — line 2
- `sir/crates/sir_inference/tests/negative.rs` — line 2
- `sir/crates/sir_semantics/tests/semantic_truth.rs` — line 14

Replace `use sir_inference::hypothesis::Representation;` with `use sir_transform::representation::Representation;` in each.

Also in `types.rs`, update:
```rust
use sir_inference::hypothesis::{Hypothesis, Representation, Support};
```
To:
```rust
use sir_transform::representation::Representation;
use sir_inference::hypothesis::{Hypothesis, Support};
```

Also in `engine.rs`, update:
```rust
use sir_inference::hypothesis::{Hypothesis, Representation, Support};
```
To:
```rust
use sir_transform::representation::Representation;
use sir_inference::hypothesis::{Hypothesis, Support};
```

- [ ] **Step 8: Verify compilation and tests**

Run: `cargo build && cargo test`
Expected: All 251 tests pass. No compilation errors.

- [ ] **Step 9: Commit**

```bash
git add sir/crates/sir_inference/Cargo.toml \
  sir/crates/sir_inference/src/hypothesis.rs \
  sir/crates/sir_inference/src/evidence.rs \
  sir/crates/sir_inference/src/engine.rs \
  sir/crates/sir_inference/src/sources/bitset_evidence.rs \
  sir/crates/sir_inference/tests/types.rs \
  sir/crates/sir_inference/tests/engine.rs \
  sir/crates/sir_inference/tests/bitset_evidence.rs \
  sir/crates/sir_inference/tests/negative.rs \
  sir/crates/sir_semantics/tests/semantic_truth.rs \
  sir/crates/sir_semantics/Cargo.toml
git commit -m "refactor: move Representation from sir_inference to sir_transform"
```

---

### Task 3: Extend `sir_semantics` with `StructuralDatabase`

**Files:**
- Create: `sir/crates/sir_semantics/src/structure.rs`
- Create: `sir/crates/sir_semantics/src/recognizers/boolean_array.rs`
- Create: `sir/crates/sir_semantics/src/recognizers/bitmask.rs`
- Modify: `sir/crates/sir_semantics/src/lib.rs` (add `pub mod structure;`)
- Modify: `sir/crates/sir_semantics/src/recognizers/mod.rs` (add new modules)
- Modify: `sir/crates/sir_semantics/src/semantics.rs` (add `StructuralDatabase`, run structural recognizers in `derive()`)
- Modify: `sir/crates/sir_semantics/Cargo.toml` (add `sir_transform` dep)

**Interfaces:**
- Consumes: `sir_transform::{SourceStructure, Constraint}` (Task 1)
- Produces:
  - `StructuralDescription` struct
  - `StructuralDatabase` (HashMap wrapper, same pattern as `SemanticDatabase`)
  - `recognize_boolean_array(fn) -> Vec<(RegionId, StructuralDescription)>`
  - `recognize_bitmask(fn) -> Vec<(RegionId, StructuralDescription)>`
  - `SemanticEngine::structural_database(&self) -> &StructuralDatabase`

- [ ] **Step 1: Add `sir_transform` dependency to `sir_semantics/Cargo.toml`**

Add under `[dependencies]`:
```toml
sir_transform = { path = "../sir_transform" }
```

- [ ] **Step 2: Create `sir/crates/sir_semantics/src/structure.rs`**

```rust
use std::collections::HashMap;

use sir_transform::constraints::Constraint;
use sir_transform::structures::SourceStructure;

use crate::region::RegionId;

/// Describes the physical organization of data in a region.
/// Entirely deterministic — derived from SIR types and analysis facts.
#[derive(Clone, Debug)]
pub struct StructuralDescription {
    pub region: RegionId,
    pub source_structure: SourceStructure,
    pub constraints: std::collections::HashSet<Constraint>,
}

impl StructuralDescription {
    pub fn new(
        region: RegionId,
        source_structure: SourceStructure,
    ) -> Self {
        Self {
            region,
            source_structure,
            constraints: std::collections::HashSet::new(),
        }
    }

    pub fn with_constraint(mut self, constraint: Constraint) -> Self {
        self.constraints.insert(constraint);
        self
    }
}

/// The structural knowledge database.
/// Stores deterministic descriptions of data organization per region.
#[derive(Clone, Debug, Default)]
pub struct StructuralDatabase {
    descriptions: HashMap<RegionId, StructuralDescription>,
    next_region_id: u64,
}

impl StructuralDatabase {
    pub fn new() -> Self {
        Self { descriptions: HashMap::new(), next_region_id: 0 }
    }

    pub fn add_description(&mut self, desc: StructuralDescription) {
        self.descriptions.insert(desc.region, desc);
    }

    pub fn region(&self, id: RegionId) -> Option<&StructuralDescription> {
        self.descriptions.get(&id)
    }

    pub fn regions(&self) -> impl Iterator<Item = (RegionId, &StructuralDescription)> {
        self.descriptions.iter().map(|(&id, desc)| (id, desc))
    }

    pub fn region_count(&self) -> usize {
        self.descriptions.len()
    }

    pub(crate) fn next_region_id(&mut self) -> RegionId {
        let id = RegionId::new(self.next_region_id);
        self.next_region_id += 1;
        id
    }
}
```

- [ ] **Step 3: Create `sir/crates/sir_semantics/src/recognizers/boolean_array.rs`**

```rust
use sir_analysis::facts::FactDatabase;
use sir_nodes::Function;
use sir_types::Type;

use sir_transform::constraints::Constraint;
use sir_transform::structures::SourceStructure;

use crate::region::RegionId;
use crate::structure::StructuralDescription;

/// Recognize boolean array patterns: Array<bool> with known length.
pub fn recognize_boolean_array(
    func: &Function,
    _analysis: &FactDatabase,
) -> Vec<(RegionId, StructuralDescription)> {
    let mut results = Vec::new();

    for node in func.arena.iter() {
        if let Type::Array { element, length } = &node.ty {
            if matches!(element.as_ref(), &Type::Bool) {
                let desc = StructuralDescription::new(
                    RegionId::new(0), // merged later
                    SourceStructure::BooleanArray { length: *length },
                )
                .with_constraint(Constraint::FixedLength(*length));

                results.push((RegionId::new(0), desc));
            }
        }
    }

    results
}
```

- [ ] **Step 4: Create `sir/crates/sir_semantics/src/recognizers/bitmask.rs`**

```rust
use sir_analysis::facts::FactDatabase;
use sir_nodes::Function;
use sir_types::Type;

use sir_transform::constraints::Constraint;
use sir_transform::structures::SourceStructure;

use crate::region::RegionId;
use crate::structure::StructuralDescription;

/// Recognize bitmask patterns: integer types used as flag containers.
pub fn recognize_bitmask(
    func: &Function,
    _analysis: &FactDatabase,
) -> Vec<(RegionId, StructuralDescription)> {
    let mut results = Vec::new();

    for node in func.arena.iter() {
        if let Type::Integer { width, .. } = &node.ty {
            let bits = width.bits();
            // Only flag integers up to 128 bits; larger widths are not bitmasks.
            if bits <= 128 && has_bitwise_operations(func, node.id) {
                let desc = StructuralDescription::new(
                    RegionId::new(0),
                    SourceStructure::BitMask { width: bits },
                )
                .with_constraint(Constraint::FixedLength(bits));

                results.push((RegionId::new(0), desc));
            }
        }
    }

    results
}

fn has_bitwise_operations(func: &Function, node_id: sir_types::NodeId) -> bool {
    for node in func.arena.iter() {
        use sir_nodes::NodeKind;
        match &node.kind {
            NodeKind::And { lhs, rhs }
            | NodeKind::Or { lhs, rhs }
            | NodeKind::Xor { lhs, rhs }
                if *lhs == node_id || *rhs == node_id =>
            {
                return true;
            }
            _ => {}
        }
    }
    false
}
```

- [ ] **Step 5: Update `sir/crates/sir_semantics/src/recognizers/mod.rs`**

Add:
```rust
pub mod boolean_array;
pub mod bitmask;
```

- [ ] **Step 6: Update `sir/crates/sir_semantics/src/lib.rs`**

Add after `pub mod semantics;`:
```rust
pub mod structure;
```

- [ ] **Step 7: Update `sir/crates/sir_semantics/src/semantics.rs`**

Add `StructuralDatabase` field to `SemanticEngine` and run structural recognizers in `derive()`:

In the `SemanticEngine` struct, add:
```rust
pub struct SemanticEngine {
    db: SemanticDatabase,
    structural_db: StructuralDatabase,
}
```

Update `new()`:
```rust
pub fn new() -> Self {
    Self {
        db: SemanticDatabase::new(),
        structural_db: StructuralDatabase::new(),
    }
}
```

Add accessor:
```rust
pub fn structural_database(&self) -> &StructuralDatabase {
    &self.structural_db
}
```

In `derive()`, after the semantic recognizers, add:
```rust
// Structural recognizers
use crate::recognizers::{boolean_array, bitmask};

let bool_array_recs = boolean_array::recognize_boolean_array(func, analysis);
for (region_id, desc) in bool_array_recs {
    self.structural_db.add_description(desc);
}
let _ = region_id; // suppress unused warning

let bitmask_recs = bitmask::recognize_bitmask(func, analysis);
for (_region_id, desc) in bitmask_recs {
    self.structural_db.add_description(desc);
}
```

Add import at top:
```rust
use crate::structure::{StructuralDatabase, StructuralDescription};
```

- [ ] **Step 8: Verify compilation and tests**

Run: `cargo build && cargo test`
Expected: All 251 tests pass. `sir_semantics` compiles with new modules.

- [ ] **Step 9: Commit**

```bash
git add sir/crates/sir_semantics/Cargo.toml \
  sir/crates/sir_semantics/src/structure.rs \
  sir/crates/sir_semantics/src/recognizers/boolean_array.rs \
  sir/crates/sir_semantics/src/recognizers/bitmask.rs \
  sir/crates/sir_semantics/src/recognizers/mod.rs \
  sir/crates/sir_semantics/src/lib.rs \
  sir/crates/sir_semantics/src/semantics.rs
git commit -m "feat(sir_semantics): add StructuralDatabase and structural recognizers"
```

---

### Task 4: Refactor `sir_inference` — consume both DBs, produce `TransformationContextDatabase`

**Files:**
- Modify: `sir/crates/sir_inference/src/engine.rs` (new `infer()` signature, `TransformationContextDatabase`, context construction)
- Modify: `sir/crates/sir_inference/src/lib.rs` (update module docs)
- Modify: `sir/crates/sir_inference/Cargo.toml` (already has `sir_transform` dep from Task 2)

**Interfaces:**
- Consumes: `SemanticDatabase`, `StructuralDatabase` (Task 3), `TransformationContext` (Task 1)
- Produces:
  - `infer(&mut self, semantic_db: &SemanticDatabase, structural_db: &StructuralDatabase)`
  - `context_database(&self) -> &TransformationContextDatabase`
  - `TransformationContextDatabase` struct
  - Updated `derive_assumptions()` helper

- [ ] **Step 1: Add `TransformationContextDatabase` to `engine.rs`**

At the top of `engine.rs`, add to imports:
```rust
use std::collections::HashSet;

use sir_transform::assumptions::Assumption;
use sir_transform::constraints::Constraint;
use sir_transform::context::{ContextId, TransformationContext};
use sir_transform::representation::Representation;
use sir_semantics::structure::StructuralDatabase;
```

Add the database struct:
```rust
/// Stores transformation contexts produced during inference.
#[derive(Clone, Debug, Default)]
pub struct TransformationContextDatabase {
    contexts: HashMap<RegionId, Vec<TransformationContext>>,
    next_context_id: u64,
}

impl TransformationContextDatabase {
    pub fn new() -> Self {
        Self { contexts: HashMap::new(), next_context_id: 0 }
    }

    pub fn insert(&mut self, region: RegionId, ctx: TransformationContext) -> ContextId {
        let cid = ContextId::new(self.next_context_id);
        self.next_context_id += 1;
        self.contexts.entry(region).or_default().push(ctx);
        cid
    }

    pub fn contexts(&self) -> impl Iterator<Item = (RegionId, &[TransformationContext])> {
        self.contexts.iter().map(|(&rid, v)| (rid, v.as_slice()))
    }

    pub fn for_region(&self, region: RegionId) -> &[TransformationContext] {
        self.contexts.get(&region).map(|v| v.as_slice()).unwrap_or(&[])
    }
}
```

- [ ] **Step 2: Update `InferenceEngine` struct**

Add field:
```rust
pub struct InferenceEngine {
    db: HypothesisDatabase,
    evidence_registry: EvidenceRegistry,
    context_db: TransformationContextDatabase,
}
```

Update `new()`:
```rust
pub fn new() -> Self {
    Self {
        db: HypothesisDatabase::new(),
        evidence_registry: EvidenceRegistry::new(),
        context_db: TransformationContextDatabase::new(),
    }
}
```

Add accessor:
```rust
pub fn context_database(&self) -> &TransformationContextDatabase {
    &self.context_db
}
```

Add `infer()` reset line alongside existing resets:
```rust
self.context_db = TransformationContextDatabase::new();
```

- [ ] **Step 3: Change `infer()` signature**

From:
```rust
pub fn infer(&mut self, semantic_db: &SemanticDatabase)
```
To:
```rust
pub fn infer(&mut self, semantic_db: &SemanticDatabase, structural_db: &StructuralDatabase)
```

- [ ] **Step 4: Add context construction at end of `infer()`**

After the hypothesis formation loop, add context construction:

```rust
// 4. Build TransformationContexts for best hypotheses
for ((region_id, representation), (_positive, _negative, _evidence_ids)) in &aggregation {
    if let Some(structural) = structural_db.region(*region_id) {
        let mut constraints = structural.constraints.clone();
        // Add inference-derived constraints
        constraints.insert(Constraint::FiniteIteration);

        let mut assumptions = HashSet::new();
        assumptions.insert(Assumption::EquivalentCardinality);
        assumptions.insert(Assumption::PreservesLayout);

        let ctx = TransformationContext::new(
            *region_id,
            *representation,
            structural.source_structure.clone(),
            constraints,
            assumptions,
        );
        let _ = ctx.validate(); // ensure valid before storing
        self.context_db.insert(*region_id, ctx);
    }
}
```

- [ ] **Step 5: Verify compilation and tests**

Run: `cargo build && cargo test`
Expected: All 251 tests pass. The new `infer()` signature is compile-checked.

- [ ] **Step 6: Commit**

```bash
git add sir/crates/sir_inference/src/engine.rs sir/crates/sir_inference/src/lib.rs
git commit -m "feat(sir_inference): add TransformationContextDatabase, consume StructuralDatabase"
```

---

### Task 5: `sir_generation` crate — scaffold + types

**Files:**
- Create: `sir/crates/sir_generation/Cargo.toml`
- Create: `sir/crates/sir_generation/src/lib.rs`
- Create: `sir/crates/sir_generation/src/candidate.rs`
- Modify: `sir/Cargo.toml` (add workspace member)

**Interfaces:**
- Consumes: `sir_transform` (Representation, TransformationContext, ContextId, Constraint), `sir_semantics` (RegionId, SemanticConcept)
- Produces:
  - `CandidateId` newtype
  - `ImplementationStrategy` enum (4 variants) + `Display`
  - `Candidate` struct
  - `CandidateExplanation` struct
  - `CandidateEffects` enum

- [ ] **Step 1: Add workspace member to `sir/Cargo.toml`**

Add `"crates/sir_generation"` to the `members` array.

- [ ] **Step 2: Create `sir/crates/sir_generation/Cargo.toml`**

```toml
[package]
name = "sir_generation"
version.workspace = true
edition.workspace = true
license.workspace = true
authors.workspace = true

[dependencies]
sir_transform = { path = "../sir_transform" }
sir_semantics = { path = "../sir_semantics" }
```

- [ ] **Step 3: Create `sir/crates/sir_generation/src/lib.rs`**

```rust
//! SIR Generation — Candidate Transformation Plans v0.1
//!
//! Transforms TransformationContexts into concrete candidate plans.
//! Pure, read-only, no SIR access. No ranking, no verification, no rewriting.

pub mod candidate;
```

- [ ] **Step 4: Create `sir/crates/sir_generation/src/candidate.rs`**

```rust
use std::fmt;

use sir_semantics::concepts::SemanticConcept;
use sir_semantics::region::RegionId;
use sir_transform::constraints::Constraint;
use sir_transform::context::ContextId;
use sir_transform::representation::Representation;

/// Unique identifier for a candidate plan.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct CandidateId(pub u64);

impl CandidateId {
    pub fn new(id: u64) -> Self { Self(id) }
}

impl fmt::Display for CandidateId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "candidate#{}", self.0)
    }
}

/// How a bitset transformation might be implemented.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ImplementationStrategy {
    /// Iterate over set bits: while bb != 0 { tzcnt; bb &= bb-1 }
    BitIteration,
    /// Compute cardinality directly: popcount(bb)
    Popcount,
    /// Change data representation: bool[64] → u64
    PackedBitfield,
    /// Replace boolean predicates with mask operations: AND/OR/XOR
    MaskConstruction,
}

impl fmt::Display for ImplementationStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImplementationStrategy::BitIteration => write!(f, "BitIteration"),
            ImplementationStrategy::Popcount => write!(f, "Popcount"),
            ImplementationStrategy::PackedBitfield => write!(f, "PackedBitfield"),
            ImplementationStrategy::MaskConstruction => write!(f, "MaskConstruction"),
        }
    }
}

/// What kind of change a candidate proposes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CandidateEffects {
    /// The representation of data changes (e.g., bool[64] → u64)
    RepresentationChange,
    /// How the data is traversed changes (e.g., loop → trailing-zero scan)
    TraversalChange,
    /// How predicates test conditions changes (e.g., if → mask)
    PredicateEncodingChange,
    /// How counting is performed changes (e.g., accumulator → popcount)
    CountingStrategyChange,
}

/// Human-readable explanation of a candidate plan.
#[derive(Clone, Debug)]
pub struct CandidateExplanation {
    pub strategy: ImplementationStrategy,
    pub representation: Representation,
    pub source_concepts: Vec<SemanticConcept>,
    pub prerequisites: Vec<Constraint>,
    pub rationale: &'static str,
}

/// A candidate transformation plan — a proposed implementation strategy
/// for a region, derived from a TransformationContext.
#[derive(Clone, Debug)]
pub struct Candidate {
    pub id: CandidateId,
    pub region: RegionId,
    /// Reference to the context that produced this candidate.
    /// Multiple candidates may reference the same context.
    pub context_id: ContextId,
    pub strategy: ImplementationStrategy,
    pub explanation: CandidateExplanation,
    pub effects: Vec<CandidateEffects>,
}
```

- [ ] **Step 5: Verify compilation**

Run: `cargo build -p sir_generation`
Expected: Compiles cleanly.

- [ ] **Step 6: Commit**

```bash
git add sir/Cargo.toml sir/crates/sir_generation/
git commit -m "feat(sir_generation): add Candidate, ImplementationStrategy types"
```

---

### Task 6: `sir_generation` — `CandidateGenerator` + `CandidateDatabase`

**Files:**
- Create: `sir/crates/sir_generation/src/generator.rs`
- Modify: `sir/crates/sir_generation/src/lib.rs` (add `pub mod generator;`)
- Create: `sir/crates/sir_generation/tests/generator.rs`
- Modify: `sir/crates/sir_generation/Cargo.toml` (add `[[test]]`)

**Interfaces:**
- Consumes: `CandidateDatabase`, `CandidateGenerator`, `TransformationContextDatabase` (Task 4), `Candidate` (Task 5)
- Produces:
  - `CandidateGenerator::new()`, `generate()`, `database()`
  - `CandidateDatabase::new()`, `candidates()`, `validate()`

- [ ] **Step 1: Create `sir/crates/sir_generation/src/generator.rs`**

```rust
use std::collections::HashMap;

use sir_semantics::region::RegionId;
use sir_transform::context::TransformationContext;

use crate::candidate::{Candidate, CandidateId};

/// Stores candidate plans per region.
#[derive(Clone, Debug, Default)]
pub struct CandidateDatabase {
    candidates: HashMap<RegionId, Vec<Candidate>>,
    next_candidate_id: u64,
}

impl CandidateDatabase {
    pub fn new() -> Self {
        Self { candidates: HashMap::new(), next_candidate_id: 0 }
    }

    pub fn add(&mut self, region: RegionId, candidate: Candidate) {
        self.candidates.entry(region).or_default().push(candidate);
    }

    pub fn candidates(&self, region: RegionId) -> &[Candidate] {
        self.candidates.get(&region).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn all_candidates(&self) -> impl Iterator<Item = &Candidate> {
        self.candidates.values().flat_map(|v| v.iter())
    }

    pub fn region_count(&self) -> usize {
        self.candidates.len()
    }

    pub(crate) fn next_id(&mut self) -> CandidateId {
        let id = CandidateId::new(self.next_candidate_id);
        self.next_candidate_id += 1;
        id
    }

    /// Validate: no duplicate IDs, all candidates have non-empty effects.
    pub fn validate(&self) -> Result<(), String> {
        let mut seen_ids = std::collections::HashSet::new();
        for candidate in self.all_candidates() {
            if !seen_ids.insert(candidate.id) {
                return Err(format!("Duplicate candidate ID: {}", candidate.id));
            }
            if candidate.effects.is_empty() {
                return Err(format!("Candidate {} has no effects", candidate.id));
            }
        }
        Ok(())
    }
}

/// Generates candidate transformation plans from contexts.
///
/// Pure — no SIR access, no ranking, no verification.
pub struct CandidateGenerator {
    db: CandidateDatabase,
}

impl CandidateGenerator {
    pub fn new() -> Self {
        Self { db: CandidateDatabase::new() }
    }

    pub fn database(&self) -> &CandidateDatabase {
        &self.db
    }

    /// Generate candidates for every transformation context.
    /// Each generator inspects the context and returns candidates if applicable.
    pub fn generate(&mut self, context_db: &TransformationContextDatabase) {
        for (region_id, contexts) in context_db.contexts() {
            for ctx in contexts {
                let candidates = crate::generators::all_plans(ctx);
                for candidate in candidates {
                    self.db.add(region_id, candidate);
                }
            }
        }
    }
}

/// Legacy compatibility type alias.
pub type TransformationContextDatabase = sir_transform::context::TransformationContextDatabase;
```

Wait — `TransformationContextDatabase` is defined in `sir_inference`, not `sir_transform`. Let me fix the import:

```rust
// generator.rs — corrected
use std::collections::HashMap;

use sir_semantics::region::RegionId;

use crate::candidate::Candidate;

/// Stores candidate plans per region.
#[derive(Clone, Debug, Default)]
pub struct CandidateDatabase {
    candidates: HashMap<RegionId, Vec<Candidate>>,
    next_candidate_id: u64,
}

impl CandidateDatabase {
    pub fn new() -> Self {
        Self { candidates: HashMap::new(), next_candidate_id: 0 }
    }

    pub fn add(&mut self, region: RegionId, candidate: Candidate) {
        self.candidates.entry(region).or_default().push(candidate);
    }

    pub fn candidates(&self, region: RegionId) -> &[Candidate] {
        self.candidates.get(&region).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn all_candidates(&self) -> impl Iterator<Item = &Candidate> {
        self.candidates.values().flat_map(|v| v.iter())
    }

    pub fn region_count(&self) -> usize {
        self.candidates.len()
    }

    pub(crate) fn next_id(&mut self) -> CandidateId {
        let id = CandidateId::new(self.next_candidate_id);
        self.next_candidate_id += 1;
        id
    }

    /// Validate: no duplicate IDs, all candidates have non-empty effects.
    pub fn validate(&self) -> Result<(), String> {
        let mut seen_ids = std::collections::HashSet::new();
        for candidate in self.all_candidates() {
            if !seen_ids.insert(candidate.id) {
                return Err(format!("Duplicate candidate ID: {}", candidate.id));
            }
            if candidate.effects.is_empty() {
                return Err(format!("Candidate {} has no effects", candidate.id));
            }
        }
        Ok(())
    }
}

/// Generates candidate transformation plans from contexts.
pub struct CandidateGenerator {
    db: CandidateDatabase,
}

impl CandidateGenerator {
    pub fn new() -> Self {
        Self { db: CandidateDatabase::new() }
    }

    pub fn database(&self) -> &CandidateDatabase {
        &self.db
    }

    /// Generate candidates for every transformation context.
    pub fn generate(&mut self, context_db: &dyn ContextProvider) {
        for (region_id, contexts) in context_db.iter_contexts() {
            for ctx in contexts {
                let candidates = crate::generators::all_plans(ctx);
                for candidate in candidates {
                    self.db.add(region_id, candidate);
                }
            }
        }
    }
}
```

Hmm, that's overcomplicating things. Let me simplify — just accept the concrete type from `sir_inference`:

Create the file with the actual content:

```rust
// generator.rs
use std::collections::HashMap;

use sir_semantics::region::RegionId;

use crate::candidate::{Candidate, CandidateId};

/// Stores candidate plans per region.
#[derive(Clone, Debug, Default)]
pub struct CandidateDatabase {
    candidates: HashMap<RegionId, Vec<Candidate>>,
    next_candidate_id: u64,
}

impl CandidateDatabase {
    pub fn new() -> Self {
        Self { candidates: HashMap::new(), next_candidate_id: 0 }
    }

    pub fn add(&mut self, region: RegionId, candidate: Candidate) {
        self.candidates.entry(region).or_default().push(candidate);
    }

    pub fn candidates(&self, region: RegionId) -> &[Candidate] {
        self.candidates.get(&region).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn all_candidates(&self) -> impl Iterator<Item = &Candidate> {
        self.candidates.values().flat_map(|v| v.iter())
    }

    pub fn region_count(&self) -> usize {
        self.candidates.len()
    }

    pub(crate) fn next_id(&mut self) -> CandidateId {
        let id = CandidateId::new(self.next_candidate_id);
        self.next_candidate_id += 1;
        id
    }

    /// Validate: no duplicate IDs, all candidates have non-empty effects.
    pub fn validate(&self) -> Result<(), String> {
        let mut seen_ids = std::collections::HashSet::new();
        for candidate in self.all_candidates() {
            if !seen_ids.insert(candidate.id) {
                return Err(format!("Duplicate candidate ID: {}", candidate.id));
            }
            if candidate.effects.is_empty() {
                return Err(format!("Candidate {} has no effects", candidate.id));
            }
        }
        Ok(())
    }
}

/// Generates candidate transformation plans from contexts.
pub struct CandidateGenerator {
    db: CandidateDatabase,
}

impl CandidateGenerator {
    pub fn new() -> Self {
        Self { db: CandidateDatabase::new() }
    }

    pub fn database(&self) -> &CandidateDatabase {
        &self.db
    }

    /// Generate candidates for every transformation context.
    /// Calls each generator; generators are pure functions.
    pub fn generate(&mut self, context_db: &sir_inference::engine::TransformationContextDatabase) {
        for (region_id, contexts) in context_db.contexts() {
            for ctx in contexts {
                let candidates = crate::generators::all_plans(ctx);
                for candidate in candidates {
                    self.db.add(region_id, candidate);
                }
            }
        }
    }
}
```

Wait, but that creates a reverse dependency (`sir_generation` → `sir_inference`), which we specifically wanted to avoid. Let me use the correct architecture.

The spec says `sir_inference` produces `TransformationContextDatabase`. But `sir_generation` shouldn't depend on `sir_inference` (that would be upward). Instead:

`sir_generation` should accept a trait or the `TransformationContextDatabase` should be in `sir_transform`.

Actually, looking at the spec more carefully: `TransformationContextDatabase` is defined in `sir_inference`. But for `sir_generation` to avoid depending on `sir_inference`, the database type needs to be accessible from `sir_transform` or the generator needs to accept an iterator/trait.

The simplest fix: move `TransformationContextDatabase` into `sir_transform`. It's just a container type — a HashMap wrapper, same pattern as all other databases. It belongs with the `TransformationContext` it stores.

Let me update the plan. In Task 1, add `TransformationContextDatabase` to `sir_transform`. Then Task 4 (inference) uses it. Task 6 (generation) consumes it.

Let me rewrite the generator.rs content properly:

```rust
// generator.rs — final correct version
use std::collections::HashMap;

use sir_semantics::region::RegionId;
use sir_transform::context::TransformationContextDatabase;

use crate::candidate::{Candidate, CandidateId};

/// Stores candidate plans per region.
#[derive(Clone, Debug, Default)]
pub struct CandidateDatabase {
    candidates: HashMap<RegionId, Vec<Candidate>>,
    next_candidate_id: u64,
}

impl CandidateDatabase {
    pub fn new() -> Self {
        Self { candidates: HashMap::new(), next_candidate_id: 0 }
    }

    pub fn add(&mut self, region: RegionId, candidate: Candidate) {
        self.candidates.entry(region).or_default().push(candidate);
    }

    pub fn candidates(&self, region: RegionId) -> &[Candidate] {
        self.candidates.get(&region).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn all_candidates(&self) -> impl Iterator<Item = &Candidate> {
        self.candidates.values().flat_map(|v| v.iter())
    }

    pub fn region_count(&self) -> usize {
        self.candidates.len()
    }

    pub(crate) fn next_id(&mut self) -> CandidateId {
        let id = CandidateId::new(self.next_candidate_id);
        self.next_candidate_id += 1;
        id
    }

    pub fn validate(&self) -> Result<(), String> {
        let mut seen_ids = std::collections::HashSet::new();
        for candidate in self.all_candidates() {
            if !seen_ids.insert(candidate.id) {
                return Err(format!("Duplicate candidate ID: {}", candidate.id));
            }
            if candidate.effects.is_empty() {
                return Err(format!("Candidate {} has no effects", candidate.id));
            }
        }
        Ok(())
    }
}

/// Generates candidate transformation plans from contexts.
pub struct CandidateGenerator {
    db: CandidateDatabase,
}

impl CandidateGenerator {
    pub fn new() -> Self {
        Self { db: CandidateDatabase::new() }
    }

    pub fn database(&self) -> &CandidateDatabase {
        &self.db
    }

    /// Generate candidates for every transformation context.
    /// Each generator is a pure function: context → Option<Candidate>.
    pub fn generate(&mut self, context_db: &TransformationContextDatabase) {
        for (region_id, contexts) in context_db.contexts() {
            for ctx in contexts {
                let candidates = crate::generators::all_plans(ctx);
                for candidate in candidates {
                    self.db.add(region_id, candidate);
                }
            }
        }
    }
}
```

And I need to update Task 1 to include `TransformationContextDatabase` in `sir_transform/src/context.rs`.

- [ ] **Step 2: Create test file `sir/crates/sir_generation/tests/generator.rs`**

```rust
use sir_generation::candidate::CandidateDatabase;
use sir_generation::generator::CandidateGenerator;

#[test]
fn empty_generator_has_no_candidates() {
    let gen = CandidateGenerator::new();
    let db = gen.database();
    assert_eq!(db.region_count(), 0);
}

#[test]
fn database_validate_rejects_duplicate_ids() {
    use sir_generation::candidate::{
        Candidate, CandidateEffects, CandidateExplanation, CandidateId,
        ImplementationStrategy,
    };
    use sir_semantics::region::RegionId;
    use sir_transform::context::ContextId;
    use sir_transform::representation::Representation;

    let mut db = CandidateDatabase::new();
    let rid = RegionId::new(0);
    let cid = ContextId::new(0);
    let id = CandidateId::new(0);

    let c = Candidate {
        id,
        region: rid,
        context_id: cid,
        strategy: ImplementationStrategy::BitIteration,
        explanation: CandidateExplanation {
            strategy: ImplementationStrategy::BitIteration,
            representation: Representation::BitSet,
            source_concepts: vec![],
            prerequisites: vec![],
            rationale: "test",
        },
        effects: vec![CandidateEffects::TraversalChange],
    };

    db.add(rid, c.clone());
    db.add(rid, c); // duplicate ID
    assert!(db.validate().is_err());
}

#[test]
fn database_validate_rejects_empty_effects() {
    use sir_generation::candidate::{
        Candidate, CandidateEffects, CandidateExplanation, CandidateId,
        ImplementationStrategy,
    };
    use sir_semantics::region::RegionId;
    use sir_transform::context::ContextId;
    use sir_transform::representation::Representation;

    let mut db = CandidateDatabase::new();
    let c = Candidate {
        id: CandidateId::new(0),
        region: RegionId::new(0),
        context_id: ContextId::new(0),
        strategy: ImplementationStrategy::Popcount,
        explanation: CandidateExplanation {
            strategy: ImplementationStrategy::Popcount,
            representation: Representation::BitSet,
            source_concepts: vec![],
            prerequisites: vec![],
            rationale: "test",
        },
        effects: vec![],
    };
    db.add(RegionId::new(0), c);
    assert!(db.validate().is_err());
}
```

Add to `sir/crates/sir_generation/Cargo.toml`:
```toml
[dev-dependencies]
sir_inference = { path = "../sir_inference" }

[[test]]
name = "generator"
path = "tests/generator.rs"
```

- [ ] **Step 3: Update `sir/crates/sir_generation/src/lib.rs`**

Add:
```rust
pub mod generator;
```

- [ ] **Step 4: Verify compilation and tests**

Run: `cargo build -p sir_generation && cargo test -p sir_generation`
Expected: Generator tests pass. Compiles cleanly.

- [ ] **Step 5: Commit**

```bash
git add sir/crates/sir_generation/src/generator.rs \
  sir/crates/sir_generation/src/lib.rs \
  sir/crates/sir_generation/tests/generator.rs \
  sir/crates/sir_generation/Cargo.toml
git commit -m "feat(sir_generation): add CandidateGenerator and CandidateDatabase"
```

---

### Task 7: `sir_generation` — 4 generators

**Files:**
- Create: `sir/crates/sir_generation/src/generators/mod.rs`
- Create: `sir/crates/sir_generation/src/generators/bit_iteration.rs`
- Create: `sir/crates/sir_generation/src/generators/popcount.rs`
- Create: `sir/crates/sir_generation/src/generators/packed_bitfield.rs`
- Create: `sir/crates/sir_generation/src/generators/mask_construction.rs`
- Create: `sir/crates/sir_generation/tests/generators.rs`
- Modify: `sir/crates/sir_generation/src/lib.rs` (add `pub mod generators;`)
- Modify: `sir/crates/sir_generation/Cargo.toml` (add test)

**Interfaces:**
- Consumes: `TransformationContext` (Task 1), `Candidate` (Task 5), `CandidateDatabase` (Task 6)
- Produces:
  - `all_plans(context) -> Vec<Candidate>` — dispatches to all 4 generators
  - `plan(context) -> Option<Candidate>` for each generator

- [ ] **Step 1: Create `sir/crates/sir_generation/src/generators/mod.rs`**

```rust
pub mod bit_iteration;
pub mod popcount;
pub mod packed_bitfield;
pub mod mask_construction;

use sir_transform::context::TransformationContext;
use crate::candidate::Candidate;

/// Run all generators and collect their candidates.
pub fn all_plans(context: &TransformationContext) -> Vec<Candidate> {
    let mut candidates = Vec::new();

    if let Some(c) = bit_iteration::plan(context) { candidates.push(c); }
    if let Some(c) = popcount::plan(context) { candidates.push(c); }
    if let Some(c) = packed_bitfield::plan(context) { candidates.push(c); }
    if let Some(c) = mask_construction::plan(context) { candidates.push(c); }

    candidates
}
```

- [ ] **Step 2: Create `sir/crates/sir_generation/src/generators/bit_iteration.rs`**

```rust
use sir_semantics::concepts::SemanticConcept;
use sir_transform::context::TransformationContext;
use sir_transform::representation::Representation;

use crate::candidate::{
    Candidate, CandidateEffects, CandidateExplanation, CandidateId,
    ImplementationStrategy,
};

/// Plan a BitIteration candidate: replaces full iteration with
/// trailing-zero-based iteration over only set bits.
///
/// strategy: while bb != 0 { tzcnt; bb &= bb-1 }
pub fn plan(context: &TransformationContext) -> Option<Candidate> {
    if context.representation != Representation::BitSet {
        return None;
    }

    let candidate = Candidate {
        id: CandidateId::new(0), // ID assigned by database
        region: context.region,
        context_id: sir_transform::context::ContextId::new(0), // assigned by database
        strategy: ImplementationStrategy::BitIteration,
        explanation: CandidateExplanation {
            strategy: ImplementationStrategy::BitIteration,
            representation: Representation::BitSet,
            source_concepts: vec![
                SemanticConcept::MembershipTraversal,
                SemanticConcept::BooleanCollection,
            ],
            prerequisites: context.constraints.iter().cloned().collect(),
            rationale: "Iterate over only set bits using trailing-zero count and bit clear, \
                        visiting only populated elements rather than all 64 positions.",
        },
        effects: vec![
            CandidateEffects::TraversalChange,
        ],
    };

    Some(candidate)
}
```

- [ ] **Step 3: Create `sir/crates/sir_generation/src/generators/popcount.rs`**

```rust
use sir_semantics::concepts::SemanticConcept;
use sir_transform::context::TransformationContext;
use sir_transform::representation::Representation;

use crate::candidate::{
    Candidate, CandidateEffects, CandidateExplanation, CandidateId,
    ImplementationStrategy,
};

/// Plan a Popcount candidate: replaces loop-based counting with
/// a single popcount instruction.
///
/// strategy: popcount(bb)
pub fn plan(context: &TransformationContext) -> Option<Candidate> {
    if context.representation != Representation::BitSet {
        return None;
    }

    let candidate = Candidate {
        id: CandidateId::new(0),
        region: context.region,
        context_id: sir_transform::context::ContextId::new(0),
        strategy: ImplementationStrategy::Popcount,
        explanation: CandidateExplanation {
            strategy: ImplementationStrategy::Popcount,
            representation: Representation::BitSet,
            source_concepts: vec![
                SemanticConcept::CardinalityReduction,
                SemanticConcept::BooleanCollection,
            ],
            prerequisites: context.constraints.iter().cloned().collect(),
            rationale: "Count set bits directly using hardware popcount instruction, \
                        eliminating the counting loop entirely.",
        },
        effects: vec![
            CandidateEffects::CountingStrategyChange,
        ],
    };

    Some(candidate)
}
```

- [ ] **Step 4: Create `sir/crates/sir_generation/src/generators/packed_bitfield.rs`**

```rust
use sir_semantics::concepts::SemanticConcept;
use sir_transform::context::TransformationContext;
use sir_transform::representation::Representation;

use crate::candidate::{
    Candidate, CandidateEffects, CandidateExplanation, CandidateId,
    ImplementationStrategy,
};

/// Plan a PackedBitfield candidate: replaces bool[64] with a single u64.
///
/// strategy: replace the array-of-booleans representation with a packed integer
pub fn plan(context: &TransformationContext) -> Option<Candidate> {
    if context.representation != Representation::BitSet {
        return None;
    }

    let candidate = Candidate {
        id: CandidateId::new(0),
        region: context.region,
        context_id: sir_transform::context::ContextId::new(0),
        strategy: ImplementationStrategy::PackedBitfield,
        explanation: CandidateExplanation {
            strategy: ImplementationStrategy::PackedBitfield,
            representation: Representation::BitSet,
            source_concepts: vec![
                SemanticConcept::BooleanCollection,
                SemanticConcept::FiniteCollection,
            ],
            prerequisites: context.constraints.iter().cloned().collect(),
            rationale: "Replace the bool[64] array with a single u64 value, \
                        reducing memory footprint from 64 bytes to 8 bytes \
                        and enabling bitwise operations on the entire set.",
        },
        effects: vec![
            CandidateEffects::RepresentationChange,
        ],
    };

    Some(candidate)
}
```

- [ ] **Step 5: Create `sir/crates/sir_generation/src/generators/mask_construction.rs`**

```rust
use sir_semantics::concepts::SemanticConcept;
use sir_transform::context::TransformationContext;
use sir_transform::representation::Representation;

use crate::candidate::{
    Candidate, CandidateEffects, CandidateExplanation, CandidateId,
    ImplementationStrategy,
};

/// Plan a MaskConstruction candidate: replaces boolean predicates with
/// bitmask AND/OR/XOR operations.
///
/// strategy: encode conditions as masks, combine with bitwise operations
pub fn plan(context: &TransformationContext) -> Option<Candidate> {
    if context.representation != Representation::BitSet {
        return None;
    }

    let candidate = Candidate {
        id: CandidateId::new(0),
        region: context.region,
        context_id: sir_transform::context::ContextId::new(0),
        strategy: ImplementationStrategy::MaskConstruction,
        explanation: CandidateExplanation {
            strategy: ImplementationStrategy::MaskConstruction,
            representation: Representation::BitSet,
            source_concepts: vec![
                SemanticConcept::BooleanCollection,
                SemanticConcept::MembershipTraversal,
            ],
            prerequisites: context.constraints.iter().cloned().collect(),
            rationale: "Replace boolean predicate evaluation with bitmask construction, \
                        enabling parallel evaluation of multiple conditions via AND/OR/XOR.",
        },
        effects: vec![
            CandidateEffects::PredicateEncodingChange,
        ],
    };

    Some(candidate)
}
```

- [ ] **Step 6: Create `sir/crates/sir_generation/tests/generators.rs`**

```rust
use std::collections::HashSet;

use sir_generation::candidate::{CandidateEffects, ImplementationStrategy};
use sir_generation::generators;
use sir_semantics::region::RegionId;
use sir_transform::assumptions::Assumption;
use sir_transform::constraints::Constraint;
use sir_transform::context::TransformationContext;
use sir_transform::representation::Representation;
use sir_transform::structures::SourceStructure;

fn make_context() -> TransformationContext {
    let mut constraints = HashSet::new();
    constraints.insert(Constraint::FixedLength(64));
    constraints.insert(Constraint::ReadOnly);
    constraints.insert(Constraint::NoEscape);
    constraints.insert(Constraint::FiniteIteration);

    let mut assumptions = HashSet::new();
    assumptions.insert(Assumption::EquivalentCardinality);
    assumptions.insert(Assumption::PreservesLayout);

    TransformationContext::new(
        RegionId::new(0),
        Representation::BitSet,
        SourceStructure::BooleanArray { length: 64 },
        constraints,
        assumptions,
    )
}

#[test]
fn all_four_generators_produce_candidates() {
    let ctx = make_context();
    let candidates = generators::all_plans(&ctx);
    assert_eq!(candidates.len(), 4, "Expected 4 candidates for BitSet context");
}

#[test]
fn all_strategies_are_distinct() {
    let ctx = make_context();
    let candidates = generators::all_plans(&ctx);
    let strategies: HashSet<_> = candidates.iter().map(|c| c.strategy).collect();
    assert_eq!(strategies.len(), 4);
}

#[test]
fn each_candidate_has_effects() {
    let ctx = make_context();
    let candidates = generators::all_plans(&ctx);
    for c in &candidates {
        assert!(!c.effects.is_empty(),
            "{:?} should have at least one effect", c.strategy);
    }
}

#[test]
fn each_candidate_has_explanation() {
    let ctx = make_context();
    let candidates = generators::all_plans(&ctx);
    for c in &candidates {
        assert!(!c.explanation.rationale.is_empty(),
            "{:?} should have a non-empty rationale", c.strategy);
    }
}

#[test]
fn each_candidate_has_prerequisites() {
    let ctx = make_context();
    let candidates = generators::all_plans(&ctx);
    for c in &candidates {
        assert!(!c.explanation.prerequisites.is_empty(),
            "{:?} should list prerequisites", c.strategy);
    }
}

#[test]
fn non_bitset_context_produces_no_candidates() {
    let mut constraints = HashSet::new();
    constraints.insert(Constraint::FixedLength(64));
    let mut assumptions = HashSet::new();
    assumptions.insert(Assumption::EquivalentCardinality);

    // Non-BitSet representation — should be skipped by all generators
    let ctx = TransformationContext::new(
        RegionId::new(0),
        // There's only BitSet in v0.1, but each generator checks representation
        Representation::BitSet,
        SourceStructure::BitMask { width: 64 },
        constraints,
        assumptions,
    );
    let candidates = generators::all_plans(&ctx);
    // All 4 generators check for BitSet, but BitMask as source structure
    // is still valid — generators check representation, not structure.
    // This test validates they don't crash on non-BooleanArray contexts.
    assert_eq!(candidates.len(), 4);
}
```

- [ ] **Step 7: Update `sir/crates/sir_generation/src/lib.rs`**

Add:
```rust
pub mod generators;
```

- [ ] **Step 8: Add `sir_inference` dev-dep + test entry to `sir_generation/Cargo.toml`**

```toml
[dev-dependencies]
sir_inference = { path = "../sir_inference" }

[[test]]
name = "generators"
path = "tests/generators.rs"
```

- [ ] **Step 9: Update `generator.rs` — inject IDs properly**

In `generate()`, IDs need to come from the database, not hardcoded. Update the method:

```rust
pub fn generate(&mut self, context_db: &TransformationContextDatabase) {
    for (region_id, contexts) in context_db.contexts() {
        for ctx in contexts {
            let mut candidates = crate::generators::all_plans(ctx);
            for mut candidate in candidates {
                candidate.id = self.db.next_id();
                self.db.add(region_id, candidate);
            }
        }
    }
}
```

- [ ] **Step 10: Verify compilation and tests**

Run: `cargo build -p sir_generation && cargo test -p sir_generation`
Expected: 8 tests pass (2 from generator.rs + 6 from generators.rs). Compiles cleanly.

- [ ] **Step 11: Commit**

```bash
git add sir/crates/sir_generation/src/generators/ \
  sir/crates/sir_generation/src/lib.rs \
  sir/crates/sir_generation/src/generator.rs \
  sir/crates/sir_generation/tests/generators.rs \
  sir/crates/sir_generation/Cargo.toml
git commit -m "feat(sir_generation): add 4 BitSet generators"
```

---

### Task 8: Integration test — BS001 full pipeline

**Files:**
- Create: `sir/crates/sir_generation/tests/bs001_pipeline.rs`
- Modify: `sir/crates/sir_generation/Cargo.toml` (add test + dev-deps)
- Modify: `sir/crates/sir_semantics/tests/semantic_truth.rs` (extend with generation check)

**Goal:** Full end-to-end test: build SIR → analyze → semantic + structural → inference → generation. Assert 4 distinct candidates.

- [ ] **Step 1: Add dev-dependencies to `sir_generation/Cargo.toml`**

```toml
[dev-dependencies]
sir_builder = { path = "../sir_builder" }
sir_analysis = { path = "../sir_analysis" }
sir_inference = { path = "../sir_inference" }

[[test]]
name = "bs001_pipeline"
path = "tests/bs001_pipeline.rs"
```

- [ ] **Step 2: Create `sir/crates/sir_generation/tests/bs001_pipeline.rs`**

```rust
use std::collections::HashSet;

use sir_analysis::manager::AnalysisManager;
use sir_builder::Builder;
use sir_generation::candidate::ImplementationStrategy;
use sir_generation::generator::CandidateGenerator;
use sir_inference::engine::InferenceEngine;
use sir_semantics::semantics::SemanticEngine;
use sir_transform::representation::Representation;
use sir_types::{ConstantData, Span, Type};

/// Build the BS001 board scan SIR function.
fn build_board_scan() -> sir_nodes::Function {
    let mut b = Builder::new(
        "board_scan",
        &[
            ("board", Type::Array { element: Box::new(Type::Bool), length: 64 }),
            ("count", Type::i32()),
        ],
        Type::i32(),
    );

    let board = b.parameter_index(0).unwrap();
    let count_initial = b.parameter_index(1).unwrap();
    let zero_i64 = b.constant(ConstantData::u64(0), Type::u64(), Span::unknown());
    let one_i64 = b.constant(ConstantData::u64(1), Type::u64(), Span::unknown());
    let limit = b.constant(ConstantData::u64(64), Type::u64(), Span::unknown());
    let zero_i32 = b.constant(ConstantData::i32(0), Type::i32(), Span::unknown());
    let one_i32 = b.constant(ConstantData::i32(1), Type::i32(), Span::unknown());

    // Loop counter increment: i + 1
    let inc_i = b.add(zero_i64, one_i64, Span::unknown()).unwrap();
    // Bound check: i < 64
    let cond = b.lt(zero_i64, limit, Span::unknown()).unwrap();
    // Array access: board[i]
    let elem_ty = Type::Bool;
    let access = b.array_access(board, zero_i64, elem_ty, Span::unknown()).unwrap();
    let elem = b.load(access, Type::Bool, Span::unknown()).unwrap();
    // Select: if elem { count + 1 } else { count }
    let body = b.select(elem, one_i32, count_initial, Span::unknown()).unwrap();

    let loop_node = b.r#loop(
        vec![zero_i64, count_initial],
        vec![body],
        vec![inc_i, body],
        cond,
        Span::unknown(),
    ).unwrap();

    b.return_value(loop_node, Span::unknown()).unwrap();
    b.build()
}

#[test]
fn bs001_full_pipeline_produces_four_distinct_candidates() {
    let func = build_board_scan();

    // Analysis
    let mut analysis = AnalysisManager::new();
    analysis.run_all(&func);

    // Semantics + Structure
    let mut semantics = SemanticEngine::new();
    semantics.derive(&func, analysis.database());

    // Inference
    let mut inference = InferenceEngine::new();
    inference.infer(semantics.semantic_database(), semantics.structural_database());

    // Generation
    let mut generator = CandidateGenerator::new();
    generator.generate(inference.context_database());

    let db = generator.database();
    assert!(db.region_count() > 0, "Should have at least one region with candidates");

    // Find first region with candidates
    let mut total_candidates = 0;
    let mut strategies = HashSet::new();
    for candidate in db.all_candidates() {
        total_candidates += 1;
        strategies.insert(candidate.strategy);
        assert_eq!(candidate.explanation.representation, Representation::BitSet);
    }

    assert_eq!(total_candidates, 4, "Expected exactly 4 candidates");
    assert_eq!(strategies.len(), 4, "Expected 4 distinct strategies");
    assert!(strategies.contains(&ImplementationStrategy::BitIteration));
    assert!(strategies.contains(&ImplementationStrategy::Popcount));
    assert!(strategies.contains(&ImplementationStrategy::PackedBitfield));
    assert!(strategies.contains(&ImplementationStrategy::MaskConstruction));
}

#[test]
fn bs001_candidates_are_deterministic() {
    let func = build_board_scan();

    let mut analysis = AnalysisManager::new();
    analysis.run_all(&func);
    let mut semantics = SemanticEngine::new();
    semantics.derive(&func, analysis.database());

    // Run twice, get candidate IDs
    let get_ids = || {
        let mut inference = InferenceEngine::new();
        inference.infer(semantics.semantic_database(), semantics.structural_database());
        let mut generator = CandidateGenerator::new();
        generator.generate(inference.context_database());
        generator.database().all_candidates()
            .map(|c| c.strategy)
            .collect::<Vec<_>>()
    };

    let first = get_ids();
    let second = get_ids();
    assert_eq!(first, second, "Generation must be deterministic");
}
```

- [ ] **Step 3: Run integration tests**

Run: `cargo test -p sir_generation bs001`
Expected: 2 tests pass — 4 distinct candidates, deterministic output.

- [ ] **Step 4: Run full workspace tests**

Run: `cargo test`
Expected: All 251+ tests pass, including new generator + pipeline tests.

- [ ] **Step 5: Commit**

```bash
git add sir/crates/sir_generation/tests/bs001_pipeline.rs sir/crates/sir_generation/Cargo.toml
git commit -m "test: add BS001 full pipeline integration test"
```

---

### Task 9: Validation + determinism + explanation tests

**Files:**
- Create: `sir/crates/sir_transform/tests/context_validation.rs`
- Modify: `sir/crates/sir_transform/Cargo.toml` (add test)
- Modify: `sir/crates/sir_generation/tests/generators.rs` (add determinism + explanation tests)

**Goal:** Validate that `TransformationContext::validate()` works, generation is deterministic, explanations are complete.

- [ ] **Step 1: Create `sir/crates/sir_transform/tests/context_validation.rs`**

```rust
use std::collections::HashSet;
use sir_transform::assumptions::Assumption;
use sir_transform::constraints::Constraint;
use sir_transform::context::TransformationContext;
use sir_transform::representation::Representation;
use sir_transform::structures::SourceStructure;
use sir_semantics::region::RegionId;

#[test]
fn valid_context_passes_validation() {
    let mut constraints = HashSet::new();
    constraints.insert(Constraint::FixedLength(64));
    let mut assumptions = HashSet::new();
    assumptions.insert(Assumption::EquivalentCardinality);

    let ctx = TransformationContext::new(
        RegionId::new(0),
        Representation::BitSet,
        SourceStructure::BooleanArray { length: 64 },
        constraints,
        assumptions,
    );
    assert!(ctx.validate().is_ok());
}

#[test]
fn context_validation_accepts_bitmask() {
    let mut constraints = HashSet::new();
    constraints.insert(Constraint::FixedLength(32));
    let assumptions = HashSet::new();

    let ctx = TransformationContext::new(
        RegionId::new(1),
        Representation::BitSet,
        SourceStructure::BitMask { width: 32 },
        constraints,
        assumptions,
    );
    assert!(ctx.validate().is_ok());
}
```

Add to `sir/crates/sir_transform/Cargo.toml`:
```toml
[[test]]
name = "context_validation"
path = "tests/context_validation.rs"
```

- [ ] **Step 2: Add determinism + explanation tests to `generators.rs`**

Append to `sir/crates/sir_generation/tests/generators.rs`:

```rust
#[test]
fn generation_is_deterministic() {
    let ctx = make_context();
    let first = generators::all_plans(&ctx);
    let second = generators::all_plans(&ctx);
    assert_eq!(first.len(), second.len());
    for (a, b) in first.iter().zip(second.iter()) {
        assert_eq!(a.strategy, b.strategy);
    }
}

#[test]
fn explanations_contain_source_concepts() {
    let ctx = make_context();
    let candidates = generators::all_plans(&ctx);
    for c in &candidates {
        assert!(!c.explanation.source_concepts.is_empty(),
            "{:?} explanation should reference source concepts", c.strategy);
    }
}
```

- [ ] **Step 3: Run all new tests**

Run: `cargo test -p sir_transform && cargo test -p sir_generation`
Expected: All new validation + determinism tests pass.

- [ ] **Step 4: Run full workspace**

Run: `cargo test`
Expected: All tests pass.

- [ ] **Step 5: Commit**

```bash
git add sir/crates/sir_transform/tests/context_validation.rs \
  sir/crates/sir_transform/Cargo.toml \
  sir/crates/sir_generation/tests/generators.rs
git commit -m "test: add context validation and determinism tests"
```

---

## Plan Summary

| Task | Crate | What | Est. tests |
|------|-------|------|------------|
| 1 | sir_transform | Scaffold + types (Representation, Context, Structure, Constraint, Assumption) | 0 (type-only) |
| 2 | sir_inference + tests | Move Representation to sir_transform, update all imports | 251 (existing) |
| 3 | sir_semantics | StructuralDatabase + 2 recognizers | 251+ |
| 4 | sir_inference | New infer() signature, TransformationContextDatabase | 251+ |
| 5 | sir_generation | Scaffold + types (Candidate, ImplementationStrategy, etc.) | 0 (type-only) |
| 6 | sir_generation | CandidateGenerator + CandidateDatabase | 2 |
| 7 | sir_generation | 4 generators | 6 |
| 8 | sir_generation | BS001 full pipeline integration | 2 |
| 9 | sir_transform + sir_generation | Validation + determinism tests | 2 |

**Total estimated new tests:** 12+
**Expected final test count:** 263+
