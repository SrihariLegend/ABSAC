# ABSAC Arena — North-Star Architecture

> **Status: North star. Not current implementation plan.**
> Components are activated only by observed pressure from real experiments.
> Do not build any component until a current experiment requires it.

## Foundational decision

ABSAC should consist of four independent systems:

```text
1. The Board     — represents optimization problems and candidate states
2. The Referee   — verifies correctness and measures performance
3. The Engines   — search strategies that play the game
4. The Arena     — trains, evaluates, and promotes engines
```

The board and referee are the enduring platform. Search engines are replaceable:

```text
Engine 0: Current deterministic optimizer
Engine 1: Best-first/beam search
Engine 2: Non-neural MCTS
Engine 3: MCTS + incremental value model
Engine 4: Policy/value MCTS
Engine 5+: Continually improving champion population
```

---

## 1. The crucial design: two nested games

A single game is not enough because some transformations are locally equivalent, while others require speculative multi-part construction.

ABSAC should have an **outer optimization game** and an **inner synthesis game**.

### Outer game: Verified Optimization

Every outer-game position is:

- complete;
- executable;
- equivalent to the original specification;
- accompanied by proof evidence.

An outer move must commit a complete, verified transformation.

Examples:

```text
x % 8 → x & 7

Kernighan count loop → popcount

Boolean reduction loop → vector mask reduction

Array representation → bitmap representation,
including all affected producers and consumers
```

The final example can physically modify many places but is one atomic outer move.

Because every outer position is equivalent:

```text
Original ≡ State A ≡ State B ≡ State C
```

correctness composes transitively.

### Inner game: Candidate Synthesis

An outer move can invoke an inner synthesis game.

Inner states may be:

- incomplete;
- temporarily slower;
- not independently equivalent;
- partially migrated;
- full of typed holes;
- carrying unresolved obligations.

Example:

```text
Goal:
    Replace BooleanArray with BitSet

Inner move 1:
    Introduce BitSet representation
    Open obligations:
      convert producers
      convert consumers
      preserve boundary behavior

Inner move 2:
    Rewrite membership test

Inner move 3:
    Rewrite count operation to popcount

Inner move 4:
    Add guarded boundary conversion

Inner move 5:
    Complete candidate

Referee:
    Prove concrete equivalence

Commit:
    Candidate becomes one outer-game move
```

An inner candidate can never escape into production. It becomes an outer state only after verification.

This gives ABSAC both:

- the safety of proof-preserving search;
- and the freedom to discover transformations with invalid intermediate forms.

---

## 2. Game instance: Semantic Kernel Capsule

Every game starts from a self-contained capsule.

```text
KernelCapsule
├── Semantic contract
├── Original implementation
├── Inputs and outputs
├── Effects and observations
├── Preconditions
├── Target architecture
├── Workload distribution
├── Optimization objective
├── Search budget
├── Baseline measurements
└── Reference execution mechanism
```

### Semantic contract

The contract defines what "same behavior" means.

For a pure bit-vector function:

```text
For every valid input:
candidate(input) = original(input)
```

For stateful code:

```text
Return values agree
Observable memory agrees
External effects agree
Trap and termination behavior agree
```

For approximate computation:

```text
error(candidate, reference) ≤ declared tolerance
```

For security-sensitive code:

```text
Functional result agrees
Constant-time/noninterference requirement remains satisfied
```

The optimizer cannot decide which observations matter. The capsule declares them.

---

## 3. Outer-game state

A verified outer state should conceptually contain:

```text
OptimizationState
├── Capsule identifier
├── Current executable implementation
├── Proof chain to original specification
├── Semantic interpretation graph
├── Current representations
├── Runtime guards
├── Static cost estimate
├── Measured performance, if available
├── Code size and resource data
├── Transformation history
└── Canonical implementation hash
```

The original implementation is the initial state and is valid by definition.

---

## 4. Inner-game state

A synthesis state contains:

```text
SynthesisState
├── Immutable source region
├── Candidate region under construction
├── Typed holes
├── Open semantic obligations
├── Discharged obligations
├── Proposed assumptions
├── Required runtime guards
├── Representation migration status
├── Static cost estimate
├── Search history
└── Remaining synthesis budget
```

