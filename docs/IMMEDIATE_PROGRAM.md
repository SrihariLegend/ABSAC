# ABSAC Immediate Research Program — Witness Frontier

> **Status: Current implementation plan.**
> Execute these gates in order. Do not build any Arena component until these pass.

## Thesis question

The immediate question is not:

> Can we build a DeepMind-style optimizer?

It is:

> Is there a bounded semantic domain where ABSAC can automatically produce concretely verified implementations that materially outperform the strongest available compiler/library baseline on previously unseen real kernels?

That is the business and research foundation.

---

## Naming: witness frontier, not oracle

An oracle would tell us the fastest possible implementation. We do not know that — that is partly what ABSAC is intended to discover. Hand-written implementations only demonstrate that a certain amount of headroom is known to exist.

If the best hand-written implementation is 3× faster than Clang, we have strong evidence of reachable headroom.

If it ties Clang, that does **not** establish that no better implementation exists. It establishes only:

> We currently have no witness showing exploitable headroom in this kernel.

That should kill or deprioritize the **current kernel/domain hypothesis**, not the entire project. Otherwise, we would be using current human knowledge to conclude that a system intended to exceed current human knowledge cannot succeed.

---

## Execution sequence

```text
1. Establish benchmark truth.
2. Construct the strongest known implementation frontier.
3. Identify kernels with demonstrated headroom.
4. Determine which semantic/target capability reaches that headroom.
5. Test whether search contributes.
6. Build no general Arena component until these pass.
```

---

## Gate sequence

### Gate 1: Benchmark credibility

Pass when:

- both sides use equivalent build flags;
- results are stable;
- assembly explains the result;
- input-size effects are understood.

### Gate 2: Witness headroom

Pass when:

- at least one real kernel has a materially faster known candidate than the strongest baseline.

If this fails, change domains or deprioritize the kernel. Do not conclude the project is impossible.

### Gate 3: Expressive reachability

Pass when:

- ABSAC's semantic candidate language and backend can represent that candidate family.

If this fails, improve ontology/lowering — not search.

### Gate 4: Concrete correctness

```
Gate 4A: PASSED — compositional correctness argument
                    + extensive adversarial validation (16.8M tests, ASan + UBSan)

Gate 4B: OPEN   — mechanically checked concrete end-to-end equivalence
```

The six lemmas are human-written mathematical arguments supported by
testing. They are NOT machine-checked proofs. Gate 4B requires solver
verification of per-chunk identities, loop invariant, tail decomposition,
region binding, and intrinsic semantics.

### Gate 5: Search value

```
Gate 5A: COMPLETE — target-plan search landscape mapped
    69 plans enumerated, Engine 0 within 0-14% of measured best.
    Deterministic selection captures most of the benefit.
    No MCTS needed at this scale.

Gate 5B-Witness: PASSED — hand-constructed semantic composition
    Multi-reduction fusion provides 28-35% improvement over
    independent vectorization. No single recipe contains this.

Gate 5B-Automation: OPEN — ABSAC has not yet generated the fusion
    from primitive actions.

Gate 5B-Search: OPEN — search has not yet been shown to discover
    the fusion when Engine 0 does not.
```

If this fails, MCTS is unjustified for the current action space.

### Gate 6: Generalization

Gate 6A-v0:  FAILED   50% false positives on held-out negatives (3/6).
                     Preserved as a historical record — do not overwrite.

Gate 6A-Rem: PASSED   on the regression corpus only (0/6). NOT held-out
                     proof — the corpus was inspected and is now D1.

Gate 6A-v1:  FAILED   fresh blind corpus (H1): 1/4 lowered negatives
                     falsely recognized (N15 — certificate completeness
                     gap), 1 frontend soundness bug (V07 — SIR verifier
                     caught invalid two-loop lowering), 2/8 positives
                     lost to mixed-width lowering, All-reduction recall
                     miss (V03).

Remediation D2:
  0A: mandatory SIR verification gate (lower_function fails loudly)
  0B: ReductionCertificate closed-world memory-footprint check
  1:  llvm.smax/smin, pre-header emission, operand type hints
  2:  AllReduction (select_reset form) recognized — V03/H06/k18

