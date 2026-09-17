# Gate 6A-v8 — fresh held-out generation (H8): PASSED

**Generation protocol:** the 28-kernel corpus
(`gate6a/v8_corpus.c/.ll`) and its classification (`v8_expected.csv`)
were frozen and hashed (`v8_manifest.sha256`) before the frozen
candidate ran. Raw outputs (`gate6a/v8_raw/run1.txt`,
`emitter_native/diff_gate6a_v8.txt`) were committed before inspection.
No remediation was required; the corpus now becomes regression evidence
(a fresh v9 would be needed for further held-out proof).

## What it tests

The generation follows the early-exit search lowering, the scan
binding extension to 512 elements (with the >64 symbolic identity path),
the sentinel guard, and the integer predicate-collection recipe:

- early-exit searches at extents **48, 64, 80, 96, 128** over u8/u16,
  including the solver path (≤64) and the symbolic path (>64);
- sentinel discipline: a sub-range search (48 accesses, 128 no-hit
  result) and a zero no-hit sentinel must be recognized but **never
  rewritten**;
- predicate discipline: an implicit non-zero search rewrites; a
  scalar-eq search must not;
- a two-loop constant-extent kernel, a runtime-extent search, a
  stride-3 post-tested do-while, and the recorded safety classes.

## Results

| Check | Result |
|---|---|
| Positives | 13/16 recognized; 3 pre-registered known gaps (p11 map-then-sum, p14 runtime-extent search, p15/p16 composition probes — one of them recognized) |
| Safety | 0 false-positive recognitions, **0 unsafe candidates**, 0 unsafe rewrites; 3/3 contained rows |
| Native differential | **22 clean / 0 mismatched / 6 lower-refused** |
| Native rewrites | **7** — p01 (48), p02 (96), p03 (u16, 80), p05 (128), p06 (two loops), p08/p09 (cardinality) — all clean 48/48 |

### Frontier detail

- **Search extents above 64 rewrite and run natively**: p02 (96), p03
  (u16, 80) and p05 (128) each apply one rewrite (symbolic scan
  identity, issued SchemaChecked) and are native-clean 48/48; p01 (48)
  uses the concrete solver path.
- **Sentinel guard holds**: p12 (sentinel 128 over extent 48) and p13
  (sentinel 0 over extent 64) recognize `FirstOccurrence` with 2
  candidates each but apply **0 rewrites**, and their lowered loops are
  native-clean — the rewrite would have changed the no-hit result.
- **Predicate discipline holds**: p04 (search for `elem == key`) is
  recognized but applies 0 rewrites; the recipe refuses to build a
  non-zero mask for a scalar-eq predicate.
- **Multi-loop + reductions**: p06 lowers, derives 15 truths across two
  regions and rewrites its count loop (native-clean 48/48).
- **Recorded limitations (fail-closed, no unsafe behavior)**: the
  runtime-extent search (p14) and the search-then-second-loop shape
  (p16) are refused at lowering (the counted successor test and the
  merge-to-continuation composition are not modeled); stride-3 lowers
  natively but derives no reduction (non-unit stride).

## Cumulative native evidence

- All corpora: **161 clean / 0 mismatched / 61 lower-refused**, 40
  native-clean rewrites.
- v6/v7 regression re-runs unchanged (p09 rewrites=1 each); v5/v3 11/12
  with 0 unsafe; Gate 6B 10/10.

## Next

- Fresh **v9** after the next capability change.
- Open gaps: runtime-extent reductions (contained by design),
  multi-loop + early-exit composition (search then another loop),
  runtime-extent search lowering, map-then-sum candidate/recipe, a Sum
  reduction strategy, and deliberate non-unit-stride abstention.
