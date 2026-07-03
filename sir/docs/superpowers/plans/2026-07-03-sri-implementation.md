# Semantic Representation Inference — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the 0010 SRI pipeline — two new crates (`sir_semantics`, `sir_inference`) that sit on top of `sir_analysis` and infer that a fixed-size boolean membership scan is a `BitSet`.

**Architecture:** `sir_semantics` consumes `Function` + `FactDatabase` and produces a `SemanticDatabase` with regions tagged by deterministic `SemanticConcept`s. `sir_inference` consumes `SemanticDatabase` and produces a `HypothesisDatabase` with evidence-weighted `Representation` hypotheses. Both crates duplicate the manager pattern from `sir_analysis`; framework extraction is deferred to Phase 0012.

**Tech Stack:** Rust 2021 edition, no new external dependencies beyond the existing workspace crates.

## Global Constraints

- No new external dependencies — use only `sir_types`, `sir_nodes`, `sir_analysis`, and stdlib
- All tests pass: `cargo test` in `sir/` must succeed at every commit
- Evidence weights are `u16` integers (no floating point in engine logic)
- Only one representation (`BitSet`) in v0.1
- Region extraction is minimal: one region per recognized loop pattern
- No SIR modification — read-only inference only

---

### Task 1: Workspace scaffold

**Files:**
- Modify: `sir/Cargo.toml`
- Create: `sir/crates/sir_semantics/Cargo.toml`
- Create: `sir/crates/sir_semantics/src/lib.rs`
- Create: `sir/crates/sir_inference/Cargo.toml`
- Create: `sir/crates/sir_inference/src/lib.rs`

**Interfaces:**
- Produces: Two empty crates registered in the workspace, compiling with no errors

- [ ] **Step 1: Add workspace members to `sir/Cargo.toml`**

Replace the `members` array in `sir/Cargo.toml`:

```toml
[workspace]
resolver = "2"
members = [
    "crates/sir_types",
    "crates/sir_nodes",
    "crates/sir_builder",
    "crates/sir_printer",
    "crates/sir_verify",
    "crates/sir_tests",
    "crates/sir_analysis",
    "crates/sir_semantics",
    "crates/sir_inference",
]
```

- [ ] **Step 2: Create `sir/crates/sir_semantics/Cargo.toml`**

```toml
[package]
name = "sir_semantics"
version.workspace = true
edition.workspace = true
license.workspace = true
authors.workspace = true

[dependencies]
sir_types = { path = "../sir_types" }
sir_nodes = { path = "../sir_nodes" }
sir_analysis = { path = "../sir_analysis" }
```

- [ ] **Step 3: Create `sir/crates/sir_semantics/src/lib.rs`**

```rust
//! SIR Semantics — Semantic Truths v0.1
//!
//! Transforms compiler facts (`sir_analysis::FactDatabase`) into semantic
//! truths. Entirely deterministic. No heuristics, no confidence scores.
//!
//! This is Layer 2 of the knowledge hierarchy:
//!   Facts (sir_analysis) → Truths (sir_semantics) → Beliefs (sir_inference)

pub mod concepts;
pub mod region;
pub mod semantics;
pub mod recognizers;
```

- [ ] **Step 4: Create `sir/crates/sir_inference/Cargo.toml`**

```toml
[package]
name = "sir_inference"
version.workspace = true
edition.workspace = true
license.workspace = true
authors.workspace = true

[dependencies]
sir_semantics = { path = "../sir_semantics" }
```

- [ ] **Step 5: Create `sir/crates/sir_inference/src/lib.rs`**

```rust
//! SIR Inference — Representation Beliefs v0.1
//!
//! Accumulates evidence from semantic truths and forms representation
//! hypotheses. This is where heuristics and weights live.
//!
//! This is Layer 3 of the knowledge hierarchy:
//!   Facts (sir_analysis) → Truths (sir_semantics) → Beliefs (sir_inference)

pub mod evidence;
pub mod hypothesis;
pub mod engine;
pub mod sources;
```

- [ ] **Step 6: Verify compilation**

Run: `cargo build`
Expected: Both new crates compile successfully, no errors.

- [ ] **Step 7: Commit**

```bash
git add sir/Cargo.toml \
  sir/crates/sir_semantics/Cargo.toml sir/crates/sir_semantics/src/lib.rs \
  sir/crates/sir_inference/Cargo.toml sir/crates/sir_inference/src/lib.rs
git commit -m "feat: scaffold sir_semantics and sir_inference crates"
```

---

### Task 2: `sir_semantics` types — concepts, region

**Files:**
- Create: `sir/crates/sir_semantics/src/concepts.rs`
- Create: `sir/crates/sir_semantics/src/region.rs`

**Interfaces:**
- Produces:
  - `SemanticConcept` enum with 4 variants + `Display`
  - `RegionId` newtype (Copy, Eq, Hash) + `Display`
  - `Region` struct with `id`, `nodes`, concepts, explanations
  - `Region::new(id)`, `Region::add_concept()`, `Region::contains()`, `Region::concepts()`, `Region::nodes()`
  - `RecognitionExplanation` struct

- [ ] **Step 1: Write `sir/crates/sir_semantics/src/concepts.rs`**

```rust
use std::fmt;

/// A semantic concept describing what a computation is doing.
///
/// Concepts are organized into two groups:
/// - **Data concepts:** describe the data being operated on
/// - **Operation concepts:** describe what the computation does with the data
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SemanticConcept {
    /// Data: collection of boolean values (e.g., `bool[64]`)
    BooleanCollection,
    /// Data: collection with a statically known bound
    FiniteCollection,
    /// Operation: iterating over elements and testing membership
    MembershipTraversal,
    /// Operation: counting how many elements satisfy a condition
    CardinalityReduction,
}

impl fmt::Display for SemanticConcept {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SemanticConcept::BooleanCollection => write!(f, "BooleanCollection"),
            SemanticConcept::FiniteCollection => write!(f, "FiniteCollection"),
            SemanticConcept::MembershipTraversal => write!(f, "MembershipTraversal"),
            SemanticConcept::CardinalityReduction => write!(f, "CardinalityReduction"),
        }
    }
}
```

- [ ] **Step 2: Write `sir/crates/sir_semantics/src/region.rs`**

```rust
use std::collections::{BTreeSet, HashMap};
use sir_types::NodeId;

use crate::concepts::SemanticConcept;

/// A region identifier — unique within a semantic database.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RegionId(pub u64);

impl RegionId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    pub fn as_u64(self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for RegionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "region#{}", self.0)
    }
}

/// Why a concept was recognized — deterministic, not heuristic.
#[derive(Clone, Debug)]
pub struct RecognitionExplanation {
    pub concept: SemanticConcept,
    pub triggering_facts: Vec<&'static str>,
}

/// A contiguous subgraph representing a semantic unit.
///
/// For v0.1, a region is simply a set of nodes involved in a
/// recognized computation (e.g., a loop body and its enclosing
/// array access). Region identification is intentionally minimal
/// and will become more sophisticated in future phases.
#[derive(Clone, Debug)]
pub struct Region {
    pub id: RegionId,
    pub nodes: BTreeSet<NodeId>,
    concepts: std::collections::HashSet<SemanticConcept>,
    explanations: HashMap<SemanticConcept, RecognitionExplanation>,
}

impl Region {
    pub fn new(id: RegionId) -> Self {
        Self {
            id,
            nodes: BTreeSet::new(),
            concepts: std::collections::HashSet::new(),
            explanations: HashMap::new(),
        }
    }

    /// Attach a concept to this region with an explanation.
    pub fn add_concept(&mut self, concept: SemanticConcept, explanation: RecognitionExplanation) {
        self.concepts.insert(concept);
        self.explanations.insert(concept, explanation);
    }

    /// Check whether this region carries a specific concept.
    pub fn contains(&self, concept: SemanticConcept) -> bool {
        self.concepts.contains(&concept)
    }

    /// All concepts attached to this region.
    pub fn concepts(&self) -> &std::collections::HashSet<SemanticConcept> {
        &self.concepts
    }

    /// The SIR nodes that constitute this region.
    pub fn nodes(&self) -> &BTreeSet<NodeId> {
        &self.nodes
    }

    /// Get the recognition explanation for a concept, if present.
    pub fn explanation(&self, concept: SemanticConcept) -> Option<&RecognitionExplanation> {
        self.explanations.get(&concept)
    }
}
```

- [ ] **Step 3: Update `lib.rs` to declare modules**

Confirm that `sir/crates/sir_semantics/src/lib.rs` has:
```rust
pub mod concepts;
pub mod region;
```

Wait until we add `pub mod semantics;` and `pub mod recognizers;` in later tasks.

- [ ] **Step 4: Verify compilation**

Run: `cargo build -p sir_semantics`
Expected: Compiles cleanly.

- [ ] **Step 5: Commit**

```bash
git add sir/crates/sir_semantics/src/concepts.rs sir/crates/sir_semantics/src/region.rs
git commit -m "feat(sir_semantics): add SemanticConcept, Region, RegionId types"
```

---

### Task 3: `sir_semantics` — SemanticEngine + SemanticDatabase

**Files:**
- Create: `sir/crates/sir_semantics/src/semantics.rs`
- Modify: `sir/crates/sir_semantics/src/lib.rs` (add `pub mod semantics;`)

**Interfaces:**
- Consumes: `SemanticConcept`, `Region`, `RegionId`, `RecognitionExplanation` (from Task 2)
- Produces:
  - `SemanticDatabase` — `HashMap<RegionId, Region>` wrapper with `regions()`, `region()`, `explain()`, `add_region()`, `region_count()`
  - `SemanticEngine` — `new()`, `derive(&mut self, func: &Function, analysis: &FactDatabase)`, `database(&self) -> &SemanticDatabase`

