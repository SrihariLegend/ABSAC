# Phase 0012 — Equivalence Verification Design

**Status:** Research milestone (specification)
**Date:** 2026-07-03
**Depends on:** Phase 0011 (Transformation Planning)

## Purpose

Prove (or reject) candidate transformation plans. No rewriting occurs in this phase. The verifier is a mathematical filter: every accepted transformation must have a machine-checkable proof obligation that is discharged by a verification backend.

## Philosophy

Everything before this phase answered progressively richer questions:

| Phase    | Question                               |
| -------- | -------------------------------------- |
| SIR      | What is the program?                   |
| SAF      | What compiler facts are true?          |
| SRI      | What does the program mean?            |
| Planning | What transformations appear promising? |

Phase 0012 answers an entirely different question:

> **Is the proposed transformation mathematically equivalent to the original?**

Unlike previous phases, verification is expected to reject most candidates. A failed proof is a successful verification result.

## Architecture

```
CandidateDatabase + TransformationContextDatabase
        │
        ▼
TransformationDefinition::obligation(context)
        │
        ▼
ProofObligationDatabase
        │
        ▼
AssumptionValidator
        │
        ▼
VerificationEngine (policy-driven backend selection)
        │
   ┌────┴────┐
   ▼         ▼
Symbolic   Exhaustive
   │         │
   └────┬────┘
        ▼
VerificationResult (Proven | Rejected | Unknown)
        │
        ▼
VerificationReport
```

The verifier never reads or modifies SIR. Every component from `ProofObligation` onward operates solely on mathematical artifacts.

## Prerequisites

### Candidate.definition_id (sir_generation change)

`Candidate` currently holds an `ImplementationStrategy` enum. For the verifier to connect a candidate to its `TransformationDefinition`, `Candidate` needs a `definition_id: DefinitionId` field. This is a narrow, backward-compatible addition to `sir_generation`:

```rust
pub struct Candidate {
    pub id: CandidateId,
    pub region: RegionId,
    pub context_id: ContextId,
    pub definition_id: DefinitionId,    // NEW — maps to TransformationDefinition::id()
    pub strategy: ImplementationStrategy,  // retained for display/debugging
    pub explanation: CandidateExplanation,
    pub effects: Vec<CandidateEffects>,
}
```

`DefinitionId` is defined in `sir_verification`. Since `sir_generation` cannot depend on `sir_verification`, either:
- Define `DefinitionId` in `sir_transform` (the shared contract crate), or
- Store a `u64` in `Candidate` and cast to `DefinitionId` at the verification boundary

Prefer the first option: move `DefinitionId` (and `ObligationId`, `VariableId`) to `sir_transform` as thin newtypes that both `sir_generation` and `sir_verification` can reference.

## Dependency graph (v0.1)

```
sir_types        (RegionId, RegionMap)
sir_transform    (Assumption, Constraint, Representation, TransformationContext)
sir_generation   (Candidate, CandidateId)
     ↓
sir_verification
```

The verifier does **not** depend on `sir_nodes`, `sir_analysis`, `sir_semantics`, `sir_inference`, or `sir_builder`. It consumes the abstractions produced by the pipeline, not the IR.

### Design principle

> Every concept has exactly one canonical owner. Verification asks that owner questions rather than reimplementing its knowledge.

### Future extraction (not in v0.1)

When rewriting and cost modeling force the issue, two new foundational crates will be extracted:

```
sir_logic          ← semantic/ (SemanticExpression, Interpreter, Normalizer)
sir_transforms     ← registry.rs + definitions/ (TransformationDefinition trait + implementations)
```

## Crate structure

