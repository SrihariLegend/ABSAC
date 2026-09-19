# Gate 6A-v1 — Fresh Blind Evaluation (Generation H1)

## Date: 2026-09-01
## Frozen candidate: commit bed13c0 (post-remediation)

## Protocol Compliance

```
1. Corpus selected and classified BEFORE running ABSAC.        ✓ (archive_v1.sh)
2. Source + classification + SHA256 archived before the run.   ✓
   v1_corpus.c:  405f30d95925021c6890c9f368066ca7bce51c71e6b897d2e8cecf2246962bcc
   v1_corpus.ll: 9b7f9560af6ce24087b7650084422ba0ebbcad789685bb24fac8d6ab10ad5f31
3. Frozen candidate run ONCE.                                  ✓ (bed13c0, no modifications)
4. Raw results frozen before failure inspection.               ✓ (/tmp snapshot + below)
5. Failures promote this corpus to regression set D2.          ✓ (this document)
6. This corpus will NEVER be reused as held-out proof.
```

## Corpus (Generation H1)

16 fresh kernels, 8 positive / 8 negative, classified before the run:

```
V01  POSITIVE  Cardinality, do-while, signed acc, llvm.smax bound clamp
V02   POSITIVE  Sum, u16 elements, u32 accumulator and induction
V03   POSITIVE  All, i64 elements, &= form
V04   POSITIVE  Cardinality, Gt predicate, u32 elements
V05   POSITIVE  Sum, signed char elements, i64 accumulator
V06   POSITIVE  Cardinality, != predicate, while (i != n) form
V07   POSITIVE  Cardinality + Sum in two separate loops (fusion shape)
V08   POSITIVE  Sum, non-zero loop start
N09   NEGATIVE  volatile loads
N10   NEGATIVE  aliasing store inside loop
N11   NEGATIVE  reverse traversal (decrementing induction)
N12   NEGATIVE  opaque call inside loop
N13   NEGATIVE  dynamic stride (runtime step)
N14   NEGATIVE  conditional/saturating accumulator
N15   NEGATIVE  two-array relation a[i]==b[i]
N16   NEGATIVE  signed i8 sum, signed overflow semantics
```

## Raw Results (frozen — batch_run output, verbatim structure)

```
Kernel                         Lowered  Verify   Truths  Error
─────────────────────────────────────────────────────────────────────
v01_count_matches_do_while     NO       -        0       unsupported call: llvm.smax.i32
v02_sum_u16                    NO       -        0       cannot resolve rhs '%5' (icmp)
v03_all_same_i64               YES      PASS     3       -
v04_count_above_u32            YES      PASS     4       -
v05_sum_signed                 YES      PASS     2       -
v06_count_not_equal            YES      PASS     4       -
v07_count_then_sum             YES      FAIL     5       (verification failure!)
v08_sum_bounded                YES      PASS     2       -
n09_atomic_load                YES      PASS     3       -
n10_store_alias                NO       -        0       TypeMismatch (store)
n11_reverse_sum                YES      PASS     3       -
n12_count_with_call            NO       -        0       cannot resolve return value
n13_dynamic_stride             YES      PASS     2       -
n14_saturating_count           NO       -        0       cannot resolve select false
n15_count_equal_pairs          YES      PASS     4       -
n16_sum_signed_nsw             NO       -        0       cannot resolve rhs '%5'

Summary: 10/16 lowered, 9/16 verified, 10/16 recognized, 0/16 rewrote
Concepts: CardinalityReduction ×4, SumReduction ×2, IsZero ×8, PredicateMap ×10
```

## Recognition Detail (frozen probe output)

```
v03_all_same_i64    (POS, All):      NO reduction truth       → recall miss
v04_count_above_u32 (POS):           CardinalityReduction     → ✓ correct
v05_sum_signed      (POS, Sum):      SumReduction             → ✓ correct
v06_count_not_equal (POS):           CardinalityReduction     → ✓ correct
v07_count_then_sum  (POS, 2 loops):  CardinalityReduction on
                    count loop; VERIFICATION FAILED on the function → frontend bug
v08_sum_bounded     (POS, Sum):      SumReduction             → ✓ correct
n09_atomic-like     (NEG, volatile): no reduction truth       → ✓ refused
n11_reverse_sum     (NEG):           no reduction truth       → ✓ refused
n13_dynamic_stride  (NEG):           no reduction truth       → ✓ refused
n15_equal_pairs     (NEG?):          CardinalityReduction FIRED → see analysis
```