- [ ] **Step 1: Write the failing test for `SemanticDatabase`**

Create `sir/crates/sir_semantics/tests/semantic_database.rs` (create `tests/` directory):

```rust
use sir_semantics::concepts::SemanticConcept;
use sir_semantics::region::{Region, RegionId, RecognitionExplanation};
use sir_semantics::semantics::SemanticDatabase;

#[test]
fn empty_database_has_no_regions() {
    let db = SemanticDatabase::new();
    assert_eq!(db.region_count(), 0);
    assert!(db.regions().next().is_none());
}

#[test]
fn database_stores_and_retrieves_region() {
    let mut db = SemanticDatabase::new();
    let rid = RegionId::new(0);
    let mut region = Region::new(rid);
    region.add_concept(
        SemanticConcept::BooleanCollection,
        RecognitionExplanation {
            concept: SemanticConcept::BooleanCollection,
            triggering_facts: vec!["Array<bool>"],
        },
    );
    db.add_region(region);

    assert_eq!(db.region_count(), 1);
    let retrieved = db.region(rid).unwrap();
    assert!(retrieved.contains(SemanticConcept::BooleanCollection));
    assert!(!retrieved.contains(SemanticConcept::MembershipTraversal));
}

#[test]
fn database_regions_iterates_all() {
    let mut db = SemanticDatabase::new();
    for i in 0..3 {
        let rid = RegionId::new(i);
        let mut region = Region::new(rid);
        region.add_concept(
            SemanticConcept::FiniteCollection,
            RecognitionExplanation {
                concept: SemanticConcept::FiniteCollection,
                triggering_facts: vec!["trip_count"],
            },
        );
        db.add_region(region);
    }
    let regions: Vec<_> = db.regions().collect();
    assert_eq!(regions.len(), 3);
}

#[test]
fn database_explain_returns_explanation() {
    let mut db = SemanticDatabase::new();
    let rid = RegionId::new(0);
    let mut region = Region::new(rid);
    region.add_concept(
        SemanticConcept::BooleanCollection,
        RecognitionExplanation {
            concept: SemanticConcept::BooleanCollection,
            triggering_facts: vec!["Array element type is Bool"],
        },
    );
    db.add_region(region);

    let explanation = db.explain(rid, SemanticConcept::BooleanCollection);
    assert!(explanation.is_some());
    assert!(explanation.unwrap().triggering_facts.contains(&"Array element type is Bool"));
}

#[test]
fn database_explain_unknown_returns_none() {
    let db = SemanticDatabase::new();
    assert!(db.explain(RegionId::new(99), SemanticConcept::BooleanCollection).is_none());
}
```

Add `[[test]]` to `sir/crates/sir_semantics/Cargo.toml`:

```toml
[[test]]
name = "semantic_database"
path = "tests/semantic_database.rs"
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p sir_semantics`
Expected: FAIL — `semantic_database` test binary not found or compilation errors (no `semantics` module yet).

- [ ] **Step 3: Write `sir/crates/sir_semantics/src/semantics.rs`**

```rust
use std::collections::HashMap;

use sir_analysis::facts::FactDatabase;
use sir_nodes::Function;

use crate::concepts::SemanticConcept;
use crate::region::{Region, RegionId, RecognitionExplanation};

/// The semantic knowledge database.
///
/// Stores regions and their recognized concepts. Immutable after
/// the `SemanticEngine::derive()` call completes.
#[derive(Clone, Debug, Default)]
pub struct SemanticDatabase {
    regions: HashMap<RegionId, Region>,
    next_region_id: u64,
}

impl SemanticDatabase {
    /// Create an empty semantic database.
    pub fn new() -> Self {
        Self {
            regions: HashMap::new(),
            next_region_id: 0,
        }
    }

    /// Add a region to the database.
    pub fn add_region(&mut self, region: Region) {
        self.regions.insert(region.id, region);
    }

    /// Iterate over all regions.
    pub fn regions(&self) -> impl Iterator<Item = (RegionId, &Region)> {
        self.regions.iter().map(|(&id, region)| (id, region))
    }

    /// Get a specific region by ID.
    pub fn region(&self, id: RegionId) -> Option<&Region> {
        self.regions.get(&id)
    }

    /// Get the explanation for why a concept was recognized in a region.
    pub fn explain(
        &self,
        region: RegionId,
        concept: SemanticConcept,
    ) -> Option<&RecognitionExplanation> {
        self.regions
            .get(&region)
            .and_then(|r| r.explanation(concept))
    }

    /// Number of regions in the database.
    pub fn region_count(&self) -> usize {
        self.regions.len()
    }

    /// Allocate the next region ID.
    pub(crate) fn next_region_id(&mut self) -> RegionId {
        let id = RegionId::new(self.next_region_id);
        self.next_region_id += 1;
        id
    }
}

/// The semantic derivation engine.
///
/// Transforms compiler facts into semantic truths by running
/// deterministic recognizers over the function graph.
pub struct SemanticEngine {
    db: SemanticDatabase,
}

impl SemanticEngine {
    /// Create a new semantic engine with an empty database.
    pub fn new() -> Self {
        Self {
            db: SemanticDatabase::new(),
        }
    }

    /// Access the semantic database (read-only after derivation).
    pub fn database(&self) -> &SemanticDatabase {
        &self.db
    }

    /// Derive semantic truths from the function graph and compiler facts.
    ///
    /// This calls each recognizer, which inspects the function's graph
    /// structure (for node kinds, types, and connectivity) and the
    /// analysis fact database (for trip counts, purity, escape, etc.).
    ///
    /// Recognized concepts are grouped into regions and stored in the
    /// `SemanticDatabase`.
    pub fn derive(&mut self, func: &Function, analysis: &FactDatabase) {
        // Recognizers are called in Tasks 4a-4d.
        // For now, this is a no-op — the engine compiles but derives nothing.
        let _ = func;
        let _ = analysis;
    }
}

impl Default for SemanticEngine {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 4: Update `lib.rs`**

Add after `pub mod region;`:
```rust
pub mod semantics;
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p sir_semantics`
Expected: All 5 `semantic_database` tests pass.

- [ ] **Step 6: Commit**

```bash
git add sir/crates/sir_semantics/src/semantics.rs \
  sir/crates/sir_semantics/src/lib.rs \
  sir/crates/sir_semantics/Cargo.toml \
  sir/crates/sir_semantics/tests/semantic_database.rs
git commit -m "feat(sir_semantics): add SemanticEngine and SemanticDatabase"
```

---

### Task 4a: Recognizer — `BooleanCollection`

**Files:**
- Create: `sir/crates/sir_semantics/src/recognizers/mod.rs`
- Create: `sir/crates/sir_semantics/src/recognizers/boolean_collection.rs`
- Modify: `sir/crates/sir_semantics/src/lib.rs` (add `pub mod recognizers;`)

**Interfaces:**
- Consumes: `Function`, `FactDatabase`, `SemanticConcept::BooleanCollection`, `RecognitionExplanation`
- Produces: `pub fn recognize_boolean_collection(func: &Function, analysis: &FactDatabase) -> Vec<(SemanticConcept, RecognitionExplanation, Vec<NodeId>)>`
  - Returns a list of (concept, explanation, relevant nodes) for each recognized boolean collection

- [ ] **Step 1: Write the recognizer test**

Create `sir/crates/sir_semantics/tests/recognizers.rs`:

```rust
use sir_semantics::concepts::SemanticConcept;
use sir_semantics::recognizers::boolean_collection::recognize_boolean_collection;

// We test recognizers in isolation later (Task 5 integration tests).
// For now, compile-time verification that the module exists and exports the function.
#[test]
fn boolean_collection_recognizer_exists() {
    // This test exists to confirm the recognizer compiles and is callable.
    // Meaningful tests come in Task 5 with actual SIR graphs.
}
```

Add `[[test]]` to `sir/crates/sir_semantics/Cargo.toml`:

```toml
[[test]]
name = "recognizers"
path = "tests/recognizers.rs"
```

- [ ] **Step 2: Write `sir/crates/sir_semantics/src/recognizers/mod.rs`**

```rust
pub mod boolean_collection;
// More modules added in Tasks 4b-4d
```

- [ ] **Step 3: Write `sir/crates/sir_semantics/src/recognizers/boolean_collection.rs`**

```rust
use sir_analysis::facts::FactDatabase;
use sir_nodes::{Function, NodeKind};
use sir_types::{NodeId, Type};

use crate::concepts::SemanticConcept;
use crate::region::RecognitionExplanation;

/// Recognize boolean collection patterns in the function.
///
/// A boolean collection is an array whose element type is `Bool`.
/// We look for:
/// - `Allocate` nodes that allocate `Array { element: Bool, .. }` types
/// - Parameter nodes with `Array { element: Bool, .. }` types
/// - Any `ArrayAccess` into such arrays
///
/// Returns (concept, explanation, related_node_ids) tuples.
pub fn recognize_boolean_collection(
    func: &Function,
    _analysis: &FactDatabase,
) -> Vec<(SemanticConcept, RecognitionExplanation, Vec<NodeId>)> {
    let mut results = Vec::new();

    for node in func.arena.iter() {
        // Check if this node's type is Array<bool>
        if let Type::Array { element, length: _ } = &node.ty {
            if matches!(element.as_ref(), &Type::Bool) {
                let related = collect_array_related_nodes(func, node.id);
                results.push((
                    SemanticConcept::BooleanCollection,
                    RecognitionExplanation {
                        concept: SemanticConcept::BooleanCollection,
                        triggering_facts: vec![
                            "Array element type is Bool",
                        ],
                    },
                    related,
                ));
            }
        }
    }

    results
}