```
sir_verification/
    Cargo.toml
    src/
        lib.rs                  // Verifier, VerificationResult, VerificationPolicy, public API
        obligation.rs           // ProofObligation, ObligationId, ProofObligationDatabase
        registry.rs             // TransformationDefinition trait + TransformationRegistry
        definitions/
            mod.rs              // Registry population
            popcount.rs         // PopcountDefinition (the one BS001 definition)

        semantic/
            mod.rs
            expression.rs       // SemanticExpression (closed enum)
            value.rs            // Value (operational semantics result types)
            interpreter.rs      // Operational semantics (standalone, no optimization)
            normalizer.rs       // Normalization framework
            theorem.rs          // Theorem (two SemanticExpressions)
            rules/
                mod.rs
                count_filter_to_popcount.rs  // The one rule for BS001

        backends/
            mod.rs
            symbolic.rs         // Symbolic verifier (normalize + structural compare)
            exhaustive.rs       // Exhaustive verifier (enumerate + interpret)

        validation.rs           // AssumptionValidator
        report.rs               // VerificationReport
        errors.rs               // RejectReason, UnknownReason, InterpreterError
```

## Core types

### Identifier newtypes

```rust
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct VariableId(pub u64);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct DefinitionId(pub u64);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ObligationId(pub u64);
```

### SemanticExpression

Closed enum. Intentionally minimal. Exhaustiveness is a feature, not a limitation.

```rust
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SemanticExpression {
    Variable(VariableId),
    Constant(ConstantData),
    BooleanArray { variable: VariableId },
    Pack(Box<SemanticExpression>),
    Filter { input: Box<SemanticExpression>, predicate: Predicate },
    Count(Box<SemanticExpression>),
    Popcount(Box<SemanticExpression>),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Predicate {
    True,
}
```

**Design rule:** Every new `SemanticExpression` variant must justify itself by enabling the proof of at least one new transformation theorem. Generality is not a goal of Phase 0012.

**Key properties:**
- Closed algebra — the compiler guarantees exhaustive matching in normalization, interpretation, and future SMT encoding
- Denotational, not operational — different SIR implementations collapse to the same `SemanticExpression`
- No SIR references — these are mathematical objects, not compiler IR nodes

### Value

The result type of the operational semantics (interpreter).

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Bool(bool),
    Integer(u64),
    BooleanArray(Vec<bool>),
    BitVector(BitVectorValue),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BitVectorValue {
    pub bits: u128,
    pub width: usize,  // semantically significant, not an implementation detail
}
```

### Theorem

```rust
/// A mathematical statement: lhs ≡ rhs under the stated assumptions.
#[derive(Clone, Debug)]
pub struct Theorem {
    pub lhs: SemanticExpression,
    pub rhs: SemanticExpression,
}
```

### ProofObligation

A self-contained verification problem. No SIR references — portable across backends.

```rust
#[derive(Clone, Debug)]
pub struct ProofObligation {
    pub id: ObligationId,
    pub region: RegionId,
    pub candidate: CandidateId,
    pub definition: DefinitionId,
    pub theorem: Theorem,
    pub assumptions: Vec<Assumption>,
    pub domain: Option<FiniteDomain>,
}
```

### FiniteDomain

```rust
#[derive(Clone, Debug)]
pub struct FiniteDomain {
    pub variables: Vec<VariableSpec>,
}

impl FiniteDomain {
    /// Compute total state count from variable specs. Not stored — derived.
    /// Returns None on overflow (e.g., bool[65] exceeds u64::MAX states).
    /// The exhaustive verifier treats overflow as DomainTooLarge.
    pub fn total_states(&self) -> Option<u64>;
    pub fn enumerate(&self) -> DomainIterator;
}

pub struct VariableSpec {
    pub id: VariableId,
    pub kind: VariableKind,
}

pub enum VariableKind {
    BooleanArray { length: usize },
}
```

### ProofObligationDatabase

Following the pattern established by every previous phase: `FactDatabase`, `SemanticDatabase`, `TransformationContextDatabase`, `CandidateDatabase`.

```rust
#[derive(Clone, Debug, Default)]
pub struct ProofObligationDatabase {
    obligations: Vec<ProofObligation>,
    by_region: RegionMap<Vec<usize>>,   // indices into obligations
    by_candidate: HashMap<CandidateId, usize>,
}
```

### TransformationDefinition

The canonical owner of a transformation's mathematics. One implementation per transformation family. The planner, verifier, and (future) rewriter all ask the same definition.

```rust
pub trait TransformationDefinition {
    fn id(&self) -> DefinitionId;
    fn name(&self) -> &'static str;