The source remains immutable. All construction happens on a candidate branch.

---

## 5. Action system

Actions should be typed and hierarchical rather than arbitrary token generation.

### Outer actions

```text
ApplyProvenLocalRewrite
OpenRegionSynthesis
OpenRepresentationMigration
SpecializeUnderGuard
FuseVerifiedRegions
LowerSemanticOperation
InvokeDomainOptimizer
StopAndReturnCurrentBest
```

### Inner actions

```text
SelectOntology
SelectAlternativeRepresentation
AddCandidateOperation
ReplaceCandidateSubgraph
RewriteProducer
RewriteConsumer
IntroduceRuntimeGuard
DischargeObligation
RequestCounterexample
RequestLocalProof
CompleteCandidate
AbandonCandidate
```

### Action hierarchy

The search should reason from large decisions to small ones:

```text
Choose semantic interpretation
    ↓
Choose algorithm family
    ↓
Choose representation
    ↓
Choose data layout
    ↓
Choose loop strategy
    ↓
Choose vector strategy
    ↓
Choose instructions
```

MCTS should not initially generate arbitrary source tokens. That action space is too large and too easy to exploit incorrectly.

---

## 6. Legality model

"Legal" must not be one Boolean.

```text
StructurallyLegal
    Well-typed and valid for continued construction.

LocallyProven
    Independently semantics-preserving.

ObligationProducing
    Allowed, but creates explicit proof obligations.

Guarded
    Valid only when a generated runtime predicate succeeds.

Speculative
    Permitted only inside the inner synthesis game.

TerminallyVerified
    Complete and concretely equivalent to the source.

Rejected
    Invalid, impossible, or disproven by counterexample.
```

Only `TerminallyVerified` implementations may enter the outer game.

---

## 7. Ontology interface

Every ontology should implement the conceptual equivalent of:

```text
recognize(region, facts)
    → possible semantic interpretations

enumerate_outer_actions(state)
    → immediately provable transformations

open_synthesis_goal(state)
    → larger transformation objectives

enumerate_inner_actions(synthesis_state)
    → candidate-construction actions

derive_obligations(action)
    → proof requirements

canonicalize(candidate)
    → stable semantic/implementation identity

extract_features(state)
    → search and learning features

lower(semantic_candidate, target)
    → executable implementation candidates
```

### Initial ontology hierarchy

```text
BitVector
├── arithmetic identities
├── masks
├── scans
├── permutations
└── bounded modular operations

FiniteSet
├── membership
├── union/intersection
├── cardinality
└── position queries

LogicalSequence
├── any
├── all
├── parity
├── count
└── first/last occurrence

Representation
├── boolean array
├── scalar bitset
├── multiword bitset
├── SIMD vector
└── lookup table

Hardware
├── scalar
├── branchless
├── SIMD
├── target intrinsics
└── memory/layout alternatives
```

Ontologies should cooperate. The finite-set ontology describes meaning; the representation ontology proposes bitsets; the hardware ontology proposes `popcnt` or SIMD lowering.

---

## 8. The referee

The referee must be more trusted than any engine.

### Referee pipeline

```text
1. Structural validation
2. Type/effect validation
3. Assumption validation
4. Concrete semantic encoding
5. Equivalence proof
6. Counterexample search
7. Differential fuzzing
8. Native compilation
9. Native differential execution
10. Performance measurement
```

### Verification statuses

```text
Proven
Rejected with counterexample
Unknown
Timed out
Unsupported semantics
Compiler failure
Runtime mismatch
Performance regression
Verified and profitable
```

`Unknown` never means "probably correct."

### Independent validation

Where practical, use multiple independent methods:

```text
Symbolic proof
Exhaustive finite-domain checking
Random differential testing
Native differential testing
Sanitizers
Independent solver/backend
```

The model cannot override the referee.

---

## 9. Reward construction

Correctness is a gate, not a reward component.

An unverified candidate cannot win, regardless of predicted speed.

For verified candidates, use a target-specific reward such as:

```text
performance_reward = log(baseline_runtime / candidate_runtime)
```

Then account for other concerns:

```text
reward =
    performance_reward
    - code_size_penalty
    - guard_overhead
    - memory_penalty
    - variance_penalty
    - portability_penalty
```

