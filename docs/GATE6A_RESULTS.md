# Gate 6A — Held-Out Single-Reduction Generalization

## Date: 2025-07-14 (v0) — see GATE6A_V1_RESULTS.md for the H1 generation

## Status Summary

```
Gate 6A-v0:          FAILED  on frozen v0.1 (tag 0948e2f)
                     3/6 held-out negatives falsely recognized

Gate 6A-Remediation: PASSED  on the now-known regression corpus D1
                     0/6 false positives, 6/8 positive coverage
                     THIS IS NOT HELD-OUT PROOF — the corpus was
                     inspected and the code changed in response

Gate 6A-v1:          FAILED  fresh blind corpus (see GATE6A_V1_RESULTS.md)
                     1/4 false positive + 1 frontend soundness bug

Gate 6A-v2:          OPEN    requires a fresh untouched corpus
```

---

# Part 1: Gate 6A-v0 — Blind Evaluation of Frozen v0.1

## Objective

Test the frozen system (commit 0948e2f, tag v0.1) on unseen code without
modifying any recognizer or lowering rule.

## Corpus

16 hand-written kernels in `gate6a/heldout_corpus.c`:

- 8 positive cases (H01-H08): unseen All, Sum, and Cardinality reductions
  with syntactic variations (while loops, different types, reordered
  operands, constants, dead code, pointer arithmetic, inverted predicates)
- 8 negative cases (N01-N08): side effects, volatile, early termination,
  non-standard stride, float, data dependency, null-terminated, search

Compiled with `clang -O1 -emit-llvm -S`. The system at tag v0.1 was run
as-is, blind, with no recognizer modified.

## Raw Results (v0.1, frozen)

```
Kernel                     Lowered  Recognized             Rewrite
──────────────────────────────────────────────────────────────────
h01_sum_while              NO       -                      -       (type mismatch I32/I64)
h02_all_match              NO       -                      -       (type mismatch I32/I64)
h03_count_nonzero          YES      CardinalityReduction   -       ✓ correct
h04_count_masked_reversed  YES      CardinalityReduction   -       ✓ correct
h05_sum_int32              YES      CardinalityReduction   -       ✗ MISIDENTIFIED (Sum → Cardinality)
h06_all_zero               YES      IsZero, PredicateMap   -       partial (no All-reduction concept)
h07_count_zero             YES      CardinalityReduction   -       ✓ correct
h08_count_above            YES      CardinalityReduction   -       ✓ correct
n01_count_with_write       NO       -                      -       (store pointer type)
n02_volatile_read          YES      CardinalityReduction   -       ✗ FALSE POSITIVE
n03_early_terminate        NO       -                      -       (GEP index)
n04_stride2                YES      CardinalityReduction   -       ✗ FALSE POSITIVE
n05_fsum                   NO       -                      -       (float phi)
n06_dependent              YES      CardinalityReduction   -       ✗ FALSE POSITIVE
n07_strlen                 YES      IsZero, PredicateMap   -       marginal (no CardinalityReduction)
n08_find_first_mismatch    NO       -                      -       (GEP index)
```

## v0 Metrics (by pipeline stage)

### Positive cases (8 input, 6 lowered)

```
Input positives:                       8
Successfully lowered:                  6/8
Correctly recognized among lowered:    4/6  (H05 misidentified, H06 partial)
End-to-end positive coverage:          4/8 = 50%
```

### Negative cases — containment layers

```
Lowered:                               6/8  (2 failed to lower — safe, but by frontend gap)
False semantic recognition:            3/6  (N02 volatile, N04 stride2, N06 dependent)
Unsafe candidate generated:            0/3  (no recipes fired — containment by absence)
Unsafe candidate selected:             0/0
Unsafe rewrite accepted:               0/0
Runtime mismatch:                      0/0
```

**A false semantic truth is serious, but downstream verification may still
prevent it from becoming an accepted rewrite.** In v0, the only functioning
containment layer was recipe absence — no applicable recipe existed. That
is containment by accident, not defense in depth by design.

### Headline v0 metrics

```
False-positive recognition rate:   3/6 lowered negatives = 50%
Held-out recognition rate:         4/8 positives = 50%
Held-out profitable optimization:  0/16 = 0%
False-positive rewrite rate:       0/0 = N/A (no rewrites at all)
Abstained safely (explicitly):     0/16 — the system never abstains by design;
                                   it either recognizes or fails to lower
```

