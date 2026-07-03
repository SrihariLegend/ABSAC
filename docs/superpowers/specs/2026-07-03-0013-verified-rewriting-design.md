# Phase 0013 — Verified Rewriting Design

**Status:** Research milestone (specification)
**Date:** 2026-07-03
**Depends on:** Phase 0012 (Equivalence Verification)

## Purpose

Phase 0013 is the first phase that mutates programs.

Previous phases answered:

- What does this program do?
- What representation does it implement?
- What transformations are possible?
- Are those transformations mathematically correct?

This phase answers only one question:

> **How do we mechanically construct an equivalent SIR graph from a proven transformation?**

No discovery. No inference. No optimization. No proving. Only deterministic graph construction.

## Philosophy

The rewrite engine never invents transformations. It never performs analysis. It never performs theorem proving. It never evaluates cost. It only executes transformations that have already been proven equivalent.

Every rewrite must satisfy:

```text
Candidate
    │
    ▼
TransformationDefinition
    │
    ▼
Proof
    │
    ▼
RewriteRecipe
    │
    ▼
Equivalent SIR
```

If any prerequisite is missing, rewriting does not occur.

## Pipeline

Current compiler:

```text
Program
    ↓
SIR
    ↓
Analysis        (discovers facts)
    ↓
Semantics       (discovers truths)
    ↓
Inference       (forms beliefs)
    ↓
Generation      (proposes plans)
    ↓
Verification    (proves plans)
```

After Phase 0013:

```text
Program
    ↓
SIR
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
Verified Rewrite (executes proven plans)
    ↓
Structural Verification
    ↓
Rewritten SIR
```

## Crate

```
sir_rewrite/
```

### Dependencies

```
sir_types
sir_nodes
sir_transform
sir_generation
sir_verification
sir_verify
```

Plus `sir_builder` only if `SubgraphBuilder` genuinely reuses its construction internals (not its public `Builder` API). If node construction logic is extracted into a shared `NodeFactory`, `sir_rewrite` depends on that instead. The important dependency is node construction capability, not the crate name.

### Notably absent

```
sir_analysis
sir_semantics
sir_inference
```

The rewrite engine must never rediscover anything.

### Invariant

`sir_rewrite` performs no discovery. Every decision required for rewriting must already have been established by an upstream phase and be available through the artifacts supplied to the rewrite engine (the verified candidate, proof, transformation context, or structural region). If a rewrite requires rediscovering graph structure or recomputing semantic knowledge, that information belongs in an upstream phase.

`sir_rewrite` is a deterministic execution engine. It consumes previously established knowledge but never derives new knowledge. Every rewrite is fully determined by its inputs before graph mutation begins.

### Prerequisite: StructuralRegion gains RegionRoles

A prerequisite change lands in `sir_semantics`: `StructuralRegion` gains a `RegionRoles` enum. This is not a rewrite-specific addition — semantic recognition should preserve the semantic roles it discovered. Rewriting is merely one downstream consumer.

```rust
pub enum RegionRoles {
    BooleanCollectionReduction {
        collection: NodeId,
        accumulator: Option<NodeId>,
        result: NodeId,
    },
    // Future patterns define their own roles.
}
```

The recognizer assigns roles during pattern matching. No downstream phase rediscovers them.

`StructuralRegion` and `RegionRoles` are defined in `sir_transform` (the shared contract crate, alongside `TransformationContext`). `sir_semantics` populates them during recognition; `sir_rewrite` consumes them during graph construction. Neither crate depends on the other.

## Architecture

```
RewriteEngine
    │
    ▼
RewriteRegion       (transient, assembled from StructuralRegion)
    │
    ▼
RewriteRecipe       (builds detached replacement subgraph)
    │
    ▼
ReplacementPatch    (closed subgraph, owned by recipe output)
    │
    ▼
RewritePlan         (immutable: region + patch + proof)
    │
    ▼
RewriteBuilder      (graph surgery: clone, import, reconnect, repair)
    │
    ▼
sir_verify          (structural validation only)
```

