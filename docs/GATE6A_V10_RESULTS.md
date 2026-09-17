# Gate 6A-v10 — fresh held-out generation (H10): FAILED, remediated

**Generation protocol:** the 24-kernel corpus
(`gate6a/v10_corpus.c/.ll`) and its classification
(`v10_expected.csv`) were frozen and hashed (`v10_manifest.sha256`)
before the frozen candidate ran. The one-shot result failed and is
preserved (`v10_raw/run1.txt`); the remediation is recorded separately.
The corpus is now a regression set; held-out proof needs a fresh v11.

## What it tests

The runtime-search zero-trip rule was generalized by merge form
(identity vs `umin` clamp) and hardened to require a guard on the trip
bound; v10 blind-tests unseen variants:

- sentinel `n` (clamp), sentinel `-1` (identity), computed `n-1`, and
  `bool`-element searches;
- loop shapes: do-while search, descending search, search-then-loop;
- sentinel discipline on a constant extent (sentinel 99 over 32
  elements); stable cardinality/multi-loop/sum positives; the recorded
  safety classes.

## Run 1 — frozen one-shot: **FAIL** (preserved)

| Check | Result |
|---|---|
| Positives | 7/12 recognized, 1 recall miss, 4 known gaps |
| Safety | 0 false positives / 0 unsafe candidates / 0 unsafe rewrites |
| Native | 16 clean / 0 mismatched / 8 lower-refused |

The recall miss was **p04_runtime_search_bool** (`lowered=false`,
`counter phi is not latch-carried`). clang emits the hit-on-true
polarity for bool elements (`br %trunc, label %merge, label %latch`),
and the `n == 0` entry guard also branches back to the header. The
triple search matched that **entry guard as the latch**, so the header
phi looked non-latch-carried and a supported shape was refused.

## Remediation

A latch must be a non-entry block that contributes a **back-edge phi
incoming** to the header. The entry guard (block 0) is excluded and the
candidate latch's label must appear as an incoming of a header phi.
This selects the real latch for both branch polarities.

Regression test:
`hit_on_true_runtime_search_selects_the_real_latch`.

**Side effect (verified):** the fix also enables v2
`x08_early_exit_write` (native-clean). Its LLVM IR contains no store —
clang eliminated the dead global write — so the IR is a pure runtime
search; recognition/candidates/rewrites remain 0.

## Run 2 — remediation on the frozen regression set: **PASS**

| Check | Result |
|---|---|
| Positives | 8/12 recognized; 4 pre-registered known gaps (p03 computed sentinel, p05 do-while, p06 descending, p07 search-then-loop) |
| Safety | 0 false-positive recognitions, **0 unsafe candidates**, 0 unsafe rewrites; 3/3 contained rows |
| Native | **17 clean / 0 mismatched / 7 lower-refused** |
| Native rewrites | 3 (p09 constant search, p10 cardinality, p11 two-loop) |

Frontier detail: the clamp (p01) and identity (p02) runtime-search forms
both lower and recognize `FirstOccurrence`; p04 (bool, hit-on-true)
lowers after the fix; the constant search with sentinel 99 recognizes
but applies 0 rewrites (sentinel guard); the known-shape gaps stay
fail-closed.

## Cumulative native evidence

- All corpora: **205 clean / 0 mismatched / 70 lower-refused**, 57
  native-clean rewrites.
- v5 11/12, v6 9/10, v7 8/11, v8 13/16, v9 15/17, v10 8/12 — all with
  0 false positives and 0 unsafe candidates/rewrites; Gate 6B 10/10.
- Workspace: **688 passed / 0 failed / 120 targets**.

## Next

- Fresh **v11** after the next capability change.
- Open gaps: runtime-extent reductions, multi-loop + early-exit
  composition (search-then-loop), computed no-hit sentinels,
  map-then-sum candidate/recipe, Sum reduction strategy, deliberate
  non-unit-stride abstention.

### Follow-up (same day): descending search lowering

p07 (`p07_runtime_descending_search`) now lowers: the pre-decrement
successor is normalized to `Sub(counter, 1)`, the scan derives the
correct `LastOccurrence` (never `FirstOccurrence`), and the
exclusive-counter form stays unauthorized with 0 candidates. v10
native-clean rises 17 → 18 (lower-refused 7 → 6); cumulative native
evidence 225 clean / 0 mismatched / 74 lower-refused.

### Follow-up 2 (same day): peeled computed-sentinel search

p03 (the computed `n - 1` sentinel shape) now lowers via de-peeling into
one canonical found-flag loop (pure detection, then one loop over
`0 .. bound-1` plus the sentinel select). The scan derives the correct
`FirstOccurrence`, applies 0 rewrites (runtime extent) and is
native/sanitizer-clean. v10 native-clean rises 18 → 19
(lower-refused 6 → 5); cumulative 227 clean / 0 mismatched /
72 lower-refused.