## Root Cause Analysis of the Three False Positives

### N02 (volatile read) — volatile not tracked

The LLVM IR contains `load volatile i8, ptr %9`. The lowerer parses the
`load` instruction but does not parse or preserve the `volatile` keyword.
The resulting SIR node is identical to a non-volatile load. The recognizer
sees an accumulate pattern and fires CardinalityReduction.

**Risk:** A CardinalityReduction recipe would merge volatile loads into a
single vector load, violating volatile semantics.

### N04 (stride 2) — stride not checked

The LLVM IR increments the index by 2 (`add nuw i64 %8, 2`). The recognizer
fires CardinalityReduction without verifying that the access stride is 1.

**Risk:** Vectorizing as a contiguous load would process `buf[0], buf[1], ...`
instead of `buf[0], buf[2], buf[4], ...` — wrong results.

### N06 (data-dependent accumulators) — independence not checked

Two accumulators: `sum` and `count`, where `count += (sum > 1000)` depends
on the running sum. The recognizer sees both as separate reductions without
checking accumulator independence.

**Risk:** Independent vectorization would compute `count` from the final
`sum` rather than the running value — wrong results.

## v0 Verdict

**Gate 6A-v0: FAILED.**

The recognition layer — the gate for all downstream optimization — was
not safe. The system was safe only by accident (no matching recipes),
not by design (correct refusal). This failure arrived before automated
fusion exists, which is the best possible time to discover that
semantic recognition is the primary correctness risk.

---

# Part 2: Gate 6A-Remediation

## Sample-Size Caution

`0/6` false positives after remediation means:

> No false positives were observed in these six cases.

It does NOT establish a generally low false-positive rate. Under the
"rule of three," zero failures in six observations still permits an
underlying failure rate as high as ~39% at 95% confidence. These six
cases are now a regression corpus, not evidence of general safety.
Larger adversarial corpora (Gate 6A-v1+) are required to probe for
remaining omissions in the semantic model.

## Fixes Applied

1. **VOLATILE effect added to SIR** — the lowerer detects `load volatile`
   and marks the node with `Effects::VOLATILE`. The recognizer rejects any
   loop containing volatile nodes.

2. **Stride check** — the recognizer verifies the loop counter's invariant
   value is a constant 1. Non-unit strides are rejected.

3. **Accumulator independence check** — `transitive_inputs` verifies that
   no accumulator's invariant value depends on another accumulator's
   variable.

4. **Sum vs Cardinality distinction** — the recognizer checks whether the
   accumulated value is a boolean predicate. Raw-value accumulation is
   no longer misidentified as CardinalityReduction.

5. **SumReduction recognizer added** — k43 and H05 now correctly
   recognized as SumReduction.

## Remediation Results (on the now-known regression corpus)

```
Kernel                     v0 Recognition         Post-fix Recognition
─────────────────────────────────────────────────────────────────────────
h03_count_nonzero          CardinalityReduction   CardinalityReduction   ✓
h04_count_masked_reversed  CardinalityReduction   CardinalityReduction   ✓
h05_sum_int32              CardinalityReduction   SumReduction           ✓ fixed
h06_all_zero               IsZero+PredicateMap    IsZero+PredicateMap    partial (no All concept)
h07_count_zero             CardinalityReduction   CardinalityReduction   ✓
h08_count_above            CardinalityReduction   CardinalityReduction   ✓
n02_volatile_read          CardinalityReduction   (none)                 ✓ fixed
n04_stride2                CardinalityReduction   (none)                 ✓ fixed
n06_dependent              CardinalityReduction   (none)                 ✓ fixed
n07_strlen                 IsZero+PredicateMap    IsZero+PredicateMap    marginal (unchanged)
```

## Remediation Metrics (by pipeline stage)

```
Input positives:                        8
Successfully lowered:                   6/8   (H01, H02 fail on I32/I64 type mismatch)
Correctly recognized among lowered:     6/6   (100% recall on visible inputs)
End-to-end positive coverage:           6/8 = 75%

The recognizer has 100% recall on inputs it can see.
The complete compiler currently has 75% coverage.
The remaining limitation is frontend/lowering support, not recognition.
```

```
Negative containment (regression corpus):
  False semantic recognition:      0/6
  Unsafe candidate generated:      0/6  (no recipes fire)
  Unsafe candidate selected:       0/0
  Unsafe rewrite accepted:         0/0
  Runtime mismatch:                0/0
```

## Honest Assessment of the Four Fixes