### Responsibility boundaries

```
Recipe
    constructs SIR in a detached arena

RewriteBuilder
    performs graph surgery

RewriteEngine
    orchestrates, never touches nodes or SSA

sir_verify
    validates structural correctness
```

## Core types

### NodeFactory

Shared node construction logic used by both `sir_builder::Builder` and `sir_rewrite::SubgraphBuilder`. Owns node construction, type checking, and local arena insertion. Global `NodeId` allocation remains the exclusive responsibility of `RewriteBuilder`.

### LocalNodeId

A `Copy` newtype over `u64`, displayed as `local#0`, `local#1`, etc. Monotonically increasing within a single `DetachedArena`. Never escapes the arena — `RewriteBuilder` maps each `LocalNodeId` to a global `NodeId` during import.

### DetachedArena

A `BTreeMap<LocalNodeId, Node>` with deterministic iteration. Identical structure to `NodeArena` but keyed by `LocalNodeId`. Holds the replacement subgraph before it is spliced into the cloned function.

### SubgraphBuilder

Wraps a `DetachedArena` plus a `NodeFactory`. Exposes the same construction API as `sir_builder::Builder` — every method corresponds 1:1 with a `NodeKind` variant. No hidden graph synthesis, no convenience macros, no recipe-specific methods. Consumed by value in `RewriteRecipe::build_patch`.

The builder owns its own lifecycle. Recipes never touch `DetachedArena` or assemble `ReplacementPatch` manually:

```rust
pub struct SubgraphBuilder {
    arena: DetachedArena,
    factory: NodeFactory,
    next_local_id: u64,
}

impl SubgraphBuilder {
    /// Seal the builder and produce a ReplacementPatch.
    /// Consumes self — the builder cannot be used after finishing.
    pub fn finish(self, replacements: Vec<ReplacementValue>) -> ReplacementPatch;
}
```

### RewriteRegion

A transient execution object assembled by the engine at rewrite time. Not persisted.

```rust
pub struct RewriteRegion {
    pub structural: StructuralRegion,
    pub external_users: BTreeSet<NodeId>,
}

impl RewriteRegion {
    pub fn collection(&self) -> Result<NodeId, RewriteError> {
        match &self.structural.roles {
            RegionRoles::BooleanCollectionReduction { collection, .. } => Ok(*collection),
        }
    }

    pub fn result(&self) -> Result<NodeId, RewriteError> {
        match &self.structural.roles {
            RegionRoles::BooleanCollectionReduction { result, .. } => Ok(*result),
        }
    }
}
```

Named accessors delegate to `StructuralRegion::roles`. No searching, no type inspection, no graph walking — the recognizer already recorded the roles.

### ReplacementPatch

```rust
pub struct ReplacementValue {
    pub old: NodeId,       // original exported SSA value
    pub new: LocalNodeId,  // replacement in the detached arena
}

pub struct ReplacementPatch {
    pub arena: DetachedArena,
    /// Roots of the detached subgraph to be imported.
    pub roots: Vec<LocalNodeId>,
    pub replacements: Vec<ReplacementValue>,
}
```

**Invariant:** A `ReplacementPatch` is a closed subgraph. It may reference only nodes created within its own `DetachedArena` and values explicitly provided through the `RewriteRegion` boundary. It must never reference arbitrary nodes in the original `Function`.

`replacements` is intentionally a `Vec` — a single region can produce multiple exported values.

### RewriteRecipe

```rust
pub trait RewriteRecipe {
    fn definition(&self) -> DefinitionId;

    fn build_patch(
        &self,
        region: &RewriteRegion,
        builder: SubgraphBuilder,
    ) -> Result<ReplacementPatch, RewriteError>;
}
```

Recipes consume the builder by value, call `builder.finish(...)` to seal the patch. No cloning, no SSA reconnection, no diff computation, no IR validation.

### RewritePlan

