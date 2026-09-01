# Gate 6A-v2 — Fresh Blind Evaluation (Generation H2)

## Date: 2026-09-01
## Frozen candidate C2: commit 5355778

## Protocol Compliance

```
1. Corpus selected and classified BEFORE running ABSAC.        ✓ (archive_v2.sh)
2. Source + classification + SHA256 archived before the run.   ✓
   v2_corpus.c:  9e548116f1133becb548a69ca7e968ffa1ef4dd1aed794434fcd01ef3f112157
   v2_corpus.ll: 1cf1059c0542d78945fbd47fc07fe1dc0432dd792a3edb86018748ac460e59c0
3. Frozen candidate run ONCE (5355778, unmodified).            ✓
4. Raw results frozen before failure inspection.               ✓ (/tmp snapshot)
5. Failures promote this corpus to regression set D3.          ✓
6. This corpus will NEVER be reused as held-out proof.
```

## Corpus (Generation H2) — metamorphic near-miss design

Each negative deliberately differs from a valid positive by one semantic
property. Classified and hashed before the run (archive_v2.sh output).

## Raw Results (frozen)

```
Kernel                  Lowered Verify  Reduction truth        Assessment
───────────────────────────────────────────────────────────────────────────
w01_count_hits_i16      YES     PASS    CardinalityReduction   ✓ correct
w02_sum_u32             YES     PASS    SumReduction           ✓ correct
w03_sum_plus_one        YES     PASS    (none)                 ✗ recall miss (map-then-sum)
w04_all_equal_i8        YES     PASS    ConjunctiveReduction   ✓ correct (new All recognizer)
w05_count_positive      YES     PASS    CardinalityReduction   ✓ correct (signed Gt)
w06_count_mismatch_const YES    PASS    CardinalityReduction   ✓ correct (const bound, Ne)
w07_all_min             NO      -       -                      frontend: u8/I64 mismatch
w08_two_reductions      NO      -       -                      soundness gate (two loops, known)
x01_volatile_store      NO      -       -                      safe abstain (store gap)
x02_sum_i32_nsw         YES     PASS    SumReduction           ✗ FALSE POSITIVE (overflow)
x03_sliding_sum         YES     PASS    (none)                 ✓ refused
x04_stride4             YES     PASS    (none)                 ✓ refused
x05_atomic_flags        NO      -       -                      safe abstain (atomic load unhandled)
x06_running_max         YES     PASS    (IsZero, PredicateMap) see analysis — candidate fired!
x07_self_recurrence     YES     PASS    (none)                 ✓ refused
x08_early_exit_write    NO      -       -                      safe abstain (gep gap)

Summary: 11/16 lowered, 11/16 verified, 0/16 rewrote
```

## v2 Metrics (by pipeline stage)

### Positive cases (8 input)

```
Input positives:                        8
Successfully lowered:                   6/8   (W07 u8/I64 mismatch; W08 two loops)
Reduction recognized among lowered:     5/6   (W03 map-then-sum missed)
End-to-end positive coverage:           5/8 = 62.5%
  W01 Cardinality ✓  W02 Sum ✓  W04 All ✓  W05 Cardinality ✓  W06 Cardinality ✓
  W03 ✗ (no map-then-sum)  W07 lowering  W08 two-loop soundness gate
```

### Negative cases — containment layers

```
Lowered:                                7/8   (X01, X05, X08 refused by frontend)
False semantic recognition:             1/7   (X02 — signed overflow nsw)
Unsafe candidate generated:             1     (X06 running max → BitScanForward candidate!)
Unsafe candidate selected:              0     (rewrite failed: MissingRole)
Unsafe rewrite accepted:                0
Runtime mismatch:                       0
```

### Containment analysis — the most important H2 finding

**X06 (running max): the FIRST unsafe candidate generation.** The max
loop was recognized as IsZero + PredicateMap, from which the inference
layer derived a PositionSearch belief and the generator produced a
BitScanForward candidate. The rewrite failed with `MissingRole {
role: "collection" }` — a role-completeness failure, NOT a semantic
refusal. Had the roles matched, the rewrite layer would have been the
only remaining containment.

This is the exact escalation the advisor's terminology predicts:
Recognition (PredicateMap is true — max IS computed via a predicate)
escalated to a Belief and a Candidate without a certificate check.
The candidate was contained by the rewrite layer failing for
accidental (role) reasons — the same "safe by absence" weakness the
v0 evaluation exposed at the recognition layer, now observed one
layer deeper.

**X02 (signed overflow): the second certificate gap.** `s += buf[i]`
with signed i32 accumulators is attributed nsw by the frontend; vector
accumulation changes overflow behavior. SIR models only Wrapping
overflow. The recognizer fired SumReduction because the certificate
has no overflow-semantics field yet — precisely the
"RecursionCertificate completeness" failure mode predicted by the
advisor. The fix is architectural (overflow field in the certificate),
not another ad-hoc check.