    /// Is this transformation applicable to the given context?
    fn applicability(&self, context: &TransformationContext) -> bool;

    /// Construct the full proof obligation for a given context.
    /// Owns: theorem construction, assumption enumeration, domain specification.
    fn obligation(&self, context: &TransformationContext) -> ProofObligation;
}
```

### VerificationResult

```rust
pub enum VerificationResult {
    Proven(Proof),
    Rejected(RejectReason),
    Unknown(UnknownReason),
}
```

### Proof + ProofStep

```rust
pub struct Proof {
    pub theorem: Theorem,
    pub normalized_theorem: Theorem,  // the theorem after canonicalization
    pub backend: VerificationBackend,
    pub steps: Vec<ProofStep>,
}

pub enum ProofStep {
    Normalization {
        rule: &'static str,
        before: SemanticExpression,
        after: SemanticExpression,
    },
    ExhaustiveCheck {
        states_checked: u64,
    },
}

pub enum VerificationBackend {
    Symbolic,
    Exhaustive,
}
```

### RejectReason

```rust
pub enum RejectReason {
    AssumptionViolated { assumption: Assumption },
    SemanticMismatch { lhs: SemanticExpression, rhs: SemanticExpression },
    CounterExample { environment: Environment, lhs: Value, rhs: Value },
    UnsupportedExpression { expr: SemanticExpression },
}
```

### UnknownReason

```rust
pub enum UnknownReason {
    NoApplicableBackend,
    DomainTooLarge { states: Option<u64>, max: u64 },
    DomainOverflow,
    UnsupportedExpression { expr: SemanticExpression },
    NonTerminatingNormalization { steps: usize },
}
```

## Semantic components

### Interpreter

The canonical operational semantics of `SemanticExpression`. Deliberately dumb — one recursive walk, no optimization, no caching. The reference implementation against which all backends are validated.

**Invariant:** Every verification backend (symbolic, exhaustive, SMT, SAT, theorem prover) must agree with the interpreter on all supported expressions.

```rust
pub struct Interpreter;

impl Interpreter {
    /// Evaluate an expression in the given environment.
    /// Never panics — returns InterpreterError on malformed states.
    pub fn evaluate(
        expr: &SemanticExpression,
        env: &Environment,
    ) -> Result<Value, InterpreterError>;
}