### 1. Volatile effects — correct but incomplete

Preserving `load volatile` and rejecting it is correct. The larger lesson:
effects must be semantic, not incidental metadata. Not yet covered:

```
volatile stores
atomic loads and stores
unknown calls
I/O-like effects
loads that may trap
ordered memory operations
```

### 2. Contiguous stride — safe but conservative

Requiring increment-by-exactly-one is a safe conservative restriction.
It is acceptable to miss valid variants; it is not acceptable to classify
an unsupported traversal as contiguous. Not yet handled:

```
negative stride, dynamic stride
two-dimensional indexing
pointer increments
equivalent canonicalized forms of +1
integer wraparound in the induction variable
```

### 3. Accumulator independence — SSA-only

`transitive_inputs` captures SSA data dependencies but NOT dependencies
through memory. A loop where an accumulator depends on a load that
aliases a store inside the loop is not adequately characterized by
graph reachability. The long-term eligibility condition must include
memory dependencies, aliasing, loop-carried memory state, effects, and
traps.

### 4. Sum vs Cardinality — correct but should be structured

The distinction is semantically important. The recognizer should
identify what values are reduced, what map/predicate produces them,
what accumulator type is used, and what overflow behavior applies.
That information belongs in a structured reduction certificate (below),
not in ad-hoc recognizer checks.

## Development Kernel Impact

- **k43_sum_ascii**: was misidentified as CardinalityReduction since the
  beginning (a latent error the development corpus never exposed).
  Now correctly recognized as SumReduction.
- **k50_count_masked**: unchanged, correctly CardinalityReduction.
- **k18_all_equal**: unchanged (All reduction; no AllReduction concept yet).

---

# Part 3: Reduction Eligibility Certificate (design direction)

The four fixes must not remain a growing collection of independent
recognizer checks. They should converge into an explicit object
describing why a transformation is legal:

```
ReductionCertificate
├── region identity
├── iteration domain
│   ├── start
│   ├── length
│   ├── stride
│   └── induction semantics (wraparound, nuw/nsw)
├── input regions
├── memory effects
├── aliasing conditions
├── map/predicate semantics
├── reduction operation (All / Sum / Cardinality / ...)
├── identity
├── accumulator recurrence
├── accumulator type
├── overflow semantics
├── output binding
├── purity/trap conditions
└── provenance for every fact
```

Design rules:

- The target planner must consume this certificate rather than
  independently assuming that a recognized loop is safe.
- For fusion, several certificates must be proven compatible
  (aligned domains, compatible effects, non-interfering memory).
- Every field carries provenance — which fact or check established it.

This is not yet implemented. The current four checks are its first
partial instances, embedded in recognizer code. Formalizing the
certificate is the next architectural step after Gate 6A-v1.

---

# Part 4: Gate 6A-v1 Protocol (fresh blind corpus)

Before further recognizer tuning, the post-remediation commit is frozen
and evaluated on a NEW corpus.

## Positive categories

Unseen forms of All, Sum, Cardinality with variations in: loop shape,
induction representation, constants vs parameters, predicate orientation,
comparison type, integer width, accumulator width, loop bound
representation, surrounding dead code, multiple loops per function,
different source-project style.

## Negative categories

```
volatile and atomic access
non-unit and dynamic strides
reverse traversal
aliasing stores
loop-carried memory dependencies
data-dependent accumulator updates
side-effecting calls
possible traps
signed overflow differences
early-exit side effects
non-contiguous access
multiple arrays with related indices
lookalike reductions with different semantics
```

## Blindness protocol

1. Select and classify the corpus BEFORE running ABSAC.
2. Archive source, expected classification, and hashes.
3. Run the frozen candidate ONCE.
4. Freeze the complete result before inspecting failures.
5. Promote failures into the development corpus.
6. Never reuse that corpus as proof of held-out generalization.
7. Use a new corpus for the next generalization claim.

This creates advancing test-frontier generations:

```
H0 (this corpus) exposed failures → became regression set D1
H1 will expose new failures       → will become regression set D2
H2 ...
```

---

# Files

- `gate6a/heldout_corpus.c` — the v0 corpus (now a regression set, D1)
- `gate6a/heldout_corpus.ll` — compiled LLVM IR

## Historical record preserved

The v0 failure (3/6 false positives, 50%) is documented above in full
and must remain prominent. It is not to be overwritten by the remediation
result. The remediation result holds only on the regression corpus.