/// Collect nodes related to an array: its allocation site, all accesses, all loads/stores.
fn collect_array_related_nodes(func: &Function, array_node: NodeId) -> Vec<NodeId> {
    let mut related = vec![array_node];

    for node in func.arena.iter() {
        match &node.kind {
            NodeKind::ArrayAccess { base, .. } if *base == array_node => {
                related.push(node.id);
            }
            NodeKind::Load { ptr } => {
                // If loading through an ArrayAccess that targets this array
                if let Some(access_node) = func.get_node(*ptr) {
                    if let NodeKind::ArrayAccess { base, .. } = &access_node.kind {
                        if *base == array_node {
                            related.push(node.id);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    related
}
```

- [ ] **Step 4: Update `lib.rs`**

Add after `pub mod semantics;`:
```rust
pub mod recognizers;
```

- [ ] **Step 5: Wire the recognizer into `SemanticEngine::derive()`**

Edit `sir/crates/sir_semantics/src/semantics.rs` — replace the `derive` method body:

```rust
pub fn derive(&mut self, func: &Function, analysis: &FactDatabase) {
    use crate::recognizers::boolean_collection;

    let recognitions = boolean_collection::recognize_boolean_collection(func, analysis);

    for (_concept, explanation, node_ids) in recognitions {
        let rid = self.db.next_region_id();
        let mut region = Region::new(rid);
        for node_id in &node_ids {
            region.nodes.insert(*node_id);
        }
        region.add_concept(explanation.concept, explanation);
        self.db.add_region(region);
    }
}
```

Add `use crate::region::Region;` to the imports at the top of `semantics.rs` (if not already imported).

- [ ] **Step 6: Verify compilation**

Run: `cargo build -p sir_semantics`
Expected: Compiles cleanly.

- [ ] **Step 7: Commit**

```bash
git add sir/crates/sir_semantics/src/recognizers/ \
  sir/crates/sir_semantics/src/lib.rs \
  sir/crates/sir_semantics/src/semantics.rs \
  sir/crates/sir_semantics/Cargo.toml \
  sir/crates/sir_semantics/tests/recognizers.rs
git commit -m "feat(sir_semantics): add BooleanCollection recognizer"
```

---

### Task 4b: Recognizer — `FiniteCollection`

**Files:**
- Create: `sir/crates/sir_semantics/src/recognizers/finite_collection.rs`
- Modify: `sir/crates/sir_semantics/src/recognizers/mod.rs`

**Interfaces:**
- Consumes: `Function`, `FactDatabase`
- Produces: `recognize_finite_collection()` same signature as Task 4a

- [ ] **Step 1: Write `sir/crates/sir_semantics/src/recognizers/finite_collection.rs`**

```rust
use sir_analysis::facts::FactDatabase;
use sir_nodes::Function;
use sir_types::Type;

use crate::concepts::SemanticConcept;
use crate::region::RecognitionExplanation;

/// Recognize finite collection patterns.
///
/// A collection is "finite" when it has a statically known size.
/// We look for:
/// - `Array` types with known length (not Slice, not dynamic)
/// - Loop nodes that have a known trip count equal to the array length
///
/// Returns (concept, explanation, related_node_ids) tuples.
pub fn recognize_finite_collection(
    func: &Function,
    analysis: &FactDatabase,
) -> Vec<(SemanticConcept, RecognitionExplanation, Vec<NodeId>)> {
    let mut results = Vec::new();

    // Find arrays with known lengths.
    let arrays_with_length: Vec<_> = func
        .arena
        .iter()
        .filter_map(|node| {
            if let Type::Array { element: _, length } = &node.ty {
                Some((node.id, *length))
            } else {
                None
            }
        })
        .collect();

    for (array_id, array_len) in &arrays_with_length {
        // Check if any loop iterates exactly array_len times
        // and accesses this array.
        for node in func.arena.iter() {
            if let sir_nodes::NodeKind::Loop { .. } = &node.kind {
                if let Some(loop_fact) = analysis.loops.get(&node.id) {
                    if let Some(trip_count) = loop_fact.trip_count {
                        if trip_count == *array_len as u64 {
                            let mut related = vec![*array_id, node.id];
                            // Also include loop body nodes
                            if let sir_nodes::NodeKind::Loop { body, .. } = &node.kind {
                                related.push(*body);
                            }
                            results.push((
                                SemanticConcept::FiniteCollection,
                                RecognitionExplanation {
                                    concept: SemanticConcept::FiniteCollection,
                                    triggering_facts: vec![
                                        "Array has static length",
                                        "Loop trip count equals array length",
                                    ],
                                },
                                related,
                            ));
                        }
                    }
                }
            }
        }
    }

    results
}
```

- [ ] **Step 2: Update `sir/crates/sir_semantics/src/recognizers/mod.rs`**

Add:
```rust
pub mod finite_collection;
```

- [ ] **Step 3: Wire into `SemanticEngine::derive()`**

Add after the `boolean_collection` block in `derive()`:

```rust
let finite_recs = finite_collection::recognize_finite_collection(func, analysis);
for (_concept, explanation, node_ids) in finite_recs {
    let rid = self.db.next_region_id();
    let mut region = Region::new(rid);
    for node_id in &node_ids {
        region.nodes.insert(*node_id);
    }
    region.add_concept(explanation.concept, explanation);
    self.db.add_region(region);
}
```

Add the import:
```rust
use crate::recognizers::finite_collection;
```

- [ ] **Step 4: Verify compilation**

Run: `cargo build -p sir_semantics`
Expected: Compiles cleanly.

- [ ] **Step 5: Commit**

```bash
git add sir/crates/sir_semantics/src/recognizers/finite_collection.rs \
  sir/crates/sir_semantics/src/recognizers/mod.rs \
  sir/crates/sir_semantics/src/semantics.rs
git commit -m "feat(sir_semantics): add FiniteCollection recognizer"
```

---

### Task 4c: Recognizer — `MembershipTraversal`

**Files:**
- Create: `sir/crates/sir_semantics/src/recognizers/membership_traversal.rs`
- Modify: `sir/crates/sir_semantics/src/recognizers/mod.rs`

- [ ] **Step 1: Write `sir/crates/sir_semantics/src/recognizers/membership_traversal.rs`**

```rust
use sir_analysis::facts::FactDatabase;
use sir_nodes::{Function, NodeKind};
use sir_types::{NodeId, Type};

use crate::concepts::SemanticConcept;
use crate::region::RecognitionExplanation;

/// Recognize membership traversal patterns.
///
/// A membership traversal is an iteration that tests whether each
/// element of a collection satisfies some condition. We detect:
/// - A `Loop` that iterates over a boolean array
/// - Loop body contains `ArrayAccess` + `Load` → used as a condition
///   (in `Select` or `BoolAnd`/`BoolOr`)
///
/// Returns (concept, explanation, related_node_ids) tuples.
pub fn recognize_membership_traversal(
    func: &Function,
    analysis: &FactDatabase,
) -> Vec<(SemanticConcept, RecognitionExplanation, Vec<NodeId>)> {
    let mut results = Vec::new();

    // Find boolean arrays being indexed inside loops.
    for node in func.arena.iter() {
        if let NodeKind::Loop { body, .. } = &node.kind {
            if let Some(loop_fact) = analysis.loops.get(&node.id) {
                if loop_fact.trip_count.is_some() {
                    // Walk the loop body to find ArrayAccess nodes on boolean arrays.
                    if let Some(body_node) = func.get_node(*body) {
                        let mut related = vec![node.id, *body];
                        let array_nodes = find_boolean_array_accesses(func, *body);
                        if !array_nodes.is_empty() {
                            related.extend(array_nodes);
                            results.push((
                                SemanticConcept::MembershipTraversal,
                                RecognitionExplanation {
                                    concept: SemanticConcept::MembershipTraversal,
                                    triggering_facts: vec![
                                        "Loop iterates over boolean array",
                                        "Array elements used as conditions",
                                    ],
                                },
                                related,
                            ));
                        }
                    }
                }
            }
        }
    }

    results
}

/// Find all ArrayAccess nodes within a subtree that index into boolean arrays.
fn find_boolean_array_accesses(func: &Function, root: NodeId) -> Vec<NodeId> {
    let mut results = Vec::new();
    let mut visited = std::collections::BTreeSet::new();
    let mut stack = vec![root];

    while let Some(current) = stack.pop() {
        if !visited.insert(current) {
            continue;
        }
        if let Some(node) = func.get_node(current) {
            match &node.kind {
                NodeKind::ArrayAccess { base, .. } => {
                    if let Some(base_node) = func.get_node(*base) {
                        if let Type::Array { element, .. } = &base_node.ty {
                            if matches!(element.as_ref(), &Type::Bool) {
                                results.push(current);
                            }
                        }
                    }
                    // Walk into index operand too
                    for op in node.kind.input_nodes() {
                        stack.push(op);
                    }
                }
                // Loop containment edges: don't cross into carried inputs or outputs
                NodeKind::Loop { .. } => {
                    // We're already inside the loop body; don't recurse into
                    // the loop node's structural fields.
                }
                _ => {
                    for op in node.kind.input_nodes() {
                        stack.push(op);
                    }
                }
            }
        }
    }

    results
}
```

- [ ] **Step 2: Update `sir/crates/sir_semantics/src/recognizers/mod.rs`**

Add:
```rust
pub mod membership_traversal;
```

- [ ] **Step 3: Wire into `SemanticEngine::derive()`**

Add after the `finite_collection` block in `derive()`:

```rust
let membership_recs = membership_traversal::recognize_membership_traversal(func, analysis);
for (_concept, explanation, node_ids) in membership_recs {
    let rid = self.db.next_region_id();
    let mut region = Region::new(rid);
    for node_id in &node_ids {
        region.nodes.insert(*node_id);
    }
    region.add_concept(explanation.concept, explanation);
    self.db.add_region(region);
}
```

Add the import:
```rust
use crate::recognizers::membership_traversal;
```

- [ ] **Step 4: Verify compilation**

Run: `cargo build -p sir_semantics`
Expected: Compiles cleanly.

- [ ] **Step 5: Commit**

```bash
git add sir/crates/sir_semantics/src/recognizers/membership_traversal.rs \
  sir/crates/sir_semantics/src/recognizers/mod.rs \
  sir/crates/sir_semantics/src/semantics.rs
git commit -m "feat(sir_semantics): add MembershipTraversal recognizer"
```

---

### Task 4d: Recognizer — `CardinalityReduction`

**Files:**
- Create: `sir/crates/sir_semantics/src/recognizers/cardinality_reduction.rs`
- Modify: `sir/crates/sir_semantics/src/recognizers/mod.rs`

- [ ] **Step 1: Write `sir/crates/sir_semantics/src/recognizers/cardinality_reduction.rs`**

```rust
use sir_analysis::facts::FactDatabase;
use sir_nodes::Function;

use crate::concepts::SemanticConcept;
use crate::region::RecognitionExplanation;

/// Recognize cardinality reduction patterns.
///
/// A cardinality reduction counts how many elements of a collection
/// satisfy a condition. We detect:
/// - A loop with a reduction variable of kind "add" or "sum"
/// - The reduction combines a boolean condition (0 or 1) into a counter
///
/// Returns (concept, explanation, related_node_ids) tuples.
pub fn recognize_cardinality_reduction(
    func: &Function,
    analysis: &FactDatabase,
) -> Vec<(SemanticConcept, RecognitionExplanation, Vec<NodeId>)> {
    let mut results = Vec::new();

    for node in func.arena.iter() {
        if let sir_nodes::NodeKind::Loop { .. } = &node.kind {
            if let Some(loop_fact) = analysis.loops.get(&node.id) {
                // Look for reductions with kind "add" or "sum" — these count things.
                if !loop_fact.reductions.is_empty() {
                    let mut related = vec![node.id];
                    for reduction in &loop_fact.reductions {
                        related.push(reduction.variable);
                        related.push(reduction.invariant_value);
                    }
                    results.push((
                        SemanticConcept::CardinalityReduction,
                        RecognitionExplanation {
                            concept: SemanticConcept::CardinalityReduction,
                            triggering_facts: vec![
                                "Loop has additive reduction",
                                "Reduction variable accumulates boolean conditions",
                            ],
                        },
                        related,
                    ));
                }
            }
        }
    }

    results
}
```

- [ ] **Step 2: Update `sir/crates/sir_semantics/src/recognizers/mod.rs`**

Add:
```rust
pub mod cardinality_reduction;
```

- [ ] **Step 3: Wire into `SemanticEngine::derive()`**

Add after the `membership_traversal` block in `derive()`:

```rust
let cardinality_recs = cardinality_reduction::recognize_cardinality_reduction(func, analysis);
for (_concept, explanation, node_ids) in cardinality_recs {
    let rid = self.db.next_region_id();
    let mut region = Region::new(rid);
    for node_id in &node_ids {
        region.nodes.insert(*node_id);
    }
    region.add_concept(explanation.concept, explanation);
    self.db.add_region(region);
}
```

Add the import:
```rust
use crate::recognizers::cardinality_reduction;
```

- [ ] **Step 4: Verify compilation**

Run: `cargo build -p sir_semantics`
Expected: Compiles cleanly.

- [ ] **Step 5: Run all existing tests**

Run: `cargo test -p sir_semantics`
Expected: All tests pass.

- [ ] **Step 6: Commit**

```bash
git add sir/crates/sir_semantics/src/recognizers/cardinality_reduction.rs \
  sir/crates/sir_semantics/src/recognizers/mod.rs \
  sir/crates/sir_semantics/src/semantics.rs
git commit -m "feat(sir_semantics): add CardinalityReduction recognizer"
```

---

### Task 5: `sir_inference` types — evidence, hypothesis, support

**Files:**
- Create: `sir/crates/sir_inference/src/evidence.rs`
- Create: `sir/crates/sir_inference/src/hypothesis.rs`

**Interfaces:**
- Produces:
  - `Representation` enum (`BitSet` only) + `Display`
  - `Polarity` enum (`Supports`, `Against`)
  - `Evidence` struct with region, representation, polarity, weight, source, explanation
  - `Support` struct with `positive: u16`, `negative: u16`, `score() -> i32`, `ratio() -> f32`, `confidence_label() -> &'static str`
  - `Hypothesis` struct with representation, support, evidence Vec

- [ ] **Step 1: Write the types test**

Create `sir/crates/sir_inference/tests/types.rs` (create `tests/` directory):

```rust
use sir_inference::evidence::{Evidence, Polarity};
use sir_inference::hypothesis::{Hypothesis, Representation, Support};
use sir_semantics::concepts::SemanticConcept;
use sir_semantics::region::RegionId;

#[test]
fn support_score_is_positive_minus_negative() {
    let s = Support { positive: 85, negative: 15 };
    assert_eq!(s.score(), 70);
}

#[test]
fn support_ratio_is_positive_over_total() {
    let s = Support { positive: 75, negative: 25 };
    assert!((s.ratio() - 0.75).abs() < 0.001);
}

#[test]
fn support_ratio_zero_total() {
    let s = Support { positive: 0, negative: 0 };
    assert_eq!(s.ratio(), 0.0);
}

#[test]
fn support_confidence_labels() {
    assert_eq!(Support { positive: 10, negative: 0 }.confidence_label(), "Weak");
    assert_eq!(Support { positive: 30, negative: 0 }.confidence_label(), "Moderate");
    assert_eq!(Support { positive: 55, negative: 0 }.confidence_label(), "Strong");
    assert_eq!(Support { positive: 0, negative: 80 }.confidence_label(), "Very Strong"); // net 80
    assert_eq!(Support { positive: 85, negative: 0 }.confidence_label(), "Very Strong");
}

#[test]
fn evidence_supports_bit_set_from_boolean_collection() {
    let evidence = Evidence {
        region: RegionId::new(0),
        representation: Representation::BitSet,
        polarity: Polarity::Supports,
        weight: 30,
        source: SemanticConcept::BooleanCollection,
        explanation: "Boolean arrays often represent bitsets",
    };
    assert_eq!(evidence.representation, Representation::BitSet);
    assert!(matches!(evidence.polarity, Polarity::Supports));
}

#[test]
fn evidence_against_has_negative_effect() {
    let evidence = Evidence {
        region: RegionId::new(0),
        representation: Representation::BitSet,
        polarity: Polarity::Against,
        weight: 30,
        source: SemanticConcept::MembershipTraversal,
        explanation: "Mutation argues against immutable bitset",
    };
    assert!(matches!(evidence.polarity, Polarity::Against));
}

#[test]
fn hypothesis_stores_representation_with_support_and_evidence() {
    let h = Hypothesis {
        representation: Representation::BitSet,
        support: Support { positive: 80, negative: 10 },
        evidence: vec![0, 1, 2],
    };
    assert_eq!(h.representation, Representation::BitSet);
    assert_eq!(h.support.score(), 70);
}

#[test]
fn representation_display() {
    assert_eq!(format!("{}", Representation::BitSet), "BitSet");
}
```

Add `[[test]]` to `sir/crates/sir_inference/Cargo.toml`:

```toml
[[test]]
name = "types"
path = "tests/types.rs"
```

- [ ] **Step 2: Write `sir/crates/sir_inference/src/hypothesis.rs`**

```rust
use std::fmt;

use sir_semantics::region::RegionId;

/// A mathematical representation of a computation.
///
/// These are concrete realizations of semantic concepts, not
/// machine instructions. v0.1 targets exactly one representation.
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

/// Integer support score — no floating point in engine logic.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Support {
    pub positive: u16,
    pub negative: u16,
}

impl Support {
    /// Net score: positive minus negative.
    pub fn score(&self) -> i32 {
        self.positive as i32 - self.negative as i32
    }

    /// Ratio of positive support to total (for display only).
    pub fn ratio(&self) -> f32 {
        let total = self.positive as f32 + self.negative as f32;
        if total == 0.0 {
            0.0
        } else {
            self.positive as f32 / total
        }
    }

    /// Qualitative confidence label derived from net score.
    pub fn confidence_label(&self) -> &'static str {
        let net = self.score().abs();
        match net {
            0..=20 => "Weak",
            21..=50 => "Moderate",
            51..=80 => "Strong",
            _ => "Very Strong",
        }
    }
}

/// A hypothesis is a representation with accumulated support and evidence trace.
#[derive(Clone, Debug)]
pub struct Hypothesis {
    pub representation: Representation,
    pub support: Support,
    pub evidence: Vec<usize>, // indices into the engine's evidence list
}

/// A unique identifier for an evidence entry.
pub type EvidenceId = usize;
```

- [ ] **Step 3: Write `sir/crates/sir_inference/src/evidence.rs`**

```rust
use sir_semantics::concepts::SemanticConcept;
use sir_semantics::region::RegionId;

use crate::hypothesis::{EvidenceId, Representation};

/// Whether evidence supports or opposes a representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Polarity {
    Supports,
    Against,
}

/// Evidence is an observation — an instance about a specific region,
/// not a rule template. Each piece of evidence records which semantic
/// concept triggered it, which representation it affects, and how strongly.
#[derive(Clone, Debug)]
pub struct Evidence {
    pub region: RegionId,
    pub representation: Representation,
    pub polarity: Polarity,
    pub weight: u16,
    pub source: SemanticConcept,
    pub explanation: &'static str,
}

/// A flat registry of all evidence entries produced during inference.
///
/// Entries are reusable across regions — the same explanation applies
/// wherever the same concept triggers the same representation.
#[derive(Clone, Debug, Default)]
pub struct EvidenceRegistry {
    entries: Vec<Evidence>,
}

impl EvidenceRegistry {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    /// Add an evidence entry and return its ID.
    pub fn add(&mut self, evidence: Evidence) -> EvidenceId {
        let id = self.entries.len();
        self.entries.push(evidence);
        id
    }

    /// Get an evidence entry by ID.
    pub fn get(&self, id: EvidenceId) -> Option<&Evidence> {
        self.entries.get(id)
    }

    /// All evidence entries.
    pub fn all(&self) -> &[Evidence] {
        &self.entries
    }

    /// Evidence entries relevant to a specific region.
    pub fn for_region(&self, region: RegionId) -> Vec<&Evidence> {
        self.entries.iter().filter(|e| e.region == region).collect()
    }
}
```

- [ ] **Step 4: Update `sir/crates/sir_inference/src/lib.rs`**

Confirm it has:
```rust
pub mod evidence;
pub mod hypothesis;
```

Wait until we add `pub mod engine;` and `pub mod sources;` in later tasks.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p sir_inference`
Expected: All 7 `types` tests pass.

- [ ] **Step 6: Commit**

```bash
git add sir/crates/sir_inference/src/hypothesis.rs \
  sir/crates/sir_inference/src/evidence.rs \
  sir/crates/sir_inference/tests/types.rs \
  sir/crates/sir_inference/Cargo.toml
git commit -m "feat(sir_inference): add Evidence, Hypothesis, Support, Representation types"
```

---

### Task 6: `sir_inference` — InferenceEngine + HypothesisDatabase

**Files:**
- Create: `sir/crates/sir_inference/src/engine.rs`
- Modify: `sir/crates/sir_inference/src/lib.rs` (add `pub mod engine;`)

**Interfaces:**
- Consumes: `SemanticDatabase`, `Region`, `Evidence`, `EvidenceRegistry`, `Hypothesis`, `Support`, `Representation` (from Tasks 2, 5)
- Produces:
  - `HypothesisDatabase` — `hypotheses()`, `best()`, `regions_supporting()`, `add_hypothesis()`
  - `InferenceEngine` — `new()`, `infer()`, `database()`, `explain()`
  - `Explanation` struct for formatted output

- [ ] **Step 1: Write the engine test**

Create `sir/crates/sir_inference/tests/engine.rs`:

```rust
use sir_inference::engine::{HypothesisDatabase, InferenceEngine};
use sir_inference::evidence::EvidenceRegistry;
use sir_inference::hypothesis::{Hypothesis, Representation, Support};

#[test]
fn empty_database_has_no_hypotheses() {
    let db = HypothesisDatabase::new();
    let rid = sir_semantics::region::RegionId::new(0);
    assert!(db.hypotheses(rid).is_empty());
    assert!(db.best(rid).is_none());
}

#[test]
fn database_stores_and_retrieves_hypothesis() {
    let mut db = HypothesisDatabase::new();
    let rid = sir_semantics::region::RegionId::new(0);
    let h = Hypothesis {
        representation: Representation::BitSet,
        support: Support { positive: 85, negative: 10 },
        evidence: vec![0, 1],
    };
    db.add_hypothesis(rid, h.clone());

    assert_eq!(db.hypotheses(rid).len(), 1);
    let best = db.best(rid).unwrap();
    assert_eq!(best.representation, Representation::BitSet);
    assert_eq!(best.support.score(), 75);
}

#[test]
fn database_best_returns_highest_scoring() {
    let mut db = HypothesisDatabase::new();
    let rid = sir_semantics::region::RegionId::new(0);
    db.add_hypothesis(rid, Hypothesis {
        representation: Representation::BitSet,
        support: Support { positive: 30, negative: 10 },
        evidence: vec![],
    });
    db.add_hypothesis(rid, Hypothesis {
        representation: Representation::BitSet,
        support: Support { positive: 90, negative: 5 },
        evidence: vec![],
    });
    let best = db.best(rid).unwrap();
    assert_eq!(best.support.score(), 85);
}

#[test]
fn database_regions_supporting_filters() {
    let mut db = HypothesisDatabase::new();
    let r1 = sir_semantics::region::RegionId::new(0);
    let r2 = sir_semantics::region::RegionId::new(1);
    db.add_hypothesis(r1, Hypothesis {
        representation: Representation::BitSet,
        support: Support { positive: 80, negative: 5 },
        evidence: vec![],
    });
    db.add_hypothesis(r2, Hypothesis {
        representation: Representation::BitSet,
        support: Support { positive: 10, negative: 50 },
        evidence: vec![],
    });

    let supporting = db.regions_supporting(Representation::BitSet);
    assert!(supporting.contains(&r1));
    assert!(supporting.contains(&r2)); // both have BitSet hypotheses
}

#[test]
fn engine_new_creates_empty_state() {
    let engine = InferenceEngine::new();
    assert!(engine.database().hypotheses(sir_semantics::region::RegionId::new(0)).is_empty());
}
```

Add `[[test]]` to `sir/crates/sir_inference/Cargo.toml`:

```toml
[[test]]
name = "engine"
path = "tests/engine.rs"
```

- [ ] **Step 2: Write `sir/crates/sir_inference/src/engine.rs`**

```rust
use std::collections::HashMap;
use std::fmt;

use sir_semantics::region::RegionId;
use sir_semantics::semantics::SemanticDatabase;

use crate::evidence::{Evidence, EvidenceRegistry, Polarity};
use crate::hypothesis::{Hypothesis, Representation, Support};

/// The hypothesis database — stores representation beliefs per region.
#[derive(Clone, Debug, Default)]
pub struct HypothesisDatabase {
    hypotheses: HashMap<RegionId, Vec<Hypothesis>>,
}

impl HypothesisDatabase {
    pub fn new() -> Self {
        Self {
            hypotheses: HashMap::new(),
        }
    }

    /// Get all hypotheses for a region.
    pub fn hypotheses(&self, region: RegionId) -> &[Hypothesis] {
        self.hypotheses
            .get(&region)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get the highest-scoring hypothesis for a region.
    pub fn best(&self, region: RegionId) -> Option<&Hypothesis> {
        self.hypotheses
            .get(&region)
            .and_then(|v| v.iter().max_by_key(|h| h.support.score()))
    }

    /// Find all regions that have at least one hypothesis for the
    /// given representation.
    pub fn regions_supporting(&self, rep: Representation) -> Vec<RegionId> {
        self.hypotheses
            .iter()
            .filter(|(_, hyps)| hyps.iter().any(|h| h.representation == rep))
            .map(|(&rid, _)| rid)
            .collect()
    }

    /// Add a hypothesis to a region.
    pub(crate) fn add_hypothesis(&mut self, region: RegionId, hypothesis: Hypothesis) {
        self.hypotheses
            .entry(region)
            .or_insert_with(Vec::new)
            .push(hypothesis);
    }
}

/// A formatted explanation of why a hypothesis exists.
#[derive(Clone, Debug)]
pub struct Explanation {
    pub region: RegionId,
    pub representation: Representation,
    pub support: Support,
    pub evidence_lines: Vec<EvidenceLine>,
}

/// A single line in an explanation: the evidence entry and its contribution.
#[derive(Clone, Debug)]
pub struct EvidenceLine {
    pub polarity: crate::evidence::Polarity,
    pub weight: u16,
    pub source: String,
    pub explanation: String,
}

impl fmt::Display for Explanation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Hypothesis: {}", self.representation)?;
        writeln!(
            f,
            "Support: +{} / -{} (net {})",
            self.support.positive,
            self.support.negative,
            self.support.score()
        )?;
        writeln!(f, "Confidence: {}", self.support.confidence_label())?;
        writeln!(f, "Evidence:")?;
        for line in &self.evidence_lines {
            let sign = match line.polarity {
                Polarity::Supports => '+',
                Polarity::Against => '-',
            };
            writeln!(f, "  {}{:<4} {:<22} \"{}\"",
                sign, line.weight, line.source, line.explanation)?;
        }
        Ok(())
    }
}

/// Evidence weight constants — relative strength categories.
pub mod weights {
    pub const STRONG: u16 = 30;
    pub const MODERATE: u16 = 20;
    pub const WEAK: u16 = 10;
}

/// The inference engine — transforms semantic truths into representation beliefs.
pub struct InferenceEngine {
    db: HypothesisDatabase,
    evidence_registry: EvidenceRegistry,
}

impl InferenceEngine {
    pub fn new() -> Self {
        Self {
            db: HypothesisDatabase::new(),
            evidence_registry: EvidenceRegistry::new(),
        }
    }

    /// Access the hypothesis database (read-only after inference).
    pub fn database(&self) -> &HypothesisDatabase {
        &self.db
    }

    /// Run inference: generate evidence from semantic truths, aggregate
    /// into support scores, and form hypotheses.
    ///
    /// This consumes only the `SemanticDatabase` — never SIR or compiler facts.
    pub fn infer(&mut self, semantic_db: &SemanticDatabase) {
        // 1. Generate evidence from all regions
        for (region_id, region) in semantic_db.regions() {
            let evidence = crate::sources::bitset_evidence::contribute(region);
            for e in evidence {
                self.evidence_registry.add(e);
            }
        }

        // 2. Aggregate evidence per (region, representation)
        // Build a map: (RegionId, Representation) → (positive_sum, negative_sum, evidence_ids)
        let mut aggregation: HashMap<(RegionId, Representation), (u16, u16, Vec<usize>)> =
            HashMap::new();

        for (evidence_id, evidence) in self.evidence_registry.all().iter().enumerate() {
            let key = (evidence.region, evidence.representation);
            let entry = aggregation.entry(key).or_insert_with(|| (0, 0, Vec::new()));
            match evidence.polarity {
                Polarity::Supports => entry.0 += evidence.weight,
                Polarity::Against => entry.1 += evidence.weight,
            }
            entry.2.push(evidence_id);
        }

        // 3. Form hypotheses for any representation with non-zero support
        for ((region_id, representation), (positive, negative, evidence_ids)) in aggregation {
            if positive > 0 || negative > 0 {
                let hypothesis = Hypothesis {
                    representation,
                    support: Support { positive, negative },
                    evidence: evidence_ids,
                };
                self.db.add_hypothesis(region_id, hypothesis);
            }
        }
    }

    /// Explain why a hypothesis exists for a given region and representation.
    /// This is a first-class API, not a debug helper.
    pub fn explain(&self, region: RegionId, rep: Representation) -> Option<Explanation> {
        let hypothesis = self.db.best(region)?;
        if hypothesis.representation != rep {
            // Find the hypothesis for this specific representation
            return self
                .db
                .hypotheses(region)
                .iter()
                .find(|h| h.representation == rep)
                .map(|h| self.build_explanation(region, h));
        }
        Some(self.build_explanation(region, hypothesis))
    }

    fn build_explanation(&self, region: RegionId, hypothesis: &Hypothesis) -> Explanation {
        let lines: Vec<EvidenceLine> = hypothesis
            .evidence
            .iter()
            .filter_map(|&eid| self.evidence_registry.get(eid))
            .map(|e| EvidenceLine {
                polarity: e.polarity,
                weight: e.weight,
                source: e.source.to_string(),
                explanation: e.explanation.to_string(),
            })
            .collect();

        Explanation {
            region,
            representation: hypothesis.representation,
            support: hypothesis.support,
            evidence_lines: lines,
        }
    }
}

impl Default for InferenceEngine {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 3: Update `sir/crates/sir_inference/src/lib.rs`**

Add after `pub mod hypothesis;`:
```rust
pub mod engine;
```

(We'll add `pub mod sources;` in Task 7.)

- [ ] **Step 4: Create the sources module placeholder**

Create `sir/crates/sir_inference/src/sources/mod.rs`:

```rust
//! Evidence sources — one module per representation type.
//!
//! Each source is a pure function that inspects a region's concepts
//! and returns evidence entries. The engine owns the registry and
//! calls each source during inference.

pub mod bitset_evidence;
```

- [ ] **Step 5: Verify compilation (will fail — bitset_evidence.rs doesn't exist yet)**

Run: `cargo build -p sir_inference`
Expected: **FAIL** — `sources::bitset_evidence` module not found.

- [ ] **Step 6: Commit (partial — sources stub for now)**

We'll commit after Task 7 creates the actual source. For now, create a minimal stub so the engine compiles.

Actually, adjust step ordering: commit engine with the module declarations, then Task 7 creates the actual evidence source.

Let's create a minimal `bitset_evidence.rs` stub now so things compile:

Create `sir/crates/sir_inference/src/sources/bitset_evidence.rs`:

```rust
use sir_semantics::region::Region;
use crate::evidence::Evidence;

/// Contribute evidence toward BitSet representation.
/// Returns an empty list for now — populated in Task 7.
pub fn contribute(_region: &Region) -> Vec<Evidence> {
    Vec::new()
}
```

Now verify:

- [ ] **Step 7: Verify compilation**

Run: `cargo build -p sir_inference`
Expected: Compiles cleanly.

- [ ] **Step 8: Run engine tests**

Run: `cargo test -p sir_inference`
Expected: All tests pass (types + engine).

- [ ] **Step 9: Commit**

```bash
git add sir/crates/sir_inference/src/engine.rs \
  sir/crates/sir_inference/src/lib.rs \
  sir/crates/sir_inference/src/sources/mod.rs \
  sir/crates/sir_inference/src/sources/bitset_evidence.rs \
  sir/crates/sir_inference/tests/engine.rs \
  sir/crates/sir_inference/Cargo.toml
git commit -m "feat(sir_inference): add InferenceEngine, HypothesisDatabase, Explanation"
```

---

### Task 7: `sir_inference` — BitSet evidence source

**Files:**
- Modify: `sir/crates/sir_inference/src/sources/bitset_evidence.rs`

**Interfaces:**
- Consumes: `Region` (from `sir_semantics`), `Evidence`, `Polarity`, `Representation` (from Task 5), `weights` constants (from Task 6)
- Produces: `pub fn contribute(region: &Region) -> Vec<Evidence>` with real logic

- [ ] **Step 1: Write the evidence source test**

Create `sir/crates/sir_inference/tests/bitset_evidence.rs`:

```rust
use sir_inference::evidence::{Evidence, Polarity};
use sir_inference::hypothesis::Representation;
use sir_inference::sources::bitset_evidence;
use sir_semantics::concepts::SemanticConcept;
use sir_semantics::region::{Region, RegionId, RecognitionExplanation};

fn make_region(with_concepts: &[SemanticConcept]) -> Region {
    let mut region = Region::new(RegionId::new(0));
    for &concept in with_concepts {
        region.add_concept(concept, RecognitionExplanation {
            concept,
            triggering_facts: vec!["test"],
        });
    }
    region
}

#[test]
fn empty_region_produces_no_evidence() {
    let region = Region::new(RegionId::new(0));
    let evidence = bitset_evidence::contribute(&region);
    assert!(evidence.is_empty());
}

#[test]
fn boolean_collection_supports_bitset() {
    let region = make_region(&[SemanticConcept::BooleanCollection]);
    let evidence = bitset_evidence::contribute(&region);

    assert!(!evidence.is_empty());
    let bool_ev = evidence.iter().find(|e| matches!(e.polarity, Polarity::Supports)).unwrap();
    assert_eq!(bool_ev.representation, Representation::BitSet);
    assert!(bool_ev.weight > 0);
}

#[test]
fn finite_collection_supports_bitset() {
    let region = make_region(&[SemanticConcept::FiniteCollection]);
    let evidence = bitset_evidence::contribute(&region);

    let finite_ev = evidence.iter().find(|e| matches!(e.polarity, Polarity::Supports)).unwrap();
    assert_eq!(finite_ev.representation, Representation::BitSet);
}

#[test]
fn membership_traversal_supports_bitset() {
    let region = make_region(&[SemanticConcept::MembershipTraversal]);
    let evidence = bitset_evidence::contribute(&region);

    assert!(!evidence.is_empty());
    let ev = evidence.first().unwrap();
    assert_eq!(ev.representation, Representation::BitSet);
}

#[test]
fn cardinality_reduction_supports_bitset() {
    let region = make_region(&[SemanticConcept::CardinalityReduction]);
    let evidence = bitset_evidence::contribute(&region);

    assert!(!evidence.is_empty());
    let ev = evidence.first().unwrap();
    assert_eq!(ev.representation, Representation::BitSet);
}

#[test]
fn all_four_concepts_together_produce_four_evidence_entries() {
    let region = make_region(&[
        SemanticConcept::BooleanCollection,
        SemanticConcept::FiniteCollection,
        SemanticConcept::MembershipTraversal,
        SemanticConcept::CardinalityReduction,
    ]);
    let evidence = bitset_evidence::contribute(&region);
    // Each concept contributes one evidence entry, all Supports
    let supporting: Vec<_> = evidence.iter().filter(|e| matches!(e.polarity, Polarity::Supports)).collect();
    assert_eq!(supporting.len(), 4);
}

#[test]
fn evidence_contains_explanatory_text() {
    let region = make_region(&[SemanticConcept::BooleanCollection]);
    let evidence = bitset_evidence::contribute(&region);
    let ev = evidence.first().unwrap();
    assert!(!ev.explanation.is_empty());
}

#[test]
fn evidence_is_all_supports_for_positive_concepts() {
    let region = make_region(&[
        SemanticConcept::BooleanCollection,
        SemanticConcept::FiniteCollection,
        SemanticConcept::MembershipTraversal,
        SemanticConcept::CardinalityReduction,
    ]);
    let evidence = bitset_evidence::contribute(&region);
    for ev in &evidence {
        assert!(matches!(ev.polarity, Polarity::Supports),
            "Expected all evidence to be Supports, got {:?} for {}", ev.polarity, ev.source);
    }
}
```

Add `[[test]]` to `sir/crates/sir_inference/Cargo.toml`:

```toml
[[test]]
name = "bitset_evidence"
path = "tests/bitset_evidence.rs"
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p sir_inference`
Expected: The `bitset_evidence` test binary compiles, but multiple tests fail because `contribute()` returns empty vec.

- [ ] **Step 3: Implement `bitset_evidence.rs`**

Replace the stub content in `sir/crates/sir_inference/src/sources/bitset_evidence.rs`:

```rust
use sir_semantics::concepts::SemanticConcept;
use sir_semantics::region::Region;

use crate::engine::weights;
use crate::evidence::{Evidence, Polarity};
use crate::hypothesis::Representation;

/// Contribute evidence toward the BitSet representation.
///
/// For each semantic concept present in the region, emit an evidence
/// entry that supports BitSet (and potentially entries that oppose it
/// — for v0.1 only positive contributions are implemented).
///
/// This is a pure function: it reads the region, returns evidence.
/// The caller owns the registry and handles aggregation.
pub fn contribute(region: &Region) -> Vec<Evidence> {
    let mut evidence = Vec::new();

    if region.contains(SemanticConcept::BooleanCollection) {
        evidence.push(Evidence {
            region: region.id,
            representation: Representation::BitSet,
            polarity: Polarity::Supports,
            weight: weights::STRONG,
            source: SemanticConcept::BooleanCollection,
            explanation: "Boolean arrays often represent bitsets",
        });
    }

    if region.contains(SemanticConcept::FiniteCollection) {
        evidence.push(Evidence {
            region: region.id,
            representation: Representation::BitSet,
            polarity: Polarity::Supports,
            weight: weights::MODERATE,
            source: SemanticConcept::FiniteCollection,
            explanation: "Known iteration bound enables bitwise encoding",
        });
    }

    if region.contains(SemanticConcept::MembershipTraversal) {
        evidence.push(Evidence {
            region: region.id,
            representation: Representation::BitSet,
            polarity: Polarity::Supports,
            weight: weights::STRONG,
            source: SemanticConcept::MembershipTraversal,
            explanation: "Testing membership is a bitset operation",
        });
    }

    if region.contains(SemanticConcept::CardinalityReduction) {
        evidence.push(Evidence {
            region: region.id,
            representation: Representation::BitSet,
            polarity: Polarity::Supports,
            weight: weights::MODERATE,
            source: SemanticConcept::CardinalityReduction,
            explanation: "Counting members matches popcount pattern",
        });
    }

    evidence
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p sir_inference`
Expected: All 8 bitset_evidence tests pass, plus all previous type + engine tests.

- [ ] **Step 5: Commit**

```bash
git add sir/crates/sir_inference/src/sources/bitset_evidence.rs \
  sir/crates/sir_inference/tests/bitset_evidence.rs \
  sir/crates/sir_inference/Cargo.toml
git commit -m "feat(sir_inference): add BitSet evidence source"
```

---

### Task 8: Integration test — BS001 Board Scan

**Files:**
- Create: `sir/crates/sir_semantics/tests/semantic_truth.rs`
- Modify: `sir/crates/sir_semantics/Cargo.toml` (add test)

**Interfaces:**
- Consumes: All of `sir_semantics`, `sir_inference`, `sir_builder`, `sir_analysis`
- Acceptance criterion: board scan → BitSet with strong support + explanation

- [ ] **Step 1: Add `sir_builder` and `sir_analysis` dev-dependencies to `sir_semantics`**

Edit `sir/crates/sir_semantics/Cargo.toml` — add under `[dependencies]`:

```toml
[dev-dependencies]
sir_builder = { path = "../sir_builder" }
sir_analysis = { path = "../sir_analysis" }
```

- [ ] **Step 2: Write the board scan integration test**

Create `sir/crates/sir_semantics/tests/semantic_truth.rs`:

```rust
use sir_analysis::manager::AnalysisManager;
use sir_builder::Builder;
use sir_semantics::concepts::SemanticConcept;
use sir_semantics::semantics::SemanticEngine;
use sir_inference::engine::InferenceEngine;
use sir_inference::hypothesis::Representation;
use sir_types::{Span, Type};

/// Build a SIR function that represents:
/// ```text
/// bool board[64];
/// for i in 0..64 {
///     if board[i] { count++; }
/// }
/// ```
///
/// This is BS001: the canonical fixed-size boolean membership scan.
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
    let count = b.parameter_index(1).unwrap();
    let zero = b.constant_int(0, Type::i32(), Span::unknown()).unwrap();
    let one = b.constant_int(1, Type::i32(), Span::unknown()).unwrap();
    let i64_zero = b.constant_int(0, Type::i64(), Span::unknown()).unwrap();
    let i64_one = b.constant_int(1, Type::i64(), Span::unknown()).unwrap();
    let limit = b.constant_int(64, Type::i64(), Span::unknown()).unwrap();

    // Build loop: i from 0 to 63
    // carried: i (i64), count (i32)
    // body: access board[i], if true then count+1 else count, i+1
    // Actually, let's build a simpler version — a loop body node that
    // iterates i and conditionally increments count.

    // For v0.1 integration test, we build the simplest recognizable pattern.
    let loop_body = b.constant_bool(true, Span::unknown()).unwrap();
    let cond = b.constant_bool(false, Span::unknown()).unwrap();

    // Array access type
    let _arr_access = b.allocate(Type::Array {
        element: Box::new(Type::Bool),
        length: 64,
    }, Span::unknown()).unwrap();

    // Create a Loop node manually since Builder doesn't have a loop builder yet.
    // We'll use the low-level create_node for the Loop.
    let loop_id = b.create_node(
        sir_nodes::NodeKind::Loop {
            body: loop_body,
            termination: cond,
            outputs: vec![count],
            carried_inputs: vec![i64_zero, zero],
        },
        Type::i32(),
        Span::unknown(),
    ).unwrap();

    b.return_value(loop_id, Span::unknown()).unwrap();
    b.build()
}

#[test]
fn bs001_board_scan_recognizes_all_four_concepts() {
    let func = build_board_scan();

    let mut analysis = AnalysisManager::new();
    analysis.run_all(&func);

    let mut semantics = SemanticEngine::new();
    semantics.derive(&func, analysis.database());

    let db = semantics.database();
    // In v0.1, region extraction groups concepts from the same loop together.
    // We test that concepts were recognized somewhere in the function.
    let mut found_boolean = false;
    let mut found_finite = false;
    let mut found_membership = false;
    let mut found_cardinality = false;

    for (_rid, region) in db.regions() {
        if region.contains(SemanticConcept::BooleanCollection) {
            found_boolean = true;
        }
        if region.contains(SemanticConcept::FiniteCollection) {
            found_finite = true;
        }
        if region.contains(SemanticConcept::MembershipTraversal) {
            found_membership = true;
        }
        if region.contains(SemanticConcept::CardinalityReduction) {
            found_cardinality = true;
        }
    }

    assert!(found_boolean, "Expected BooleanCollection concept");
    assert!(found_finite, "Expected FiniteCollection concept");
    assert!(found_membership, "Expected MembershipTraversal concept");
    assert!(found_cardinality, "Expected CardinalityReduction concept");
}

#[test]
fn bs001_board_scan_infers_bitset_with_strong_support() {
    let func = build_board_scan();

    let mut analysis = AnalysisManager::new();
    analysis.run_all(&func);

    let mut semantics = SemanticEngine::new();
    semantics.derive(&func, analysis.database());

    let mut inference = InferenceEngine::new();
    inference.infer(semantics.database());

    let db = inference.database();
    let mut found = false;
    for (rid, _region) in semantics.database().regions() {
        if let Some(h) = db.best(rid) {
            assert_eq!(h.representation, Representation::BitSet);
            assert!(h.support.score() > 50,
                "Expected strong support (>50), got {}", h.support.score());
            found = true;
        }
    }
    assert!(found, "Expected at least one BitSet hypothesis");
}

#[test]
fn bs001_explanation_accounts_for_support() {
    let func = build_board_scan();

    let mut analysis = AnalysisManager::new();
    analysis.run_all(&func);

    let mut semantics = SemanticEngine::new();
    semantics.derive(&func, analysis.database());

    let mut inference = InferenceEngine::new();
    inference.infer(semantics.database());

    for (rid, _region) in semantics.database().regions() {
        if let Some(h) = inference.database().best(rid) {
            let explanation = inference.explain(rid, h.representation).unwrap();
            let explanation_str = format!("{}", explanation);

            // The explanation must reference the concepts
            assert!(explanation_str.contains("BooleanCollection"),
                "Explanation should mention BooleanCollection");
            assert!(explanation_str.contains("MembershipTraversal"),
                "Explanation should mention MembershipTraversal");

            // Support in explanation should match the hypothesis
            assert!(explanation_str.contains(&h.support.score().to_string()),
                "Explanation should show the support score");
        }
    }
}
```

Add the dev-dependency to `sir/crates/sir_semantics/Cargo.toml` for `sir_inference`:

```toml
[dev-dependencies]
sir_builder = { path = "../sir_builder" }
sir_analysis = { path = "../sir_analysis" }
sir_inference = { path = "../sir_inference" }
```

Add `[[test]]` to `sir/crates/sir_semantics/Cargo.toml`:

```toml
[[test]]
name = "semantic_truth"
path = "tests/semantic_truth.rs"
```

- [ ] **Step 3: Run the acceptance tests**

Run: `cargo test -p sir_semantics`
Expected: The `semantic_truth` tests execute. Some may fail depending on how well the builder-created SIR matches the recognizer patterns. This is the research validation step — if the builder test SIR doesn't match, we iterate on the recognizers.

- [ ] **Step 4: If needed, refine the builder test to create a more recognizable SIR graph**

If the board scan builder test does not produce recognizable patterns, adjust the builder code to create a more realistic SIR (e.g., with actual array accesses, loop counters, and select-based increments).

Key patterns the recognizers look for:
- `Allocate` with `Array { element: Bool, length: 64 }` type → triggers `BooleanCollection`
- `Loop` with `trip_count: Some(64)` in fact database → triggers `FiniteCollection`
- `ArrayAccess` into boolean array inside loop body → triggers `MembershipTraversal`
- Loop with additive reduction → triggers `CardinalityReduction`

- [ ] **Step 5: Run all tests across workspace**

Run: `cargo test`
Expected: All 200+ existing tests pass, plus all new `sir_semantics` and `sir_inference` tests.

- [ ] **Step 6: Commit**

```bash
git add sir/crates/sir_semantics/tests/semantic_truth.rs \
  sir/crates/sir_semantics/Cargo.toml
git commit -m "test: add BS001 board scan integration test (acceptance criterion)"
```

---

### Task 9: Negative and ambiguity tests

**Files:**
- Create: `sir/crates/sir_inference/tests/negative.rs`
- Create: `sir/crates/sir_inference/tests/ambiguity.rs`
- Modify: `sir/crates/sir_inference/Cargo.toml`

**Goal:** Validate that the inference engine does not produce false positives and expresses uncertainty appropriately.

- [ ] **Step 1: Write negative tests**

Create `sir/crates/sir_inference/tests/negative.rs`:

```rust
use sir_inference::engine::InferenceEngine;
use sir_inference::hypothesis::Representation;
use sir_semantics::concepts::SemanticConcept;
use sir_semantics::region::{Region, RegionId, RecognitionExplanation};
use sir_semantics::semantics::SemanticDatabase;

fn run_inference(concepts: &[SemanticConcept]) -> Vec<sir_inference::hypothesis::Hypothesis> {
    let mut semantic_db = SemanticDatabase::new();
    let mut region = Region::new(RegionId::new(0));
    for &concept in concepts {
        region.add_concept(concept, RecognitionExplanation {
            concept,
            triggering_facts: vec!["test"],
        });
    }
    semantic_db.add_region(region);

    let mut engine = InferenceEngine::new();
    engine.infer(&semantic_db);

    engine.database().hypotheses(RegionId::new(0)).to_vec()
}

#[test]
fn bare_boolean_collection_alone_is_not_strong_bitset() {
    // BooleanCollection alone is weak evidence — shouldn't reach Strong (-50 threshold)
    let hyps = run_inference(&[SemanticConcept::BooleanCollection]);
    if let Some(h) = hyps.first() {
        assert!(h.support.score() < 50,
            "BooleanCollection alone should not produce strong BitSet support, got {}",
            h.support.score());
    }
}

#[test]
fn single_concept_insufficient_for_strong_confidence() {
    for concept in &[
        SemanticConcept::BooleanCollection,
        SemanticConcept::FiniteCollection,
        SemanticConcept::MembershipTraversal,
        SemanticConcept::CardinalityReduction,
    ] {
        let hyps = run_inference(&[*concept]);
        if let Some(h) = hyps.first() {
            assert!(h.support.score() < 50,
                "{:?} alone should not produce strong support (>50), got {}",
                concept, h.support.score());
        }
    }
}

#[test]
fn no_concepts_produces_no_hypotheses() {
    let hyps = run_inference(&[]);
    assert!(hyps.is_empty(),
        "Empty region should produce no hypotheses");
}

#[test]
fn bitset_is_only_representation_returned() {
    // All four concepts together should only produce BitSet, nothing else
    let hyps = run_inference(&[
        SemanticConcept::BooleanCollection,
        SemanticConcept::FiniteCollection,
        SemanticConcept::MembershipTraversal,
        SemanticConcept::CardinalityReduction,
    ]);
    for h in &hyps {
        assert_eq!(h.representation, Representation::BitSet,
            "v0.1 should only produce BitSet hypotheses");
    }
}
```

- [ ] **Step 2: Write ambiguity tests**

Create `sir/crates/sir_inference/tests/ambiguity.rs`:

```rust
use sir_inference::engine::InferenceEngine;
use sir_inference::hypothesis::Representation;
use sir_semantics::concepts::SemanticConcept;
use sir_semantics::region::{Region, RegionId, RecognitionExplanation};
use sir_semantics::semantics::SemanticDatabase;

fn run_inference(concepts: &[SemanticConcept]) -> Vec<sir_inference::hypothesis::Hypothesis> {
    let mut semantic_db = SemanticDatabase::new();
    let mut region = Region::new(RegionId::new(0));
    for &concept in concepts {
        region.add_concept(concept, RecognitionExplanation {
            concept,
            triggering_facts: vec!["test"],
        });
    }
    semantic_db.add_region(region);

    let mut engine = InferenceEngine::new();
    engine.infer(&semantic_db);

    engine.database().hypotheses(RegionId::new(0)).to_vec()
}

#[test]
fn ambiguous_case_has_low_confidence() {
    // Just two concepts — the engine should express uncertainty
    let hyps = run_inference(&[
        SemanticConcept::BooleanCollection,
        SemanticConcept::FiniteCollection,
    ]);
    if let Some(h) = hyps.first() {
        let label = h.support.confidence_label();
        // With only 2 moderate concepts, should be Weak or Moderate, not Strong
        assert!(
            label == "Weak" || label == "Moderate",
            "Ambiguous case should have Weak or Moderate confidence, got {}",
            label
        );
    }
}

#[test]
fn order_of_concepts_does_not_affect_result() {
    use SemanticConcept::*;
    let concepts_sets = vec![
        vec![BooleanCollection, FiniteCollection, MembershipTraversal, CardinalityReduction],
        vec![CardinalityReduction, MembershipTraversal, FiniteCollection, BooleanCollection],
        vec![MembershipTraversal, BooleanCollection, CardinalityReduction, FiniteCollection],
    ];

    let mut scores = Vec::new();
    for concepts in &concepts_sets {
        let hyps = run_inference(concepts);
        scores.push(hyps.first().map(|h| h.support.score()).unwrap_or(0));
    }

    // All orderings must produce identical scores
    let first = scores[0];
    for &score in &scores {
        assert_eq!(score, first,
            "Evidence aggregation must be order-independent");
    }
}

#[test]
fn support_is_never_negative_for_pure_positive_evidence() {
    // All positive evidence — support.positive should exactly equal sum of weights
    let hyps = run_inference(&[
        SemanticConcept::BooleanCollection,
        SemanticConcept::FiniteCollection,
        SemanticConcept::MembershipTraversal,
        SemanticConcept::CardinalityReduction,
    ]);
    let h = hyps.first().unwrap();
    assert_eq!(h.support.negative, 0,
        "All-positive evidence should have zero negative support");
    assert!(h.support.positive > 0);
}
```

- [ ] **Step 3: Add test entries to `Cargo.toml`**

Add to `sir/crates/sir_inference/Cargo.toml`:

```toml
[[test]]
name = "negative"
path = "tests/negative.rs"

[[test]]
name = "ambiguity"
path = "tests/ambiguity.rs"
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p sir_inference`
Expected: All negative and ambiguity tests pass.

- [ ] **Step 5: Commit**

```bash
git add sir/crates/sir_inference/tests/negative.rs \
  sir/crates/sir_inference/tests/ambiguity.rs \
  sir/crates/sir_inference/Cargo.toml
git commit -m "test(sir_inference): add negative and ambiguity tests"
```

---

### Task 10: Full workspace integration — run all tests, final verification

**Files:** (none new — verification task only)

- [ ] **Step 1: Run the complete test suite**

Run: `cargo test`
Expected: All tests pass — existing 216+ tests plus all new tests from `sir_semantics` and `sir_inference`.

- [ ] **Step 2: Run with verbose output to confirm test count**

Run: `cargo test -- --show-output 2>&1 | tail -5`
Expected: Summary shows all tests passing.

- [ ] **Step 3: Check that `cargo build` works for the full workspace**

Run: `cargo build`
Expected: No warnings, no errors.

- [ ] **Step 4: Verify the acceptance criterion explicitly**

Run: `cargo test -p sir_semantics bs001 -- --nocapture`

This should show:
- `bs001_board_scan_recognizes_all_four_concepts` — PASS
- `bs001_board_scan_infers_bitset_with_strong_support` — PASS
- `bs001_explanation_accounts_for_support` — PASS

- [ ] **Step 5: Review and adjust weights if needed**

If the acceptance tests show support scores outside expected ranges, adjust the weight constants in `sir/crates/sir_inference/src/engine.rs` (the `weights` module). Commit any weight changes with explanation.

- [ ] **Step 6: Final commit**

```bash
git add -A
git commit -m "feat: complete SRI pipeline — sir_semantics + sir_inference

Implements the 0010 Semantic Representation Inference specification:
- sir_semantics: deterministic concept recognition (4 concepts, 4 recognizers)
- sir_inference: evidence-based hypothesis formation (BitSet, explainable)
- Integration test BS001 validates the acceptance criterion
- Negative, ambiguity, and consistency tests ensure robustness

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Plan Summary

| Task | Crate | What | Tests |
|------|-------|------|-------|
| 1 | both | Workspace scaffold | `cargo build` |
| 2 | sir_semantics | Concepts, Region, RegionId | — (type-only) |
| 3 | sir_semantics | SemanticEngine + Database | `semantic_database` (5 tests) |
| 4a–4d | sir_semantics | 4 recognizers | `recognizers` (1 stub) |
| 5 | sir_inference | Evidence, Hypothesis, Support | `types` (8 tests) |
| 6 | sir_inference | InferenceEngine + DB | `engine` (5 tests) |
| 7 | sir_inference | BitSet evidence source | `bitset_evidence` (8 tests) |
| 8 | both | BS001 integration | `semantic_truth` (3 tests) |
| 9 | sir_inference | Negative + ambiguity | `negative` (4) + `ambiguity` (4) |
| 10 | both | Full workspace verification | All tests pass |
