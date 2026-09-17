# Gate 6A-v11 — fresh held-out generation (H11): PASSED

**Generation protocol:** the 24-kernel corpus
(`gate6a/v11_corpus.c/.ll`) and its classification
(`v11_expected.csv`) were frozen and hashed
(`v11_manifest.sha256`) before the frozen candidate ran. Raw outputs
(`v11_raw/run1.txt`, `emitter_native/diff_gate6a_v11.txt`) were
committed before inspection. No remediation was required; the corpus is
now regression evidence (a fresh v12 would be needed for further
held-out proof).

## What it tests

The early-exit synthesis just gained merge-form matching (identity vs
`umin` clamp), a guard-on-trip-bound requirement, and latch selection
that ignores the entry guard. v11 blind-tests unseen header polarities,
predicates, sentinels and element widths:

- runtime searches for zero (`!buf[i]`), `== key` with a `-1` sentinel,
  `!= key` with an `n` sentinel, and u16 elements;
- computed `n-1` sentinel, do-while, and descending runtime searches;
- constant searches for zero and ordered (`>= key`) predicates, stable
  cardinality/multi-loop/sum positives, and the safety classes.

## Results

| Check | Result |
|---|---|
| Positives | 9/12 recognized; 3 pre-registered known gaps (p05 computed sentinel, p06 do-while, p07 descending) |
| Safety | 0 false-positive recognitions, **0 unsafe candidates**, 0 unsafe rewrites; 3/3 contained rows |
| Native differential | **18 clean / 0 mismatched / 6 lower-refused** |
| Native rewrites | 4 — p08 (zero-element search), p09 (Ge search), p10 (cardinality), p11 (two-loop) — all 48/48 clean |

### Frontier detail

- **Unseen polarities/predicates hold**: p01 (`!buf[i]`), p02
  (`== key`, `-1` identity sentinel), p03 (`!= key`, `n` sentinel) and
  p04 (u16) all lower, verify and derive `FirstOccurrence` with 0
  rewrites (runtime extent — no fabricated mask).
- Constant zero-element and ordered searches rewrite to the exact mask +
  ctz and are native-clean, extending the predicate coverage blind.
- The recorded shape limits behave fail-closed: the computed `n-1`
  sentinel and the descending search are refused at lowering; the
  do-while search lowers but derives no `FirstOccurrence` (recorded
  gap).

## Cumulative native evidence

- All corpora: **223 clean / 0 mismatched / 76 lower-refused**, 61
  native-clean rewrites.
- Harnesses v5–v11: 11/12, 9/10, 8/11, 13/16, 15/17, 8/12, 9/12 — all
  with 0 false positives and 0 unsafe candidates/rewrites; Gate 6B
  10/10.

## Next

- Fresh **v12** after the next capability change.
- Open gaps: runtime-extent reductions, multi-loop + early-exit
  composition (search-then-loop), do-while and descending runtime
  searches, computed no-hit sentinels, map-then-sum candidate/recipe,
  Sum reduction strategy, deliberate non-unit-stride abstention.
