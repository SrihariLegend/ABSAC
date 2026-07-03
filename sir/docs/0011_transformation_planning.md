# 0011 — Transformation Planning

**Version:** 0.1
**Depends on:** SIR v0.1, SAF v0.1 (`sir_analysis`), SRI v0.1 (`sir_semantics`, `sir_inference`)

---

## Architecture

### The Four Layers

```
Understanding
────────────────────────────
  sir_analysis       (Facts)
  sir_semantics      (Truths + Structure)
  sir_inference      (Beliefs + Contexts)

────────────────────────────
  sir_transform      (Transformation Contract)

────────────────────────────
  sir_generation     (Plans)

────────────────────────────
  (future)
  sir_verification   (Proofs)
  sir_rewrite        (Mutations)
  sir_cost           (Selection)
```

The project has naturally settled into four layers:

| Layer | Crates | Question |
|-------|--------|----------|
| Representation | `sir_types`, `sir_nodes` | How is the program encoded? |
| Knowledge | `sir_analysis`, `sir_semantics`, `sir_inference` | What is the program? |
| Planning | `sir_transform`, `sir_generation` | What should we do about it? |
| Execution | (future) | Is it correct and worthwhile? |

### Knowledge Hierarchy

| Layer | Question | Nature |
|-------|----------|--------|
| Facts | What is provably true? | Provable |
| Truths | What computation is being performed? | Deterministic |
| Beliefs | Which representation best explains it? | Heuristic |
| Plans | Which transformations are worth investigating? | Proposed |

> **Core invariant:** A transformation plan is **not** evidence that a transformation is correct. Every plan must be independently verified before it may be rewritten into SIR. Planning produces possibilities; verification produces certainty.

---

## Vision

The Transformation Planning engine transforms representation beliefs into concrete, actionable transformation plans.

It does **not**:
- verify correctness
- estimate performance
- modify SIR
- choose the best implementation

Its sole responsibility:

> Given a representation hypothesis, what mathematically plausible implementation strategies exist?

---

## Motivation

Phase 0010 established the reasoning pipeline — the system can now say *"this computation is a BitSet."* Phase 0011 closes the loop from understanding to planning:

```
Program → Facts → Truths → Beliefs → **Transformation Contexts** → **Candidate Plans**
```

This is the first phase where the system proposes concrete actions. It marks the transition from a reasoning system to a planning system. Every subsequent phase (verification, rewriting, cost modeling) now has a stable interface to consume.

---

## Pipeline

```
Source Program
      │
      ▼
SIR
      │
      ▼  sir_analysis
Compiler Facts
      │
      ▼  sir_semantics
Semantic Truths  +  Structural Descriptions
      │                  │
      └──────┬───────────┘
             ▼  sir_inference
Representation Beliefs  +  Transformation Contexts
             │
             ▼  sir_generation
Candidate Transformation Plans
```

Data flow is strictly one-way, read-only. No crate reads upward or across.

---

## `sir_transform` — The Transformation Contract

### Purpose

`sir_transform` defines the immutable contract between program understanding and program transformation. It contains only data types and invariants. It contains no algorithms, analyses, or rewrite logic.

### Dependency

```
sir_types → sir_nodes → sir_analysis → sir_semantics → sir_transform
                                                              │
                                              ┌───────────────┘
                                              ▼
                                        sir_inference
                                              │
                                              ▼
                                        sir_generation
```

`sir_transform` depends only on `sir_semantics` (for `RegionId`). All downstream crates depend on it.

> **Design note:** `RegionId` is currently defined in `sir_semantics`. It may be promoted to a shared foundational type in a future refactoring, as it is now consumed by semantics, inference, transform, generation, verification, and rewrite.

### Repository

```
sir_transform/
  Cargo.toml
  src/
    lib.rs
    representation.rs   — Representation enum (moved from sir_inference)
    context.rs           — TransformationContext
    structures.rs        — SourceStructure enum
    constraints.rs       — Constraint enum
    assumptions.rs       — Assumption enum
```

### Core types