#[derive(Clone, Debug)]
pub enum InterpreterError {
    UnboundVariable(VariableId),
    TypeMismatch { expected: &'static str, found: Value },
}
```

### pack_bits specification

```
pack_bits(bits: &[bool]) -> BitVectorValue

Maps a boolean array to a bitvector where:
  bit i of the resulting BitVector = element i of the input array
  (little-endian bit numbering: element 0 → bit 0)

width = bits.len()
unused high bits (beyond width) in the u128 are zero.
```

**Host-endianness independence:** Bit ordering is defined purely in terms of bit shifts (`1 << i`), never memory-casting or pointer transmutation. This ensures identical results on x86_64 (little-endian), ARM/RISC-V (bi-endian), and Wasm runtimes. The `BitVectorValue` is a mathematical value, not a host-dependent machine word.

This is the canonical bit-ordering. Any change to this specification would invalidate all proofs that involve `Pack` or `Popcount`.

### Environment

```rust
#[derive(Clone, Debug)]
pub struct Environment {
    bindings: BTreeMap<VariableId, Value>,  // deterministic iteration
}
```

### Normalizer

A canonicalization engine, not a rewrite engine. Rewrite engines search; normalizers reduce.

```rust
pub struct Normalizer {
    rules: Vec<Box<dyn NormalizationRule>>,
    max_steps: usize,  // safety limit: prevent non-termination
}

impl Normalizer {
    /// Recursively normalize children, then apply rules at this node.
    /// Applies first-matching rule with restart strategy.
    /// Returns the normal form and the sequence of applied rules.
    pub fn normalize(&self, expr: &SemanticExpression) -> (SemanticExpression, Vec<ProofStep>);
}
```

**Properties:**
- Recursive: normalizes children before attempting to rewrite parent nodes
- Fixed-point: repeats until no rule matches (idempotent by construction)
- Deterministic: rules applied in registration order, first-match restart
- Guarded: `max_steps` prevents infinite loops from future rule conflicts

### NormalizationRule

```rust
/// A single semantic-preserving rewrite rule.
///
/// Invariant: A rule may only inspect the subtree rooted at the supplied
/// expression. It may not depend on global context, proof obligations,
/// or external state. This keeps normalization purely equational.
///
/// v0.1: Rules are purely structural — they match on expression shape only.
/// In future phases, if a rule's validity depends on assumptions (e.g.,
/// "this rewrite is only valid when len > 0"), the `apply` signature may
/// be extended to accept `&[Assumption]`. For now, the single BS001 rule
/// is universally valid for any BooleanArray width ≤ 128.
pub trait NormalizationRule {
    fn name(&self) -> &'static str;
    fn apply(&self, expr: &SemanticExpression) -> Option<SemanticExpression>;
}
```

### CountFilterToPopcount (the one rule for v0.1)

```
Count(Filter(BooleanArray(v), True)) → Popcount(Pack(BooleanArray(v)))
```

This is the mathematical identity that powers the BS001 proof. It is the only normalization rule in v0.1. The rule is purely local — it matches on the expression structure and rewrites in one step.

## Backends

### Symbolic verifier

1. Normalize `lhs` to canonical form
2. Normalize `rhs` to canonical form
3. Structural equality comparison
4. If equal → `Proven` with normalization trace
5. If different → `Rejected(SemanticMismatch)`

The symbolic verifier handles infinite domains (like `bool[64]`) because it never enumerates inputs.

### Exhaustive verifier

1. Check domain is finite, `total_states()` does not overflow, and `total_states() <= max_states`
2. Enumerate all input combinations in deterministic order
3. For each: `interpreter.evaluate(lhs) == interpreter.evaluate(rhs)`
4. **Short-circuit on first mismatch** — if any input produces `lhs != rhs`, return `Rejected(CounterExample { ... })` immediately without evaluating remaining states
5. If all match → `Proven` with `ExhaustiveCheck { states_checked }`

The exhaustive verifier serves double duty:
- Fallback for finite domains the symbolic verifier cannot handle
- **Reference oracle** — validates the symbolic engine against concrete execution

### Backend selection policy

```rust
pub enum VerificationPolicy {
    /// Symbolic first, fall back to exhaustive if unknown.
    Default,
    /// Symbolic only.
    SymbolicOnly,
    /// Exhaustive only (requires finite domain).
    ExhaustiveOnly,
}
```

The `Default` policy: symbolic → (if unknown) exhaustive → (if unknown) `Unknown(NoApplicableBackend)`.

## Assumption validation

Assumptions are admissibility conditions, not proofs. They are validated as an explicit stage before backend verification.

```rust
pub struct AssumptionValidator;

impl AssumptionValidator {
    /// Check that all required assumptions are satisfied by the context.
    /// Returns Ok if the obligation is admissible, Err with the violated assumption otherwise.
    pub fn validate(
        obligation: &ProofObligation,
        context: &TransformationContext,
    ) -> Result<(), Assumption>;
}
```

## Public API

```rust
/// Registry of known transformation definitions.
/// Lookup by DefinitionId or by ImplementationStrategy (via Candidate).
pub struct TransformationRegistry {
    definitions: Vec<Box<dyn TransformationDefinition>>,
}

impl TransformationRegistry {
    pub fn new() -> Self;
    pub fn register(&mut self, def: Box<dyn TransformationDefinition>);
    pub fn lookup(&self, id: DefinitionId) -> Option<&dyn TransformationDefinition>;
    /// Find the definition applicable to a given candidate + context.
    pub fn find_for(
        &self,
        candidate: &Candidate,
        context: &TransformationContext,
    ) -> Option<&dyn TransformationDefinition>;
}

pub struct Verifier {
    registry: TransformationRegistry,
    policy: VerificationPolicy,
    limits: VerificationLimits,
}

pub struct VerificationLimits {
    pub max_states: u64,  // exhaustive enumeration cap (default: 1_048_576 = 2^20)
}

/// Summary statistics from a verification run.
#[derive(Clone, Debug, Default)]
pub struct Statistics {
    pub total: usize,
    pub proven: usize,
    pub rejected: usize,
    pub unknown: usize,
    pub by_backend: HashMap<VerificationBackend, usize>,
}

impl Verifier {
    /// Create a verifier with default policy and all built-in definitions registered.
    pub fn new() -> Self;

