# 0010 — Semantic Representation Inference

**Version:** 0.1
**Depends on:** SIR v0.1, SAF v0.1 (sir_analysis)

---

## Knowledge Hierarchy

```
Program
    │
    ▼
Facts          (sir_analysis)
    │
    ▼
Truths         (sir_semantics)
    │
    ▼
Beliefs        (sir_inference)
    │
    ▼
Optimization   (future)
```

| Layer     | Question                                                | Nature         |
| --------- | ------------------------------------------------------- | -------------- |
| Analysis  | **What is true about this program?**                    | Provable facts |
| Semantics | **What operation is this program performing?**          | Deterministic  |
| Inference | **What representation best explains these operations?** | Heuristic      |

> **Core invariant:** Semantic truths are deterministic and reproducible. Representation beliefs are heuristic, explainable, and revisable. The system must never confuse one for the other.

---

## Vision

The Semantic Representation Inference (SRI) engine discovers the **mathematical representation** of a computation.

It does **not** rewrite programs. It does **not** prove correctness. It does **not** optimize. Its sole responsibility:

> Infer what the program is actually representing.

---

## Motivation

Compilers optimize syntax. Humans optimize representations. Given:

```rust
let mut count = 0;
for i in 0..64 {
    if board[i] {
        count += 1;
    }
}
```

LLVM sees: loop, load, branch, add. A human sees: **cardinality of a finite set.**

SRI bridges this gap by layering semantic understanding on top of compiler analysis:

```
Analysis facts:  TripCount=64, Array<bool>, ReadOnly, NoEscape, Pure
       ↓
Semantic truths: FiniteCollection, BooleanCollection, MembershipTraversal, CardinalityReduction
       ↓
Evidence:        BooleanCollection (strong), FiniteCollection (strong),
                 MembershipTraversal (strong), CardinalityReduction (strong)
       ↓
Belief:          BitSet (support net +93, confidence: Very Strong)
```

> Concrete weights in diagrams and examples are illustrative. The architecture
> defines relative strength categories (Strong/Moderate/Weak); exact integers
> are implementation tuning parameters.

---

## Architecture

### Crate dependency graph

```
sir_inference  ──► sir_semantics  ──► sir_analysis  ──► sir_nodes  ──► sir_types
```

Dependencies are strictly one-way. Each layer knows nothing about the layers above it:

- `sir_analysis` has no knowledge of `sir_semantics` or `sir_inference`
- `sir_semantics` has no knowledge of `sir_inference`
- No layer reads upward or across

### Knowledge Layer Invariant

> Every reasoning layer consumes only the knowledge produced by the immediately preceding layer. Higher-level layers must never inspect lower-level representations directly.

### Database immutability

All three databases are immutable after construction. The engines produce them; consumers query them. This preserves caching and enables future concurrency.

### Repository structure

```
sir/
  crates/
    sir_analysis/          (existing — compiler facts)
    sir_semantics/          (new — semantic truths)
      Cargo.toml
      src/
        lib.rs
        concepts.rs         — SemanticConcept enum
        region.rs           — Region, RegionId
        semantics.rs         — SemanticEngine, SemanticDatabase
        recognizers/
          boolean_collection.rs
          finite_collection.rs
          membership_traversal.rs
          cardinality_reduction.rs
    sir_inference/          (new — representation beliefs)
      Cargo.toml
      src/
        lib.rs
        evidence.rs         — Evidence, Polarity, EvidenceRegistry
        hypothesis.rs       — Hypothesis, Support, Representation enum
        engine.rs           — InferenceEngine, HypothesisDatabase
        sources/
          bitset_evidence.rs
```

Cargo workspace gains two members: `crates/sir_semantics`, `crates/sir_inference`.

---

## Layer 1: `sir_semantics` — Semantic Truths

### Purpose

Transform compiler facts into semantic truths. Entirely deterministic. No heuristics, no confidence scores, no weights.

### Must not