```rust
// representation.rs — moved from sir_inference::hypothesis
//
// Representations are transformation-domain concepts, not inference concepts.
// Inference predicts them; generation implements them; verification proves them;
// rewrite applies them. All four phases use the same definition.
pub enum Representation {
    BitSet,
}

// structures.rs — describes the physical organization of data
//
// SourceStructure describes data layout, not computational behavior.
// Computational behavior belongs to the semantic layer (SemanticConcept).
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

// constraints.rs — properties already established by analysis or semantics
//
// A Constraint is already established. It cannot become false unless
// the underlying analysis changes. Constraints are NOT assumptions
// waiting to be proven — they are facts that have been determined.
pub enum Constraint {
    /// The structure has a statically known size
    FixedLength(usize),
    /// The structure is not mutated (read-only access)
    ReadOnly,
    /// The structure does not escape the function
    NoEscape,
    /// The structure is not aliased
    NoAlias,
    /// The computation iterates a finite, known number of times
    FiniteIteration,
}

// assumptions.rs — properties that must be proven before transformation
//
// An Assumption is NOT yet established. It must eventually become
// either Proven (by SMT or formal reasoning) or Refuted.
// Assumptions must never be left unresolved.
pub enum Assumption {
    /// The transformed computation produces identical cardinality
    EquivalentCardinality,
    /// The order of iteration is preserved (or does not matter)
    PreservesIterationOrder,
    /// The external memory layout is unchanged
    PreservesLayout,
}

// context.rs — the semantic package connecting belief to action
//
// A TransformationContext must contain all information required to
// generate candidate transformation plans without consulting SIR,
// compiler analyses, or semantic recognizers.
pub struct TransformationContext {
    pub region: RegionId,
    pub representation: Representation,
    pub source_structure: SourceStructure,
    pub constraints: HashSet<Constraint>,
    pub assumptions: HashSet<Assumption>,
    /// Back-reference to the hypothesis that produced this context
    pub hypothesis_id: HypothesisId,
}

impl TransformationContext {
    /// Validate invariants: exactly one representation, no duplicate
    /// constraints, no contradictory assumptions, source structure present.
    pub fn validate(&self) -> Result<(), ValidationError>;
}
```

### Invariants

> **Transformation Contract Invariant:** All transformation-domain concepts are defined in `sir_transform`. No reasoning crate may define transformation types, and no transformation crate may redefine reasoning concepts.

> **Transformation Context Invariant:** A `TransformationContext` must contain all information required to generate candidate transformation plans without consulting SIR, compiler analyses, or semantic recognizers.

> **Stability:** The public API of `sir_transform` is expected to change infrequently. Future optimization families should extend existing enums rather than introducing parallel transformation vocabularies.

---

## `sir_semantics` — Structural Descriptions

### Purpose

Extend `sir_semantics` to produce a second deterministic database alongside `SemanticDatabase`. While `SemanticDatabase` answers *"what computation is being performed?"*, the new `StructuralDatabase` answers *"how is the data organized?"*

### New types

```rust
// sir_semantics/src/structure.rs
pub struct StructuralDatabase {
    descriptions: HashMap<RegionId, StructuralDescription>,
}

/// Describes the physical organization of data in a region.
/// Entirely deterministic — derived from SIR types and analysis facts.
pub struct StructuralDescription {
    pub region: RegionId,
    pub source_structure: SourceStructure,
    pub constraints: HashSet<Constraint>,
}
```

### New recognizers

```
recognizers/
  boolean_array.rs     — Array<bool> with known length → BooleanArray { length }
  bitmask.rs           — Integer used as flag container → BitMask { width }
```

Each recognizer is a pure function following the same pattern as semantic recognizers:

```rust
pub fn recognize_boolean_array(
    func: &Function,
    analysis: &FactDatabase,
) -> Vec<(RegionId, StructuralDescription)>;
```

### Engine change

```rust
impl SemanticEngine {
    /// Derive both semantic truths and structural descriptions.
    pub fn derive(&mut self, func: &Function, analysis: &FactDatabase);

    pub fn semantic_database(&self) -> &SemanticDatabase;
    pub fn structural_database(&self) -> &StructuralDatabase;
}
```

