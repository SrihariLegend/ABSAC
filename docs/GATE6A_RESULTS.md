# Gate 6A — Held-Out Single-Reduction Generalization

## Date: 2025-07-14
## Status: FAILED (false-positive recognition on 3/6 negative cases)

## Objective

Test the frozen system (commit 0948e2f, tag v0.1) on unseen code without
modifying any recognizer or lowering rule. Report:

- Lowered
- Recognized correctly
- False positive
- Candidate generated
- Candidate correct
- Performance improvement
- Abstained safely

## Corpus

16 hand-written kernels in `gate6a/heldout_corpus.c`:

- 8 positive cases (H01-H08): unseen All, Sum, and Cardinality reductions
  with syntactic variations (while loops, different types, reordered
  operands, constants, dead code, pointer arithmetic, inverted predicates)
- 8 negative cases (N01-N08): side effects, volatile, early termination,
  non-standard stride, float, data dependency, null-terminated, search

Compiled with `clang -O1 -emit-llvm -S` to produce unoptimized LLVM IR
that preserves loop structure.

**No recognizer or lowering rule was modified for this test.** The system
frozen at tag v0.1 was run as-is.

## Raw Results

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

## Metrics

```
Lowered:                          10/16  (62.5%)
Verified:                         10/16
Recognized (any truth):           10/10  (100% of lowered)

Positive cases:
  Recognized correctly:             4/6 lowered  (H03, H04, H07, H08)
  Misidentified:                    1/6           (H05 Sum → Cardinality)
  Partial recognition:              1/6           (H06 All, no reduction concept)
  Failed to lower:                  2/8           (H01, H02 type mismatch)

Negative cases:
  Correctly refused:                0/6 lowered   (NONE)
  False positive recognition:       3/6 lowered   (N02, N04, N06)
  Failed to lower (safe):           2/8           (N01 store, N03 GEP)
  Marginal:                         1/6           (N07 strlen, no Cardinality)
  Failed to lower (safe):           1/8           (N05 float, N08 GEP)

Candidate generated:               0/16
Candidate correct:                 N/A
Performance improvement:           0/16
Abstained safely:                  0/16
```

### Most Important Metrics

```
False-positive rewrite rate:      0/0 = N/A  (no rewrites occurred at all)
Held-out recognition rate:        4/8 positives = 50%  (2 failed to lower, 1 misidentified, 1 partial)
Held-out profitable optimization: 0/16 = 0%
False-positive recognition rate:  3/6 lowered negatives = 50%
```

## Root Cause Analysis of False Positives

### N02 (volatile read) — volatile not tracked

The LLVM IR contains `load volatile i8, ptr %9`. The lowerer parses the
`load` instruction but does not parse or preserve the `volatile` keyword.
The resulting SIR Load node is identical to a non-volatile load. The
recognizer then sees an accumulate pattern and fires CardinalityReduction.

**Risk:** If a CardinalityReduction recipe existed and fired, it would
merge volatile loads into a single vector load, violating the volatile
semantics contract. The system is currently safe only because no recipe
fires, not because it correctly refuses.

### N04 (stride 2) — stride not checked

The LLVM IR increments the index by 2: `%14 = add nuw i64 %8, 2`. The
lowerer translates this as a standard loop. The recognizer sees the
accumulate pattern and fires CardinalityReduction without verifying that
the access stride is 1.

**Risk:** If a CardinalityReduction recipe fired, it would vectorize as
a contiguous load, processing `buf[0], buf[1], buf[2], ...` instead of
`buf[0], buf[2], buf[4], ...`. The result would be incorrect.

### N06 (data-dependent accumulators) — independence not checked

The LLVM IR has two accumulators: `sum` (`%13 = add i64 %9, %12`) and
`count` (`%16 = add i64 %8, %15`). The `count` accumulator depends on
`sum > 1000` evaluated on the running sum. The recognizer sees both as
separate CardinalityReduction patterns without checking that the
accumulators are independent.

**Risk:** If both were vectorized independently, `count` would use the
final `sum` value rather than the running value at each iteration,
producing incorrect results.

## Root Cause Analysis of Lowering Failures

### H01, H02 (I32/I64 type mismatch)

The functions use `int` parameters (I32 in LLVM) but the loop counter
is I64. The lowerer's type checking rejects the mixed types. This is a
lowerer limitation, not a recognizer issue.

### N01 (store to output)

The function writes to an output buffer. The lowerer doesn't handle
Store instructions to pointer-typed values. This is a known lowerer gap.

### N03, N08 (GEP index resolution)

The GEP indices involve expressions the lowerer can't resolve. This is
a lowerer limitation with complex index expressions.

### N05 (float phi)

The lowerer doesn't handle float constants in phi nodes. This is a
lowerer limitation with float types.

## What Generalized

The recognizer **does generalize across syntactic variants**:
- While loops vs for loops → both recognized
- Different variable names → recognized
- Reordered operands (`mask & buf` vs `buf & mask`) → recognized
- Constants instead of parameters → recognized
- Dead surrounding code → recognized (H03)
- Different predicates (`> threshold` vs `== target`) → recognized (H08)

The CardinalityReduction recognizer fires on any `accumulate(predicate(buf[i]))`
pattern, regardless of syntactic form. This is good for generalization
but bad for safety (see false positives).

## What Did NOT Generalize

1. **Sum recognition:** H05 (a Sum reduction) is misidentified as
   CardinalityReduction. The recognizer does not distinguish "sum of
   values" from "count of matching predicates." This is the same
   issue seen with k43 in the development set.

2. **All reduction:** H06 (an All reduction) is recognized as IsZero +
   PredicateMap but not as a reducible All pattern. There is no
   All-reduction concept in the ontology.

3. **Safety checks:** The recognizer does not verify:
   - volatile semantics
   - access stride
   - accumulator independence
   - side effects in the loop body

## Honest Assessment

**Gate 6A: FAILED.**

The system has a 50% false-positive recognition rate on held-out
negative cases. While no incorrect rewrite occurred (because no recipes
fired), the recognition layer — which is the gate for all downstream
optimization — is not safe. It recognizes patterns that should be
refused.

The system is currently safe only by accident (no matching recipes),
not by design (correct refusal).

### What This Means

1. **The recognizer generalizes too broadly.** It fires on any
   accumulate pattern without checking safety preconditions.

2. **Adding recipes would be dangerous** until the recognizer gains
   safety checks for volatile, stride, and accumulator independence.

3. **The lowerer loses critical semantic information.** Volatility is
   dropped during lowering, making it impossible for the recognizer
   to check it even if it wanted to.

4. **No held-out optimization occurred.** Even on correctly recognized
   positive cases, the system generated no candidates and performed no
   rewrites. The development kernels (k18, k43, k50) remain the only
   cases where the full pipeline runs end-to-end.

### What Needs to Change Before Gate 6B

1. **Preserve volatile in SIR** — the lowerer must track volatility
   through to the Load node's effects or metadata.

2. **Check stride in recognizer** — CardinalityReduction must verify
   the loop increment is 1 (contiguous access).

3. **Check accumulator independence** — when multiple accumulators
   exist in one loop, the recognizer must verify they don't depend on
   each other.

4. **Distinguish Sum from Cardinality** — the recognizer must check
   whether the accumulated value is the loaded byte (Sum) or a
   predicate of the loaded byte (Cardinality).

5. **Add safe abstention** — the recognizer should explicitly refuse
   patterns that fail safety checks, not silently recognize them.

## Files

- `gate6a/heldout_corpus.c` — 16 held-out kernels (8 positive, 8 negative)
- `gate6a/heldout_corpus.ll` — compiled LLVM IR