### Correctly refused (generalized again)

```
X03 sliding window (buf[i] + buf[i-1]): refused — reduction facts
    correctly see the accumulator's invariant is not a simple element
    value. (Two access functions on one base: the footprint is one
    base, but the pattern did not reduce — correct.)
X04 fixed stride 4: refused (stride check generalizes from dynamic
    stride to constant non-unit stride)
X07 self-referential recurrence: refused
X01 volatile store: refused at frontend (store parsing gap — safe but
    for the wrong reason; volatile STORE handling still untested)
X05 C11 atomic loads: refused by frontend (load atomic unsupported —
    the first probe of atomics; refusal is safe but frontend cannot
    yet distinguish atomic from plain loads structurally)
```

## Generational History

```
Generation   Negatives reaching recognition   False positives
H0 (v0.1)    6                                3  (50%)
H1           4                                1  (25%)
H2           7                                1  (~14%)

Positives recognized (of lowered):
H0: 4/6   H1: 4/6 (V07 lowered-but-broken)   H2: 5/6
```

The trend continues to improve, and every generation kept the sample
too small for statistical claims (rule of three: 1/7 observed still
permits ~56% underlying rate at 95% confidence). The defensible
qualitative statement remains:

> The H0 remediations generalized to unseen volatile, reverse-traversal
> and dynamic-stride negatives. The H1 remediations generalized to
> stride-4 and self-recurrence negatives. H2 exposed two new failure
> classes: overflow semantics (X02) and unguarded candidate generation
> from partial truths (X06).

## H2's Two New Failure Modes

1. **X02 — overflow semantics gap (certificate field):** SIR models
   Wrapping overflow only. Signed nsw accumulation cannot be authorized
   for vector reduction without overflow-semantics analysis. The
   ReductionCertificate's `overflow semantics` field (forecast in the
   v0 design) is now justified by a witness, not speculation.

2. **X06 — candidate generation without a certificate.** The
   PositionSearch path produced a candidate from PredicateMap + IsZero
   truths alone. The rewrite failed for structural reasons (missing
   role), not because the system recognized the semantic mismatch.
   This is Priority 0B applied to the SECOND recognizer family:
   every candidate generator must consume complete certificates.
   (Currently only reduction recognizers consult the footprint check.)

## W03 (map-then-sum) recall miss

`total += buf[i] + 1` — the accumulated invariant is add(load, 1). The
Sum recognizer's predicate check classifies it as "raw value" (correct —
it's not a predicate) but no truth fired. Root cause: the loop fact's
reduction structure for a nested add (carry + (load + 1)) — the
reduction detection likely fails to see the top-level Add(carry, ·)
pattern. This is a recognition-recall gap for map-then-sum, a common
real-world shape.

## Updated Status

```
Gate 6A-v0:  FAILED   (H0: 3/6 false positives — preserved)
Remediation: PASSED on D1
Gate 6A-v1:  FAILED   (H1: N15 certificate gap, V07 soundness)
Remediation: PASSED on D2 (mandatory verify gate, footprint certificate,
             coverage: smax, pre-headers; All-recognition added)
Gate 6A-v2:  FAILED   (H2: 1/7 false positive — overflow certificate
             gap; first unsafe candidate generation — X06)
Gate 6A-v3:  OPEN     after next remediation
Gate 6B:     RESERVED — corpus not yet created; blindness protocol
             must be established before fusion automation begins
```

## Remediation Queue (from H2, to become regression set D3)

1. **Overflow semantics in the certificate** (X02): the accumulator
   recurrence must record overflow behavior (nsw/nuw flags from LLVM;
   signed vs wrapping in SIR). No authorization for signed sums until
   the certificate carries overflow semantics.
2. **Certificate consumption in candidate generation** (X06): the
   PositionSearch/belief path generated a candidate from partial
   truths. Candidate generation must require complete certificates
   for memory-touching regions — the same closed-world invariant,
   enforced below recognition.
3. **Volatile stores** (X01): extend effect tracking to stores
   (currently a store-parsing gap masks it).
4. **Map-then-sum** (W03): recognize transformed-element sums or
   diagnose why the reduction fact was absent.
5. **Two-loop lowering** (W08, third corpus in a row — v07, w08).
6. **Atomic load handling** (X05): probe and decide (refuse loudly
   is acceptable; silent misinterpretation is not).

## Files

- `gate6a/v2_corpus.c` / `.ll` — H2 corpus (promotes to regression set D3)
- `gate6a/archive_v2.sh` — pre-run classification + hash archive