Both recognizer families run in a single pass. Regions are merged across both databases.

### Stability

> The public interfaces and conceptual responsibilities of `sir_semantics` are considered stable after this phase. Future work should extend recognition rules rather than redesign the architecture.

---

## `sir_inference` — Transformation Contexts

### Refactored API

```rust
impl InferenceEngine {
    /// Consume both knowledge databases and produce beliefs + contexts.
    pub fn infer(
        &mut self,
        semantic_db: &SemanticDatabase,
        structural_db: &StructuralDatabase,
    );

    pub fn database(&self) -> &HypothesisDatabase;
    pub fn context_database(&self) -> &TransformationContextDatabase;
}
```

### New output

```rust
pub struct TransformationContextDatabase {
    contexts: HashMap<RegionId, Vec<TransformationContext>>,
}
```

Multiple contexts per region are supported — a single region may produce both `BitSet` and `Bitmap` contexts in the future.

### Context construction

When inference concludes a representation for a region, it combines the hypothesis with the structural description:

1. Copy deterministic constraints from `StructuralDescription`.
2. Synthesize assumptions based on the transformation being proposed.
3. Record the hypothesis ID for traceability.

Constraints are **copied**, not derived — they already exist in the structural database. Only assumptions are synthesized.

### What moves

| Item | From | To |
|------|------|-----|
| `Representation` enum | `sir_inference::hypothesis` | `sir_transform::representation` |
| All `use sir_inference::Representation` | — | `use sir_transform::Representation` |

### New dependency

`sir_inference` adds a dependency on `sir_transform` for `Representation` and `TransformationContext`.

---

## `sir_generation` — Candidate Plans

### Purpose

Transform `TransformationContext`s into concrete candidate transformation plans. Pure, read-only, no SIR access. No ranking, no verification, no rewriting.

### Repository

```
sir_generation/
  Cargo.toml
  src/
    lib.rs
    candidate.rs       — Candidate, CandidateId, ImplementationStrategy
    generator.rs       — CandidateGenerator, CandidateDatabase
    generators/
      bit_iteration.rs
      popcount.rs
      packed_bitfield.rs
      mask_construction.rs
```

### Core types

```rust
// candidate.rs
pub struct Candidate {
    pub id: CandidateId,
    pub region: RegionId,
    /// Reference to the context that produced this candidate.
    /// Multiple candidates may reference the same context.
    pub context_id: ContextId,
    pub strategy: ImplementationStrategy,
    pub explanation: CandidateExplanation,
    pub effects: CandidateEffects,
}

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

pub struct CandidateExplanation {
    pub strategy: ImplementationStrategy,
    pub representation: Representation,
    pub source_concepts: Vec<SemanticConcept>,
    pub prerequisites: Vec<Constraint>,
    pub rationale: &'static str,
}

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
```

### Generator API

```rust
// generator.rs
pub struct CandidateGenerator {
    db: CandidateDatabase,
}

impl CandidateGenerator {
    pub fn new() -> Self;

    /// Generate candidates for every transformation context.
    /// Pure — no mutation, no SIR access, no ranking.
    pub fn generate(&mut self, contexts: &TransformationContextDatabase);

    pub fn database(&self) -> &CandidateDatabase;
}

pub struct CandidateDatabase {
    candidates: HashMap<RegionId, Vec<Candidate>>,
}

impl CandidateDatabase {
    pub fn candidates(&self, region: RegionId) -> &[Candidate];
    pub fn validate(&self) -> Result<(), ValidationError>;
}
```

### Individual generator signature

```rust
// generators/bit_iteration.rs
///
/// Never mutates state — returns a Candidate or None.
/// The TransformationContext is the sole source of information.
pub fn plan(context: &TransformationContext) -> Option<Candidate>;
```

### Candidate Invariant

> A candidate plan must be fully derivable from a `TransformationContext` alone. No hidden state. No global analysis. No SIR inspection. No semantic queries. If a generator needs more information, the context is incomplete.

### Scope