Gate 6A-v2:  FAILED   fresh blind corpus (H2): 1/7 lowered negatives
                     false-recognized (X02 signed-overflow — certificate
                     lacks overflow semantics), AND the first unsafe
                     candidate generation (X06 running max → PositionSearch
                     belief → BitScanForward candidate, contained only by
                     a role-completeness failure in the rewrite layer).
                     Generational trend: 3/6 → 1/4 → 1/7 observed (not
                     statistically meaningful — qualitative only).

Gate 6A-v3:  OPEN     requires a fresh corpus after D3 remediation.

Gate 6B:     RESERVED — blindness protocol must be established before
             fusion automation (seal corpus now or independent post-freeze
             construction). Must not be inspected while automating fusion.

Generational protocol: freeze → evaluate blindly → fail → preserve the
failure → understand the missing semantics → remediate → evaluate on a
new frontier. Each failing corpus becomes the next regression set
(H0→D1, H1→D2). Failures are never overwritten by remediation results.

Pass when:

- the system produces verified wins on held-out real kernels;
- false-positive recognition rate is negligible;
- safe abstention works correctly.

Only after Gate 6 should the general Arena become a major implementation effort.

---

## Experiment 1: Benchmark truth

### Build matrix

Use identical optimization conditions:

```text
Clang -O3 -march=native
GCC   -O3 -march=native
```

Where relevant, add:

```text
LTO
PGO
explicit target architecture
```

Do not use `-ffast-math` or relaxed semantics unless the source contract permits them.

### Prevent benchmark corruption

- make benchmark functions `noinline` where appropriate;
- consume results so calls cannot be eliminated;
- inspect assembly;
- verify that inputs are not compile-time constants;
- interleave candidates to reduce temporal bias.

### Input-size matrix

```text
16 B
64 B
256 B
1 KiB
4 KiB
16 KiB
64 KiB
1 MiB
16 MiB
```

The point is to expose distinct operating regimes:

```text
Tiny:
    setup and call latency

Cache-resident:
    instruction quality and vectorization

Medium:
    sustained compute throughput

Large:
    memory bandwidth and prefetching
```

A 1 MiB-only benchmark can hide optimization quality by turning every implementation into the same memory-bandwidth test.

### Input distributions

Each kernel needs multiple data distributions.

#### `all_equal`

At minimum:

```text
All elements equal
Mismatch at first element
Mismatch near beginning
Mismatch in middle
Mismatch at last element
Random mismatch position
No mismatch
```

A scalar early-exit loop may dominate when mismatches occur early. SIMD may dominate when all values must be inspected. Neither implementation is universally faster.

The benchmark reward should reflect the customer's actual input distribution.

#### `count_masked`

Test varying selectivity and predictability:

```text
0% matches
1% matches
25% matches
50% matches
99% matches
100% matches
Alternating/adversarial patterns
Random patterns
```

This distinguishes branch-prediction gains from true arithmetic throughput.

#### `sum_ascii`

Test:

- representative ASCII;
- random bytes if valid;
- boundary values;
- lengths with every possible tail modulo vector width.

### Measurement protocol

Use calibrated iterations so each sample is long enough to rise above timer noise.

Record:

```text
Median
Minimum or best stable estimate
MAD or standard deviation
95% confidence interval
Cycles per byte/element
Throughput
Code size
```

When useful, record hardware counters:

```text
instructions
cycles
branches
branch misses
cache misses
vector instructions
```

Pin to a physical core, warm up, and randomize candidate order.

A credible win should satisfy something like:

```text
95% lower confidence bound exceeds 1.10×
```

on at least one economically relevant workload regime, without unacceptable regressions elsewhere.

Three-times faster would be excellent, but it is not required. A stable 15–30% gain on a sufficiently expensive kernel can support a business.

---

## Experiment 2: Witness frontier

### Kernels

Begin with:

- `k43_sum_ascii`
- `k50_count_masked`
- `k18_all_equal`

But classify them as development kernels afterward. They cannot also be the final held-out evidence.

### Candidate families

For each kernel:

```text
A. Original source
B. Clean conventional scalar
C. Chunked scalar
D. SWAR
E. Compiler builtins
F. LLVM semantic intrinsics where applicable
G. SSE implementation
H. AVX2 implementation
I. AVX-512 implementation where supported
J. Size-specialized/multi-versioned implementation
K. Relevant library implementation, if one exists
```

Not every family applies to every kernel.

For example:

