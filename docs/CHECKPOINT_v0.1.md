# ABSAC Semantic Vectorizer v0.1 — Project Checkpoint

> **Historical snapshot** at commit `5ef173d` (2025-07-14). Statuses below
> are as of that commit. Superseded since: Gate 4B PASSED 2026-09-15
> (docs/GATE4B_PROOF.md), H3/D4 remediation completed 2026-09-03/12
> (docs/H3_RESULTS.md, d4/README.md). Current gate table:
> docs/IMMEDIATE_PROGRAM.md.

## Date: 2025-07-14
## Commit: 5ef173d

## Capabilities Demonstrated

```
Three semantic reductions (All, Sum, Cardinality)
    → recovered from real LLVM IR
    → lowered to target-aware AVX2 implementations

Target-plan enumeration
    → 69 valid plans across ISA × reduction × unroll × tail × control
    → deterministic selector within 0-14% of measured best

Near-expert AVX2 lowering
    → 4.5-7.8× faster than Clang baseline at 4096 bytes
    → within 10% of best tested hand-written implementations

Large gains over compiler baselines
    → Clang: 4.5-7.8× (narrow vectors + lane accumulation)
    → GCC: 7-13× (wide vectors + less efficient reduction structure)
    → Root cause: semantic algorithm selection, not just wider vectors

Strong adversarial validation
    → 16,869,913 differential tests (all passing)
    → ASan + UBSan clean
    → Guard-page over-read detection
    → Exhaustive byte/mask/target combinations
    → All alignment offsets 0-31

Cross-reduction fusion witness
    → 28-35% improvement over independent vectorization
    → One shared vector load vs three
    → No single recipe contains this composition
```

## Limitations (Honest)

```
Machine-checked end-to-end proof:     OPEN (Gate 4B)
    Six lemmas are human arguments + testing, not solver-verified.

Fusion automation:                    OPEN (Gate 5B-Automation)
    Fusion was hand-constructed, not discovered by search.

Search contribution to composition:   OPEN (Gate 5B-Search)
    Search has not yet been shown to discover fusion.

Held-out generalization:              OPEN (Gate 6)
    Current 3 kernels are development cases.

Platform scope:                       x86-64 AVX2 only
    No AVX-512, no ARM SVE, no GPU.

Corpus scope:                         3 kernels, 50 in corpus
    40/50 lowered, 3 deeply tested.
```

## Gate Status

```
Gate 1:   PASSED   Benchmark credibility
Gate 2:   PASSED   Witness headroom (4-10× over clang -O3)
Gate 3:   PASSED   Expressive reachability (3 distinct reductions)
Gate 4A:  PASSED   Compositional argument + 16.8M differential tests
Gate 4B:  OPEN     Mechanically checked equivalence
Gate 5A:  COMPLETE Target-plan landscape (69 plans, E0 within 0-14%)
Gate 5B-W:  PASSED Hand-constructed fusion witness (28-35%)
Gate 5B-A:  OPEN   Automated fusion from primitive actions
Gate 5B-S:  OPEN   Search discovers fusion
Gate 6A-v0:  FAILED   50% false-positive recognition on held-out negatives
                     (3/6 — preserved in GATE6A_RESULTS.md)
Gate 6A-Rem: PASSED   on regression corpus only (0/6 — not held-out proof)
Gate 6A-v1:  FAILED   fresh blind corpus: 1/4 false positive (N15
                     certificate gap), 1 frontend soundness bug (V07),
                     2/8 positives lost to lowering, All-reduction miss
Gate 6A-v2:  OPEN     fresh corpus after next remediation
Gate 6B:   RESERVED  uncontaminated fusion corpus
```

## Revised Next Sequence

```
1.  Freeze current checkpoint and evidence.                    [DONE]
2.  Run Gate 6A on unseen single-reduction kernels.            [NEXT]
3.  Define generic reduction signatures.
4.  Automate legality-aware fusion.
5.  Enumerate fusion partitions.
6.  Reproduce hand-written fusion benefit automatically.
7.  Determine whether search beats deterministic fusion.
8.  Continue Gate 4B machine verification.
9.  Run Gate 6B on pre-frozen unseen fusion opportunities.
10. Reconsider Arena/MCTS only from measured composition graph.
```

## Honest Public Claim

> On three development kernels, ABSAC recovered reduction semantics
> from LLVM IR and selected target-aware AVX2 implementations that
> were 4.5–7.8× faster than the Clang baseline at 4096 bytes and
> within 10% of the best tested hand-written implementations.
> Additionally, hand-constructed cross-reduction fusion provides
> 28–35% improvement beyond independent vectorization.

## Key Insight

The performance gap is not primarily about vector width. GCC uses
full 32-byte vectors but is still 7-13× slower than ABSAC. The gap
is about **semantic algorithm selection** — mapping each reduction
type to its best observed target instruction:

```
Cardinality → movemask + popcount     (not vector AND tower)
Sum         → psadbw                   (not zero-extend + add)
All         → movemask + full-mask test (not lane-wise AND)
```

This is the core ABSAC thesis validated: semantic knowledge selects
a better reduction structure, not merely a larger vector width.

## Second Layer

Cross-reduction semantic fusion provides a second optimization level
beyond per-loop vectorization:

```
Level 1: Optimize each reduction separately
Level 2: Recognize shared traversals, fuse into one pass
```

This is where conventional local compiler pipelines struggle, and
where ABSAC's semantic view can make fusion legality and profitability
explicit.