An immutable value aggregating everything `RewriteBuilder` needs. `RewriteBuilder` knows nothing about candidates, proofs, or recipes — it only executes plans.

```rust
pub struct RewritePlan {
    pub region: RewriteRegion,
    pub patch: ReplacementPatch,
    pub proof: Proof,
}
```

The engine constructs the plan from recipe output; the builder consumes the plan. This keeps `RewriteBuilder` completely generic — it performs graph surgery on any valid plan, regardless of which transformation produced it.

### RewriteBuilder

The graph surgery engine. Owns the persistent transformation.

```rust
pub struct RewriteBuilder;

impl RewriteBuilder {
    /// Clone the original function, import the patch, reconnect SSA, repair boundaries.
    /// Returns a structurally valid rewritten function or an error.
    pub fn apply(
        function: &Function,
        plan: RewritePlan,
    ) -> Result<Function, RewriteError>;
}
```

Responsibilities:

- Clone the original function graph
- Allocate global `NodeId`s for every `LocalNodeId` in the patch
- Rewrite `LocalNodeId` references to `NodeId` references within the detached arena
- Import the detached arena into the cloned function
- Reconnect SSA edges (external users → replacement values)
- Replace exported values (`old → new`)
- Omit obsolete region internal nodes from the rewritten graph
- Preserve spans and metadata on unmodified nodes
- Preserve dominance: `RewriteBuilder` is responsible for preserving SSA dominance. For BS001 this follows directly from insertion at the region entry point. Future recipes may require additional dominance repair.
- Preserve SSA validity (each `LocalNodeId` maps to exactly one global `NodeId`)
- Return the rewritten function

**Invariant:** Boundary repair is the only phase permitted to modify SSA connectivity. `RewriteRecipe`s construct only replacement nodes. `RewriteBuilder` is the sole owner of SSA rewiring, dominance preservation, and use-def repair.

### RewriteEngine

Orchestration only. Never builds nodes, never manipulates SSA.

Pipeline:

```text
1. Look up recipe from TransformationRegistry via Candidate.definition_id
2. Verify IDs: Candidate.definition_id == Proof.definition_id == Recipe.definition()
3. Fetch StructuralRegion for Candidate.region
4. Compute external_users from the function graph
5. Assemble RewriteRegion
6. Invoke recipe → ReplacementPatch
7. Assemble RewritePlan { region, patch, proof }
8. RewriteBuilder::apply(function, plan) → rewritten Function
9. Run sir_verify (structural verifier, not the Phase 0012 semantic verifier) on the rewritten function
10. If verification fails: discard rewritten graph, return RewriteError
11. Otherwise: compute provenance, compute GraphDiff, return RewriteResult
```

The transactional nature is explicit: a structurally invalid rewrite is discarded and the original function is retained.

### RewriteResult

```rust
pub struct RewriteResult {
    pub rewritten: Function,
    pub provenance: Vec<NodeProvenance>,
    pub diff: GraphDiff,
    pub proof: Proof,
}
```

The caller already holds the original — it is not duplicated in the result.

### NodeProvenance

```rust
pub struct NodeProvenance {
    pub new_node: NodeId,
    pub originates_from: Vec<NodeId>,
    pub recipe: DefinitionId,
}
```

Records why each synthetic node exists: which original nodes it derives from and which transformation produced it. Supports debugging, source mapping, future explanation, and paper visualizations.

### GraphDiff

```rust
pub struct GraphDiff {
    pub removed_nodes: BTreeSet<NodeId>,
    pub added_nodes: BTreeSet<NodeId>,
    pub modified_edges: Vec<EdgeChange>,
}
```

Every rewrite produces a complete diff. Node sets use `BTreeSet` for deterministic iteration and uniqueness.

## SSA Boundary Repair

The hardest mechanical problem in the rewrite. Replacing a loop with a straight-line computation changes which SSA values are exported. Boundary repair is owned entirely by `RewriteBuilder` — no recipe touches it.