- `sum_ascii` may use vector widening and horizontal summation.
- `count_masked` may use vector comparison, mask extraction, and popcount.
- `all_equal` may use wide comparison and reduction, but early-exit behavior makes workload distribution especially important.

### Correctness contract first

Before benchmarking, define exactly:

```text
Input type and valid lengths
Return type
Overflow behavior
Alignment guarantees
Aliasing guarantees
Endianness assumptions
Whether out-of-bounds wide reads are forbidden
Whether early termination is observable or required
Whether timing/constant-time behavior matters
```

A fast implementation that reads seven bytes past the end or violates strict aliasing is not a valid witness.

For wide scalar loads, avoid accidental undefined behavior. The experiment must distinguish:

- guaranteed alignment;
- unaligned but valid access;
- page-boundary safety;
- legal tails.

### Required output

For every kernel and size/distribution combination:

| Candidate | Correct | Median | Confidence | Cycles/element | Speedup | Notes |
|---|---:|---:|---:|---:|---:|---|
| Clang original | Yes | | | | 1.00× | |
| GCC original | Yes | | | | | |
| Chunked scalar | Yes | | | | | |
| SWAR | Yes | | | | | |
| SSE | Yes | | | | | |
| AVX2 | Yes | | | | | |
| AVX-512 | Yes | | | | | |

Also retain generated assembly for every meaningful result.

---

## Interpreting the witness frontier

### Outcome A: Large known headroom

Example:

```text
Clang baseline: 1.00×
AVX2 witness:   2.40×
```

Conclusion:

```text
The domain contains reachable headroom.
ABSAC needs a semantic path and lowering path to this family.
```

Next step: represent the semantic endpoint and emit vector/intrinsic LLVM IR.

### Outcome B: Headroom only in certain regimes

Example:

```text
16 B: scalar wins
4 KiB: AVX2 wins 1.7×
1 MiB: tie due to memory bandwidth
```

Conclusion:

```text
ABSAC needs guarded or size-specialized multi-versioning.
```

That is itself a valuable semantic optimization.

### Outcome C: Hand candidates tie Clang

Conclusion:

```text
No currently demonstrated headroom in this kernel.
```

Inspect assembly to determine whether Clang already produced the same algorithm. Then:

- deprioritize the kernel;
- search for cross-representation or cross-function opportunities;
- or retain it as a "compiler already optimal" negative example.

It does not disprove the broader project.

### Outcome D: Manual candidate is slower

Conclusion:

```text
The proposed optimization intuition was wrong for this hardware/workload.
```

That is valuable training and cost-model data.

---

## Experiment 3: Target-aware semantic lowering

Once a winning candidate family exists, ABSAC needs an action space capable of reaching it.

### Near-term backend priority

1. LLVM semantic intrinsics (`ctpop`, `ctlz`, `cttz`, rotations, byte swaps).
2. LLVM vector IR.
3. Portable builtins.
4. Explicit target intrinsics where necessary.
5. Handwritten assembly only for experiments or true backend gaps.

ABSAC should win by giving LLVM a better computation, not by prematurely reimplementing LLVM's backend.

### Candidate lowering set

For a population-count reduction, for example:

```text
Semantic operation:
    Cardinality(BitSet)

Lowering candidates:
    scalar loop
    Kernighan loop
    compiler ctpop intrinsic
    table lookup
    SWAR
    scalar POPCNT
    vector popcount
    vector lookup/emulation
    multi-versioned size-dependent implementation
```

The key is not merely adding an emitter special case. The semantic operation should survive far enough down the pipeline that target-aware lowering can make the choice.

### Important qualification about SWAR

"Add SWAR/SIMD" should not itself become dogma. Depending on the CPU:

- scalar `popcnt` may beat SWAR;
- AVX-512 may beat scalar only above certain sizes;
- SIMD setup may lose on small buffers;
- unaligned wide loads may introduce correctness or performance issues;
- large-buffer scans may be memory-bandwidth bound.

The correct candidate set is:

```text
portable scalar
compiler builtin
scalar target instruction
SWAR
SIMD
multi-versioned implementation
```

Then measure the Pareto frontier.

---

## Experiment 4: Search value

### Step 1: Establish reachability

Can ABSAC represent the semantic endpoint?

For example:

```text
CountMasked
→ PredicateMask
→ Cardinality
→ vector compare
→ mask extraction
→ ctpop
```

If not, the missing work is ontology/action/lowering — not search.