Search expenditure should be recorded separately and optionally penalized:

```text
economic_reward =
    expected_lifetime_savings
    - search_compute_cost
    - integration_cost
    - maintenance_cost
```

### Noise-resistant scoring

Do not reward a raw benchmark measurement. Use a conservative estimate such as:

```text
lower confidence bound of measured speedup
```

If the result is not statistically distinguishable from noise, score it as no improvement.

---

## 10. Canonicalization and transpositions

Optimization has enormous path convergence:

```text
A → B → D
A → C → D
```

ABSAC needs canonical identity at multiple levels:

```text
Syntactic implementation hash
Normalized SIR hash
Semantic-expression hash
Representation-state hash
Proof-obligation-set hash
```

MCTS and beam search should share a transposition table.

Equivalent candidate implementations can be grouped, but distinct implementations with the same semantics should remain distinguishable when they have different runtime behavior.

---

## 11. Engine progression

### Engine 0: Deterministic baseline

The current optimizer becomes the first player:

```text
Analyze
Recognize
Infer
Generate
Verify
Select
Rewrite
Repeat
```

Its purpose is to prove that the board API can express the existing pipeline without losing capability.

#### Promotion gate

The board-driven Engine 0 reproduces current successful rewrites and failures deterministically.

### Engine 1: Best-first/beam search

Explore multiple verified outer states and selected inner synthesis branches.

Priority might combine:

```text
static cost estimate
proof-success probability
distance to completion
novelty
potential performance ceiling
```

#### Promotion gate

It finds multi-step improvements that Engine 0 misses.

### Engine 2: Non-neural MCTS

Introduce MCTS before neural training.

Use:

- UCT/PUCT with hand-written priors;
- progressive widening;
- transposition tables;
- virtual loss for parallel search;
- static leaf evaluation;
- selective concrete benchmarking.

This determines whether MCTS fits the game before investing in model training.

#### Promotion gate

Against an equal compute budget, MCTS beats Engine 1 on held-out capsules by:

- finding more verified wins;
- finding larger speedups;
- or reaching wins with less search.

### Engine 3: NNUE-like incremental evaluator

Program transformations often change only a small region, making incremental evaluation attractive.

Candidate features:

```text
Operation histogram
Critical dependency depth
Loads/stores
Branch structure
Loop structure
Known trip counts
Vector width
Memory stride
Register-pressure estimate
Representation choices
Code size
Guard count
Target instruction availability
Proof complexity
```

A local transformation updates only affected accumulators.

Prediction heads:

```text
Runtime/cost prediction
Proof-success probability
Compilation-success probability
Expected subtree value
Uncertainty
```

#### Promotion gate

It improves search efficiency on held-out programs without changing the referee.

### Engine 4: Policy/value MCTS

Train:

```text
Policy:
    Which action should be attempted?

Value:
    How good can this state eventually become?

Proof head:
    Will this branch verify?

Performance head:
    What cost is likely?

Uncertainty head:
    How much evidence is missing?
```

MCTS remains responsible for policy improvement; the model supplies priors and leaf values.

### Engine 5: Population league

Maintain several engine personalities:

```text
Fast tactical optimizer
Deep representation optimizer
Proof-friendly optimizer
Exploratory novelty optimizer
Target-specific specialists
General champion
```

Evaluate them in a tournament over hidden capsules and fixed search budgets.

Do not automatically replace the champion because a newer model has a better training loss. Promotion requires verified tournament superiority.

---

## 12. Builder–Breaker–Judge training

A particularly valuable design is to maintain three roles.

### Builder

Proposes optimized candidates.

### Breaker

Attempts to defeat candidates by finding:

- semantic counterexamples;
- unhandled boundaries;
- performance regressions;
- benchmark-distribution weaknesses;
- assumption violations;
- side-channel regressions.

### Judge

The deterministic trusted system:

- solver;
- interpreter;
- compiler;
- native test harness;
- benchmark harness.

The Builder and Breaker may be learned. The Judge should not be.

This prevents the optimization model from learning only how to exploit weak benchmarks or incomplete tests.

---

## 13. Self-generated curriculum