Responsibilities:

- Replace exported values: each `ReplacementValue { old, new }` says "where the original graph produced `old`, the rewritten graph now produces `new`"
- Reconnect external users: every node outside the region that referenced `old` now references `new`
- Omit obsolete region internal nodes from the rewritten graph (internal loop body, accumulator, branch)
- Rewrite `LocalNodeId → NodeId` references inside the detached arena as an explicit, separate pass before import. Every edge within the patch must be remapped.
- Preserve dominance: the replacement nodes must dominate their users. Since the replacement is a straight-line sequence inserted at the region's entry point, and all external users are downstream of the region, dominance holds by construction
- Preserve SSA validity: each `LocalNodeId` maps to exactly one global `NodeId`; no duplicate definitions

**Invariant:** Boundary repair is the only phase permitted to modify SSA connectivity. `RewriteRecipe`s construct only replacement nodes. `RewriteBuilder` is the sole owner of SSA rewiring, dominance preservation, and use-def repair.

`ReplacementValue` is intentionally a `Vec` — a single region can produce multiple exported values. BS001 has one; future rewrites may have more.

## Safety checks

Before any graph mutation:

```text
Candidate.definition_id
    │
    ▼
TransformationRegistry
    │
    ▼
RewriteRecipe
    │
    ▼
Proof.definition_id == Recipe.definition()
```

Mismatch → `RewriteRejected::DefinitionMismatch`. The rewrite is aborted before any graph is cloned.

`RewriteError` includes a reserved `InternalInvariantViolation` variant for conditions that indicate a compiler bug rather than a user-facing failure. Any code path that reaches this variant represents an implementation error in either a recipe or `RewriteBuilder`.

## Structural verification

`sir_verify` (the structural SIR verifier, not the semantic verifier from Phase 0012) runs automatically on every rewritten function immediately after construction. A failure:

- Discards the rewritten graph
- Retains the original
- Returns `RewriteError::StructuralVerificationFailed`

A failed rewrite is treated as an implementation bug in either the recipe or `RewriteBuilder`.

## Provenance

Node provenance records why every synthetic node exists. A rewritten node may originate from multiple original nodes. The rewrite engine records provenance for every synthetic node. This supports debugging, source mapping, future explanation, and paper visualizations.

## Determinism

**Invariant:** A rewrite is a pure function. Given the same `Function`, `Candidate`, `Proof`, `StructuralRegion`, and `RewriteRecipe`, the engine must always produce byte-for-byte identical rewritten SIR.

This guarantees deterministic builds, reproducible compiler output, stable regression tests, and no hidden state.

## Acceptance benchmark (BS001)

Input:

```rust
for i in 0..64 {
    if board[i] {
        count += 1;
    }
}
```

Output: the replacement region contains `Pack` and `Popcount` nodes. The surrounding function preserves all non-region nodes unchanged.

The rewritten SIR contains no loop, no accumulator, no branch, and passes `sir_verify`.

## Test strategy

| Tier | Scope | What it verifies |
|------|-------|-----------------|
| 1 | Recipe construction | `PopcountRecipe` produces correct `ReplacementPatch` |
| 2 | RewriteRegion construction | Boundaries isolated correctly, roles accessible |
| 3 | SSA Boundary Repair | External users reconnect, no dangling values |
| 4 | Structural verification | Every rewritten graph passes `sir_verify` |
| 5 | BS001 end-to-end | Candidate → Proof → Rewrite → `sir_verify` → Success |
| 6 | Definition mismatch | ID disagreement → rewrite rejected before mutation |
| 7 | Negative tests | Malformed patches, missing outputs, broken SSA → rejected |
| 8 | Regression | Every proven rewrite remains valid forever |
| 9 | Provenance | Every added node has provenance. Every removed node appears in `GraphDiff`. |

## Explicit non-goals

This phase does **not** implement:

- Cost modeling
- Rewrite selection
- Speculative optimization
- Partial rewriting
- Iterative optimization
- SMT integration
- Ranking candidates
- Performance estimation
- Graph canonicalization — the rewrite engine emits exactly what the recipe specifies. No peepholes, no cleanup, no simplification. That belongs in later optimization passes.

## Success criterion

For the canonical BS001 benchmark:

1. A `Candidate` identifies the Popcount transformation.
2. A backend-independent `Proof` establishes semantic equivalence.
3. `RewriteEngine` verifies ID alignment and constructs a `RewriteRegion`.
4. `PopcountRecipe` emits a `ReplacementPatch`.
5. `RewriteBuilder` clones, imports, reconnects, and repairs.
6. `sir_verify` confirms structural correctness.
7. A `RewriteResult` records the rewritten graph, provenance, graph diff, and proof.
8. The rewritten SIR contains no loop, branch, or accumulator.
9. If structural verification fails, the rewritten graph is discarded, the original is preserved, and a `RewriteError` is returned.
10. The rewritten SIR is byte-for-byte deterministic given the same inputs.

## Design decisions log

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Rewrite boundary source | `StructuralRegion` in `sir_semantics` | Structural recognition, not heuristic inference |
| Builder architecture | Detached arena with `LocalNodeId` | Recipes construct genuine SIR, not an intermediate DSL |
| Builder API surface | General-purpose, mirrors `sir_builder::Builder` | One canonical way to construct SIR |
| Recipe → Builder handoff | `RewritePlan` value object | Builder is generic; knows nothing about candidates or recipes |
| Recipe-specific methods | None — every method is 1:1 with a `NodeKind` | Prevents fragmentation and DSL proliferation |
| `RegionRoles` | Named enum per recognized pattern | Recognizer records roles once; no downstream rediscovery |
| `RewriteRegion` | Transient, not persisted | Assembled from `StructuralRegion` + live graph query |
| `SubgraphBuilder` lifecycle | Consumed by value, sealed via `finish()` | Recipes can't accidentally reuse or leak builders |
| `ReplacementPatch` isolation | Closed subgraph invariant | Prevents coupling between recipe and original function |
| SSA rewiring | `RewriteBuilder` exclusively | One owner of graph connectivity |
| Rewrite persistence | Clone → mutate → verify → commit or discard | Transactional, never mutates original in place |
| Structural verification | Automatic, post-construction | Failed rewrite = implementation bug, not user error |
| `RewriteResult` | Does not duplicate original `Function` | Caller already holds it |
| Node collections | `BTreeSet` for `GraphDiff` fields | Deterministic iteration, uniqueness guarantee |
| Provenance | `recipe: DefinitionId` included | Answers "why does this node exist?" for debugging |
| Determinism | Byte-for-byte identical output | Reproducible builds, stable regression tests |
| `sir_analysis`/`sir_semantics`/`sir_inference` | Absent from dependencies | Rewrite engine never rediscovers anything |
| Graph canonicalization | Explicitly excluded | Belongs in later optimization passes |
| Error reserve | `InternalInvariantViolation` variant | "This indicates a compiler bug" — catches impossible states |

## Invariants

1. **No discovery:** `sir_rewrite` performs no discovery. Every decision required for rewriting must already have been established by an upstream phase.

2. **Closed subgraph:** A `ReplacementPatch` may reference only nodes created within its own `DetachedArena` and values explicitly provided through the `RewriteRegion` boundary.

3. **Single SSA owner:** `RewriteBuilder` is the sole owner of SSA rewiring, dominance preservation, and use-def repair. Recipes never touch SSA connectivity.

4. **Transactional rewrite:** A structurally invalid rewrite is discarded; the original function is retained unchanged.

5. **Deterministic output:** Given identical inputs, the engine produces byte-for-byte identical rewritten SIR.

6. **One canonical builder:** Every method on `SubgraphBuilder` corresponds 1:1 with exactly one `NodeKind` variant. No hidden synthesis, no macros, no DSLs.