### Step 2: Enumerate before beam

If the action graph is small, enumerate every unique state to bounded depth.

Measure:

```text
Average branching factor
Unique canonical states by depth
Percentage of candidates that compile
Percentage that verify
Percentage that are performance-distinct
Frequency of temporary performance valleys
Number of useful multi-step paths
```

This tells us whether search is genuinely needed.

### Step 3: Compare engines

Under equal budgets:

```text
Current deterministic optimizer
Greedy measured selector
Exhaustive bounded search
Current beam scaffold with node-count heuristic
Beam with target-aware static heuristic
Beam with measured terminal evaluation
Random search
```

Only then decide whether MCTS is warranted.

### Search evaluator ablation

Compare candidate ordering by:

```text
SIR node count
Static target cost
Compiled instruction count
Predicted performance
Actual measured performance
```

This will show whether an advanced evaluator is necessary.

### MCTS readiness test

Do not build MCTS unless the experiment shows:

- enough unique states to make enumeration costly;
- delayed rewards;
- interacting transformations;
- meaningful uncertainty in action value;
- beam performance dependent on ordering.

If exhaustive search handles the domain, keep exhaustive search. "Grandmaster" means choosing the right method, not using the most fashionable one.

---

## Experiment 5: Generalization test

The kernels used to design the transformations are the development set.

A separate hidden set must contain:

- different source projects;
- syntactically different implementations;
- different widths and bounds;
- different input distributions;
- multiple similar operations in one function;
- irrelevant surrounding code;
- cases that must not optimize.

A convincing result is:

> ABSAC finds a verified target-aware optimization on a kernel that was not used to construct its recognizer or tune its lowering.

Without this, the system risks becoming benchmark-specific engineering.

Reserve other kernels as hidden evaluation cases.

---

## What not to do during this phase

- Do not build new Arena infrastructure during this experiment.
- Do not build MCTS before measuring the action graph.
- Do not use tests as a substitute for equivalence.
- Do not use one benchmark input as truth.
- Do not claim victory over `-O3` without strong target flags and baselines.
- Do not train and evaluate on the same kernels.
- Do not let scalar C be the only backend.
- Do not let SWAR/SIMD become dogma — measure the Pareto frontier.
- Do not conclude the project is impossible from a null witness result.

---

## Commercial success threshold

ABSAC does not need to beat every compiler on every program. That is not a realistic initial business condition.

It needs to be uniquely valuable in one expensive domain.

A commercially meaningful claim could be:

> On supported bitmap and logical-reduction kernels that remain hot after Clang `-O3`, ABSAC automatically discovers concretely verified implementations delivering a median X% improvement on target hardware.

Or:

> ABSAC finds cross-representation optimizations that conventional compilers cannot perform because they do not recover the required high-level semantics.

Clang is extraordinarily strong at:

- local simplification;
- scalar optimization;
- loop transformation;
- vectorization;
- instruction selection.

The long-term opportunity is not primarily to beat LLVM at those jobs. It is to give LLVM a better algorithm and representation:

```text
Clang:
    Optimize this loop.

ABSAC:
    This is not fundamentally a loop.
    It is finite-set cardinality under a representation contract.
    Replace the representation and all consumers.
    Now lower the resulting semantic operation.
```

That is the defensible advantage.

If a customer spends $10 million annually on a workload and ABSAC safely reduces it by 15%, that is approximately $1.5 million of potential annual value.

What matters is:

```text
large hot path
meaningful verified speedup
safe deployment
low integration cost
repeatability across customers
```

A narrow optimizer that repeatedly saves millions is a company. A universal optimizer that occasionally produces impressive demos but cannot be trusted is not.

---

## Governing decision

The project now has two documents:

### North star

```text
Two-game Arena
Universal construction
Proof-gated commitment
Learning engines
Evolving macro-actions
```

### Immediate research program

```text
Fair benchmark
→ witness frontier
→ identify demonstrated headroom
→ expose winning semantic/target actions
→ concrete verification
→ exhaustive/beam search
→ held-out generalization
→ only then Arena/MCTS
```

That is the right balance between ambition and empirical discipline. The vision remains intact, but each expensive layer must now earn its existence.

The immediate mission is sharp:

> **Find the headroom. Represent the winning implementation. Prove it. Measure it. Then establish whether search — not a hardcoded pass — was necessary to reach it.**