## v1 Metrics (by pipeline stage)

### Positive cases (8 input)

```
Input positives:                        8
Successfully lowered:                   6/8   (V01 llvm.smax, V02 icmp rhs)
Verified:                               5/6   (V07 fails verification!)
Reduction recognized among lowered:     5/6   (V03 All missed — no AllReduction concept)
  Correct concept:                      4/5   (V04, V06 Cardinality; V05, V08 Sum)
End-to-end positive coverage:           4/8 = 50%
```

### Negative cases — containment layers

```
Lowered:                                4/8   (N10, N12, N14, N16 failed to lower — safe by frontend gap)
False semantic recognition:             1/4   (N15 — see analysis)
Unsafe candidate generated:             0/1   (no recipes fire)
Unsafe candidate selected:              0/0
Unsafe rewrite accepted:                0/0
Runtime mismatch:                       0/0
```

### Headline v1 metrics

```
False-positive recognition rate:   1/4 lowered negatives = 25%
  (0/6 → 1/4 across generations; sample remains far too small to
   establish a rate — rule of three on 4 observations permits ~53%)
Positive recall on lowered inputs: 4/6 = 67%  (V03 All miss, V07 lowered-but-broken)
End-to-end positive coverage:      4/8 = 50%
```

## Failure Analysis

### N15 (two-array relation) — NEW false positive, certificate gap

`same += (a[i] == b[i])` IS mathematically a cardinality reduction of the
predicate `a[i] == b[i]` over the index domain. The recognition is
semantically TRUE. The problem is that the emitted truth carries a single
input ValueId with no description of the second memory region. A target
emitter that assumes "one input buffer" (as the current Gate 3 pipeline
does) would lower this incorrectly or unsafely.

**Classification:** the expected classification in the archive said "must
NOT fire as CardinalityReduction on a alone." The recognizer did not fire
"on a alone" — it fired on the two-input predicate. This is a
**certificate-completeness failure**, not a semantic falsehood: the truth
is real but incomplete. Under the ReductionCertificate design, this
failure mode is exactly what the `input regions` field exists to prevent:
a certificate listing both memory regions would either be complete (and
safe to lower with a two-input plan) or would not be issued.

**Verdict: false positive by the standard we set (recognition must carry
everything the downstream planner needs to be safe). Counted as a false
positive.**

### V07 (two loops in one function) — frontend soundness bug

The lowerer produced SIR that FAILS the SIR verifier for a
two-loop function. The verify layer caught it (defense in depth worked —
an invalid IR never reached recognition-based rewriting), but this is a
soundness bug in the frontend: a function with two sequential loops
produces structurally invalid SIR.

### V03 (All reduction, i64, &= form) — recall miss

No AllReduction concept exists in the ontology. Recognized only as
IsZero + PredicateMap fragments. Known gap, now confirmed on a second
independent corpus (H06 in D1, V03 in H1).

### V01 (llvm.smax bound clamp) — frontend gap

`do { } while` loops with a `llvm.smax` bound clamp are not lowered.
Note: clang inserted `smax(i32 %1, 1)` to normalize the do-while
iteration domain — this is a **loop bound representation** the frontend
must eventually handle.

### V02, N16 (icmp rhs resolution) — i32/i64 mixed-width comparisons

Same lowerer limitation as H01/H02 in D1: mixed-width comparisons
(`icmp eq i64 %15, %5` where %5 is a truncated/sexted parameter) are
not resolved. This is now the dominant lowering gap — it blocked 2/8
positives in H1 and 2/8 positives in H0.

### N09 (volatile), N11 (reverse), N13 (dynamic stride) — all correctly refused

