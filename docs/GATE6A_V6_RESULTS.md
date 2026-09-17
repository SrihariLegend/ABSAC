# Gate 6A-v6 — fresh held-out generation (H6): FAILED, remediated

**Generation protocol:** the corpus (`gate6a/v6_corpus.c/.ll`,
22 kernels) and its classification (`v6_expected.csv`) were frozen and
hashed (`v6_manifest.sha256`) before the frozen candidate ran. The
one-shot result failed and is preserved; the remediation is recorded
separately. v6 is now a **regression set**; held-out proof requires a
fresh v7.

## Why this generation

Three capability changes landed 2026-09-17 that no blind corpus had yet
tested:

1. **F6–F8 emitter fixes** — the execution bridge is natively compiled
   code, not the SIR interpreter.
2. **Sequential multi-loop emission + lowering** (namespaced
   carriers/outputs; TupleExtract/FieldAccess resolve to the producing
   loop).
3. **Constant-extent buffer promotion** (pointer → `[T; K]` only when
   every access is inside one proven constant extent).

The corpus mixes constant-extent reductions (three element
widths/predicates), two sums, conjunctive/disjunctive reductions, a
sequential two-loop kernel, two pre-registered known gaps
(early-return first-set search, map-then-sum), three contained rows
(runtime-extent sum/cardinality, signed nsw), and nine safety negatives
(X06 running max, D5 masked sum over a constant extent, stride 4,
volatile store, atomic load, may-alias store, two-array relation, the
unsigned `i >= 0` reverse search, early exit with a global write).

## Run 1 — frozen one-shot: **FAIL** (preserved)

Raw outputs:

- `gate6a/v6_raw/run1.txt` — recognition/safety harness.
- `gate6a/v6_raw/run1_native_failure.txt` — native differential from
  the pre-remediation binary (rebuilt from commit `9fb1762`).

| Check | Result |
|---|---|
| Positives | 9/10 recognized, 1 known gap (early-return p09) |
| Safety | **3 unsafe candidates** (n04 running max, n05 masked sum, n09 two arrays); 0 unsafe rewrites |
| Native differential | **15 clean / 1 mismatched / 6 lower-refused**; n06 stride-4 **48/48 mismatches** |

## Findings and remediation

### 1. Candidate gate bypass (P0A) — 3 negatives produced candidates

Constant-extent promotion gave the negative regions a structural
description, so inference formed a context and a plan whose citations
were **entirely descriptive** (LogicalSequence, ElementSequence,
PredicateMap, FiniteCollection) minted a candidate even though the
region carried **no certificate at all**. The gate admitted proposals
when every cited concept was a data concept, with an empty authorized
set.

**Fix:** a region must carry at least one certificate
(`!region_auths.is_empty()`), on top of the existing rule that every
cited concept is data-or-authorized. Certified regions keep their
descriptive plans — the BS001 count loop still produces its four
distinct strategies (`sir_generation/tests/bs001_pipeline.rs`).

### 2. Post-tested do-while lowered as pre-tested — native mismatch

For a constant stride 4, clang emits an unguarded self-loop whose test
is on the **carry** with a back-edge on true (`i < 60`, body first,
stride 4). The lowerer passed the test through as a pre-tested SIR
domain, dropping the forced final iteration (index 60): 48/48 native
mismatches.

**Fix:** `post_tested_carry_domain` reconstructs the faithful
pre-tested domain for unguarded carry-compare do-whiles —
`carry < K + step` (ascending; checked overflow) or
`carry > K - step` (descending), requiring a constant step, a constant
bound, and a verified forced first iteration. Anything else is refused
loudly. Tests: `post_tested_stride_loop_reconstructs_the_forced_iteration`
(bound becomes 64) and `post_tested_loop_with_runtime_step_is_refused`.

## Run 2/3 — remediation on the frozen regression set: **PASS**

| Check | Result |
|---|---|
| Positives | 9/10 recognized (p09 early-return is the recorded gap); p10 derives `MappedSumReduction` |
| Safety | 0 false-positive recognitions, **0 unsafe candidates**, 0 unsafe rewrites; 3/3 contained rows |
| Native differential | **16 clean / 0 mismatched / 6 lower-refused**; 5 rewrites native-clean (p01/p02/p03 cardinality masks, p07 disjunctive OR, p08 whole-function two-loop count) |

Observations recorded: sums (p04/p05) recognize with 0 candidates (no
Sum strategy); p08 initially recognized with 0 candidates, and a
follow-up fix (below) closed that; p10 map-then-sum
recognizes the D5-safe concept but has no candidate; p09 is refused at
lowering (early-exit control flow); the unsigned reverse shape (n11) is
refused at lowering on this C form, complementing the totality-gate
unit tests.

### Follow-up (same day): whole-function multi-loop candidates

The sequential composer re-emitted loop1's body as straight-line code
while lowering loop2; the dead duplicate `ArrayAccess` sat outside any
loop and blocked constant-extent promotion, so p08/w08/p12 regions had
no structural description and produced 0 candidates. The composer now
tracks the blocks consumed by already-lowered loops and skips them.
Result for p08: buffer promoted to `[u8; 48]`, 15 truths, 2 regions,
**3 candidates and 1 rewrite** — the first whole-function multi-loop
rewrite, native-clean 48/48 (count loop → mask compare + popcount,
sum loop unchanged). Regression test:
`constant_two_loops_promote_and_do_not_duplicate_the_body`.
Runtime-bound two-loop kernels (w08, v5/v3/v4 p12) still promote
nothing and stay candidate-free by design.

### Follow-up 2 (same day): early-exit search lowering

p09 (`p09_first_set_const`, extent 96) now lowers: the header/latch/
merge CFG is synthesized into the canonical found-flag SIR loop, the
buffer promotes to `[u8; 96]`, `FirstOccurrence` derives, and the
native differential is clean. The 96-element extent exceeds the
64-bit solver cap, so it stays unrewritten; the ≤64 rewrite chain is
covered by the 48-element `emit_c_native` test.

## Cross-corpus regression after the fixes

- Native sweep over v2/v3/v4/v5/h3/h4/h4b/h4c/d4 unchanged:
  **103 clean / 0 mismatched / 45 lower-refused**, 22 native rewrites.
- v5 and v3: 11/12 recognized, 0 false positives, 0 unsafe candidates,
  0 unsafe rewrites, PASS; Gate 6B: 10/10.
- Workspace: **675 passed / 0 failed / 119 targets**.

## Next

- Fresh **v7** for held-out proof (v6 is now a regression set).
- Open capability gaps: runtime-extent reductions (no fabricated
  extent), whole-function multi-loop candidate generation, map-then-sum
  candidate/recipe, early-exit lowering, and a Sum reduction strategy.