ABSAC can generate its own optimization puzzles.

### Semantic-first generation

1. Generate a semantic specification.
2. Generate one efficient implementation.
3. Apply semantics-preserving "deoptimization" moves.
4. Produce several inefficient but equivalent implementations.
5. Ask the engine to recover an efficient implementation.

Example:

```text
Specification:
    population count of 64-bit value

Generated inefficient forms:
    64-iteration loop
    branch-heavy scan
    table decomposition
    recursive formulation
    redundant mask sequence
    obfuscated arithmetic identity
```

Because deoptimization moves preserve semantics, the system has known-equivalent training tasks by construction.

### Curriculum levels

```text
Level 0: Single fixed-width identities
Level 1: Multi-step bit-vector rewrites
Level 2: Bounded reductions
Level 3: Representation changes
Level 4: Temporary performance valleys
Level 5: SIMD and target lowering
Level 6: Memory and layout
Level 7: Guarded specialization
Level 8: Real extracted production kernels
```

Synthetic performance is not enough. Promotion must depend on hidden real kernels.

### Limitation

A deoptimization curriculum can teach:

- syntactic invariance;
- transformations under obfuscation;
- multi-step composition;
- action ordering;
- recovery from performance valleys;
- generalization across widths and control-flow forms;
- recognition of known semantics in unfamiliar implementations.

It cannot, by itself, produce transformations outside its action grammar.

Deoptimization is useful for bootstrapping search intelligence, but cannot be the sole source of optimization knowledge. Real discovery comes from the candidate grammar plus search and verification.

---

## 14. Data record

Every attempted game should produce a trajectory:

```text
Capsule
State sequence
Actions
Ontology choices
Open/discharged obligations
Proof outcomes
Counterexamples
Compiler outcomes
Static predictions
Raw benchmark samples
Final verified reward
Search budget consumed
Engine/model version
```

Failures must remain in the dataset.

Particularly valuable labels include:

```text
This action was structurally invalid
This branch failed proof
This candidate compiled but was slower
This candidate won only on training inputs
This representation change required another consumer rewrite
This static evaluator was confidently wrong
```

---

## 15. Evaluation leagues

Maintain separate leaderboards.

### Correctness league

```text
False optimizations accepted: must be zero
Counterexamples found
Unsupported cases safely rejected
Mutation bugs detected
```

### Optimization-quality league

```text
Verified speedup
Number of profitable kernels
Maximum speedup
Geometric mean speedup
Tail-latency improvement
```

### Search-efficiency league

```text
Time to first profitable candidate
Compilation count
Solver time
Benchmark time
Total compute spent
```

### Generalization league

```text
Unseen syntax
Unseen kernels
Unseen projects
Unseen ontology combinations
Unseen target architecture
```

A champion must perform well across the relevant leagues.

---

## 16. First bounded game

The initial board should not include all LLVM.

### Supported initial game

```text
Pure fixed-width integer kernels
Widths: 8, 16, 32, 64
No unrestricted pointers
No concurrency
No floating point
No external effects
Statically bounded loops
Explicit modular/trapping semantics
x86-64 target initially
```

### Initial transformations

```text
Bitwise arithmetic
Masks
Power-of-two arithmetic
Bit scans
Popcount/parity
Rotations
Byte/bit permutations
Bounded logical reductions
Scalar-to-bitset representation changes
```

This is large enough to test the architecture and small enough to referee rigorously.

---

## 17. Concrete stage gates

### Milestone A: Board specification

Complete when:

- game instances are fully specified;
- state and action identity are deterministic;
- every action records provenance;
- games can be replayed exactly;
- the referee cannot be bypassed.

### Milestone B: Existing optimizer as Engine 0

Complete when:

- current successful transformations are representable as game actions;
- current benchmark outcomes are reproduced;
- existing expected failures remain expected failures.

### Milestone C: Transactional chunk rewriting

Complete when:

- representation migrations can span multiple physical edits;
- incomplete candidates remain isolated;
- open obligations are explicit;
- only verified transactions can commit.

### Milestone D: Concrete terminal equivalence

Complete when:

- the concrete source and complete candidate are encoded;
- false candidates produce counterexamples;
- theorem templates are helpful but not sufficient for acceptance.