    /// Build proof obligations for all candidates in the database.
    pub fn build_obligations(
        &self,
        candidates: &CandidateDatabase,
        contexts: &TransformationContextDatabase,
    ) -> ProofObligationDatabase;

    /// Verify a single obligation using the configured policy.
    pub fn verify(&self, obligation: &ProofObligation) -> VerificationResult;

    /// Generate a human-readable verification report.
    pub fn report(&self, results: &[VerificationResult]) -> VerificationReport;

    /// Return verification statistics (proven/rejected/unknown counts).
    pub fn statistics(&self, results: &[VerificationResult]) -> Statistics;
}
```

## Verification report format

```
Obligation #0
Transformation: Popcount
Backend: Symbolic
Status: PROVEN

Theorem:
  Count(Filter(BooleanArray(%0), True))
  ≡
  Popcount(Pack(BooleanArray(%0)))

Normalized theorem:
  Popcount(Pack(BooleanArray(%0)))
  ≡
  Popcount(Pack(BooleanArray(%0)))

Assumptions:
  ✓ EquivalentCardinality
  ✓ FixedLength(64)
  ✓ ReadOnly

Proof steps:
  1. Normalization(CountFilterToPopcount)
     Count(Filter(BooleanArray(%0), True))
     → Popcount(Pack(BooleanArray(%0)))
```

## Acceptance benchmark

Only one theorem — the canonical BS001 transformation.

### Original
```
for i in 0..64 {
    if board[i] {
        count += 1;
    }
}
```

### Candidate
```
bb = pack(board);
count = popcount(bb);
```

### Theorem
```
∀ board: bool[64]
  Count(Filter(BooleanArray(board), True))
  ≡
  Popcount(Pack(BooleanArray(board)))
```

### Assumptions
- `EquivalentCardinality` — the transformed computation produces identical cardinality
- `FixedLength(64)` — the array has a statically known size
- `ReadOnly` — the structure is not mutated

## Tests

### Tier 1 — Obligation construction
`PopcountDefinition::obligation()` produces the correct theorem with the right assumptions and domain.

### Tier 2 — Symbolic normalization
`CountFilterToPopcount` rewrites correctly. Normalizer reaches fixed point. Idempotency holds (`normalize(normalize(x)) == normalize(x)`).

### Tier 3 — Reference oracle (exhaustive)
`bool[4]` — 16 states. Interpreter produces identical results for both sides. Exhaustive verifier returns `Proven` with `states_checked = 16`.

### Tier 4 — BS001 canonical theorem
Full pipeline integration: build SIR → analyze → semantics → inference → generation → **verification**. Symbolic backend proves `Count ≡ Popcount` under the three assumptions. Produces a `Proof` artifact with normalization trace.

### Tier 5 — Negative tests
Deliberately broken theorem (e.g., `Count ≡ Count + 1`). Symbolic verifier returns `Rejected(SemanticMismatch)`. Exhaustive verifier returns `Rejected(CounterExample{...})`.

### Tier 6 — Unknown tests
Unsupported expression (a variant not in v0.1). Verifier returns `Unknown(UnsupportedExpression)`.

### Tier 7 — Regression
Every theorem ever proven becomes permanently part of the regression suite. Re-running the test suite must confirm that no previously proven theorem has become rejected or unknown.

### Cross-validation (T3 extended)
For domains small enough to enumerate: both symbolic and exhaustive backends must agree. If they disagree, one has a bug. This test is run as part of T3.

## Non-goals

Phase 0012 explicitly does **not**:

- Rewrite SIR
- Emit Rust
- Invoke LLVM
- Call Z3 or any SMT solver
- Perform performance modeling
- Rank candidates
- Build a general theorem prover
- Create a rewrite engine (this is a normalization engine)

## Invariants

1. **SIR independence:** The verifier never reads or modifies SIR. All information needed for verification is encoded in the `ProofObligation`.

2. **Backend-independent obligations:** A `ProofObligation` is a portable mathematical artifact. Any backend can attempt to discharge it.

3. **Interpreter as reference semantics:** The `Interpreter` defines the canonical operational semantics of `SemanticExpression`. All backends must agree with it on all supported expressions.

4. **Monotonic verification:** A candidate that has been proven equivalent may never subsequently become rejected unless either the transformation definition or the semantic language changes.

5. **One canonical owner:** Transformation mathematics is owned by `TransformationDefinition`. No other component duplicates this knowledge.

## Success criterion

By the end of Phase 0012, the system shall complete the following pipeline:

```
Source Program → SIR → Compiler Facts → Semantic Truths
→ Representation Beliefs → Transformation Contexts
→ Candidate Plans → Proof Obligations → Machine Verification
```

For the canonical BS001 benchmark, the verifier shall prove that replacing a fixed-length boolean-array counting loop with `popcount(pack(board))` preserves the observable result for all possible inputs under the stated assumptions, producing a backend-independent `Proof` artifact that completely records how equivalence was established.

## Scope discipline

The most important constraint for 0012 is **restraint**. This phase does not build a general theorem prover or integrate SMT solvers. It establishes the verification architecture around **proof obligations**, **semantic expressions**, and **verification backends**, and proves exactly one canonical transformation end to end. Later phases can add SMT, SAT, translation validation, richer proof backends, and additional transformation families without redesigning the core verifier.

## Design decisions log

| Decision | Choice | Rationale |
|----------|--------|-----------|
| `SemanticExpression` form | Closed enum | Exhaustiveness guarantees. Open trait loses this. |
| Expression scope | Minimal (7 variants) | Only variants needed for BS001. Expand theorem-by-theorem. |
| `TransformationDefinition` location | `sir_verification` (v0.1) | Only verification needs `obligation()` today. Extract to `sir_transforms` later. |
| Verifier reads SIR? | No | Every phase consumes previous abstraction. Reaching back violates layering. |
| Obligation builder design | Thin dispatcher over `TransformationDefinition` | One canonical owner per transformation family. No giant match. |
| Symbolic backend design | Normalization framework, not rewrite engine | Rewrite engines search; normalizers reduce. One rule for v0.1. |
| Interpreter design | Standalone component, not inside any backend | Operational semantics is a language property, not a backend property. |
| `ProofObligation` storage | `ProofObligationDatabase` | Consistent with every previous phase's database pattern. |
| `Environment` storage | `BTreeMap<VariableId, Value>` | Deterministic iteration, O(log n) lookup. Consistent with SIR's preference for ordered maps. |
| `BitVector` representation | Struct with explicit width | Width is semantically significant, not an implementation detail. |
| `FiniteDomain::total_states` | Computed, not stored | Avoids redundant state. Derived from variable specs. |
| Assumption validation | Separate `AssumptionValidator` stage | Assumptions are admissibility conditions, not proofs. Mathematically distinct. |
| Backend selection | `VerificationPolicy` enum | Extensible to SMT/SAT without changing verifier architecture. |
| Max normalization steps | Guarded (`max_steps`) | Prevents infinite loops from future conflicting rules. |
| Interpreter error handling | Returns `Result`, never panics | Research tools hit malformed states during development. Panics make debugging harder. |