- Assign support or confidence
- Rank representations
- Rewrite SIR
- Inspect SIR nodes directly (consumes `AnalysisDatabase`, not `Function`)

### Core types

```rust
/// A semantic concept describing what a computation is doing.
pub enum SemanticConcept {
    /// The data: collection of boolean values
    BooleanCollection,
    /// The data: collection with a statically known bound
    FiniteCollection,
    /// The operation: iterating over elements and testing membership
    MembershipTraversal,
    /// The operation: counting how many elements satisfy a condition
    CardinalityReduction,
}

/// A contiguous subgraph representing a semantic unit.
pub struct Region {
    pub id: RegionId,
    pub nodes: BTreeSet<NodeId>,
    concepts: HashSet<SemanticConcept>,
    explanations: HashMap<SemanticConcept, RecognitionExplanation>,
}

/// Why a concept was recognized — deterministic, not heuristic.
pub struct RecognitionExplanation {
    pub concept: SemanticConcept,
    pub triggering_facts: Vec<&'static str>,
}

/// The semantic knowledge database.
pub struct SemanticDatabase {
    regions: HashMap<RegionId, Region>,
}
```

### Public API

```rust
impl SemanticEngine {
    pub fn new() -> Self;
    /// Derive semantic truths from compiler facts.
    /// Consumes the AnalysisDatabase, not the Function — this enforces layering.
    pub fn derive(&mut self, analysis: &AnalysisDatabase);
    pub fn database(&self) -> &SemanticDatabase;
}

impl SemanticDatabase {
    pub fn regions(&self) -> impl Iterator<Item = (RegionId, &Region)>;
    pub fn region(&self, id: RegionId) -> Option<&Region>;
    pub fn explain(&self, region: RegionId, concept: SemanticConcept) -> Option<&RecognitionExplanation>;
}

impl Region {
    pub fn contains(&self, concept: SemanticConcept) -> bool;
    pub fn concepts(&self) -> &HashSet<SemanticConcept>;
    pub fn nodes(&self) -> &BTreeSet<NodeId>;
}
```

### Recognizers

Each concept has one recognizer module — a pure function:

```rust
/// Returns (concept, explanation) if the analysis facts support recognition.
pub fn recognize_boolean_collection(analysis: &AnalysisDatabase) -> Vec<(SemanticConcept, RecognitionExplanation)>;
```

The `SemanticEngine::derive()` method calls each recognizer, groups concepts into regions, and populates the `SemanticDatabase`.

Region extraction is intentionally minimal in v0.1. A region is simply the set of nodes involved in a recognized computation (e.g., a loop body and its enclosing array access). Region identification will become more sophisticated in future phases; for v0.1 the goal is to attach concepts to the correct set of nodes, not to build a general region detection subsystem.

### Invariant

> Every concept attached to a region must be derivable solely from analysis facts. The semantic layer invents nothing: no guessing, no weights, no probabilities.

---

## Layer 2: `sir_inference` — Representation Beliefs

### Purpose

Accumulate evidence from semantic truths and form representation hypotheses. This is where heuristics and weights live.

### Must not

- Inspect SIR nodes directly
- Invent semantic concepts
- Rewrite SIR

### Core types

```rust
/// A representation is a concrete mathematical structure,
/// not a machine instruction.
pub enum Representation {
    BitSet,
}

/// Evidence is an instance — an observation about a specific region,
/// not a rule template.
pub struct Evidence {
    pub region: RegionId,
    pub representation: Representation,
    pub polarity: Polarity,
    pub weight: u16,
    pub source: SemanticConcept,   // which concept triggered this evidence
    pub explanation: &'static str,
}

pub enum Polarity {
    Supports,
    Against,
}

/// Integer support score — no floating point in the engine.
pub struct Support {
    pub positive: u16,
    pub negative: u16,
}

impl Support {
    pub fn score(&self) -> i32 { self.positive as i32 - self.negative as i32 }
    pub fn ratio(&self) -> f32 { ... }  // display only
}

/// A hypothesis is a representation with accumulated support.
pub struct Hypothesis {
    pub representation: Representation,
    pub support: Support,
    pub evidence: Vec<EvidenceId>,
}

pub struct HypothesisDatabase {
    hypotheses: HashMap<RegionId, Vec<Hypothesis>>,
}
```