### Milestone E: Search baseline

Complete when:

- beam/best-first search finds multi-step transformations;
- equivalent states are deduplicated;
- performance valleys can be crossed;
- search cost is recorded.

### Milestone F: Non-neural MCTS

Complete when:

- MCTS plays the same board;
- it is evaluated under equal budgets;
- it improves over Engine 1 on held-out games.

### Milestone G: Incremental evaluator

Complete when:

- predictions update after local changes;
- uncertainty is calibrated;
- search becomes measurably more efficient.

### Milestone H: Policy/value MCTS

Complete when:

- search trajectories train policy and value heads;
- promotion uses hidden tournaments;
- no correctness authority is delegated to the model.

### Milestone I: First commercial proof

Complete when a real kernel has:

- concrete equivalence proof;
- adversarial testing;
- stable material speedup;
- independently reproducible benchmark;
- safe integration and fallback;
- economically meaningful savings.

---

## 18. What not to do

Do not:

- let the neural model define legality;
- make every tiny edit independently profitable;
- forbid chunk rewrites;
- require speculative intermediate candidates to be executable;
- use tests as a substitute for equivalence;
- use one benchmark input as truth;
- train and evaluate on the same kernels;
- claim victory over `-O3` without strong target flags and baselines;
- build MCTS before deterministic state transition and replay work;
- collapse all objectives into an unstable arbitrary reward;
- freeze architecture before real programs pressure it.

---

## 19. Two action languages

### Semantic macro-actions

Efficient, human-understandable moves supplied by ontologies:

```text
Change finite-set representation to bitset
Turn reduction into popcount
Fuse predicate and cardinality
Vectorize logical sequence
Specialize for bounded domain
```

### Universal synthesis actions

A lower-level escape hatch capable of constructing implementations that have no existing semantic name:

```text
Create typed operation
Connect values
Introduce local state
Construct bounded loop
Construct vector operation
Create lookup representation
Partition input domain
Introduce guarded implementation
Replace arbitrary typed region
```

The semantic actions provide intelligence and efficiency. The universal synthesis language provides completeness within the supported computational domain.

Without the second layer, the ontology becomes a prison built from our current knowledge.

### Universal synthesis v0: typed SSA expression construction

Scope:

```text
Pure
Acyclic
Fixed-width bit-vectors
No memory
No loops
No calls
Explicit target operation grammar
```

A synthesis state is a typed DAG containing inputs, constructed nodes, and typed output holes.

#### Actions

Choose the next output hole in canonical order and fill it with one of:

```text
Input reference
Existing value reference
Typed constant
Unary operation
Binary operation
Comparison
Select
Extract
Concatenate
Bitcast
Target-neutral intrinsic
```

Initial grammar:

```text
not, neg
and, or, xor
add, sub, mul
shl, logical-shr, arithmetic-shr
eq, ne, lt, le
select
ctpop, ctlz, cttz
rotate
byte-swap
extract/concat
```

Every operation is width-checked.

#### Search constraints

```text
Maximum node count
Maximum dependency depth
Maximum constant count
Maximum expensive-operation count
No duplicate commutative forms
No dead nodes
Canonical operand ordering
Constant folding
Common-subexpression reuse
```

#### Verification

Use CEGIS-like iteration:

```text
Generate candidate
Test against current counterexample set
Reject quickly if it fails
Attempt SMT equivalence if it survives
If false:
    add counterexample
If true:
    benchmark candidate
```

#### Search algorithm

For this first finite grammar:

1. Enumerative search.
2. Cost-guided best-first search.
3. E-graph normalization/deduplication.
4. Beam search if enumeration becomes too large.
5. MCTS only after measuring the resulting graph.

This gives exact ground truth for small expressions and provides training data later.

### Universal synthesis v1: structured kernel sketches

Only after v0 works, add typed structural primitives:

```text
Map
Reduce
Scan
Chunk
Vectorize
Guard
Tail
Load
Store
Permutation
Lookup
```

Instead of generating arbitrary loops instruction by instruction, search fills structured sketches:

```text
Reduce(
    input = Chunk<8>(buffer),
    map = ?,
    combine = ?,
    identity = ?
)
```