The volatile effect tracking and stride checks generalized to three
new negative variants they were not written for. This is encouraging
evidence (not proof) that the fixes are principled rather than
case-fitted: the volatile check fired on `volatile` qualifier again;
the stride check rejected both a decrementing induction (N11) and a
runtime-parameter stride (N13) via the constant-1 requirement.

Note on N09: the source used `volatile` (compiled without atomics).
True atomic accesses (`llvm.atomic.load`) are NOT yet tested — the
lowerer's behavior on them is unknown and must be probed in H2.

## Updated Generational Status

```
Gate 6A-v0:          FAILED   3/6 false positives (preserved above)
Gate 6A-Remediation: PASSED   on regression corpus D1 (0/6)
Gate 6A-v1:          FAILED   1/4 lowered negatives false-recognized (N15)
                     + 1 frontend soundness bug (V07 verify failure)
                     + 2/8 positive recall lost to lowering (V01, V02)
                     + All-reduction recall miss (V03)
Gate 6A-v2:          OPEN     requires fresh corpus after next remediation
```

**Progress across generations:**

```
Generation   Positives recognized   False positives
H0 (v0.1)    4/6 lowered            3/6 = 50%
H1 (v1)      4/6 lowered            1/4 = 25%
```

The false-positive rate is improving generation over generation, but both
samples are far too small for statistical claims (rule of three: 0/4 in
H1's lowered negatives still permits ~53% underlying rate; 1/4 observed).
The trend is encouraging; the rate is not established.

## What H1 Exposed That H0 Did Not

1. **Certificate incompleteness (N15):** a recognition can be semantically
   true yet unsafe to act on if the truth omits part of the memory
   footprint. This is the strongest argument yet for the
   ReductionCertificate: recognition must be complete, not just correct.

2. **Frontend soundness (V07):** the lowerer can produce SIR that fails
   the verifier. The verify layer contained it (no recognition pipeline
   ran on broken IR for that function's second loop), but a frontend
   that emits invalid IR is a correctness risk independent of
   recognition.

3. **llvm.smax loop-bound normalization (V01):** real-world loops arrive
   canonicalized by clang; the lowerer must handle intrinsic-dominated
   entry guards. This confirms that corpus H0's "clean" loop shapes were
   unrepresentative.

4. **Mixed-width induction (V02, N16 — same root cause as H01/H02):**
   mixed i32/i64 comparisons remain the top lowering gap. Two
   independent corpora have now failed on the same frontend defect.

## Containment Verdict (defense in depth)

```
Layer                       H0              H1
────────────────────────────────────────────────────
Frontend refuses (gap)      2/8 negs        6/16 negs
Recognizer refuses          3/6 lowered     3/4 lowered
Recipe layer (absence)      contained all   contained all
Verification layer          N/A             caught V07 invalid IR
Runtime mismatch            0               0
```

The verification layer demonstrably caught malformed IR (V07) — the first
observed case of a containment layer below recognition doing real work.

## Remediation Queue (from H1, to become regression set D2)

1. N15: two-input reduction predicates must either be recognized
   completely (both regions in the certificate) or not at all.
2. V07: fix the two-loop lowering soundness bug (verifier failure).
3. V01: support llvm.smax entry guards (do-while normalization).
4. V02/N16: mixed-width icmp operand resolution (third corpus in a row
   hits the i32/i64 gap — highest-frequency frontend defect).
5. V03: AllReduction concept (still missing after two generations).

## Sample-Size Statement (mandatory)

Both H0 (6 lowered negatives) and H1 (4 lowered negatives) are far too
small to bound the false-positive rate statistically. The generational
protocol exists to keep exposing omissions, not to certify safety.
Correctness for production must ultimately come from machine-checked
certificates (Gate 4B) plus complete certificates (Part 3 of the v0
document), with adversarial corpora serving as omission-finders only.

## Files

- `gate6a/v1_corpus.c` / `.ll` — H1 corpus (now promotes to regression set D2)
- `gate6a/archive_v1.sh` — pre-run classification + hash archive