### Public API

```rust
impl InferenceEngine {
    pub fn new() -> Self;
    /// Infer representation hypotheses from semantic truths.
    /// Consumes only the SemanticDatabase — never SIR or compiler facts.
    pub fn infer(&mut self, semantic_db: &SemanticDatabase);
    pub fn database(&self) -> &HypothesisDatabase;
    /// Explain why a hypothesis exists — first-class API, not a debug helper.
    pub fn explain(&self, region: RegionId, rep: Representation) -> Explanation;
}

impl HypothesisDatabase {
    pub fn hypotheses(&self, region: RegionId) -> &[Hypothesis];
    pub fn best(&self, region: RegionId) -> Option<&Hypothesis>;
    pub fn regions_supporting(&self, rep: Representation) -> Vec<RegionId>;
}
```

### Evidence sources

Each source is a pure function that inspects a region's concepts and returns evidence:

```rust
/// Never mutates a registry — returns evidence for the engine to register.
pub fn contribute(region: &Region) -> Vec<Evidence>;
```

Example contributions for BitSet:

| Concept present         | Polarity  | Weight          |
| ----------------------- | --------- | --------------- |
| `BooleanCollection`     | Supports  | Strong positive |
| `FiniteCollection`      | Supports  | Strong positive |
| `MembershipTraversal`   | Supports  | Strong positive |
| `CardinalityReduction`  | Supports  | Strong positive |
| Mutable writes detected | Against   | Strong negative |

Weights are tuning parameters. The architecture document specifies relative strength (Strong/Moderate/Weak), not concrete integers. Concrete weights live in implementation constants.

### Explain output

```
Hypothesis: BitSet
Support: +93 / -12 (net +81)
Confidence: Very Strong
Evidence:
  +30  BooleanCollection      "Boolean arrays often represent bitsets"
  +20  FiniteCollection       "Known iteration bound enables bitwise encoding"
  +25  MembershipTraversal    "Testing membership is a bitset operation"
  +18  CardinalityReduction   "Counting members matches popcount pattern"
  -12  MutableWrites          "Mutation argues against immutable bitset view"
```

Qualitative labels are derived from net score, not stored:

| Net score | Label       |
| --------- | ----------- |
| 0–20      | Weak        |
| 21–50     | Moderate    |
| 51–80     | Strong      |
| 81+       | Very Strong |

### Invariants

- **Evidence completeness:** Every hypothesis must satisfy `Support.score() = sum(evidence contributions)`. No hidden weights, no invisible heuristics.
- **Order independence:** Evidence accumulation must be commutative. Reordering semantic concepts or evidence sources must produce identical support scores.
- **Monotonicity:** Adding a supporting concept must never decrease support. Adding a counter-evidence concept must never increase support.

---

## Pipeline

```rust
// 1. Build SIR (existing)
let func = builder.build();

// 2. Run analysis (existing)
let mut analysis = AnalysisManager::new();
analysis.run_all(&func);

// 3. Derive semantic truths (new)
let mut semantics = SemanticEngine::new();
semantics.derive(analysis.database());

// 4. Infer representations (new)
let mut inference = InferenceEngine::new();
inference.infer(semantics.database());

// 5. Query
for (region_id, region) in semantics.database().regions() {
    if let Some(h) = inference.database().best(region_id) {
        println!("{}: {:?} support={}", region_id, h.representation, h.support.score());
        println!("{}", inference.explain(region_id, h.representation));
    }
}
```

Data flow is strictly one-way, read-only:

```
Function ──► AnalysisManager ──► AnalysisDatabase (immutable)
                                       │
                                       ▼
                              SemanticEngine ──► SemanticDatabase (immutable)
                                                       │
                                                       ▼
                                              InferenceEngine ──► HypothesisDatabase (immutable)
```

---

## Acceptance Criterion

Given the canonical fixed-size boolean membership scan:

```rust
bool board[64];
for i in 0..64 {
    if board[i] {
        count++;
    }
}
```

The engine shall produce a `BitSet` hypothesis with **strong net support** (score > 50) and an **explanation whose evidence exactly accounts for the reported support.**

The pipeline must correctly identify all four semantic concepts (`BooleanCollection`, `FiniteCollection`, `MembershipTraversal`, `CardinalityReduction`) and aggregate them into a BitSet hypothesis with an inspectable evidence trace.

This is the only representation targeted in v0.1. Additional representations (Bitmap, DenseSet, BitField, etc.) are extensions — the architecture is designed to accommodate them without structural changes.

---

## Testing Strategy

### Tier 1: Semantic Truth Tests (highest priority)

End-to-end builder-driven tests validating that the engine recognizes semantic concepts from compiler facts. These form the canonical benchmark suite for the project.

```
BS001  Board Scan           — Full recognition + BitSet inference
BS002  Flag Accumulator     — flags |= x → mask construction
BS003  Popcount Loop        — standalone population count
BS004  Boolean Mask         — repeated AND/OR/XOR
BS005  Fixed Membership     — index-based membership query
```

### Tier 2: Evidence Unit Tests

Feed semantic concepts into evidence sources, assert correct evidence generation. No SIR, no builder. Fast and isolated.

### Tier 3: Negative Tests

Validate that the engine does NOT produce false positives. `Vec<bool>`, `HashSet<bool>`, dynamic-sized arrays, and mutable-aliased collections must not infer `BitSet` with strong support. Negative tests should outnumber positive tests (~60% of the inference corpus).

### Tier 4: Ambiguity Tests

Validate that the engine expresses appropriate uncertainty. A bare `Array<bool>` with no operations should produce low support and no clear winner.

### Tier 5: Explainability Tests

Every hypothesis must account for its support score. The explanation output must reference every contributing concept and produce a complete trace from concept → evidence → support.

### Tier 6: Consistency & Monotonicity Tests

- **Consistency:** Evidence aggregation is order-independent — reordering concepts produces identical hypotheses.
- **Monotonicity:** Adding supporting concepts never decreases support; adding counter-evidence never increases it.

### Tier 7: Ablation Tests

Removing any positive evidence source must decrease support. Removing negative evidence must increase net score. These measure the contribution of each evidence source.

### Guidelines

- Assert on score **thresholds**, not exact values (weights evolve)
- Maximum 3 snapshot/golden tests (for papers and documentation)
- Region extraction is tested only through integration, not as a standalone subsystem

---

## Explicit Non-Goals

This phase **must not**:

- Rewrite SIR or modify nodes
- Generate bitwise code
- Invoke an SMT solver
- Estimate performance or rank machine instructions
- Search for equivalent programs
- Implement any representation beyond `BitSet`

It is purely an inference engine. Representation recognition, not representation synthesis.

---

## Future Work

- `sir_framework` extraction (Phase 0012): unify caching, statistics, and engine infrastructure once three engines exist
- Additional representations: `Bitmap`, `DenseSet`, `SparseSet`, `BitField`, `BooleanVector`
- Additional semantic concepts: `Selection`, `Partition`, `Permutation`, `Projection`, `MaskConstruction`
- ML-based evidence sources: learned models as additional evidence contributors
- `KnowledgeBase` trait: common query interface across all three databases

---

## Document History

| Version | Date       | Changes                                              |
| ------- | ---------- | ---------------------------------------------------- |
| 0.1     | 2026-07-03 | Initial specification. Three-layer knowledge architecture. BitSet only. |