| What | Count |
|------|-------|
| Representation | `BitSet` (one) |
| Implementation strategies | 4 (`BitIteration`, `Popcount`, `PackedBitfield`, `MaskConstruction`) |
| Generators | 4 (one per strategy) |

---

## Acceptance Criterion

Given the canonical fixed-size boolean membership scan (BS001):

```rust
bool board[64];
for i in 0..64 {
    if board[i] {
        count++;
    }
}
```

The following pipeline must execute successfully:

```
SIR
  │
  ▼  sir_analysis
Facts: TripCount=64, ReadOnly, Array<bool>, NoEscape, Pure
  │
  ▼  sir_semantics
Semantic: BooleanCollection, FiniteCollection, MembershipTraversal, CardinalityReduction
Structural: BooleanArray(64), constraints: [FixedLength(64), ReadOnly, NoEscape]
  │
  ▼  sir_inference
Belief: BitSet (support +100)
Context: TransformationContext {
    region, BitSet, BooleanArray(64),
    constraints: [FixedLength(64), ReadOnly, NoEscape, FiniteIteration],
    assumptions: [EquivalentCardinality, PreservesLayout]
}
  │
  ▼  sir_generation
Candidates:
  1. BitIteration     — visits only set bits via trailing-zero loop
  2. Popcount         — single popcount(bb) instruction
  3. PackedBitfield   — replace bool[64] with u64
  4. MaskConstruction — AND/OR/XOR over bitmask predicates
```

Each candidate must carry:
- the target representation,
- the implementation strategy,
- the structural context it depends on (by reference),
- the prerequisites (constraints) that must hold,
- the effects the transformation would have,
- and a human-readable explanation.

No verification, rewriting, or cost estimation is required.

---

## Testing Strategy

### Tier 1: Pipeline Integration (BS001)

Full pipeline test: build SIR → analyze → semantic + structural → inference → generate. Assert 4 distinct candidates with complete explanations.

### Tier 2: Individual Generator Tests

Feed a `TransformationContext` to each generator. Assert it produces `Some(Candidate)` with the expected strategy and effects.

### Tier 3: Explanation Tests

Every candidate must have a non-empty rationale, correct strategy tag, and source concept references.

### Tier 4: Determinism Tests

Running generation twice on the same contexts must produce identical candidates.

### Tier 5: Uniqueness Tests

No duplicate candidate IDs within a region.

### Tier 6: Validation Tests

- `TransformationContext::validate()` rejects contexts with contradictory constraints.
- `CandidateDatabase::validate()` rejects duplicate IDs or invalid context references.

### Guidelines

- Assert on counts (4 candidates), not ordering
- No ranking assertions — order is explicitly not guaranteed

---

## Explicit Non-Goals

This phase **must not**:

- Rewrite SIR or modify nodes
- Generate Rust, LLVM IR, or any target code
- Invoke an SMT solver or formal verifier
- Estimate performance or rank candidates
- Eliminate candidates based on cost
- Implement any representation beyond `BitSet`
- Add more than 4 implementation strategies

It is purely a planning engine. Generation, not selection. Possibility, not certainty.

---

## Deliverables

At the end of this phase, the system should be able to answer:

> "Given my belief that this region represents a bitset, here are the known classes of mathematically plausible implementation strategies, the structural context they depend on, the prerequisites that must hold, and a human-readable explanation of each."

---

## Future Work

- `sir_verification` (Phase 0012): take a transformation plan and prove it correct via SMT
- `sir_rewrite` (Phase 0013): apply a verified plan to mutate SIR
- `sir_cost` (Phase 0014): rank verified plans by estimated performance
- Additional `SourceStructure` variants (BitSlice, SparseBooleanArray, etc.)
- Additional `ImplementationStrategy` variants beyond BitSet (SIMD, GPU, cache-aware)
- Additional constraint/assumption categories
- `RegionId` promotion to a shared foundational type

---

## Document History

| Version | Date | Changes |
|---------|------|---------|
| 0.1 | 2026-07-03 | Initial specification. Four-layer architecture. Transformation contract. BitSet only, 4 strategies. |