This constrains search enough to remain tractable while allowing implementations not encoded as named recipes.

### Universal synthesis v2: speculative representation transactions

Then add:

```text
new representation
producer migration
consumer migration
boundary conversion
runtime guard
commit
```

That is where the inner game becomes essential.

"Universal" always means universal within an explicitly supported grammar—not arbitrary computation in the absolute sense.

---

## 20. Multi-view board

The best representation is probably not one graph. It is a synchronized **semantic implementation hypergraph** containing several views:

```text
Immutable behavioral specification
Semantic interpretations
Current physical implementation
Candidate implementation under construction
Control and data flow
Memory and effect structure
Alternative representations
Target hardware
Workload distribution
Proof obligations
Measured performance evidence
```

Relationships between views are as important as the views themselves:

```text
This loop implements cardinality
This array represents a finite set
This target operation implements popcount
This assumption permits this specialization
This candidate region replaces this source region
```

The model can then reason both:

- upward toward meaning;
- downward toward hardware.

---

## 21. Long combinations and performance valleys

The game must support:

- temporary slowdowns;
- incomplete inner candidates;
- chunk rewrites;
- representation migrations;
- simultaneous producer/consumer changes;
- arbitrary-length combinations;
- runtime specialization;
- multiple implementation versions.

Only the final committed candidate needs complete behavioral equivalence.

That is how the system can discover a line analogous to a strange chess sacrifice:

```text
Current position temporarily looks worse
→ representation changes
→ several consumers become simpler
→ operations fuse
→ vector lowering becomes possible
→ final implementation is dramatically faster
```

A greedy compiler rejects the first step. A sufficiently deep search can understand its eventual value.

---

## 22. Evolving macro-actions

Chess has fixed rules and fixed pieces. ABSAC's foundational rules can remain fixed while its learned macro-actions evolve.

The permanent rules are:

```text
Typed construction
Explicit semantics
Proof obligations
Concrete verification
Performance measurement
```

But between generations, ABSAC can discover recurring successful sequences:

```text
action A → action F → action Q → action B
```

and compress them into a learned macro-action:

```text
NewMove137
```

Successful unnamed transformations could be:

1. discovered through low-level synthesis;
2. concretely verified;
3. observed across multiple kernels;
4. clustered by semantic behavior;
5. generalized;
6. promoted into a reusable ontology concept or macro-move.

That creates a loop:

```text
Search with current concepts
→ discover unfamiliar implementation
→ verify it
→ abstract recurring structure
→ learn a new concept
→ add a new macro-action
→ search deeper with the expanded vocabulary
```

The optimizer would not merely learn to play the game. It would learn better pieces and strategic abstractions while retaining an immutable correctness referee.

---

## 23. Division of responsibility with LLVM

ABSAC should not attempt to replace LLVM's entire backend. The best division is:

```text
ABSAC:
    discovers better semantics, algorithms, representations and vector plans

LLVM:
    performs target instruction selection, scheduling and register allocation
```

ABSAC should be capable of emitting:

- LLVM semantic intrinsics such as `ctpop`, `ctlz`, `cttz`, rotations and byte swaps;
- vector LLVM IR;
- target-independent vector operations;
- target intrinsics when LLVM cannot express or reliably select the intended operation;
- guarded multi-versioned candidates.

Portable scalar C should remain one backend, not the only backend.

ABSAC should win by giving LLVM a better computation, not by prematurely reimplementing LLVM's backend.

---

## 24. Final direction

The central design is:

```text
The outer game searches only among verified equivalent programs.

The inner game freely constructs large speculative chunk replacements.

The referee is stronger and more trusted than every engine.

The deterministic optimizer becomes the first player.

Search creates the dataset.

The dataset trains the evaluator.

The evaluator strengthens MCTS.

MCTS generates better data.

Only hidden, verified tournaments promote a new champion.
```

The most important invention is not the first neural network. It is constructing a game where **creativity is unconstrained inside transactions, but nothing becomes real without proof and measurement**.

The governing design principle:

> **Make correctness rigid, but make construction universal. Give the engine semantic abstractions without restricting it to known abstractions. Allow arbitrarily large speculative transformations, and judge only complete candidates by proof and real performance.**
